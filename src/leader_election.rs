// Copyright 2025 The Kubernetes Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Leader election support.
//!
//! This module provides leader election functionality to ensure only one
//! instance of the controller manager is active at a time.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Configuration for leader election.
#[derive(Debug, Clone)]
pub struct LeaderElectionConfig {
    /// Resource lock type (e.g., "leases", "endpoints").
    pub resource_lock: String,

    /// Resource namespace.
    pub resource_namespace: String,

    /// Resource name (lease name).
    pub resource_name: String,

    /// Identity of this candidate (hostname + random ID).
    pub identity: String,

    /// Lease duration.
    pub lease_duration: Duration,

    /// Renew deadline.
    pub renew_deadline: Duration,

    /// Retry period.
    pub retry_period: Duration,
}

impl LeaderElectionConfig {
    /// Creates a new leader election configuration.
    pub fn new(
        resource_namespace: String,
        resource_name: String,
        identity: String,
    ) -> Self {
        Self {
            resource_lock: "leases".to_string(),
            resource_namespace,
            resource_name,
            identity,
            lease_duration: Duration::from_secs(15),
            renew_deadline: Duration::from_secs(10),
            retry_period: Duration::from_secs(2),
        }
    }

    /// Sets the lease duration.
    pub fn with_lease_duration(mut self, duration: Duration) -> Self {
        self.lease_duration = duration;
        self
    }

    /// Sets the renew deadline.
    pub fn with_renew_deadline(mut self, deadline: Duration) -> Self {
        self.renew_deadline = deadline;
        self
    }

    /// Sets the retry period.
    pub fn with_retry_period(mut self, period: Duration) -> Self {
        self.retry_period = period;
        self
    }

    /// Sets the resource lock type.
    pub fn with_resource_lock(mut self, lock_type: String) -> Self {
        self.resource_lock = lock_type;
        self
    }
}

/// Leader election result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderState {
    /// This instance is the leader.
    Leader,
    /// This instance is a follower.
    Follower,
    /// Leader election failed.
    Failed,
}

/// Callbacks for leader election events.
pub trait LeaderCallbacks: Send + Sync + 'static {
    /// Called when this instance becomes the leader.
    fn on_started_leading(&self);

    /// Called when this instance stops being the leader.
    fn on_stopped_leading(&self);
}

/// Function-based leader callbacks.
pub struct FunctionLeaderCallbacks {
    /// Called when leadership is acquired.
    pub on_started_leading: Arc<dyn Fn() + Send + Sync>,

    /// Called when leadership is lost.
    pub on_stopped_leading: Arc<dyn Fn() + Send + Sync>,
}

impl LeaderCallbacks for FunctionLeaderCallbacks {
    fn on_started_leading(&self) {
        (self.on_started_leading)();
    }

    fn on_stopped_leading(&self) {
        (self.on_stopped_leading)();
    }
}

/// Leader election interface.
///
/// Implementations provide different strategies for leader election.
#[async_trait::async_trait]
pub trait LeaderElection: Send + Sync + 'static {
    /// Runs the leader election loop.
    ///
    /// This method will:
    /// 1. Attempt to acquire leadership
    /// 2. Call `on_started_leading` if successful
    /// 3. Continuously renew the lease
    /// 4. Call `on_stopped_leading` if leadership is lost
    /// 5. Retry if the lease cannot be acquired/renewed
    async fn run(&self, callbacks: Arc<dyn LeaderCallbacks>) -> anyhow::Result<()>;

    /// Returns the current leader state.
    async fn state(&self) -> LeaderState;

    /// Stops the leader election.
    async fn stop(&self);
}

/// A simple leader election implementation using a lock file.
///
/// This is useful for testing and local development. In production,
/// you should use a Kubernetes-based implementation.
#[derive(Clone)]
pub struct LockFileLeaderElection {
    config: LeaderElectionConfig,
    state: Arc<RwLock<LeaderState>>,
    cancel: CancellationToken,
}

impl LockFileLeaderElection {
    /// Creates a new lock file-based leader election.
    pub fn new(config: LeaderElectionConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(LeaderState::Follower)),
            cancel: CancellationToken::new(),
        }
    }
}

