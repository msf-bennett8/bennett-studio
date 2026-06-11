use crate::models::database::{DatabaseInstance, DatabaseStatus};
use crate::runtime::container::docker::DockerRuntime;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, warn};

pub struct ProcessSupervisor {
    docker: DockerRuntime,
}

impl ProcessSupervisor {
    pub fn new() -> Result<Self, crate::runtime::container::docker::DockerError> {
        Ok(Self {
            docker: DockerRuntime::new()?,
        })
    }

    pub async fn start_monitoring(
        &self,
        instances: std::sync::Arc<std::sync::Mutex<Vec<DatabaseInstance>>>,
    ) {
        let mut ticker = interval(Duration::from_secs(5));

        loop {
            ticker.tick().await;

            let mut db_list = instances.lock().unwrap();
            for instance in db_list.iter_mut() {
                if instance.status != DatabaseStatus::Running {
                    continue;
                }

                if let Some(container_id) = &instance.container_id {
                    match self.docker.is_running(container_id).await {
                        Ok(true) => {
                            // Healthy
                        }
                        Ok(false) => {
                            warn!(
                                "Container {} for {} is not running, marking as stopped",
                                container_id, instance.name
                            );
                            instance.status = DatabaseStatus::Stopped;
                        }
                        Err(e) => {
                            error!("Health check failed for {}: {}", instance.name, e);
                            instance.status = DatabaseStatus::Error;
                        }
                    }
                }
            }
        }
    }

    pub async fn get_logs(&self, container_id: &str) -> Result<String, crate::runtime::container::docker::DockerError> {
        self.docker.get_logs(container_id).await
    }
}
