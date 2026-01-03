//! SSH Connection Pool and Multiplexing
//!
//! Provides connection pooling to reuse SSH connections across multiple
//! operations, reducing connection overhead for deployments.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{Result, SshError};
use crate::session::SshSession;
use crate::retry::{RetryPolicy, with_retry};

/// Configuration for the connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum connections per server
    pub max_connections_per_server: usize,
    /// Maximum total connections
    pub max_total_connections: usize,
    /// Idle timeout before connection is closed
    pub idle_timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Retry policy for connections
    pub retry_policy: RetryPolicy,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_server: 5,
            max_total_connections: 100,
            idle_timeout: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
            retry_policy: RetryPolicy::connection(),
        }
    }
}

/// A pooled SSH connection
struct PooledConnection {
    session: Arc<SshSession>,
    created_at: Instant,
    last_used: RwLock<Instant>,
    in_use: bool,
}

impl PooledConnection {
    fn new(session: SshSession) -> Self {
        let now = Instant::now();
        Self {
            session: Arc::new(session),
            created_at: now,
            last_used: RwLock::new(now),
            in_use: false,
        }
    }

    fn touch(&self) {
        *self.last_used.write() = Instant::now();
    }

    fn idle_time(&self) -> Duration {
        self.last_used.read().elapsed()
    }

    fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.idle_time() > idle_timeout
    }
}

/// Server connection key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerKey {
    pub ip: String,
    pub port: u16,
    pub user: String,
}

impl ServerKey {
    pub fn new(ip: impl Into<String>, port: u16, user: impl Into<String>) -> Self {
        Self {
            ip: ip.into(),
            port,
            user: user.into(),
        }
    }
}

impl From<&kornetti_core::models::Server> for ServerKey {
    fn from(server: &kornetti_core::models::Server) -> Self {
        Self {
            ip: server.ip.clone(),
            port: server.port,
            user: server.user.clone(),
        }
    }
}

/// Connection pool for SSH connections
pub struct ConnectionPool {
    config: PoolConfig,
    connections: DashMap<ServerKey, Vec<Arc<RwLock<PooledConnection>>>>,
    total_semaphore: Arc<Semaphore>,
    server_semaphores: DashMap<ServerKey, Arc<Semaphore>>,
    private_keys: DashMap<Uuid, String>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        let total_semaphore = Arc::new(Semaphore::new(config.max_total_connections));

        Self {
            config,
            connections: DashMap::new(),
            total_semaphore,
            server_semaphores: DashMap::new(),
            private_keys: DashMap::new(),
        }
    }

    /// Register a private key for a server
    pub fn register_key(&self, key_id: Uuid, private_key: String) {
        self.private_keys.insert(key_id, private_key);
    }

    /// Get a connection from the pool, creating one if necessary
    pub async fn get_connection(
        &self,
        server: &kornetti_core::models::Server,
    ) -> Result<PooledConnectionGuard> {
        let key = ServerKey::from(server);

        // Acquire semaphore permit
        let _total_permit = self.total_semaphore.clone()
            .acquire_owned()
            .await
            .map_err(|_| SshError::PoolExhausted)?;

        // Try to get an existing idle connection
        if let Some(conn) = self.get_idle_connection(&key) {
            debug!(server = %key.ip, "Reusing pooled connection");
            conn.touch();
            return Ok(PooledConnectionGuard {
                connection: conn,
                key,
                pool: self,
            });
        }

        // Create a new connection with retry
        debug!(server = %key.ip, "Creating new pooled connection");

        let private_key = server.private_key_id
            .and_then(|id| self.private_keys.get(&id).map(|k| k.clone()));

        let session = with_retry(
            &self.config.retry_policy,
            "ssh_connect",
            || async {
                SshSession::connect(
                    &server.ip,
                    server.port,
                    &server.user,
                    private_key.as_deref(),
                    self.config.connect_timeout,
                ).await
            },
        ).await?;

        let pooled = Arc::new(RwLock::new(PooledConnection::new(session)));

        // Add to pool
        self.connections
            .entry(key.clone())
            .or_insert_with(Vec::new)
            .push(pooled.clone());

        info!(server = %key.ip, "New SSH connection established");

        Ok(PooledConnectionGuard {
            connection: pooled,
            key,
            pool: self,
        })
    }

    /// Get an idle connection from the pool
    fn get_idle_connection(&self, key: &ServerKey) -> Option<Arc<RwLock<PooledConnection>>> {
        if let Some(mut conns) = self.connections.get_mut(key) {
            // Remove expired connections
            conns.retain(|c| {
                let conn = c.read();
                !conn.is_expired(self.config.idle_timeout)
            });

            // Find an idle connection
            for conn in conns.iter() {
                let mut pooled = conn.write();
                if !pooled.in_use {
                    pooled.in_use = true;
                    return Some(conn.clone());
                }
            }
        }
        None
    }

    /// Return a connection to the pool
    fn return_connection(&self, key: &ServerKey, conn: Arc<RwLock<PooledConnection>>) {
        let mut pooled = conn.write();
        pooled.in_use = false;
        pooled.touch();
        debug!(server = %key.ip, "Connection returned to pool");
    }

    /// Clean up idle connections
    pub async fn cleanup(&self) {
        let mut total_removed = 0;

        for mut entry in self.connections.iter_mut() {
            let before = entry.value().len();
            entry.value_mut().retain(|c| {
                let conn = c.read();
                !conn.is_expired(self.config.idle_timeout) || conn.in_use
            });
            total_removed += before - entry.value().len();
        }

        // Remove empty entries
        self.connections.retain(|_, v| !v.is_empty());

        if total_removed > 0 {
            info!(removed = total_removed, "Cleaned up idle SSH connections");
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let mut total = 0;
        let mut idle = 0;
        let mut in_use = 0;

        for entry in self.connections.iter() {
            for conn in entry.value() {
                total += 1;
                if conn.read().in_use {
                    in_use += 1;
                } else {
                    idle += 1;
                }
            }
        }

        PoolStats {
            total_connections: total,
            idle_connections: idle,
            in_use_connections: in_use,
            servers_connected: self.connections.len(),
        }
    }
}

/// Guard that automatically returns connection to pool on drop
pub struct PooledConnectionGuard<'a> {
    connection: Arc<RwLock<PooledConnection>>,
    key: ServerKey,
    pool: &'a ConnectionPool,
}

impl<'a> PooledConnectionGuard<'a> {
    /// Get the underlying session
    pub fn session(&self) -> Arc<SshSession> {
        self.connection.read().session.clone()
    }
}

impl<'a> Drop for PooledConnectionGuard<'a> {
    fn drop(&mut self) {
        self.pool.return_connection(&self.key, self.connection.clone());
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub idle_connections: usize,
    pub in_use_connections: usize,
    pub servers_connected: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_key_equality() {
        let key1 = ServerKey::new("192.168.1.1", 22, "root");
        let key2 = ServerKey::new("192.168.1.1", 22, "root");
        let key3 = ServerKey::new("192.168.1.2", 22, "root");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections_per_server, 5);
        assert_eq!(config.max_total_connections, 100);
    }
}
