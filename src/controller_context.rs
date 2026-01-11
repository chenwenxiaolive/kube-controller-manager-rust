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

//! Controller context - shared resources for all controllers.

use std::sync::Arc;
use std::time::Duration;

use kube::Client;

use crate::config::ControllerManagerConfig;

// Re-export for use in resync_period_fn
pub use rand;

// Import Rng trait for gen_range
use rand::Rng;

/// Shared context for all controllers.
///
/// This struct contains references to shared resources that controllers
/// need to operate, such as the Kubernetes client, informers, and configuration.
#[derive(Clone)]
pub struct ControllerContext {
    /// Kubernetes client for making API requests.
    pub client: Client,

    /// Shared informer factory for typed resources.
    pub informer_factory: Arc<InformerFactory>,

    /// Configuration for this controller manager instance.
    pub config: Arc<ControllerManagerConfig>,

    /// Function to generate resync periods.
    ///
    /// This is randomized per controller to avoid all controllers
    /// hitting the API server at the same time.
    pub resync_period_fn: Arc<dyn Fn() -> Duration + Send + Sync>,

    /// Channel that signals when all informers have been started.
    pub informers_started: Arc<tokio::sync::Notify>,
}

impl ControllerContext {
    /// Creates a new controller context.
    pub fn new(
        client: Client,
        informer_factory: Arc<InformerFactory>,
        config: Arc<ControllerManagerConfig>,
        resync_period_fn: Arc<dyn Fn() -> Duration + Send + Sync>,
    ) -> Self {
        Self {
            client,
            informer_factory,
            config,
            resync_period_fn,
            informers_started: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Returns a resync period for a controller.
    pub fn resync_period(&self) -> Duration {
        (self.resync_period_fn)()
    }

    /// Checks if a controller with the given name is enabled.
    pub fn is_controller_enabled(&self, name: &str) -> bool {
        let controllers = &self.config.generic.controllers;

        if controllers == "*" {
            // Check if it's explicitly disabled
            return !self.config.generic.controllers_disabled.contains(name);
        }

        // Check if it's explicitly enabled
        if self.config.generic.controllers_enabled.contains(name) {
            return true;
        }

        false
    }

    /// Waits for all informers to be started.
    pub async fn wait_for_informers(&self) {
        self.informers_started.notified().await;
    }

    /// Notifies that informers have been started.
    pub fn notify_informers_started(&self) {
        self.informers_started.notify_waiters();
    }
}

/// Factory for creating shared informers.
///
/// Informers watch Kubernetes resources and cache them locally.
/// This type provides a centralized way to create and manage informers.
///
/// # TODO
///
/// This is a placeholder. A full implementation would provide:
/// - Typed informers for each Kubernetes resource type
/// - Metadata-only informers for dynamic resources
/// - Lifecycle management (start, stop, sync)
/// - Event handlers
#[derive(Clone, Debug)]
pub struct InformerFactory;

impl InformerFactory {
    /// Creates a new informer factory.
    pub fn new() -> Self {
        Self
    }

    /// Returns a watcher for the given resource type.
    ///
    /// # TODO
    ///
    /// This is a placeholder. A full implementation would:
    /// - Track created informers
    /// - Handle resource types dynamically
    /// - Support event filters and transformers
    pub async fn watcher<K>(&self) -> anyhow::Result<()>
    where
        K: Clone + serde::de::DeserializeOwned + std::fmt::Debug + 'static,
    {
        // TODO: Implement actual informer creation
        Ok(())
    }

    /// Starts all informers.
    ///
    /// # TODO
    ///
    /// This is a placeholder. A full implementation would:
    /// - Start all tracked informers
    /// - Wait for initial sync
    /// - Handle errors
    pub async fn start(&self) -> anyhow::Result<()> {
        // TODO: Start all informers
        Ok(())
    }

    /// Returns true if the informers have been started.
    pub fn started(&self) -> bool {
        // TODO: Track started state
        false
    }
}

impl Default for InformerFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a resync period function with the given base duration.
///
/// The returned function adds random jitter to avoid all controllers
/// syncing at the same time.
pub fn resync_period_fn(base: Duration) -> impl Fn() -> Duration + Send + Sync {
    move || {
        // Create a new RNG each time (simpler than trying to share ThreadRng)
        let mut rng = rand::thread_rng();
        // Add between 0 and 100% jitter
        let jitter = rng.gen_range(0.0..1.0);
        Duration::from_secs_f64(base.as_secs_f64() * (1.0 + jitter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resync_period_fn() {
        let base = Duration::from_secs(60);
        let fn_resync = resync_period_fn(base);

        // Call multiple times and check we get varying values
        let mut periods = std::collections::HashSet::new();
        for _ in 0..10 {
            periods.insert(fn_resync());
        }

        // Should have gotten different values due to jitter
        assert!(periods.len() > 1);
    }
}
