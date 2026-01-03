//! Docker Compose parser

use std::path::Path;
use super::ComposeFile;
use crate::{Result, Error};

/// Parse a Docker Compose file from a path
pub fn parse_compose(path: impl AsRef<Path>) -> Result<ComposeFile> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| Error::Config(format!("Failed to read compose file: {}", e)))?;
    parse_compose_str(&content)
}

/// Parse a Docker Compose file from a string
pub fn parse_compose_str(content: &str) -> Result<ComposeFile> {
    serde_yaml::from_str(content)
        .map_err(|e| Error::Validation(format!("Failed to parse compose file: {}", e)))
}

/// Validate a Docker Compose file
pub fn validate_compose(compose: &ComposeFile) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check that services exist
    if compose.services.is_empty() {
        errors.push(ValidationError {
            field: "services".to_string(),
            message: "At least one service is required".to_string(),
            severity: Severity::Error,
        });
    }

    // Validate each service
    for (name, service) in &compose.services {
        // Service must have image or build
        if service.image.is_none() && service.build.is_none() {
            errors.push(ValidationError {
                field: format!("services.{}", name),
                message: "Service must have either 'image' or 'build'".to_string(),
                severity: Severity::Error,
            });
        }

        // Validate port mappings
        for port in &service.ports {
            if !is_valid_port_mapping(port) {
                errors.push(ValidationError {
                    field: format!("services.{}.ports", name),
                    message: format!("Invalid port mapping: {}", port),
                    severity: Severity::Error,
                });
            }
        }

        // Check for privileged mode (warning)
        if service.privileged == Some(true) {
            errors.push(ValidationError {
                field: format!("services.{}.privileged", name),
                message: "Privileged mode is enabled - this is a security risk".to_string(),
                severity: Severity::Warning,
            });
        }

        // Validate depends_on references
        for dep in &service.depends_on {
            if !compose.services.contains_key(dep) {
                errors.push(ValidationError {
                    field: format!("services.{}.depends_on", name),
                    message: format!("Service '{}' depends on unknown service '{}'", name, dep),
                    severity: Severity::Error,
                });
            }
        }

        // Validate network references
        for network in &service.networks {
            if !compose.networks.contains_key(network) && !is_special_network(network) {
                errors.push(ValidationError {
                    field: format!("services.{}.networks", name),
                    message: format!("Service '{}' references undefined network '{}'", name, network),
                    severity: Severity::Warning,
                });
            }
        }
    }

    errors
}

fn is_valid_port_mapping(port: &str) -> bool {
    // Valid formats:
    // - "8080" (container port only)
    // - "8080:80" (host:container)
    // - "127.0.0.1:8080:80" (ip:host:container)
    // - "8080-8090:80-90" (port range)
    // - "8080:80/udp" (with protocol)

    let parts: Vec<&str> = port.split(':').collect();
    match parts.len() {
        1 | 2 | 3 => {
            for part in &parts {
                let port_part = part.split('/').next().unwrap_or(part);
                // Handle ranges
                if port_part.contains('-') {
                    let range: Vec<&str> = port_part.split('-').collect();
                    if range.len() != 2 {
                        return false;
                    }
                    if range[0].parse::<u16>().is_err() || range[1].parse::<u16>().is_err() {
                        return false;
                    }
                } else if parts.len() == 3 && parts[0] == *part {
                    // First part of ip:host:container can be an IP address
                    continue;
                } else if port_part.parse::<u16>().is_err() {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn is_special_network(network: &str) -> bool {
    matches!(network, "host" | "none" | "bridge")
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// Transform a compose file for deployment
pub fn transform_for_deployment(
    mut compose: ComposeFile,
    container_prefix: &str,
    network: &str,
    labels: Vec<(String, String)>,
) -> ComposeFile {
    // Add the deployment network
    compose.networks.insert(
        network.to_string(),
        super::Network {
            external: Some(true),
            ..Default::default()
        },
    );

    // Transform each service
    for (name, service) in compose.services.iter_mut() {
        // Set container name with prefix
        if service.container_name.is_none() {
            service.container_name = Some(format!("{}-{}", container_prefix, name));
        }

        // Ensure restart policy
        if service.restart.is_none() {
            service.restart = Some("unless-stopped".to_string());
        }

        // Add the deployment network
        if !service.networks.contains(&network.to_string()) {
            service.networks.push(network.to_string());
        }

        // Add deployment labels
        for (key, value) in &labels {
            service.labels.insert(key.clone(), value.clone());
        }
    }

    compose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_compose() {
        let yaml = r#"
version: '3.8'
services:
  web:
    image: nginx:alpine
    ports:
      - "8080:80"
  db:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: secret
"#;
        let compose = parse_compose_str(yaml).unwrap();
        assert_eq!(compose.services.len(), 2);
        assert!(compose.services.contains_key("web"));
        assert!(compose.services.contains_key("db"));
    }

    #[test]
    fn test_validate_compose() {
        let yaml = r#"
services:
  web:
    image: nginx
  invalid:
    depends_on:
      - nonexistent
"#;
        let compose = parse_compose_str(yaml).unwrap();
        let errors = validate_compose(&compose);

        // Should have error for missing image/build and for invalid depends_on
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.message.contains("nonexistent")));
    }

    #[test]
    fn test_port_validation() {
        assert!(is_valid_port_mapping("8080"));
        assert!(is_valid_port_mapping("8080:80"));
        assert!(is_valid_port_mapping("127.0.0.1:8080:80"));
        assert!(is_valid_port_mapping("8080:80/tcp"));
        assert!(!is_valid_port_mapping("invalid"));
        assert!(!is_valid_port_mapping("1:2:3:4"));
    }
}
