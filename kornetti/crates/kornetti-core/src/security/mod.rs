//! Security utilities and validation patterns
//!
//! This module provides security-related utilities including:
//! - Input validation and sanitization
//! - Shell-safe path validation
//! - IP allowlist checking
//! - Encryption helpers
//! - SSH key validation

use std::net::IpAddr;
use regex::Regex;
use once_cell::sync::Lazy;

pub mod encryption;
pub mod ssh_keys;

pub use encryption::{Encryptor, EncryptionKey, encrypt_string, decrypt_string, hash_password, verify_password};
pub use ssh_keys::{validate_private_key, extract_public_key, KeyType};

/// Validation patterns matching Coolify's patterns
pub mod patterns {
    use super::*;

    /// Pattern for validating names (alphanumeric, spaces, and limited special chars)
    pub static NAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9\s\-_.:/()\[\]]+$").unwrap()
    });

    /// Pattern for validating descriptions (more permissive)
    pub static DESCRIPTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"^[a-zA-Z0-9\s\-_.:/()'\",.!?@#%&+=\[\]{}|~`*]+$"#).unwrap()
    });

    /// Pattern for validating environment variable keys
    pub static ENV_KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap()
    });

    /// Pattern for validating Docker container names
    pub static CONTAINER_NAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_.-]*$").unwrap()
    });

    /// Pattern for validating domain names
    pub static DOMAIN_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$").unwrap()
    });

    /// Pattern for validating port numbers
    pub static PORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$").unwrap()
    });
}

/// Dangerous characters that should not appear in shell commands
pub const DANGEROUS_SHELL_CHARS: &[&str] = &[
    "`",    // Command substitution
    "$(",   // Command substitution
    "${",   // Variable expansion
    "|",    // Pipe
    "&",    // Background/AND
    ";",    // Command separator
    "\n",   // Newline
    "\r",   // Carriage return
    "\t",   // Tab (in some contexts)
    ">",    // Redirect
    "<",    // Redirect
    ">>",   // Append redirect
    "<<",   // Here document
    "$((",  // Arithmetic expansion
    "\\",   // Escape character
];

/// Validate that a string is safe for use in shell commands
///
/// This function checks for dangerous shell metacharacters that could
/// lead to command injection vulnerabilities.
///
/// # Arguments
/// * `input` - The string to validate
/// * `context` - Description of what the input represents (for error messages)
///
/// # Returns
/// * `Ok(String)` - The validated input
/// * `Err(SecurityError)` - If dangerous characters are found
///
/// # Example
/// ```
/// use kornetti_core::security::validate_shell_safe_path;
///
/// assert!(validate_shell_safe_path("/data/app", "path").is_ok());
/// assert!(validate_shell_safe_path("/data/$(whoami)", "path").is_err());
/// ```
pub fn validate_shell_safe_path(input: &str, context: &str) -> Result<String, SecurityError> {
    for dangerous in DANGEROUS_SHELL_CHARS {
        if input.contains(dangerous) {
            return Err(SecurityError::DangerousCharacter {
                context: context.to_string(),
                character: dangerous.to_string(),
                input: input.to_string(),
            });
        }
    }
    Ok(input.to_string())
}

/// Validate multiple paths for shell safety
pub fn validate_shell_safe_paths<'a>(
    inputs: impl IntoIterator<Item = &'a str>,
    context: &str,
) -> Result<Vec<String>, SecurityError> {
    inputs
        .into_iter()
        .map(|input| validate_shell_safe_path(input, context))
        .collect()
}

/// Sanitize a string by removing potentially dangerous content
///
/// This function:
/// 1. Removes HTML/XML tags
/// 2. Converts special characters to HTML entities
/// 3. Removes control characters
/// 4. Trims whitespace
///
/// Mirrors Coolify's `sanitize_string()` function.
pub fn sanitize_string(input: &str) -> String {
    // Remove HTML tags
    let without_tags = strip_html_tags(input);

    // HTML encode special characters
    let encoded = html_encode(&without_tags);

    // Remove control characters (except newline and carriage return in some cases)
    let without_control: String = encoded
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r')
        .collect();

    // Trim whitespace
    without_control.trim().to_string()
}

