//! Update Password Action
//!
//! Change a user's password (requires current password).

use crate::actions::{Action, ActionContext, ActionResult, ActionError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PasswordRules;

/// Update password action
pub struct UpdatePassword;

/// Input for updating password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordInput {
    /// User ID
    pub user_id: Uuid,
    /// Current password for verification
    pub current_password: String,
    /// New password
    pub new_password: String,
    /// New password confirmation
    pub new_password_confirmation: String,
    /// Invalidate other sessions after password change
    pub invalidate_sessions: bool,
}

/// Output after updating password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordOutput {
    /// User ID
    pub user_id: Uuid,
    /// Whether other sessions were invalidated
    pub sessions_invalidated: bool,
    /// Timestamp of password change
    pub changed_at: String,
}

impl Action for UpdatePassword {
    type Input = UpdatePasswordInput;
    type Output = UpdatePasswordOutput;

    fn name(&self) -> &'static str {
        "update_password"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> Result<Self::Output, ActionError> {
        // Validate new password
        if let Err(errors) = PasswordRules::validate(&input.new_password) {
            return Err(ActionError::new(
                "WEAK_PASSWORD",
                errors.join("; "),
            ));
        }

        // Confirm new password match
        if input.new_password != input.new_password_confirmation {
            return Err(ActionError::new(
                "PASSWORD_MISMATCH",
                "New password confirmation does not match",
            ));
        }

        // New password should be different from current
        if input.current_password == input.new_password {
            return Err(ActionError::new(
                "SAME_PASSWORD",
                "New password must be different from current password",
            ));
        }

        // Would:
        // 1. Fetch user's current password hash
        // 2. Verify current_password matches
        // 3. Hash new password
        // 4. Update user record
        // 5. Invalidate sessions if requested

        let now = chrono::Utc::now().to_rfc3339();

        Ok(UpdatePasswordOutput {
            user_id: input.user_id,
            sessions_invalidated: input.invalidate_sessions,
            changed_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_must_be_different() {
        let input = UpdatePasswordInput {
            user_id: Uuid::new_v4(),
            current_password: "OldPassword123".to_string(),
            new_password: "OldPassword123".to_string(),
            new_password_confirmation: "OldPassword123".to_string(),
            invalidate_sessions: true,
        };

        let action = UpdatePassword;
        let ctx = ActionContext::default();

        let result = action.execute(&ctx, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().code == "SAME_PASSWORD");
    }
}
