//! Traefik configuration generation

use crate::ServiceConfig;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct TraefikConfig {
    http: HttpConfig,
}

#[derive(Serialize)]
struct HttpConfig {
    routers: HashMap<String, Router>,
    services: HashMap<String, Service>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Router {
    rule: String,
    service: String,
    entry_points: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls: Option<TlsConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    middlewares: Vec<String>,
}

#[derive(Serialize)]
struct TlsConfig {
    #[serde(rename = "certResolver")]
    cert_resolver: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Service {
    load_balancer: LoadBalancer,
}

#[derive(Serialize)]
struct LoadBalancer {
    servers: Vec<Server>,
}

#[derive(Serialize)]
struct Server {
    url: String,
}

/// Generate Traefik dynamic configuration
pub fn generate(services: &[ServiceConfig]) -> String {
    let mut routers = HashMap::new();
    let mut svc_configs = HashMap::new();

    for svc in services {
        let router_name = format!("{}-router", svc.name);
        let service_name = format!("{}-service", svc.name);

        let mut entry_points = vec!["web".to_string()];
        let tls = if svc.https {
            entry_points = vec!["websecure".to_string()];
            Some(TlsConfig {
                cert_resolver: "letsencrypt".to_string(),
            })
        } else {
            None
        };

        let rule = format!("Host(`{}`)", svc.domain);

        routers.insert(
            router_name,
            Router {
                rule,
                service: service_name.clone(),
                entry_points,
                tls,
                middlewares: vec![],
            },
        );

        svc_configs.insert(
            service_name,
            Service {
                load_balancer: LoadBalancer {
                    servers: vec![Server {
                        url: format!("http://{}:{}", svc.container_name, svc.port),
                    }],
                },
            },
        );
    }

    let config = TraefikConfig {
        http: HttpConfig {
            routers,
            services: svc_configs,
        },
    };

    serde_yaml::to_string(&config).unwrap_or_default()
}

/// Generate Traefik labels for a container
pub fn generate_labels(svc: &ServiceConfig) -> Vec<(String, String)> {
    let mut labels = vec![
        ("traefik.enable".to_string(), "true".to_string()),
        (
            format!("traefik.http.routers.{}.rule", svc.name),
            format!("Host(`{}`)", svc.domain),
        ),
        (
            format!("traefik.http.services.{}.loadbalancer.server.port", svc.name),
            svc.port.to_string(),
        ),
    ];

    if svc.https {
        labels.push((
            format!("traefik.http.routers.{}.entrypoints", svc.name),
            "websecure".to_string(),
        ));
        labels.push((
            format!("traefik.http.routers.{}.tls.certresolver", svc.name),
            "letsencrypt".to_string(),
        ));
    } else {
        labels.push((
            format!("traefik.http.routers.{}.entrypoints", svc.name),
            "web".to_string(),
        ));
    }

    labels
}