/// Sanitize a string strictly (no newlines allowed)
pub fn sanitize_string_strict(input: &str) -> String {
    let sanitized = sanitize_string(input);
    sanitized.chars().filter(|c| *c != '\n' && *c != '\r').collect()
}

/// Strip HTML tags from a string
fn strip_html_tags(input: &str) -> String {
    static HTML_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"<[^>]*>").unwrap()
    });
    HTML_TAG_PATTERN.replace_all(input, "").to_string()
}

/// HTML encode special characters
fn html_encode(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Validate a name according to Coolify's patterns
pub fn validate_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::Empty("name".to_string()));
    }
    if name.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "name".to_string(),
            max_length: 255,
            actual_length: name.len(),
        });
    }
    if !patterns::NAME_PATTERN.is_match(name) {
        return Err(ValidationError::InvalidFormat {
            field: "name".to_string(),
            message: "Name contains invalid characters. Only alphanumeric, spaces, and -_.:/()\\ are allowed.".to_string(),
        });
    }
    Ok(())
}

/// Validate a description
pub fn validate_description(description: &str) -> Result<(), ValidationError> {
    if description.len() > 2000 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max_length: 2000,
            actual_length: description.len(),
        });
    }
    if !description.is_empty() && !patterns::DESCRIPTION_PATTERN.is_match(description) {
        return Err(ValidationError::InvalidFormat {
            field: "description".to_string(),
            message: "Description contains invalid characters.".to_string(),
        });
    }
    Ok(())
}

/// Validate an environment variable key
pub fn validate_env_key(key: &str) -> Result<(), ValidationError> {
    if key.is_empty() {
        return Err(ValidationError::Empty("environment variable key".to_string()));
    }
    if !patterns::ENV_KEY_PATTERN.is_match(key) {
        return Err(ValidationError::InvalidFormat {
            field: "environment variable key".to_string(),
            message: "Key must start with a letter or underscore and contain only alphanumeric characters and underscores.".to_string(),
        });
    }
    Ok(())
}

/// Validate a Docker container name
pub fn validate_container_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::Empty("container name".to_string()));
    }
    if name.len() > 128 {
        return Err(ValidationError::TooLong {
            field: "container name".to_string(),
            max_length: 128,
            actual_length: name.len(),
        });
    }
    if !patterns::CONTAINER_NAME_PATTERN.is_match(name) {
        return Err(ValidationError::InvalidFormat {
            field: "container name".to_string(),
            message: "Container name must start with alphanumeric and contain only alphanumeric, underscores, dots, and hyphens.".to_string(),
        });
    }
    Ok(())
}

/// Validate a domain name
pub fn validate_domain(domain: &str) -> Result<(), ValidationError> {
    if domain.is_empty() {
        return Err(ValidationError::Empty("domain".to_string()));
    }
    if !patterns::DOMAIN_PATTERN.is_match(domain) {
        return Err(ValidationError::InvalidFormat {
            field: "domain".to_string(),
            message: "Invalid domain name format.".to_string(),
        });
    }
    Ok(())
}

