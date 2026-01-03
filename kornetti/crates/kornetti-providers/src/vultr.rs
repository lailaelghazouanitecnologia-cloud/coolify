//! Vultr Cloud Provider
//!
//! API Documentation: https://www.vultr.com/api/

use async_trait::async_trait;
use kornetti_core::{
    Result, Error,
    traits::{
        CloudProvider, Region, ServerSize, OsImage,
        CreateServerConfig, ProvisionedServer, ProvisionedServerStatus,
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

const VULTR_API_BASE: &str = "https://api.vultr.com/v2";

pub struct VultrProvider {
    client: Client,
    api_key: String,
}

impl VultrProvider {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("kornetti/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", VULTR_API_BASE, path);
        debug!(url = %url, "Vultr API GET request");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Vultr API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Vultr API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Vultr response: {}", e)))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", VULTR_API_BASE, path);
        debug!(url = %url, "Vultr API POST request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Vultr API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Vultr API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Vultr response: {}", e)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", VULTR_API_BASE, path);
        debug!(url = %url, "Vultr API DELETE request");

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Vultr API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Vultr API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl CloudProvider for VultrProvider {
    fn name(&self) -> &'static str {
        "vultr"
    }

    #[instrument(skip(self))]
    async fn list_regions(&self) -> Result<Vec<Region>> {
        #[derive(Deserialize)]
        struct RegionsResponse {
            regions: Vec<VultrRegion>,
        }

        #[derive(Deserialize)]
        struct VultrRegion {
            id: String,
            city: String,
            country: String,
            continent: String,
            options: Vec<String>,
        }

        let response: RegionsResponse = self.get("/regions").await?;

        Ok(response
            .regions
            .into_iter()
            .map(|r| Region {
                id: r.id,
                name: format!("{}, {}", r.city, r.country),
                country: Some(r.country),
                available: r.options.contains(&"bare_metal".to_string())
                    || r.options.contains(&"compute".to_string()),
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_sizes(&self, region: &str) -> Result<Vec<ServerSize>> {
        #[derive(Deserialize)]
        struct PlansResponse {
            plans: Vec<VultrPlan>,
        }

        #[derive(Deserialize)]
        struct VultrPlan {
            id: String,
            vcpu_count: u32,
            ram: u32,
            disk: u32,
            bandwidth: u32,
            monthly_cost: f64,
            #[serde(rename = "type")]
            plan_type: String,
            locations: Vec<String>,
        }

        let response: PlansResponse = self.get("/plans").await?;

        Ok(response
            .plans
            .into_iter()
            .filter(|p| p.locations.contains(&region.to_string()))
            .map(|p| ServerSize {
                id: p.id,
                name: format!("{} vCPU, {} MB RAM", p.vcpu_count, p.ram),
                vcpus: p.vcpu_count,
                memory_mb: p.ram,
                disk_gb: p.disk,
                bandwidth_tb: Some(p.bandwidth as f64 / 1000.0),
                price_monthly: p.monthly_cost,
                price_hourly: Some(p.monthly_cost / 730.0),
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        #[derive(Deserialize)]
        struct OsResponse {
            os: Vec<VultrOs>,
        }

        #[derive(Deserialize)]
        struct VultrOs {
            id: u32,
            name: String,
            arch: String,
            family: String,
        }

        let response: OsResponse = self.get("/os").await?;

        Ok(response
            .os
            .into_iter()
            .filter(|o| o.family == "ubuntu" || o.family == "debian" || o.family == "centos")
            .map(|o| OsImage {
                id: o.id.to_string(),
                name: o.name.clone(),
                distribution: o.family,
                version: o.name,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer> {
        #[derive(Serialize)]
        struct CreateInstanceRequest {
            region: String,
            plan: String,
            os_id: u32,
            label: String,
            sshkey_id: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_data: Option<String>,
            tags: Vec<String>,
        }

        #[derive(Deserialize)]
        struct CreateInstanceResponse {
            instance: VultrInstance,
        }

        #[derive(Deserialize)]
        struct VultrInstance {
            id: String,
            label: String,
            main_ip: String,
            v6_main_ip: String,
            status: String,
            region: String,
            plan: String,
        }

        let os_id: u32 = config
            .image
            .parse()
            .map_err(|_| Error::Validation("Invalid OS ID".to_string()))?;

        let request = CreateInstanceRequest {
            region: config.region.clone(),
            plan: config.size.clone(),
            os_id,
            label: config.name.clone(),
            sshkey_id: config.ssh_key_ids,
            user_data: config.user_data.map(|d| base64::encode(&d)),
            tags: config.tags,
        };

        let response: CreateInstanceResponse = self.post("/instances", &request).await?;
        info!(
            instance_id = %response.instance.id,
            "Created Vultr instance"
        );

        Ok(ProvisionedServer {
            provider_id: response.instance.id,
            name: response.instance.label,
            ip_address: if response.instance.main_ip.is_empty() {
                None
            } else {
                Some(response.instance.main_ip)
            },
            ipv6_address: if response.instance.v6_main_ip.is_empty() {
                None
            } else {
                Some(response.instance.v6_main_ip)
            },
            status: parse_vultr_status(&response.instance.status),
            region: response.instance.region,
            size: response.instance.plan,
        })
    }

    #[instrument(skip(self))]
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer> {
        #[derive(Deserialize)]
        struct InstanceResponse {
            instance: VultrInstance,
        }

        #[derive(Deserialize)]
        struct VultrInstance {
            id: String,
            label: String,
            main_ip: String,
            v6_main_ip: String,
            status: String,
            region: String,
            plan: String,
        }

        let response: InstanceResponse = self.get(&format!("/instances/{}", provider_id)).await?;

        Ok(ProvisionedServer {
            provider_id: response.instance.id,
            name: response.instance.label,
            ip_address: if response.instance.main_ip.is_empty() || response.instance.main_ip == "0.0.0.0" {
                None
            } else {
                Some(response.instance.main_ip)
            },
            ipv6_address: if response.instance.v6_main_ip.is_empty() {
                None
            } else {
                Some(response.instance.v6_main_ip)
            },
            status: parse_vultr_status(&response.instance.status),
            region: response.instance.region,
            size: response.instance.plan,
        })
    }

    #[instrument(skip(self))]
    async fn delete_server(&self, provider_id: &str) -> Result<()> {
        self.delete(&format!("/instances/{}", provider_id)).await?;
        info!(instance_id = %provider_id, "Deleted Vultr instance");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reboot_server(&self, provider_id: &str) -> Result<()> {
        self.post::<serde_json::Value, _>(
            &format!("/instances/{}/reboot", provider_id),
            &serde_json::json!({}),
        )
        .await?;
        info!(instance_id = %provider_id, "Rebooted Vultr instance");
        Ok(())
    }
}

fn parse_vultr_status(status: &str) -> ProvisionedServerStatus {
    match status {
        "active" => ProvisionedServerStatus::Running,
        "pending" | "installing" => ProvisionedServerStatus::Pending,
        "stopped" | "suspended" => ProvisionedServerStatus::Stopped,
        _ => ProvisionedServerStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vultr_status() {
        assert_eq!(parse_vultr_status("active"), ProvisionedServerStatus::Running);
        assert_eq!(parse_vultr_status("pending"), ProvisionedServerStatus::Pending);
        assert_eq!(parse_vultr_status("stopped"), ProvisionedServerStatus::Stopped);
        assert_eq!(parse_vultr_status("unknown"), ProvisionedServerStatus::Error);
    }
}
