//! AWS EC2 Provider
//!
//! Uses the official AWS SDK for Rust

use async_trait::async_trait;
use kornetti_core::{
    Result, Error,
    traits::{
        CloudProvider, Region, ServerSize, OsImage,
        CreateServerConfig, ProvisionedServer, ProvisionedServerStatus,
    },
};
use tracing::{info, instrument};

pub struct AwsProvider {
    config: aws_config::SdkConfig,
    region: String,
}

impl AwsProvider {
    pub async fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        let credentials = aws_sdk_ec2::config::Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "kornetti",
        );

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(aws_sdk_ec2::config::Region::new(region.clone()))
            .load()
            .await;

        Self { config, region }
    }

    fn ec2_client(&self) -> aws_sdk_ec2::Client {
        aws_sdk_ec2::Client::new(&self.config)
    }
}

#[async_trait]
impl CloudProvider for AwsProvider {
    fn name(&self) -> &'static str {
        "aws"
    }

    #[instrument(skip(self))]
    async fn list_regions(&self) -> Result<Vec<Region>> {
        let client = self.ec2_client();

        let result = client
            .describe_regions()
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        Ok(result
            .regions()
            .iter()
            .map(|r| Region {
                id: r.region_name().unwrap_or_default().to_string(),
                name: r.region_name().unwrap_or_default().to_string(),
                country: None,
                available: true,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        let client = self.ec2_client();

        let result = client
            .describe_instance_types()
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        Ok(result
            .instance_types()
            .iter()
            .filter_map(|t| {
                let name = t.instance_type()?.as_str().to_string();
                let vcpus = t.v_cpu_info()?.default_v_cpus()? as u32;
                let memory_mb = t.memory_info()?.size_in_mib()? as u32;

                Some(ServerSize {
                    id: name.clone(),
                    name,
                    vcpus,
                    memory_mb,
                    disk_gb: 0, // EBS-backed, varies
                    bandwidth_tb: None,
                    price_monthly: 0.0, // Would need pricing API
                    price_hourly: None,
                })
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        let client = self.ec2_client();

        let result = client
            .describe_images()
            .owners("099720109477") // Canonical (Ubuntu)
            .filters(
                aws_sdk_ec2::types::Filter::builder()
                    .name("name")
                    .values("ubuntu/images/hvm-ssd/ubuntu-*-amd64-server-*")
                    .build(),
            )
            .filters(
                aws_sdk_ec2::types::Filter::builder()
                    .name("state")
                    .values("available")
                    .build(),
            )
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        Ok(result
            .images()
            .iter()
            .filter_map(|i| {
                Some(OsImage {
                    id: i.image_id()?.to_string(),
                    name: i.name()?.to_string(),
                    distribution: "ubuntu".to_string(),
                    version: i.name()?.to_string(),
                })
            })
            .take(20) // Limit results
            .collect())
    }

    #[instrument(skip(self))]
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer> {
        let client = self.ec2_client();

        let run_result = client
            .run_instances()
            .image_id(&config.image)
            .instance_type(
                config
                    .size
                    .parse()
                    .map_err(|_| Error::Validation("Invalid instance type".to_string()))?,
            )
            .min_count(1)
            .max_count(1)
            .set_key_name(config.ssh_key_ids.first().cloned())
            .set_user_data(config.user_data)
            .tag_specifications(
                aws_sdk_ec2::types::TagSpecification::builder()
                    .resource_type(aws_sdk_ec2::types::ResourceType::Instance)
                    .tags(
                        aws_sdk_ec2::types::Tag::builder()
                            .key("Name")
                            .value(&config.name)
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        let instance = run_result
            .instances()
            .first()
            .ok_or_else(|| Error::Provider("No instance created".to_string()))?;

        let instance_id = instance
            .instance_id()
            .ok_or_else(|| Error::Provider("No instance ID".to_string()))?;

        info!(instance_id = %instance_id, "Created AWS EC2 instance");

        Ok(ProvisionedServer {
            provider_id: instance_id.to_string(),
            name: config.name,
            ip_address: instance.public_ip_address().map(|s| s.to_string()),
            ipv6_address: None,
            status: parse_aws_status(instance.state()),
            region: self.region.clone(),
            size: config.size,
        })
    }

    #[instrument(skip(self))]
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer> {
        let client = self.ec2_client();

        let result = client
            .describe_instances()
            .instance_ids(provider_id)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        let instance = result
            .reservations()
            .first()
            .and_then(|r| r.instances().first())
            .ok_or_else(|| Error::NotFound(format!("Instance {} not found", provider_id)))?;

        let name = instance
            .tags()
            .iter()
            .find(|t| t.key() == Some("Name"))
            .and_then(|t| t.value())
            .unwrap_or("unnamed")
            .to_string();

        Ok(ProvisionedServer {
            provider_id: provider_id.to_string(),
            name,
            ip_address: instance.public_ip_address().map(|s| s.to_string()),
            ipv6_address: None,
            status: parse_aws_status(instance.state()),
            region: self.region.clone(),
            size: instance
                .instance_type()
                .map(|t| t.as_str().to_string())
                .unwrap_or_default(),
        })
    }

    #[instrument(skip(self))]
    async fn delete_server(&self, provider_id: &str) -> Result<()> {
        let client = self.ec2_client();

        client
            .terminate_instances()
            .instance_ids(provider_id)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        info!(instance_id = %provider_id, "Terminated AWS EC2 instance");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reboot_server(&self, provider_id: &str) -> Result<()> {
        let client = self.ec2_client();

        client
            .reboot_instances()
            .instance_ids(provider_id)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("AWS API error: {}", e)))?;

        info!(instance_id = %provider_id, "Rebooted AWS EC2 instance");
        Ok(())
    }
}

fn parse_aws_status(state: Option<&aws_sdk_ec2::types::InstanceState>) -> ProvisionedServerStatus {
    match state.and_then(|s| s.name()) {
        Some(aws_sdk_ec2::types::InstanceStateName::Running) => ProvisionedServerStatus::Running,
        Some(aws_sdk_ec2::types::InstanceStateName::Pending) => ProvisionedServerStatus::Pending,
        Some(aws_sdk_ec2::types::InstanceStateName::Stopped) => ProvisionedServerStatus::Stopped,
        Some(aws_sdk_ec2::types::InstanceStateName::Stopping) => ProvisionedServerStatus::Stopped,
        _ => ProvisionedServerStatus::Error,
    }
}