/// Check if an IP address is in an allowlist
///
/// Supports:
/// - Individual IP addresses
/// - CIDR notation (e.g., "192.168.0.0/24")
/// - Wildcard "0.0.0.0" to allow all
///
/// Mirrors Coolify's `checkIPAgainstAllowlist()` function.
pub fn check_ip_against_allowlist(ip: &str, allowlist: &[String]) -> bool {
    let client_ip: IpAddr = match ip.parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };

    for allowed in allowlist {
        let allowed = allowed.trim();

        // Wildcard allows all
        if allowed == "0.0.0.0" {
            return true;
        }

        // Check for CIDR notation
        if allowed.contains('/') {
            if check_ip_in_cidr(&client_ip, allowed) {
                return true;
            }
        } else {
            // Direct IP comparison
            if let Ok(allowed_ip) = allowed.parse::<IpAddr>() {
                if client_ip == allowed_ip {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if an IP is within a CIDR range
fn check_ip_in_cidr(ip: &IpAddr, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let subnet: IpAddr = match parts[0].parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };

    let mask: u8 = match parts[1].parse() {
        Ok(m) if m <= 32 => m,
        _ => return false,
    };

    match (ip, subnet) {
        (IpAddr::V4(ip), IpAddr::V4(subnet)) => {
            let ip_bits = u32::from(*ip);
            let subnet_bits = u32::from(subnet);
            let mask_bits = !((1u32 << (32 - mask)) - 1);
            (ip_bits & mask_bits) == (subnet_bits & mask_bits)
        }
        (IpAddr::V6(ip), IpAddr::V6(subnet)) => {
            let ip_bits = u128::from(*ip);
            let subnet_bits = u128::from(subnet);
            let effective_mask = std::cmp::min(mask, 128);
            let mask_bits = !((1u128 << (128 - effective_mask)) - 1);
            (ip_bits & mask_bits) == (subnet_bits & mask_bits)
        }
        _ => false, // IPv4 and IPv6 can't match
    }
}

/// Security-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecurityError {
    #[error("Invalid {context}: contains forbidden character '{character}'")]
    DangerousCharacter {
        context: String,
        character: String,
        input: String,
    },

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("IP address not in allowlist: {0}")]
    IpNotAllowed(String),

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,
}

/// Validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("{0} cannot be empty")]
    Empty(String),

    #[error("{field} is too long: maximum {max_length} characters, got {actual_length}")]
    TooLong {
        field: String,
        max_length: usize,
        actual_length: usize,
    },

    #[error("Invalid {field}: {message}")]
    InvalidFormat { field: String, message: String },

    #[error("Validation failed: {0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_shell_safe_path() {
        // Safe paths
        assert!(validate_shell_safe_path("/data/app", "path").is_ok());
        assert!(validate_shell_safe_path("/var/log/app.log", "path").is_ok());
        assert!(validate_shell_safe_path("my-container_name.v1", "name").is_ok());

        // Dangerous paths
        assert!(validate_shell_safe_path("/data/$(whoami)", "path").is_err());
        assert!(validate_shell_safe_path("/data/`id`", "path").is_err());
        assert!(validate_shell_safe_path("foo; rm -rf /", "command").is_err());
        assert!(validate_shell_safe_path("foo | cat /etc/passwd", "command").is_err());
        assert!(validate_shell_safe_path("foo > /etc/passwd", "path").is_err());
        assert!(validate_shell_safe_path("${HOME}", "path").is_err());
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("<script>alert('xss')</script>"), "alert(&#39;xss&#39;)");
        assert_eq!(sanitize_string("  hello world  "), "hello world");
        assert_eq!(sanitize_string("a<b>c"), "a&lt;b&gt;c");
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("my-app_v1.0").is_ok());
        assert!(validate_name("My Application (Test)").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("a".repeat(256).as_str()).is_err());
    }

    #[test]
    fn test_validate_env_key() {
        assert!(validate_env_key("MY_VAR").is_ok());
        assert!(validate_env_key("_private").is_ok());
        assert!(validate_env_key("var123").is_ok());
        assert!(validate_env_key("123var").is_err()); // Can't start with number
        assert!(validate_env_key("my-var").is_err()); // No hyphens
        assert!(validate_env_key("").is_err());
    }

    #[test]
    fn test_check_ip_against_allowlist() {
        let allowlist = vec![
            "192.168.1.100".to_string(),
            "10.0.0.0/8".to_string(),
            "172.16.0.0/12".to_string(),
        ];

        // Exact match
        assert!(check_ip_against_allowlist("192.168.1.100", &allowlist));

        // CIDR match
        assert!(check_ip_against_allowlist("10.255.255.255", &allowlist));
        assert!(check_ip_against_allowlist("172.31.255.255", &allowlist));

        // Not in allowlist
        assert!(!check_ip_against_allowlist("192.168.1.101", &allowlist));
        assert!(!check_ip_against_allowlist("8.8.8.8", &allowlist));

        // Wildcard
        let wildcard = vec!["0.0.0.0".to_string()];
        assert!(check_ip_against_allowlist("1.2.3.4", &wildcard));
    }

    #[test]
    fn test_validate_container_name() {
        assert!(validate_container_name("my-container").is_ok());
        assert!(validate_container_name("app_v1.2.3").is_ok());
        assert!(validate_container_name("a1").is_ok());
        assert!(validate_container_name("-invalid").is_err()); // Can't start with hyphen
        assert!(validate_container_name("").is_err());
    }
}
