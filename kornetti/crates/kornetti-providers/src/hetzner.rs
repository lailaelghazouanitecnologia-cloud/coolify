//! Hetzner Cloud Provider
//!
//! API Documentation: https://docs.hetzner.cloud/

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

const HETZNER_API_BASE: &str = "https://api.hetzner.cloud/v1";

pub struct HetznerProvider {
    client: Client,
    api_token: String,
}

impl HetznerProvider {
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
        let url = format!("{}{}", HETZNER_API_BASE, path);
        debug!(url = %url, "Hetzner API GET request");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Hetzner API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Hetzner API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Hetzner response: {}", e)))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", HETZNER_API_BASE, path);
        debug!(url = %url, "Hetzner API POST request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Hetzner API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Hetzner API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Failed to parse Hetzner response: {}", e)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", HETZNER_API_BASE, path);
        debug!(url = %url, "Hetzner API DELETE request");

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Hetzner API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "Hetzner API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl CloudProvider for HetznerProvider {
    fn name(&self) -> &'static str {
        "hetzner"
    }

    #[instrument(skip(self))]
    async fn list_regions(&self) -> Result<Vec<Region>> {
        #[derive(Deserialize)]
        struct LocationsResponse {
            locations: Vec<HetznerLocation>,
        }

        #[derive(Deserialize)]
        struct HetznerLocation {
            id: u64,
            name: String,
            description: String,
            country: String,
            city: String,
            network_zone: String,
        }

        let response: LocationsResponse = self.get("/locations").await?;

        Ok(response
            .locations
            .into_iter()
            .map(|l| Region {
                id: l.name,
                name: format!("{} ({})", l.city, l.description),
                country: Some(l.country),
                available: true,
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_sizes(&self, _region: &str) -> Result<Vec<ServerSize>> {
        #[derive(Deserialize)]
        struct ServerTypesResponse {
            server_types: Vec<HetznerServerType>,
        }

        #[derive(Deserialize)]
        struct HetznerServerType {
            id: u64,
            name: String,
            description: String,
            cores: u32,
            memory: f64,
            disk: u32,
            prices: Vec<HetznerPrice>,
            architecture: String,
        }

        #[derive(Deserialize)]
        struct HetznerPrice {
            location: String,
            price_monthly: HetznerPriceValue,
            price_hourly: HetznerPriceValue,
        }

        #[derive(Deserialize)]
        struct HetznerPriceValue {
            net: String,
            gross: String,
        }

        let response: ServerTypesResponse = self.get("/server_types").await?;

        Ok(response
            .server_types
            .into_iter()
            .map(|s| {
                let monthly_price = s
                    .prices
                    .first()
                    .and_then(|p| p.price_monthly.gross.parse().ok())
                    .unwrap_or(0.0);
                let hourly_price = s
                    .prices
                    .first()
                    .and_then(|p| p.price_hourly.gross.parse().ok());

                ServerSize {
                    id: s.name,
                    name: s.description,
                    vcpus: s.cores,
                    memory_mb: (s.memory * 1024.0) as u32,
                    disk_gb: s.disk,
                    bandwidth_tb: Some(20.0), // Hetzner includes 20TB
                    price_monthly: monthly_price,
                    price_hourly: hourly_price,
                }
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn list_images(&self, _region: &str) -> Result<Vec<OsImage>> {
        #[derive(Deserialize)]
        struct ImagesResponse {
            images: Vec<HetznerImage>,
        }

        #[derive(Deserialize)]
        struct HetznerImage {
            id: u64,
            name: Option<String>,
            description: String,
            os_flavor: String,
            os_version: Option<String>,
            #[serde(rename = "type")]
            image_type: String,
        }

        let response: ImagesResponse = self.get("/images?type=system").await?;

        Ok(response
            .images
            .into_iter()
            .filter(|i| {
                i.os_flavor == "ubuntu" || i.os_flavor == "debian" || i.os_flavor == "centos"
            })
            .map(|i| OsImage {
                id: i.id.to_string(),
                name: i.description.clone(),
                distribution: i.os_flavor,
                version: i.os_version.unwrap_or_else(|| i.description),
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn create_server(&self, config: CreateServerConfig) -> Result<ProvisionedServer> {
        #[derive(Serialize)]
        struct CreateServerRequest {
            name: String,
            server_type: String,
            image: String,
            location: String,
            ssh_keys: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_data: Option<String>,
            labels: std::collections::HashMap<String, String>,
        }

        #[derive(Deserialize)]
        struct CreateServerResponse {
            server: HetznerServer,
        }

        #[derive(Deserialize)]
        struct HetznerServer {
            id: u64,
            name: String,
            public_net: HetznerPublicNet,
            status: String,
            server_type: HetznerServerTypeRef,
            datacenter: HetznerDatacenter,
        }

        #[derive(Deserialize)]
        struct HetznerPublicNet {
            ipv4: Option<HetznerIp>,
            ipv6: Option<HetznerIpv6>,
        }

        #[derive(Deserialize)]
        struct HetznerIp {
            ip: String,
        }

        #[derive(Deserialize)]
        struct HetznerIpv6 {
            ip: String,
        }

        #[derive(Deserialize)]
        struct HetznerServerTypeRef {
            name: String,
        }

        #[derive(Deserialize)]
        struct HetznerDatacenter {
            location: HetznerLocationRef,
        }

        #[derive(Deserialize)]
        struct HetznerLocationRef {
            name: String,
        }

        let labels: std::collections::HashMap<String, String> = config
            .tags
            .iter()
            .enumerate()
            .map(|(i, t)| (format!("tag_{}", i), t.clone()))
            .collect();

        let request = CreateServerRequest {
            name: config.name.clone(),
            server_type: config.size,
            image: config.image,
            location: config.region.clone(),
            ssh_keys: config.ssh_key_ids,
            user_data: config.user_data,
            labels,
        };

        let response: CreateServerResponse = self.post("/servers", &request).await?;
        info!(
            server_id = %response.server.id,
            "Created Hetzner server"
        );

        Ok(ProvisionedServer {
            provider_id: response.server.id.to_string(),
            name: response.server.name,
            ip_address: response.server.public_net.ipv4.map(|ip| ip.ip),
            ipv6_address: response.server.public_net.ipv6.map(|ip| ip.ip),
            status: parse_hetzner_status(&response.server.status),
            region: response.server.datacenter.location.name,
            size: response.server.server_type.name,
        })
    }

    #[instrument(skip(self))]
    async fn get_server(&self, provider_id: &str) -> Result<ProvisionedServer> {
        #[derive(Deserialize)]
        struct ServerResponse {
            server: HetznerServer,
        }

        #[derive(Deserialize)]
        struct HetznerServer {
            id: u64,
            name: String,
            public_net: HetznerPublicNet,
            status: String,
            server_type: HetznerServerTypeRef,
            datacenter: HetznerDatacenter,
        }

        #[derive(Deserialize)]
        struct HetznerPublicNet {
            ipv4: Option<HetznerIp>,
            ipv6: Option<HetznerIpv6>,
        }

        #[derive(Deserialize)]
        struct HetznerIp {
            ip: String,
        }

        #[derive(Deserialize)]
        struct HetznerIpv6 {
            ip: String,
        }

        #[derive(Deserialize)]
        struct HetznerServerTypeRef {
            name: String,
        }

        #[derive(Deserialize)]
        struct HetznerDatacenter {
            location: HetznerLocationRef,
        }

        #[derive(Deserialize)]
        struct HetznerLocationRef {
            name: String,
        }

        let response: ServerResponse = self.get(&format!("/servers/{}", provider_id)).await?;

        Ok(ProvisionedServer {
            provider_id: response.server.id.to_string(),
            name: response.server.name,
            ip_address: response.server.public_net.ipv4.map(|ip| ip.ip),
            ipv6_address: response.server.public_net.ipv6.map(|ip| ip.ip),
            status: parse_hetzner_status(&response.server.status),
            region: response.server.datacenter.location.name,
            size: response.server.server_type.name,
        })
    }

    #[instrument(skip(self))]
    async fn delete_server(&self, provider_id: &str) -> Result<()> {
        self.delete(&format!("/servers/{}", provider_id)).await?;
        info!(server_id = %provider_id, "Deleted Hetzner server");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reboot_server(&self, provider_id: &str) -> Result<()> {
        self.post::<serde_json::Value, _>(
            &format!("/servers/{}/actions/reboot", provider_id),
            &serde_json::json!({}),
        )
        .await?;
        info!(server_id = %provider_id, "Rebooted Hetzner server");
        Ok(())
    }
}

fn parse_hetzner_status(status: &str) -> ProvisionedServerStatus {
    match status {
        "running" => ProvisionedServerStatus::Running,
        "initializing" | "starting" => ProvisionedServerStatus::Pending,
        "off" | "stopping" => ProvisionedServerStatus::Stopped,
        _ => ProvisionedServerStatus::Error,
    }
}
