//! Create User Action
//!
//! Register a new user account.

use crate::actions::{Action, ActionContext, ActionResult, ActionError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{EmailValidator, PasswordRules, TokenGenerator};

/// Create user action
pub struct CreateUser;

/// Input for creating a new user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    /// User's name
    pub name: String,
    /// User's email address
    pub email: String,
    /// Password (plaintext, will be hashed)
    pub password: String,
    /// Password confirmation
    pub password_confirmation: String,
    /// Whether to send verification email
    pub send_verification_email: bool,
    /// Invite token (if joining via invitation)
    pub invite_token: Option<String>,
}

/// Output after creating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserOutput {
    /// Created user ID
    pub user_id: Uuid,
    /// User's email
    pub email: String,
    /// Whether email verification is required
    pub email_verification_required: bool,
    /// Created personal team ID
    pub personal_team_id: Uuid,
}

impl Action for CreateUser {
    type Input = CreateUserInput;
    type Output = CreateUserOutput;

    fn name(&self) -> &'static str {
        "create_user"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        // Validate email
        let email = EmailValidator::normalize(&input.email);
        if !EmailValidator::validate(&email) {
            return Err(ActionError::new("INVALID_EMAIL", "Invalid email address format"));
        }

        // Validate password
        if let Err(errors) = PasswordRules::validate(&input.password) {
            return Err(ActionError::new(
                "WEAK_PASSWORD",
                errors.join("; "),
            ));
        }

        // Confirm password match
        if input.password != input.password_confirmation {
            return Err(ActionError::new(
                "PASSWORD_MISMATCH",
                "Password confirmation does not match",
            ));
        }

        // Validate name
        if input.name.trim().is_empty() {
            return Err(ActionError::new("INVALID_NAME", "Name is required"));
        }

        // Hash password
        let password_hash = PasswordRules::hash(&input.password)
            .map_err(|e| ActionError::new("HASH_ERROR", e))?;

        // Would:
        // 1. Check if email already exists
        // 2. Create user record
        // 3. Create personal team
        // 4. Send verification email if needed
        // 5. Handle invite token if present

        let user_id = Uuid::new_v4();
        let personal_team_id = Uuid::new_v4();

        Ok(CreateUserOutput {
            user_id,
            email,
            email_verification_required: input.send_verification_email,
            personal_team_id,
        })
    }
}

impl CreateUser {
    /// Validate input before creation
    pub fn validate_input(input: &CreateUserInput) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if input.name.trim().is_empty() {
            errors.push("Name is required".to_string());
        }

        if !EmailValidator::validate(&input.email) {
            errors.push("Invalid email address format".to_string());
        }

        if let Err(password_errors) = PasswordRules::validate(&input.password) {
            errors.extend(password_errors);
        }

        if input.password != input.password_confirmation {
            errors.push("Password confirmation does not match".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_input_success() {
        let input = CreateUserInput {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "SecurePass123".to_string(),
            password_confirmation: "SecurePass123".to_string(),
            send_verification_email: true,
            invite_token: None,
        };

        assert!(CreateUser::validate_input(&input).is_ok());
    }

    #[test]
    fn test_validate_input_empty_name() {
        let input = CreateUserInput {
            name: "".to_string(),
            email: "john@example.com".to_string(),
            password: "SecurePass123".to_string(),
            password_confirmation: "SecurePass123".to_string(),
            send_verification_email: true,
            invite_token: None,
        };

        let result = CreateUser::validate_input(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(&"Name is required".to_string()));
    }

    #[test]
    fn test_validate_input_password_mismatch() {
        let input = CreateUserInput {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            password: "SecurePass123".to_string(),
            password_confirmation: "DifferentPass123".to_string(),
            send_verification_email: true,
            invite_token: None,
        };

        let result = CreateUser::validate_input(&input);
        assert!(result.is_err());
    }
}
