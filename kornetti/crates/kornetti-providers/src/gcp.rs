//! Google Cloud Platform Provider
//!
//! Placeholder for GCP Compute Engine integration

use async_trait::async_trait;
use kornetti_core::{
    Result, Error,
    traits::{
        CloudProvider, Region, ServerSize, OsImage,
        CreateServerConfig, ProvisionedServer,
    },
};

pub struct GcpProvider {
    _project_id: String,
    _credentials_json: String,
}

impl GcpProvider {
    pub fn new(project_id: String, credentials_json: String) -> Self {
        Self {
            _project_id: project_id,
            _credentials_json: credentials_json,
        }
    }
}

#[async_trait]
impl CloudProvider for GcpProvider {
    fn name(&self) -> &'static str {
        "gcp"
    }

    async fn list_regions(&self) -> Result<Vec<Region>> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn create_server(&self, _config: CreateServerConfig) -> Result<ProvisionedServer> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn get_server(&self, _provider_id: &str) -> Result<ProvisionedServer> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn delete_server(&self, _provider_id: &str) -> Result<()> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }

    async fn reboot_server(&self, _provider_id: &str) -> Result<()> {
        Err(Error::Provider("GCP provider not yet implemented".to_string()))
    }
}
