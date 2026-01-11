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

//! Core controller traits and types.
//!
//! This module defines the base interface that all controllers must implement.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use thiserror::Error;

// Re-export cancellation token for use in controller interface
pub use tokio_util::sync::CancellationToken;

// Import ControllerContext for use in trait methods
use crate::controller_context::ControllerContext;

/// Errors that can occur when running a controller.
#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("controller {name} failed to start: {source}")]
    StartFailed {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("controller {name} runtime error: {source}")]
    Runtime {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("controller {0} is disabled")]
    Disabled(String),

    #[error("controller {0} requires feature gate {1} which is not enabled")]
    FeatureGateDisabled(String, String),

    #[error("invalid configuration for controller {name}: {reason}")]
    InvalidConfig { name: String, reason: String },
}

/// Result type for controller operations.
pub type Result<T> = std::result::Result<T, ControllerError>;

impl From<anyhow::Error> for ControllerError {
    fn from(err: anyhow::Error) -> Self {
        ControllerError::Runtime {
            name: "unknown".to_string(),
            source: err.into(),
        }
    }
}

/// The base trait that all controllers must implement.
///
/// A controller watches the shared state of the cluster through the apiserver
/// and makes changes attempting to move the current state towards the desired state.
///
/// # Lifecycle
///
/// 1. The controller is created via [`ControllerDescriptor::build_controller`]
/// 2. [`Controller::run`] is called with a cancellation token
/// 3. The controller runs until the token is cancelled or an error occurs
/// 4. [`Controller::shutdown`] is called to clean up resources
///
/// # Example
///
/// ```rust
/// use async_trait::async_trait;
/// use kube_controller_manager::controller::{Controller, ControllerContext};
/// use tokio::util::CancellationToken;
///
/// struct MyController;
///
/// #[async_trait]
/// impl Controller for MyController {
///     fn name(&self) -> &str {
///         "my-controller"
///     }
///
///     async fn run(&self, ctx: ControllerContext, cancel: CancellationToken) -> controller::Result<()> {
///         // Main controller loop
///         loop {
///             tokio::select! {
///                 _ = cancel.cancelled() => {
///                     tracing::info!("controller shutting down");
///                     return Ok(());
///                 }
///                 _ = self.do_work() => {
///                     // Continue working
///                 }
///             }
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait Controller: Send + Sync + 'static {
    /// Returns the canonical name of this controller.
    ///
    /// This name is used for:
    /// - Logging
    /// - Metrics
    /// - Health checks
    /// - Leader election locks
    fn name(&self) -> &str;

    /// Runs the controller's main loop.
    ///
    /// This method should block until:
    /// - The cancellation token is triggered
    /// - A fatal error occurs
    ///
    /// When the token is cancelled, the controller should gracefully shut down
    /// and return `Ok(())`.
    ///
    /// # Parameters
    ///
    /// - `ctx`: Shared controller context containing clients, informers, and configuration
    /// - `cancel`: Token that signals when the controller should stop
    async fn run(
        &self,
        ctx: ControllerContext,
        cancel: CancellationToken,
    ) -> Result<()>;

    /// Optional health check for the controller.
    ///
    /// The default implementation always returns `true`. Controllers that
    /// track their health status should override this.
    ///
    /// This is called periodically by the health check server to determine
    /// if the controller is functioning correctly.
    fn health_check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async { true })
    }

    /// Optional graceful shutdown hook.
    ///
    /// Called after the main loop exits but before the controller is
    /// considered fully stopped. Use this to clean up resources.
    ///
    /// The default implementation does nothing.
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    /// Returns the number of worker threads this controller uses.
    ///
    /// This is used for logging and metrics. The default is 1.
    fn worker_count(&self) -> usize {
        1
    }
}

/// A wrapper that converts a function into a [`Controller`].
///
/// This is useful for simple controllers that don't need to maintain
/// any state between runs.
///
/// # Example
///
/// ```rust
/// use kube_controller_manager::controller::FunctionController;
///
/// let controller = FunctionController::new(
///     "simple-controller",
///     |ctx, cancel| async move {
///         // Controller logic here
///         Ok(())
///     }
/// );
/// ```
pub struct FunctionController<F, Fut>
where
    F: Fn(ControllerContext, CancellationToken) -> Fut + Send + Sync,
    Fut: Future<Output = Result<()>> + Send,
{
    name: String,
    run_fn: F,
    worker_count: usize,
}

impl<F, Fut> FunctionController<F, Fut>
where
    F: Fn(ControllerContext, CancellationToken) -> Fut + Send + Sync,
    Fut: Future<Output = Result<()>> + Send,
{
    /// Creates a new function-based controller.
    pub fn new(name: impl Into<String>, run_fn: F) -> Self {
        Self {
            name: name.into(),
            run_fn,
            worker_count: 1,
        }
    }

    /// Sets the worker count for this controller.
    pub fn with_worker_count(mut self, count: usize) -> Self {
        self.worker_count = count;
        self
    }
}

impl<F, Fut> fmt::Debug for FunctionController<F, Fut>
where
    F: Fn(ControllerContext, CancellationToken) -> Fut + Send + Sync,
    Fut: Future<Output = Result<()>> + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionController")
            .field("name", &self.name)
            .field("worker_count", &self.worker_count)
            .finish()
    }
}

#[async_trait]
impl<F, Fut> Controller for FunctionController<F, Fut>
where
    F: Fn(ControllerContext, CancellationToken) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        ctx: ControllerContext,
        cancel: CancellationToken,
    ) -> Result<()> {
        (self.run_fn)(ctx, cancel).await
    }

    fn worker_count(&self) -> usize {
        self.worker_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_controller_name() {
        let controller = FunctionController::new("test", |_, _| async { Ok(()) });
        assert_eq!(controller.name(), "test");
    }

    #[test]
    fn test_function_controller_worker_count() {
        let controller = FunctionController::new("test", |_, _| async { Ok(()) })
            .with_worker_count(5);
        assert_eq!(controller.worker_count(), 5);
    }
}
