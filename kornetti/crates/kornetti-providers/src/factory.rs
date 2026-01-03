//! Provider Factory
//!
//! Creates cloud provider instances based on configuration

use std::sync::Arc;

use kornetti_core::{
    Result, Error,
    config::ProvidersConfig,
    models::CloudProvider as CloudProviderEnum,
    traits::CloudProvider,
};

use crate::{
    VultrProvider, HetznerProvider, DigitalOceanProvider,
    AwsProvider, LinodeProvider, DynProvider,
};

/// Factory for creating cloud provider instances
pub struct ProviderFactory {
    config: ProvidersConfig,
}

impl ProviderFactory {
    pub fn new(config: ProvidersConfig) -> Self {
        Self { config }
    }

    /// Get a provider by enum type
    pub async fn get_provider(&self, provider: CloudProviderEnum) -> Result<DynProvider> {
        match provider {
            CloudProviderEnum::Vultr => self.vultr(),
            CloudProviderEnum::Hetzner => self.hetzner(),
            CloudProviderEnum::DigitalOcean => self.digitalocean(),
            CloudProviderEnum::Aws => self.aws().await,
            CloudProviderEnum::Linode => self.linode(),
            CloudProviderEnum::Gcp => Err(Error::Provider("GCP not yet implemented".to_string())),
            CloudProviderEnum::Azure => Err(Error::Provider("Azure not yet implemented".to_string())),
            CloudProviderEnum::Custom => Err(Error::Provider("Custom provider requires manual configuration".to_string())),
        }
    }

    /// Get Vultr provider
    pub fn vultr(&self) -> Result<DynProvider> {
        let config = self
            .config
            .vultr
            .as_ref()
            .ok_or_else(|| Error::Config("Vultr configuration not found".to_string()))?;

        Ok(Arc::new(VultrProvider::new(config.api_key.clone())))
    }

    /// Get Hetzner provider
    pub fn hetzner(&self) -> Result<DynProvider> {
        let config = self
            .config
            .hetzner
            .as_ref()
            .ok_or_else(|| Error::Config("Hetzner configuration not found".to_string()))?;

        Ok(Arc::new(HetznerProvider::new(config.api_token.clone())))
    }

    /// Get DigitalOcean provider
    pub fn digitalocean(&self) -> Result<DynProvider> {
        let config = self
            .config
            .digitalocean
            .as_ref()
            .ok_or_else(|| Error::Config("DigitalOcean configuration not found".to_string()))?;

        Ok(Arc::new(DigitalOceanProvider::new(config.api_token.clone())))
    }

    /// Get AWS provider
    pub async fn aws(&self) -> Result<DynProvider> {
        let config = self
            .config
            .aws
            .as_ref()
            .ok_or_else(|| Error::Config("AWS configuration not found".to_string()))?;

        Ok(Arc::new(
            AwsProvider::new(
                config.access_key_id.clone(),
                config.secret_access_key.clone(),
                config.region.clone(),
            )
            .await,
        ))
    }

    /// Get Linode provider
    pub fn linode(&self) -> Result<DynProvider> {
        let config = self
            .config
            .linode
            .as_ref()
            .ok_or_else(|| Error::Config("Linode configuration not found".to_string()))?;

        Ok(Arc::new(LinodeProvider::new(config.api_token.clone())))
    }

    /// List all configured providers
    pub fn configured_providers(&self) -> Vec<&'static str> {
        let mut providers = Vec::new();

        if self.config.vultr.is_some() {
            providers.push("vultr");
        }
        if self.config.hetzner.is_some() {
            providers.push("hetzner");
        }
        if self.config.digitalocean.is_some() {
            providers.push("digitalocean");
        }
        if self.config.aws.is_some() {
            providers.push("aws");
        }
        if self.config.linode.is_some() {
            providers.push("linode");
        }

        providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kornetti_core::config::{VultrConfig, HetznerConfig};

    #[test]
    fn test_configured_providers() {
        let config = ProvidersConfig {
            vultr: Some(VultrConfig {
                api_key: "test".to_string(),
            }),
            hetzner: Some(HetznerConfig {
                api_token: "test".to_string(),
            }),
            digitalocean: None,
            aws: None,
            linode: None,
        };

        let factory = ProviderFactory::new(config);
        let providers = factory.configured_providers();

        assert!(providers.contains(&"vultr"));
        assert!(providers.contains(&"hetzner"));
        assert!(!providers.contains(&"digitalocean"));
    }
}
