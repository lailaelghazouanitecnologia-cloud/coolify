//! Container Logs Action
//!
//! Retrieves logs from a Docker container.

use crate::actions::{Action, ActionContext, ActionResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Get container logs action
pub struct GetContainerLogs;

/// Input for getting container logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainerLogsInput {
    /// Server ID
    pub server_id: Uuid,
    /// Container name or ID
    pub container: String,
    /// Number of lines to return (tail)
    pub lines: Option<u32>,
    /// Since timestamp (ISO 8601)
    pub since: Option<String>,
    /// Until timestamp (ISO 8601)
    pub until: Option<String>,
    /// Include timestamps in output
    pub timestamps: bool,
    /// Filter by regex pattern
    pub filter: Option<String>,
}

/// Output of container logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContainerLogsOutput {
    /// Container name
    pub container: String,
    /// Log lines
    pub lines: Vec<LogLine>,
    /// Whether more logs are available
    pub has_more: bool,
}

/// A single log line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    /// Timestamp (if available)
    pub timestamp: Option<String>,
    /// Log stream (stdout/stderr)
    pub stream: LogStream,
    /// Log message
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

impl Action for GetContainerLogs {
    type Input = GetContainerLogsInput;
    type Output = GetContainerLogsOutput;

    fn name(&self) -> &'static str {
        "get_container_logs"
    }

    fn execute(
        &self,
        ctx: &ActionContext,
        input: &Self::Input,
    ) -> ActionResult<Self::Output> {
        // Would SSH to server and run:
        // docker logs <container> --tail N --timestamps

        Ok(GetContainerLogsOutput {
            container: input.container.clone(),
            lines: Vec::new(),
            has_more: false,
        })
    }
}

impl GetContainerLogs {
    /// Build the docker logs command
    pub fn build_command(input: &GetContainerLogsInput) -> String {
        let mut cmd = format!("docker logs {}", input.container);

        if let Some(n) = input.lines {
            cmd.push_str(&format!(" --tail {}", n));
        }

        if let Some(since) = &input.since {
            cmd.push_str(&format!(" --since '{}'", since));
        }

        if let Some(until) = &input.until {
            cmd.push_str(&format!(" --until '{}'", until));
        }

        if input.timestamps {
            cmd.push_str(" --timestamps");
        }

        cmd
    }

    /// Parse log output into LogLines
    pub fn parse_logs(output: &str, with_timestamps: bool) -> Vec<LogLine> {
        output
            .lines()
            .map(|line| {
                if with_timestamps {
                    // Format: 2024-01-01T12:00:00.000000000Z message
                    if let Some(space_idx) = line.find(' ') {
                        let ts = &line[..space_idx];
                        let msg = &line[space_idx + 1..];
                        LogLine {
                            timestamp: Some(ts.to_string()),
                            stream: LogStream::Stdout, // Would need docker logs --details for this
                            message: msg.to_string(),
                        }
                    } else {
                        LogLine {
                            timestamp: None,
                            stream: LogStream::Stdout,
                            message: line.to_string(),
                        }
                    }
                } else {
                    LogLine {
                        timestamp: None,
                        stream: LogStream::Stdout,
                        message: line.to_string(),
                    }
                }
            })
            .collect()
    }

    /// Filter logs by regex pattern
    pub fn filter_logs(logs: Vec<LogLine>, pattern: &str) -> Vec<LogLine> {
        if let Ok(regex) = regex::Regex::new(pattern) {
            logs.into_iter()
                .filter(|log| regex.is_match(&log.message))
                .collect()
        } else {
            logs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command() {
        let input = GetContainerLogsInput {
            server_id: Uuid::new_v4(),
            container: "my-container".to_string(),
            lines: Some(100),
            since: None,
            until: None,
            timestamps: true,
            filter: None,
        };

        let cmd = GetContainerLogs::build_command(&input);
        assert!(cmd.contains("docker logs my-container"));
        assert!(cmd.contains("--tail 100"));
        assert!(cmd.contains("--timestamps"));
    }

    #[test]
    fn test_parse_logs_with_timestamps() {
        let output = "2024-01-01T12:00:00.000Z Starting server\n2024-01-01T12:00:01.000Z Server ready";
        let logs = GetContainerLogs::parse_logs(output, true);

        assert_eq!(logs.len(), 2);
        assert!(logs[0].timestamp.is_some());
        assert_eq!(logs[0].message, "Starting server");
    }

    #[test]
    fn test_parse_logs_without_timestamps() {
        let output = "Line 1\nLine 2";
        let logs = GetContainerLogs::parse_logs(output, false);

        assert_eq!(logs.len(), 2);
        assert!(logs[0].timestamp.is_none());
    }
}
