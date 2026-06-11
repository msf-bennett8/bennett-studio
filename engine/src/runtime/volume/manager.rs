use bollard::volume::{CreateVolumeOptions, ListVolumesOptions, RemoveVolumeOptions};
use tracing::info;

use crate::runtime::container::docker::DockerError;

pub struct VolumeManager {
    client: bollard::Docker,
}

impl VolumeManager {
    pub fn new() -> Result<Self, DockerError> {
        let client = bollard::Docker::connect_with_local_defaults()
            .map_err(|e| DockerError::DaemonNotReachable(e.to_string()))?;
        Ok(Self { client })
    }

    pub async fn create(&self, name: &str) -> Result<(), DockerError> {
        let mut labels = std::collections::HashMap::new();
        labels.insert("bennett-managed".to_string(), "true".to_string());

        let options = CreateVolumeOptions {
            name: name.to_string(),
            driver: "local".to_string(),
            labels,
            ..Default::default()
        };

        self.client
            .create_volume(options)
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;

        info!("Created volume: {}", name);
        Ok(())
    }

    pub async fn remove(&self, name: &str) -> Result<(), DockerError> {
        let options = RemoveVolumeOptions {
            force: true,
            ..Default::default()
        };

        self.client
            .remove_volume(name, Some(options))
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;

        info!("Removed volume: {}", name);
        Ok(())
    }

    pub async fn exists(&self, name: &str) -> Result<bool, DockerError> {
        let mut filters = std::collections::HashMap::new();
        filters.insert("name".to_string(), vec![name.to_string()]);

        let options = ListVolumesOptions {
            filters,
            ..Default::default()
        };

        let volumes = self
            .client
            .list_volumes(Some(options))
            .await
            .map_err(|e| DockerError::ContainerError(e.to_string()))?;

        Ok(volumes
            .volumes
            .unwrap_or_default()
            .into_iter()
            .any(|v| v.name == name))
    }

    pub fn generate_name(db_type: &str, name: &str) -> String {
        format!("bennett-{}-{}-data", db_type, name)
    }
}
