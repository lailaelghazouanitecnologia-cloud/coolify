//! Kornetti Providers
//!
//! Cloud provider integrations for server provisioning and management.
//!
//! Supported providers:
//! - Vultr
//! - Hetzner Cloud
//! - DigitalOcean
//! - AWS EC2
//! - Linode
//! - Google Cloud Platform (GCP)
//! - Microsoft Azure

pub mod vultr;
pub mod hetzner;
pub mod digitalocean;
pub mod aws;
pub mod linode;
pub mod gcp;
pub mod azure;
pub mod factory;

pub use vultr::VultrProvider;
pub use hetzner::HetznerProvider;
pub use digitalocean::DigitalOceanProvider;
pub use aws::AwsProvider;
pub use linode::LinodeProvider;
pub use factory::ProviderFactory;

use kornetti_core::traits::CloudProvider;
use std::sync::Arc;

/// Dynamic provider type for runtime selection
pub type DynProvider = Arc<dyn CloudProvider>;
