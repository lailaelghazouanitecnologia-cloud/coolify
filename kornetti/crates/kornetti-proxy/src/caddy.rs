//! Caddy configuration generation

use crate::ServiceConfig;

/// Generate Caddyfile configuration
pub fn generate(services: &[ServiceConfig]) -> String {
    let mut config = String::new();

    for svc in services {
        let scheme = if svc.https { "https" } else { "http" };

        config.push_str(&format!("{} {{\n", svc.domain));
        config.push_str(&format!(
            "    reverse_proxy {}:{}\n",
            svc.container_name, svc.port
        ));

        if svc.https {
            config.push_str("    tls {\n");
            config.push_str("        on_demand\n");
            config.push_str("    }\n");
        }

        for (key, value) in &svc.headers {
            config.push_str(&format!("    header {} \"{}\"\n", key, value));
        }

        if svc.www_redirect {
            config.push_str(&format!(
                "    @www host www.{}\n",
                svc.domain
            ));
            config.push_str(&format!(
                "    redir @www {}://{{}}{{}}\n",
                scheme,
            ));
        }

        config.push_str("}\n\n");
    }

    config
}

/// Generate JSON API configuration for Caddy
pub fn generate_json(services: &[ServiceConfig]) -> String {
    use serde_json::{json, Value};

    let routes: Vec<Value> = services
        .iter()
        .map(|svc| {
            json!({
                "match": [{
                    "host": [svc.domain]
                }],
                "handle": [{
                    "handler": "reverse_proxy",
                    "upstreams": [{
                        "dial": format!("{}:{}", svc.container_name, svc.port)
                    }]
                }]
            })
        })
        .collect();

    let config = json!({
        "apps": {
            "http": {
                "servers": {
                    "srv0": {
                        "listen": [":443"],
                        "routes": routes
                    }
                }
            }
        }
    });

    serde_json::to_string_pretty(&config).unwrap_or_default()
}
