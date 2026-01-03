//! Docker image utilities

use kornetti_core::{Result, Error};

/// Parse a Docker image reference
#[derive(Debug, Clone)]
pub struct ImageRef {
    pub registry: Option<String>,
    pub repository: String,
    pub tag: String,
}

impl ImageRef {
    /// Parse an image reference string
    pub fn parse(image: &str) -> Result<Self> {
        let parts: Vec<&str> = image.split('/').collect();

        let (registry, repo_with_tag) = match parts.len() {
            1 => (None, parts[0]),
            2 => {
                if parts[0].contains('.') || parts[0].contains(':') {
                    (Some(parts[0].to_string()), parts[1])
                } else {
                    (None, image)
                }
            }
            3 => (Some(parts[0].to_string()), &image[parts[0].len() + 1..]),
            _ => return Err(Error::Validation("Invalid image reference".to_string())),
        };

        let (repository, tag) = if let Some(idx) = repo_with_tag.rfind(':') {
            // Check if it's not a port number
            let after_colon = &repo_with_tag[idx + 1..];
            if after_colon.contains('/') {
                (repo_with_tag.to_string(), "latest".to_string())
            } else {
                (repo_with_tag[..idx].to_string(), after_colon.to_string())
            }
        } else {
            (repo_with_tag.to_string(), "latest".to_string())
        };

        Ok(Self {
            registry,
            repository,
            tag,
        })
    }

    /// Convert back to full image reference
    pub fn to_string(&self) -> String {
        match &self.registry {
            Some(reg) => format!("{}/{}:{}", reg, self.repository, self.tag),
            None => format!("{}:{}", self.repository, self.tag),
        }
    }

    /// Check if this is an official Docker Hub image
    pub fn is_official(&self) -> bool {
        self.registry.is_none() && !self.repository.contains('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let img = ImageRef::parse("nginx").unwrap();
        assert_eq!(img.registry, None);
        assert_eq!(img.repository, "nginx");
        assert_eq!(img.tag, "latest");
    }

    #[test]
    fn test_parse_with_tag() {
        let img = ImageRef::parse("nginx:1.21").unwrap();
        assert_eq!(img.registry, None);
        assert_eq!(img.repository, "nginx");
        assert_eq!(img.tag, "1.21");
    }

    #[test]
    fn test_parse_with_registry() {
        let img = ImageRef::parse("ghcr.io/owner/repo:v1.0").unwrap();
        assert_eq!(img.registry, Some("ghcr.io".to_string()));
        assert_eq!(img.repository, "owner/repo");
        assert_eq!(img.tag, "v1.0");
    }
}
