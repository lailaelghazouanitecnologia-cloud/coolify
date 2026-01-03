//! DigitalOcean Cloud Provider
//!
//! API Documentation: https://docs.digitalocean.com/reference/api/

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

const DO_API_BASE: &str = "https://api.digitalocean.com/v2";

pub struct DigitalOceanProvider {
    client: Client,
    api_token: String,
}

impl DigitalOceanProvider {
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
        let url = format!("{}{}", DO_API_BASE, path);
        debug!(url = %url, "DigitalOcean API GET request");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("DigitalOcean API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "DigitalOcean API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse DigitalOcean response: {}", e)))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", DO_API_BASE, path);
        debug!(url = %url, "DigitalOcean API POST request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("DigitalOcean API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "DigitalOcean API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse DigitalOcean response: {}", e)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", DO_API_BASE, path);
        debug!(url = %url, "DigitalOcean API DELETE request");

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("DigitalOcean API request failed: {}", e)))?;

        if !response.status().is_success() && response.status().as_u16() != 204 {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "DigitalOcean API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl CloudProvider for DigitalOceanProvider {
    fn name(&self) -> &'static str {
        "digitalocean"
    }

    #[instrument(skip(self))]
    async fn list_regions(&self) -> Result<Vec<Region>> {
        #[derive(Deserialize)]
        struct RegionsResponse {
            regions: Vec<DoRegion>,
        }

        #[derive(Deserialize)]
        struct DoRegion {
            slug: String,
            name: String,
            available: bool,
        }

        let response: RegionsResponse = self.get("/regions").await?;

        Ok(response
            .regions
            .into_iter()
            .map(|r| Region {
                id: r.slug,
                name: r.name,
                country: None,
                available: r.available,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        #[derive(Deserialize)]
        struct SizesResponse {
            sizes: Vec<DoSize>,
        }

        #[derive(Deserialize)]
        struct DoSize {
            slug: String,
            memory: u32,
            vcpus: u32,
            disk: u32,
            transfer: f64,
            price_monthly: f64,
            price_hourly: f64,
            description: String,
            available: bool,
        }

        let response: SizesResponse = self.get("/sizes").await?;

        Ok(response
            .sizes
            .into_iter()
            .filter(|s| s.available)
            .map(|s| ServerSize {
                id: s.slug,
                name: s.description,
                vcpus: s.vcpus,
                memory_mb: s.memory,
                disk_gb: s.disk,
                bandwidth_tb: Some(s.transfer),
                price_monthly: s.price_monthly,
                price_hourly: Some(s.price_hourly),
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        #[derive(Deserialize)]
        struct ImagesResponse {
            images: Vec<DoImage>,
        }

        #[derive(Deserialize)]
        struct DoImage {
            id: u64,
            name: String,
            distribution: String,
            slug: Option<String>,
            public: bool,
            #[serde(rename = "type")]
            image_type: String,
        }

        let response: ImagesResponse = self.get("/images?type=distribution").await?;

        Ok(response
            .images
            .into_iter()
            .filter(|i| i.public && i.slug.is_some())
            .filter(|i| {
                let dist = i.distribution.to_lowercase();
                dist.contains("ubuntu") || dist.contains("debian") || dist.contains("centos")
            })
            .map(|i| OsImage {
                id: i.slug.unwrap_or_else(|| i.id.to_string()),
                name: i.name.clone(),
                distribution: i.distribution.to_lowercase(),
                version: i.name,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer> {
        #[derive(Serialize)]
        struct CreateDropletRequest {
            name: String,
            region: String,
            size: String,
            image: String,
            ssh_keys: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_data: Option<String>,
            tags: Vec<String>,
        }

        #[derive(Deserialize)]
        struct CreateDropletResponse {
            droplet: DoDroplet,
        }

        #[derive(Deserialize)]
        struct DoDroplet {
            id: u64,
            name: String,
            status: String,
            networks: DoNetworks,
            size_slug: String,
            region: DoRegionRef,
        }

        #[derive(Deserialize)]
        struct DoNetworks {
            v4: Vec<DoNetwork>,
            v6: Vec<DoNetwork>,
        }

        #[derive(Deserialize)]
        struct DoNetwork {
            ip_address: String,
            #[serde(rename = "type")]
            network_type: String,
        }

        #[derive(Deserialize)]
        struct DoRegionRef {
            slug: String,
        }

        let request = CreateDropletRequest {
            name: config.name.clone(),
            region: config.region.clone(),
            size: config.size,
            image: config.image,
            ssh_keys: config.ssh_key_ids,
            user_data: config.user_data,
            tags: config.tags,
        };

        let response: CreateDropletResponse = self.post("/droplets", &request).await?;
        info!(
            droplet_id = %response.droplet.id,
            "Created DigitalOcean droplet"
        );

        let public_ipv4 = response
            .droplet
            .networks
            .v4
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone());

        let public_ipv6 = response
            .droplet
            .networks
            .v6
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone());

        Ok(ProvisionedServer {
            provider_id: response.droplet.id.to_string(),
            name: response.droplet.name,
            ip_address: public_ipv4,
            ipv6_address: public_ipv6,
            status: parse_do_status(&response.droplet.status),
            region: response.droplet.region.slug,
            size: response.droplet.size_slug,
        })
    }

    #[instrument(skip(self))]
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer> {
        #[derive(Deserialize)]
        struct DropletResponse {
            droplet: DoDroplet,
        }

        #[derive(Deserialize)]
        struct DoDroplet {
            id: u64,
            name: String,
            status: String,
            networks: DoNetworks,
            size_slug: String,
            region: DoRegionRef,
        }

        #[derive(Deserialize)]
        struct DoNetworks {
            v4: Vec<DoNetwork>,
            v6: Vec<DoNetwork>,
        }

        #[derive(Deserialize)]
        struct DoNetwork {
            ip_address: String,
            #[serde(rename = "type")]
            network_type: String,
        }

        #[derive(Deserialize)]
        struct DoRegionRef {
            slug: String,
        }

        let response: DropletResponse = self.get(&format!("/droplets/{}", provider_id)).await?;

        let public_ipv4 = response
            .droplet
            .networks
            .v4
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone());

        let public_ipv6 = response
            .droplet
            .networks
            .v6
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone());

        Ok(ProvisionedServer {
            provider_id: response.droplet.id.to_string(),
            name: response.droplet.name,
            ip_address: public_ipv4,
            ipv6_address: public_ipv6,
            status: parse_do_status(&response.droplet.status),
            region: response.droplet.region.slug,
            size: response.droplet.size_slug,
        })
    }

    #[instrument(skip(self))]
    async fn delete_server(&self, provider_id: &str) -> Result<()> {
        self.delete(&format!("/droplets/{}", provider_id)).await?;
        info!(droplet_id = %provider_id, "Deleted DigitalOcean droplet");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reboot_server(&self, provider_id: &str) -> Result<()> {
        self.post::<serde_json::Value, _>(
            &format!("/droplets/{}/actions", provider_id),
            &serde_json::json!({ "type": "reboot" }),
        )
        .await?;
        info!(droplet_id = %provider_id, "Rebooted DigitalOcean droplet");
        Ok(())
    }
}

fn parse_do_status(status: &str) -> ProvisionedServerStatus {
    match status {
        "active" => ProvisionedServerStatus::Running,
        "new" => ProvisionedServerStatus::Pending,
        "off" | "archive" => ProvisionedServerStatus::Stopped,
        _ => ProvisionedServerStatus::Error,
    }
}
