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

//! Controller manager - the main orchestrator.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::config::ControllerManagerConfig;
use crate::controller::{Controller, ControllerError, Result};
use crate::controller_context::{resync_period_fn, ControllerContext, InformerFactory};
use crate::controller_descriptor::{ControllerDescriptor, ControllerRegistry};
use crate::feature::FeatureGate;

/// The controller manager.
///
/// This is the main orchestrator that:
/// 1. Manages the lifecycle of all controllers
/// 2. Provides shared resources (client, informers, config)
/// 3. Handles graceful shutdown
///
/// # Example
///
/// ```rust,no_run
/// use kube_controller_manager::controller_manager::ControllerManager;
/// use kube_controller_manager::config::ControllerManagerConfig;
/// use kube::Client;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ControllerManagerConfig::default();
///     let client = Client::try_default().await?;
///     let manager = ControllerManager::new(config, client);
///     manager.run().await?;
///     Ok(())
/// }
/// ```
pub struct ControllerManager {
    /// Configuration for this instance.
    config: Arc<ControllerManagerConfig>,

    /// Kubernetes client.
    client: kube::Client,

    /// Shared informer factory.
    informer_factory: Arc<InformerFactory>,

    /// Registry of all available controllers.
    registry: ControllerRegistry,

    /// Feature gate implementation.
    feature_gate: Arc<dyn FeatureGate>,

    /// Root cancellation token for shutdown.
    shutdown_token: CancellationToken,

    /// Controllers that are currently running.
    running_controllers: Arc<tokio::sync::RwLock<HashSet<String>>>,
}

impl ControllerManager {
    /// Creates a new controller manager.
    pub fn new(config: ControllerManagerConfig, client: kube::Client) -> Self {
        // Convert feature slice to owned tuples
        let features: Vec<(&'static str, bool, bool, bool)> =
            crate::feature::kubernetes::default_features().to_vec();

        Self::with_feature_gate(
            config,
            client,
            Arc::new(crate::feature::MemoryFeatureGate::new(features)),
        )
    }

    /// Creates a new controller manager with a custom feature gate.
    pub fn with_feature_gate(
        config: ControllerManagerConfig,
        client: kube::Client,
        feature_gate: Arc<dyn FeatureGate>,
    ) -> Self {
        let shutdown_token = CancellationToken::new();
        let informer_factory = Arc::new(InformerFactory::new());
        let registry = Self::build_registry();

        Self {
            config: Arc::new(config),
            client,
            informer_factory,
            registry,
            feature_gate,
            shutdown_token,
            running_controllers: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
        }
    }

    /// Builds the default controller registry.
    ///
    /// This registers all known controllers. In a full implementation,
    /// this would call registration functions from each controller module.
    fn build_registry() -> ControllerRegistry {
        let mut registry = ControllerRegistry::new();

        // TODO: Register all controllers
        // For now, this is a placeholder. In a full implementation,
        // we would register controllers like:
        // - endpoints::register(&mut registry);
        // - deployment::register(&mut registry);
        // - etc.

        registry
    }

    /// Adds a controller descriptor to the registry.
    ///
    /// This allows external callers to register custom controllers.
    pub fn register_controller(&mut self, descriptor: ControllerDescriptor) -> &mut Self {
        self.registry.register(descriptor);
        self
    }

    /// Returns the shutdown cancellation token.
    ///
    /// Controllers can use this to listen for shutdown signals.
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    /// Runs the controller manager.
    ///
    /// This will:
    /// 1. Build the controller context
    /// 2. Build all enabled controllers
    /// 3. Start the informers
    /// 4. Run all controllers concurrently
    /// 5. Wait for shutdown signal or errors
    pub async fn run(&self) -> Result<()> {
        tracing::info!("starting kube-controller-manager");

        // Build controller context
        let ctx = self.build_controller_context();

        // Build all enabled controllers
        let controllers = self.build_controllers(ctx.clone()).await?;

        if controllers.is_empty() {
            tracing::warn!("no controllers enabled, exiting");
            return Ok(());
        }

        tracing::info!(
            "built {} controllers",
            controllers.len()
        );

        // Start informers
        if let Err(e) = self.informer_factory.start().await {
            tracing::error!("failed to start informers: {:#}", e);
            return Err(ControllerError::StartFailed {
                name: "informers".to_string(),
                source: e.into(),
            });
        }

        // Notify that informers are ready
        ctx.notify_informers_started();

        // Run all controllers
        self.run_controllers(ctx, controllers).await?;

        Ok(())
    }

    /// Builds the controller context.
    fn build_controller_context(&self) -> ControllerContext {
        let resync_fn = Arc::new(resync_period_fn(self.config.generic.min_resync_period));

        ControllerContext::new(
            self.client.clone(),
            self.informer_factory.clone(),
            self.config.clone(),
            resync_fn,
        )
    }

    /// Builds all enabled controllers.
    async fn build_controllers(
        &self,
        ctx: ControllerContext,
    ) -> Result<Vec<Arc<dyn Controller>>> {
        let mut controllers = Vec::new();

        for descriptor in self.registry.controllers() {
            let name = descriptor.name();

            // Check if controller is enabled
            if !ctx.is_controller_enabled(name) {
                tracing::debug!(controller = %name, "controller is disabled");
                continue;
            }

            // Build the controller
            let cancel = self.shutdown_token.clone();
            match descriptor.build_controller(ctx.clone(), cancel, self.feature_gate.as_ref()).await {
                Ok(Some(controller)) => {
                    tracing::info!(controller = %name, "built controller");
                    controllers.push(controller);
                }
                Ok(None) => {
                    tracing::debug!(controller = %name, "controller not built (disabled or no resources)");
                }
                Err(e) => {
                    tracing::error!(controller = %name, error = %e, "failed to build controller");
                    return Err(ControllerError::StartFailed {
                        name: name.to_string(),
                        source: Box::new(e),
                    });
                }
            }
        }

        Ok(controllers)
    }

    /// Runs all controllers concurrently.
    async fn run_controllers(
        &self,
        ctx: ControllerContext,
        controllers: Vec<Arc<dyn Controller>>,
    ) -> Result<()> {
        let start_interval = self.config.generic.controller_start_interval;
        let jitter_max = 1.0; // Maximum jitter factor

        let mut join_set = JoinSet::new();
        let mut running = self.running_controllers.write().await;

        for controller in controllers {
            let name = controller.name().to_string();
            let ctx_clone = ctx.clone();
            let cancel = self.shutdown_token.clone();
            let running_clone = self.running_controllers.clone();

            // Add jitter to start time
            let jitter = rand::random::<f64>() * jitter_max;
            let delay = Duration::from_secs_f64(
                start_interval.as_secs_f64() * (1.0 + jitter)
            );

            // Spawn the controller task
            join_set.spawn(async move {
                // Wait for jitter delay
                tokio::time::sleep(delay).await;

                tracing::info!(controller = %name, "starting controller");

                // Track as running
                {
                    let mut running = running_clone.write().await;
                    running.insert(name.clone());
                }

                // Run the controller
                let result = controller.run(ctx_clone, cancel).await;

                // Remove from running
                {
                    let mut running = running_clone.write().await;
                    running.remove(&name);
                }

                // Run shutdown hook
                let _ = controller.shutdown().await;

                match result {
                    Ok(()) => {
                        tracing::info!(controller = %name, "controller terminated successfully");
                    }
                    Err(ref e) => {
                        tracing::error!(controller = %name, error = %e, "controller terminated with error");
                    }
                }

                (name, result)
            });
        }

        drop(running);

        // Wait for shutdown signal
        tokio::select! {
            // Wait for all controllers to finish
            result = async {
                while let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((name, Ok(()))) => {
                            tracing::debug!(controller = %name, "controller finished");
                        }
                        Ok((name, Err(e))) => {
                            tracing::error!(controller = %name, error = %e, "controller failed");
                            return Err::<(), _>(ControllerError::Runtime {
                                name,
                                source: e.into(),
                            });
                        }
                        Err(e) => {
                            if e.is_panic() {
                                tracing::error!("controller task panicked");
                            }
                            return Err(ControllerError::Runtime {
                                name: "unknown".to_string(),
                                source: e.into(),
                            });
                        }
                    }
                }
                Ok(())
            } => {
                result?;
            }

            // Wait for shutdown signal
            _ = self.shutdown_token.cancelled() => {
                tracing::info!("shutdown signal received, waiting for controllers to stop");
            }
        }

