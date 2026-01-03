//! Linode (Akamai) Cloud Provider
//!
//! API Documentation: https://www.linode.com/docs/api/

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

const LINODE_API_BASE: &str = "https://api.linode.com/v4";

pub struct LinodeProvider {
    client: Client,
    api_token: String,
}

impl LinodeProvider {
    pub fn new(api_token: String) -> Self {
        let client = Client::builder()
            .user_agent("kornetti/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_token }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_token)
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", LINODE_API_BASE, path);
        debug!(url = %url, "Linode API GET request");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Linode API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Linode API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Linode response: {}", e)))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", LINODE_API_BASE, path);
        debug!(url = %url, "Linode API POST request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Linode API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Linode API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Linode response: {}", e)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", LINODE_API_BASE, path);
        debug!(url = %url, "Linode API DELETE request");

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Linode API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Linode API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl CloudProvider for LinodeProvider {
    fn name(&self) -> &'static str {
        "linode"
    }

    #[instrument(skip(self))]
    async fn list_regions(&self) -> Result<Vec<Region>> {
        #[derive(Deserialize)]
        struct RegionsResponse {
            data: Vec<LinodeRegion>,
        }

        #[derive(Deserialize)]
        struct LinodeRegion {
            id: String,
            label: String,
            country: String,
            status: String,
        }

        let response: RegionsResponse = self.get("/regions").await?;

        Ok(response
            .data
            .into_iter()
            .map(|r| Region {
                id: r.id,
                name: r.label,
                country: Some(r.country),
                available: r.status == "ok",
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        #[derive(Deserialize)]
        struct TypesResponse {
            data: Vec<LinodeType>,
        }

        #[derive(Deserialize)]
        struct LinodeType {
            id: String,
            label: String,
            vcpus: u32,
            memory: u32,
            disk: u32,
            transfer: u32,
            price: LinodePrice,
        }

        #[derive(Deserialize)]
        struct LinodePrice {
            monthly: f64,
            hourly: f64,
        }

        let response: TypesResponse = self.get("/linode/types").await?;

        Ok(response
            .data
            .into_iter()
            .map(|t| ServerSize {
                id: t.id,
                name: t.label,
                vcpus: t.vcpus,
                memory_mb: t.memory,
                disk_gb: t.disk / 1024, // MB to GB
                bandwidth_tb: Some(t.transfer as f64 / 1000.0),
                price_monthly: t.price.monthly,
                price_hourly: Some(t.price.hourly),
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        #[derive(Deserialize)]
        struct ImagesResponse {
            data: Vec<LinodeImage>,
        }

        #[derive(Deserialize)]
        struct LinodeImage {
            id: String,
            label: String,
            vendor: Option<String>,
            is_public: bool,
        }

        let response: ImagesResponse = self.get("/images").await?;

        Ok(response
            .data
            .into_iter()
            .filter(|i| i.is_public)
            .filter(|i| {
                i.vendor
                    .as_ref()
                    .map(|v| v == "Ubuntu" || v == "Debian" || v == "CentOS")
                    .unwrap_or(false)
            })
            .map(|i| OsImage {
                id: i.id,
                name: i.label.clone(),
                distribution: i.vendor.unwrap_or_default().to_lowercase(),
                version: i.label,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer> {
        #[derive(Serialize)]
        struct CreateLinodeRequest {
            label: String,
            region: String,
            #[serde(rename = "type")]
            linode_type: String,
            image: String,
            authorized_keys: Vec<String>,
            root_pass: String,
            tags: Vec<String>,
        }

        #[derive(Deserialize)]
        struct LinodeInstance {
            id: u64,
            label: String,
            status: String,
            ipv4: Vec<String>,
            ipv6: Option<String>,
            region: String,
            #[serde(rename = "type")]
            linode_type: String,
        }

        // Generate a random root password (required by Linode)
        let root_pass: String = (0..32)
            .map(|_| {
                let idx = rand::random::<usize>() % 62;
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[idx] as char
            })
            .collect();

        let request = CreateLinodeRequest {
            label: config.name.clone(),
            region: config.region.clone(),
            linode_type: config.size,
            image: config.image,
            authorized_keys: config.ssh_key_ids,
            root_pass,
            tags: config.tags,
        };

        let instance: LinodeInstance = self.post("/linode/instances", &request).await?;
        info!(linode_id = %instance.id, "Created Linode instance");

        Ok(ProvisionedServer {
            provider_id: instance.id.to_string(),
            name: instance.label,
            ip_address: instance.ipv4.first().cloned(),
            ipv6_address: instance.ipv6,
            status: parse_linode_status(&instance.status),
            region: instance.region,
            size: instance.linode_type,
        })
    }

    #[instrument(skip(self))]
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer> {
        #[derive(Deserialize)]
        struct LinodeInstance {
            id: u64,
            label: String,
            status: String,
            ipv4: Vec<String>,
            ipv6: Option<String>,
            region: String,
            #[serde(rename = "type")]
            linode_type: String,
        }

        let instance: LinodeInstance =
            self.get(&format!("/linode/instances/{}", provider_id)).await?;

        Ok(ProvisionedServer {
            provider_id: instance.id.to_string(),
            name: instance.label,
            ip_address: instance.ipv4.first().cloned(),
            ipv6_address: instance.ipv6,
            status: parse_linode_status(&instance.status),
            region: instance.region,
            size: instance.linode_type,
        })
    }

    #[instrument(skip(self))]
    async fn delete_server(&self, provider_id: &str) -> Result<()> {
        self.delete(&format!("/linode/instances/{}", provider_id))
            .await?;
        info!(linode_id = %provider_id, "Deleted Linode instance");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reboot_server(&self, provider_id: &str) -> Result<()> {
        self.post::<serde_json::Value, _>(
            &format!("/linode/instances/{}/reboot", provider_id),
            &serde_json::json!({}),
        )
        .await?;
        info!(linode_id = %provider_id, "Rebooted Linode instance");
        Ok(())
    }
}

fn parse_linode_status(status: &str) -> ProvisionedServerStatus {
    match status {
        "running" => ProvisionedServerStatus::Running,
        "booting" | "provisioning" | "rebooting" => ProvisionedServerStatus::Pending,
        "offline" | "shutting_down" => ProvisionedServerStatus::Stopped,
        _ => ProvisionedServerStatus::Error,
    }
}
