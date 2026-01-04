//! Reset Password Action
//!
//! Handle password reset flow (forgot password).

use crate::actions::{Action, ActionContext, ActionResult, ActionError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{EmailValidator, PasswordRules, TokenGenerator};

// ============================================================================
// Request Password Reset
// ============================================================================

/// Request password reset action (send email)
pub struct RequestPasswordReset;

/// Input for requesting password reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetInput {
    /// Email address
    pub email: String,
}

/// Output after requesting reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetOutput {
    /// Always true for security (don't reveal if email exists)
    pub email_sent: bool,
    /// Message to display
    pub message: String,
}

impl Action for RequestPasswordReset {
    type Input = RequestPasswordResetInput;
    type Output = RequestPasswordResetOutput;

    fn name(&self) -> &'static str {
        "request_password_reset"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        let email = EmailValidator::normalize(&input.email);

        // Validate email format
        if !EmailValidator::validate(&email) {
            return Err(ActionError::new("INVALID_EMAIL", "Invalid email address format"));
        }

        // Would:
        // 1. Check if email exists in database
        // 2. Generate reset token
        // 3. Store token with expiration
        // 4. Send email with reset link
        // 5. Always return success (security - don't reveal if email exists)

        let _token = TokenGenerator::password_reset_token();

        Ok(RequestPasswordResetOutput {
            email_sent: true,
            message: "If an account exists with that email, a password reset link has been sent.".to_string(),
        })
    }
}

// ============================================================================
// Verify Reset Token
// ============================================================================

/// Verify reset token action
pub struct VerifyResetToken;

/// Input for verifying reset token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResetTokenInput {
    /// Reset token from email
    pub token: String,
    /// Email address (optional, for additional verification)
    pub email: Option<String>,
}

/// Output of token verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResetTokenOutput {
    /// Whether token is valid
    pub valid: bool,
    /// User ID if valid
    pub user_id: Option<Uuid>,
    /// When token expires
    pub expires_at: Option<String>,
}

impl Action for VerifyResetToken {
    type Input = VerifyResetTokenInput;
    type Output = VerifyResetTokenOutput;

    fn name(&self) -> &'static str {
        "verify_reset_token"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        if input.token.len() < 32 {
            return Ok(VerifyResetTokenOutput {
                valid: false,
                user_id: None,
                expires_at: None,
            });
        }

        // Would:
        // 1. Look up token in database
        // 2. Check expiration
        // 3. Optionally verify email matches
        // 4. Return validity status

        Ok(VerifyResetTokenOutput {
            valid: true,
            user_id: Some(Uuid::new_v4()),
            expires_at: Some(chrono::Utc::now().to_rfc3339()),
        })
    }
}

// ============================================================================
// Complete Password Reset
// ============================================================================

/// Complete password reset action
pub struct CompletePasswordReset;

/// Input for completing password reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePasswordResetInput {
    /// Reset token
    pub token: String,
    /// Email address
    pub email: String,
    /// New password
    pub password: String,
    /// New password confirmation
    pub password_confirmation: String,
}

/// Output after completing reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePasswordResetOutput {
    /// User ID
    pub user_id: Uuid,
    /// Whether reset was successful
    pub success: bool,
    /// Whether sessions were invalidated
    pub sessions_invalidated: bool,
}

impl Action for CompletePasswordReset {
    type Input = CompletePasswordResetInput;
    type Output = CompletePasswordResetOutput;

    fn name(&self) -> &'static str {
        "complete_password_reset"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
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

        // Validate email
        let email = EmailValidator::normalize(&input.email);
        if !EmailValidator::validate(&email) {
            return Err(ActionError::new("INVALID_EMAIL", "Invalid email address"));
        }

        // Would:
        // 1. Verify token is valid and not expired
        // 2. Verify email matches token
        // 3. Hash new password
        // 4. Update user record
        // 5. Invalidate all sessions
        // 6. Delete reset token

        Ok(CompletePasswordResetOutput {
            user_id: Uuid::new_v4(),
            success: true,
            sessions_invalidated: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_reset_invalid_email() {
        let action = RequestPasswordReset;
        let ctx = ActionContext::default();
        let input = RequestPasswordResetInput {
            email: "invalid-email".to_string(),
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_reset_success() {
        let action = RequestPasswordReset;
        let ctx = ActionContext::default();
        let input = RequestPasswordResetInput {
            email: "user@example.com".to_string(),
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_ok());
        assert!(result.unwrap().email_sent);
    }

    #[test]
    fn test_verify_short_token() {
        let action = VerifyResetToken;
        let ctx = ActionContext::default();
        let input = VerifyResetTokenInput {
            token: "short".to_string(),
            email: None,
        };

        let result = action.execute(&ctx, &input);
        assert!(result.is_ok());
        assert!(!result.unwrap().valid);
    }
}