        // Wait for remaining controllers with timeout
        let shutdown_timeout = self.config.generic.shutdown_timeout;
        let deadline = tokio::time::Instant::now() + shutdown_timeout;

        while !join_set.is_empty() {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());

            if remaining.is_zero() {
                // Log still-running controllers
                let running = self.running_controllers.read().await;
                if !running.is_empty() {
                    tracing::warn!(
                        controllers = ?running.iter().collect::<Vec<_>>(),
                        "shutdown timeout reached, controllers still running"
                    );
                }
                break;
            }

            tokio::select! {
                result = join_set.join_next() => {
                    match result {
                        Some(Ok((name, Ok(())))) => {
                            tracing::debug!(controller = %name, "controller stopped");
                        }
                        Some(Ok((name, Err(e)))) => {
                            tracing::warn!(controller = %name, error = %e, "controller stopped with error");
                        }
                        Some(Err(e)) => {
                            tracing::warn!(error = %e, "controller task failed");
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Periodic logging of still-running controllers
                    let running = self.running_controllers.read().await;
                    if !running.is_empty() {
                        tracing::debug!(
                            controllers = ?running.iter().collect::<Vec<_>>(),
                            "still waiting for controllers"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Initiates a graceful shutdown.
    pub fn shutdown(&self) {
        tracing::info!("initiating graceful shutdown");
        self.shutdown_token.cancel();
    }

    /// Returns the controller registry.
    pub fn registry(&self) -> &ControllerRegistry {
        &self.registry
    }

    /// Returns the feature gate.
    pub fn feature_gate(&self) -> &Arc<dyn FeatureGate> {
        &self.feature_gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_manager_creation() {
        let config = ControllerManagerConfig::default();
        // Can't create a real client in tests, so we'll just test the config
        assert_eq!(config.generic.controllers, "*");
    }
}
