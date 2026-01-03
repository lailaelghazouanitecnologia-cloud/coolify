//! Retry logic with exponential backoff

use std::time::Duration;
use tracing::{debug, warn};

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a policy for connection retries
    pub fn connection() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a policy for command execution retries
    pub fn command() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a policy that never retries
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Calculate delay for a given attempt number
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1);

        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64);

        let delay_ms = if self.jitter {
            let jitter_range = delay_ms * 0.1;
            delay_ms + (rand::random::<f64>() * jitter_range * 2.0 - jitter_range)
        } else {
            delay_ms
        };

        Duration::from_millis(delay_ms as u64)
    }
}

/// Execute an async operation with retry logic
pub async fn with_retry<F, Fut, T, E>(
    policy: &RetryPolicy,
    operation_name: &str,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut last_error: Option<E> = None;

    loop {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                attempt += 1;

                if attempt > policy.max_retries {
                    warn!(
                        operation = operation_name,
                        attempts = attempt,
                        error = %e,
                        "Max retries exceeded"
                    );
                    return Err(e);
                }

                let delay = policy.delay_for_attempt(attempt);
                warn!(
                    operation = operation_name,
                    attempt = attempt,
                    max_retries = policy.max_retries,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Operation failed, retrying"
                );

                last_error = Some(e);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Determines if an error is retryable
pub trait Retryable {
    fn is_retryable(&self) -> bool;
}

impl Retryable for crate::error::SshError {
    fn is_retryable(&self) -> bool {
        use crate::error::SshError::*;
        matches!(
            self,
            ConnectionFailed(_) | SessionError(_) | ChannelError(_) | Timeout(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            jitter: false,
            ..Default::default()
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(30)); // Capped at max
    }
}
