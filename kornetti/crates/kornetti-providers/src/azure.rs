//! Microsoft Azure Provider
//!
//! Placeholder for Azure Virtual Machines integration

use async_trait::async_trait;
use kornetti_core::{
    Result, Error,
    traits::{
        CloudProvider, Region, ServerSize, OsImage,
        CreateServerConfig, ProvisionedServer,
    },
};

pub struct AzureProvider {
    _subscription_id: String,
    _tenant_id: String,
    _client_id: String,
    _client_secret: String,
}

impl AzureProvider {
    pub fn new(
        subscription_id: String,
        tenant_id: String,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            _subscription_id: subscription_id,
            _tenant_id: tenant_id,
            _client_id: client_id,
            _client_secret: client_secret,
        }
    }
}

#[async_trait]
impl CloudProvider for AzureProvider {
    fn name(&self) -> &'static str {
        "azure"
    }

    async fn list_regions(&self) -> Result<Vec<Region>> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn create_server(&self, _config: CreateServerConfig) -> Result<ProvisionedServer> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn get_server(&self, _provider_id: &str) -> Result<ProvisionedServer> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn delete_server(&self, _provider_id: &str) -> Result<()> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }

    async fn reboot_server(&self, _provider_id: &str) -> Result<()> {
        Err(Error::Provider("Azure provider not yet implemented".to_string()))
    }
}