#[async_trait::async_trait]
impl LeaderElection for LockFileLeaderElection {
    async fn run(&self, callbacks: Arc<dyn LeaderCallbacks>) -> anyhow::Result<()> {
        let lock_path = format!(
            "/tmp/kube-controller-manager-{}.lock",
            self.config.resource_name
        );

        tracing::info!(
            lock_path = %lock_path,
            identity = %self.config.identity,
            "starting leader election with lock file backend"
        );

        loop {
            // Check for cancellation
            if self.cancel.is_cancelled() {
                tracing::info!("leader election cancelled");
                return Ok(());
            }

            // Try to acquire the lock
            match self.try_acquire_lock(&lock_path).await {
                Ok(true) => {
                    // We are the leader
                    *self.state.write().await = LeaderState::Leader;
                    tracing::info!("acquired leadership");

                    // Notify listeners
                    tokio::spawn({
                        let callbacks = callbacks.clone();
                        async move {
                            callbacks.on_started_leading();
                        }
                    });

                    // Hold the lease
                    if let Err(e) = self.hold_lease(&lock_path, callbacks.clone()).await {
                        tracing::error!(error = %e, "lost leadership");
                        *self.state.write().await = LeaderState::Follower;
                        callbacks.on_stopped_leading();
                    }

                    // Wait before retrying
                    tokio::time::sleep(self.config.retry_period).await;
                }
                Ok(false) => {
                    // Not the leader
                    *self.state.write().await = LeaderState::Follower;
                    tokio::time::sleep(self.config.retry_period).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "leader election failed");
                    *self.state.write().await = LeaderState::Failed;
                    tokio::time::sleep(self.config.retry_period).await;
                }
            }
        }
    }

    async fn state(&self) -> LeaderState {
        *self.state.read().await
    }

    async fn stop(&self) {
        self.cancel.cancel();
    }
}

impl LockFileLeaderElection {
    /// Attempts to acquire the lock file.
    async fn try_acquire_lock(&self, path: &str) -> anyhow::Result<bool> {
        // Try to create the lock file exclusively
        match tokio::fs::File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .await
        {
            Ok(mut file) => {
                // Write our identity to the file
                use tokio::io::AsyncWriteExt;
                file.write_all(self.config.identity.as_bytes()).await?;
                file.sync_all().await?;
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Lock file exists, check if it's stale
                match tokio::fs::metadata(path).await {
                    Ok(metadata) => {
                        let modified = metadata.modified()?;
                        let elapsed = modified.elapsed()?;

                        // Consider stale if not modified for 2x lease duration
                        if elapsed > self.config.lease_duration * 2 {
                            // Try to remove the stale lock
                            let _ = tokio::fs::remove_file(path).await;
                            Ok(false)
                        } else {
                            Ok(false)
                        }
                    }
                    Err(_) => Ok(false),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Holds the lease by periodically updating the lock file.
    async fn hold_lease(
        &self,
        path: &str,
        callbacks: Arc<dyn LeaderCallbacks>,
    ) -> anyhow::Result<()> {
        loop {
            // Check for cancellation
            if self.cancel.is_cancelled() {
                return Ok(());
            }

            // Renew the lease
            tokio::time::sleep(self.config.renew_deadline / 2).await;

            // Try to update the lock file
            match tokio::fs::write(path, self.config.identity.as_bytes()).await {
                Ok(_) => continue,
                Err(e) => {
                    tracing::error!(error = %e, "failed to renew lease");
                    return Err(e.into());
                }
            }
        }
    }
}

/// Creates a default identity for leader election.
///
/// Uses the hostname plus a random component.
pub fn create_identity() -> anyhow::Result<String> {
    let hostname = gethostname::gethostname()
        .into_string()
        .unwrap_or_else(|_| "unknown".to_string());

    let random: String = std::iter::repeat_with(|| {
        rand::random::<u8>()
    })
    .take(4)
    .map(|b| format!("{:02x}", b))
    .collect();

    Ok(format!("{}_{}", hostname, random))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leader_election_config() {
        let config = LeaderElectionConfig::new(
            "kube-system".to_string(),
            "kube-controller-manager".to_string(),
            "test-identity".to_string(),
        );

        assert_eq!(config.resource_namespace, "kube-system");
        assert_eq!(config.resource_name, "kube-controller-manager");
        assert_eq!(config.identity, "test-identity");
        assert_eq!(config.lease_duration, Duration::from_secs(15));
    }

    #[test]
    fn test_create_identity() {
        let identity = create_identity().unwrap();
        assert!(identity.contains('_'));
        assert!(identity.len() > 10);
    }
}
