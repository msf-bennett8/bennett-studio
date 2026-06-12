use bollard::{
    container::{
        Config, CreateContainerOptions, ListContainersOptions,
    },
    image::CreateImageOptions,
    models::{HostConfig, PortBinding},
    Docker,
};
use futures_util::stream::TryStreamExt;
use std::collections::HashMap;
use tracing::info;

use crate::models::database::DatabaseInstance;

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Docker daemon not reachable: {0}")]
    DaemonNotReachable(String),
    #[error("Container operation failed: {0}")]
    ContainerError(String),
    #[error("Image pull failed: {0}")]
    ImageError(String),
    #[error("Port already in use: {0}")]
    PortConflict(u16),
}

pub struct DockerRuntime {
    client: Docker,
}

impl DockerRuntime {
    pub fn new() -> Result<Self, DockerError> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| DockerError::DaemonNotReachable(e.to_string()))?;
        Ok(Self { client })
    }

    pub async fn verify(&self) -> Result<(), DockerError> {
        self.client
            .ping()
            .await
            .map_err(|e| DockerError::DaemonNotReachable(e.to_string()))?;
        info!("Docker daemon verified");
        Ok(())
    }

    pub async fn pull_image(&self, image: &str) -> Result<(), DockerError> {
        info!("Pulling image: {}", image);
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(options), None, None);
        while let Some(result) = stream.try_next().await.ok().flatten() {
            if let Some(status) = result.status {
                info!("Pull progress: {}", status);
            }
        }

        info!("Image ready: {}", image);
        Ok(())
    }

    pub async fn create_container(
        &self,
        instance: &DatabaseInstance,
    ) -> Result<String, DockerError> {
        let image = self.resolve_image(&instance.db_type, &instance.version);
        let container_name = format!("bennett-{}-{}", instance.db_type, instance.name);

        // Pull image first
        self.pull_image(&image).await?;

        // Port bindings
        let mut port_bindings = HashMap::new();
        let port_str = format!("{}/tcp", self.default_port(&instance.db_type));
        let host_binding = PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(instance.port.to_string()),
        };
        port_bindings.insert(port_str.clone(), Some(vec![host_binding]));

        // Environment variables
        let env = self.build_env(&instance.db_type, &instance.env_vars);

        // Volume mount
        let binds = instance
            .volume_name
            .as_ref()
            .map(|v| vec![format!("{}:/var/lib/{}", v, self.data_dir(&instance.db_type))]);

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            binds,
            auto_remove: Some(false),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                maximum_retry_count: Some(0),
            }),
            ..Default::default()
        };

        let config = Config {
            image: Some(image),
            env: Some(env),
            host_config: Some(host_config),
            labels: Some({
                let mut labels = HashMap::new();
                labels.insert("bennett-managed".to_string(), "true".to_string());
                labels.insert("bennett-id".to_string(), instance.id.clone());
                labels.insert("bennett-name".to_string(), instance.name.clone());
                labels.insert("bennett-type".to_string(), instance.db_type.clone());
                labels
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name,
            ..Default::default()
        };

        let result = self
            .client
            .create_container(Some(options), config)
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;

        info!(
            "Created container {} for database {} (id: {})",
            result.id, instance.name, instance.id
        );

        Ok(result.id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<(), DockerError> {
        self.client
            .start_container::<String>(container_id, None)
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;
        info!("Started container {}", container_id);
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<(), DockerError> {
        self.client
            .stop_container(container_id, Some(bollard::container::StopContainerOptions { t: 30 }))
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;
        info!("Stopped container {}", container_id);
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<(), DockerError> {
        self.client
            .remove_container(
                container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;
        info!("Removed container {}", container_id);
        Ok(())
    }

    pub async fn inspect_container(
        &self,
        container_id: &str,
    ) -> Result<bollard::models::ContainerInspectResponse, DockerError> {
        self.client
            .inspect_container(container_id, None)
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))
    }

    pub async fn is_running(&self, container_id: &str) -> Result<bool, DockerError> {
        match self.inspect_container(container_id).await {
            Ok(inspect) => Ok(inspect.state.map(|s| s.running.unwrap_or(false)).unwrap_or(false)),
            Err(_) => Ok(false),
        }
    }

    pub async fn get_logs(&self, container_id: &str) -> Result<String, DockerError> {
        let options = bollard::container::LogsOptions {
            stdout: true,
            stderr: true,
            tail: "100",
            ..Default::default()
        };
        let mut stream = self.client.logs(container_id, Some(options));
        let mut logs = String::new();
        while let Some(result) = stream.try_next().await.ok().flatten() {
            logs.push_str(&result.to_string());
        }
        Ok(logs)
    }

    pub async fn list_bennett_containers(&self) -> Result<Vec<DatabaseInstance>, DockerError> {
        use crate::models::database::{DatabaseInstance, DatabaseStatus};

        let options = ListContainersOptions {
            all: true,
            filters: {
                let mut f = HashMap::new();
                f.insert("label".to_string(), vec!["bennett-managed=true".to_string()]);
                f
            },
            ..Default::default()
        };

        let containers = self
            .client
            .list_containers(Some(options))
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;

        let mut instances = Vec::new();

        for container in containers {
            let labels = container.labels.unwrap_or_default();
            
            // Stable ID from label (persists across restarts)
            let id = labels
                .get("bennett-id")
                .cloned()
                .or_else(|| container.id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let name = labels
                .get("bennett-name")
                .cloned()
                .unwrap_or_else(|| {
                    container.names.as_ref()
                        .and_then(|n| n.first())
                        .map(|n| n.trim_start_matches('/').to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                });

            let db_type = labels
                .get("bennett-type")
                .cloned()
                .unwrap_or_else(|| "postgres".to_string());

            // Extract host port from port bindings
            let port = container
                .ports
                .as_ref()
                .and_then(|ports| {
                    ports.iter().find_map(|p| {
                        p.public_port // This is the host port
                    })
                })
                .unwrap_or(0);

            // Determine status from container state
            let status = container
                .state
                .as_deref()
                .map(|s| {
                    if s == "running" {
                        DatabaseStatus::Running
                    } else {
                        DatabaseStatus::Stopped
                    }
                })
                .unwrap_or(DatabaseStatus::Stopped);

            // Extract version from image tag
            let image = container.image.unwrap_or_default();
            let version = image
                .split(':')
                .nth(1)
                .unwrap_or("latest")
                .to_string();

            instances.push(DatabaseInstance {
                id,
                name,
                db_type,
                version,
                status,
                port,
                size: "Unknown".to_string(),
                created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
                container_id: container.id,
                volume_name: None,
                env_vars: Vec::new(),
                source: crate::models::database::DatabaseSource::Bennett,
            });
        }

        Ok(instances)
    }

    fn resolve_image(&self, db_type: &str, version: &str) -> String {
        match db_type {
            "postgres" | "postgresql" => format!("postgres:{}-alpine", version),
            "mysql" => format!("mysql:{}", version),
            "mariadb" => format!("mariadb:{}", version),
            "redis" => format!("redis:{}-alpine", version),
            "mongo" | "mongodb" => format!("mongo:{}", version),
            _ => format!("{}:{}", db_type, version),
        }
    }

    fn default_port(&self, db_type: &str) -> u16 {
        match db_type {
            "postgres" | "postgresql" => 5432,
            "mysql" | "mariadb" => 3306,
            "redis" => 6379,
            "mongo" | "mongodb" => 27017,
            _ => 5432,
        }
    }

    fn data_dir(&self, db_type: &str) -> &'static str {
        match db_type {
            "postgres" | "postgresql" => "postgresql/data",
            "mysql" | "mariadb" => "mysql",
            "redis" => "redis",
            "mongo" | "mongodb" => "data/db",
            _ => "data",
        }
    }

    fn build_env(&self, db_type: &str, extra: &[(String, String)]) -> Vec<String> {
        let mut env = match db_type {
            "postgres" | "postgresql" => vec![
                "POSTGRES_USER=bennett".to_string(),
                "POSTGRES_PASSWORD=bennett_secret".to_string(),
                "POSTGRES_DB=bennett".to_string(),
            ],
            "mysql" => vec![
                "MYSQL_ROOT_PASSWORD=bennett_root_secret".to_string(),
                "MYSQL_DATABASE=bennett".to_string(),
                "MYSQL_USER=bennett".to_string(),
                "MYSQL_PASSWORD=bennett_secret".to_string(),
            ],
            "mariadb" => vec![
                "MARIADB_ROOT_PASSWORD=bennett_root_secret".to_string(),
                "MARIADB_DATABASE=bennett".to_string(),
                "MARIADB_USER=bennett".to_string(),
                "MARIADB_PASSWORD=bennett_secret".to_string(),
            ],
            "redis" => vec![],
            "mongo" | "mongodb" => vec![
                "MONGO_INITDB_ROOT_USERNAME=bennett".to_string(),
                "MONGO_INITDB_ROOT_PASSWORD=bennett_secret".to_string(),
            ],
            _ => vec![],
        };

        for (k, v) in extra {
            env.push(format!("{}={}", k, v));
        }

        env
    }
}
