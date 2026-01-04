//! Update User Profile Action
//!
//! Update user profile information.

use crate::actions::{Action, ActionContext, ActionResult, ActionError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{EmailValidator, TokenGenerator};

/// Update user profile action
pub struct UpdateUserProfile;

/// Input for updating profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfileInput {
    /// User ID
    pub user_id: Uuid,
    /// New name (optional)
    pub name: Option<String>,
    /// New email (optional, triggers verification)
    pub email: Option<String>,
    /// Two-factor authentication setting (optional)
    pub two_factor_enabled: Option<bool>,
    /// Timezone preference (optional)
    pub timezone: Option<String>,
    /// Locale/language preference (optional)
    pub locale: Option<String>,
}

/// Output after updating profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserProfileOutput {
    /// User ID
    pub user_id: Uuid,
    /// Fields that were updated
    pub updated_fields: Vec<String>,
    /// Whether email verification is pending
    pub email_verification_pending: bool,
    /// New email (if changed)
    pub new_email: Option<String>,
}

impl Action for UpdateUserProfile {
    type Input = UpdateUserProfileInput;
    type Output = UpdateUserProfileOutput;

    fn name(&self) -> &'static str {
        "update_user_profile"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        let mut updated_fields = Vec::new();
        let mut email_verification_pending = false;
        let mut new_email = None;

        // Validate and track name update
        if let Some(ref name) = input.name {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return Err(ActionError::new("INVALID_NAME", "Name cannot be empty"));
            }
            if trimmed.len() > 255 {
                return Err(ActionError::new("NAME_TOO_LONG", "Name is too long (max 255 characters)"));
            }
            updated_fields.push("name".to_string());
        }

        // Validate and track email update
        if let Some(ref email) = input.email {
            let normalized = EmailValidator::normalize(email);
            if !EmailValidator::validate(&normalized) {
                return Err(ActionError::new("INVALID_EMAIL", "Invalid email address format"));
            }

            // Would:
            // 1. Check if email is already taken
            // 2. Generate verification token
            // 3. Send verification email
            // 4. Store pending email change

            updated_fields.push("email".to_string());
            email_verification_pending = true;
            new_email = Some(normalized);
        }

        // Track timezone update
        if let Some(ref tz) = input.timezone {
            // Would validate timezone is valid
            if !is_valid_timezone(tz) {
                return Err(ActionError::new("INVALID_TIMEZONE", "Invalid timezone"));
            }
            updated_fields.push("timezone".to_string());
        }

        // Track locale update
        if let Some(ref locale) = input.locale {
            if !is_valid_locale(locale) {
                return Err(ActionError::new("INVALID_LOCALE", "Invalid locale"));
            }
            updated_fields.push("locale".to_string());
        }

        // Track 2FA update
        if input.two_factor_enabled.is_some() {
            updated_fields.push("two_factor_enabled".to_string());
        }

        if updated_fields.is_empty() {
            return Err(ActionError::new("NO_CHANGES", "No fields to update"));
        }

        Ok(UpdateUserProfileOutput {
            user_id: input.user_id,
            updated_fields,
            email_verification_pending,
            new_email,
        })
    }
}

/// Validate timezone string
fn is_valid_timezone(tz: &str) -> bool {
    // Common timezones - in production would use chrono-tz
    let valid_timezones = [
        "UTC", "America/New_York", "America/Los_Angeles", "America/Chicago",
        "Europe/London", "Europe/Paris", "Europe/Berlin", "Asia/Tokyo",
        "Asia/Shanghai", "Asia/Singapore", "Australia/Sydney", "Pacific/Auckland",
    ];

    valid_timezones.contains(&tz) || tz.starts_with("Etc/")
}

/// Validate locale string
fn is_valid_locale(locale: &str) -> bool {
    // ISO locale format validation
    let parts: Vec<&str> = locale.split('-').collect();
    if parts.is_empty() || parts.len() > 3 {
        return false;
    }

    // Language code (2-3 lowercase letters)
    let lang = parts[0];
    if lang.len() < 2 || lang.len() > 3 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }

    true
}

// ============================================================================
// Delete User Account
// ============================================================================

/// Delete user account action
pub struct DeleteUserAccount;

/// Input for deleting account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserAccountInput {
    /// User ID
    pub user_id: Uuid,
    /// Current password for confirmation
    pub password: String,
    /// Confirmation phrase (e.g., "DELETE")
    pub confirmation: String,
    /// Delete all associated data
    pub delete_data: bool,
}

/// Output after deleting account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserAccountOutput {
    /// User ID that was deleted
    pub user_id: Uuid,
    /// Whether deletion was successful
    pub deleted: bool,
    /// Resources that were deleted
    pub deleted_resources: DeletedUserResources,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeletedUserResources {
    pub teams_transferred: usize,
    pub teams_deleted: usize,
    pub servers_deleted: usize,
    pub applications_deleted: usize,
    pub databases_deleted: usize,
    pub services_deleted: usize,
}

impl Action for DeleteUserAccount {
    type Input = DeleteUserAccountInput;
    type Output = DeleteUserAccountOutput;

    fn name(&self) -> &'static str {
        "delete_user_account"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        // Validate confirmation
        if input.confirmation != "DELETE" {
            return Err(ActionError::new(
                "INVALID_CONFIRMATION",
                "Please type 'DELETE' to confirm account deletion",
            ));
        }

        // Would:
        // 1. Verify password
        // 2. Transfer/delete owned teams
        // 3. Remove from all teams
        // 4. Delete user data if requested
        // 5. Delete user record

        Ok(DeleteUserAccountOutput {
            user_id: input.user_id,
            deleted: true,
            deleted_resources: DeletedUserResources::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_profile_empty_name() {
        let action = UpdateUserProfile;
        let ctx = ActionContext::default();
        let input = UpdateUserProfileInput {
            user_id: Uuid::new_v4(),
            name: Some("".to_string()),
            email: None,
            two_factor_enabled: None,
            timezone: None,
            locale: None,
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_NAME");
    }

    #[test]
    fn test_update_profile_no_changes() {
        let action = UpdateUserProfile;
        let ctx = ActionContext::default();
        let input = UpdateUserProfileInput {
            user_id: Uuid::new_v4(),
            name: None,
            email: None,
            two_factor_enabled: None,
            timezone: None,
            locale: None,
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NO_CHANGES");
    }

    #[test]
    fn test_valid_timezone() {
        assert!(is_valid_timezone("UTC"));
        assert!(is_valid_timezone("America/New_York"));
        assert!(!is_valid_timezone("Invalid/Zone"));
    }

    #[test]
    fn test_valid_locale() {
        assert!(is_valid_locale("en"));
        assert!(is_valid_locale("en-US"));
        assert!(is_valid_locale("pt-BR"));
        assert!(!is_valid_locale(""));
        assert!(!is_valid_locale("INVALID"));
    }

    #[test]
    fn test_delete_account_wrong_confirmation() {
        let action = DeleteUserAccount;
        let ctx = ActionContext::default();
        let input = DeleteUserAccountInput {
            user_id: Uuid::new_v4(),
            password: "password".to_string(),
            confirmation: "wrong".to_string(),
            delete_data: true,
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_CONFIRMATION");
    }
}
