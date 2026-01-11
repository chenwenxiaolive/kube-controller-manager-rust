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

//! Health check support.
//!
//! This module provides health check endpoints for the controller manager.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hyper::{Request, Response};
use hyper::body::Incoming;
use hyper::http::StatusCode as HttpStatusCode;
use hyper::body::Bytes;
use hyper::service::service_fn;
use http_body_util::Full;
use serde::Serialize;
use serde_json;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use hyper_util::rt::TokioIo;

/// Boxed future for health checks.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Health check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// The component is healthy.
    Healthy,
    /// The component is unhealthy.
    Unhealthy,
}

/// Health check result.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    /// The overall health status.
    pub status: HealthStatus,

    /// Individual component health.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub checks: HashMap<String, HealthStatus>,
}

/// Health checker trait.
///
/// Components implement this trait to provide custom health checks.
pub trait HealthChecker: Send + Sync + 'static {
    /// Performs a health check.
    ///
    /// Returns `true` if the component is healthy.
    fn check(&self) -> BoxFuture<'_, bool>;

    /// Returns the name of this checker.
    fn name(&self) -> &str;
}

/// Adapter for implementing [`HealthChecker`] with a function.
pub struct FunctionHealthChecker<F, Fut>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = bool> + Send,
{
    name: String,
    check_fn: F,
}

impl<F, Fut> FunctionHealthChecker<F, Fut>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = bool> + Send,
{
    /// Creates a new function-based health checker.
    pub fn new(name: impl Into<String>, check_fn: F) -> Self {
        Self {
            name: name.into(),
            check_fn,
        }
    }
}

impl<F, Fut> HealthChecker for FunctionHealthChecker<F, Fut>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    fn check(&self) -> BoxFuture<'_, bool> {
        Box::pin((self.check_fn)())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Health check registry.
///
/// Manages all registered health checkers.
#[derive(Clone, Default)]
pub struct HealthRegistry {
    checkers: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
}

impl HealthRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a health checker.
    pub async fn register(&self, checker: Arc<dyn HealthChecker>) {
        let mut checkers = self.checkers.write().await;
        checkers.push(checker);
    }

    /// Removes a health checker by name.
    pub async fn unregister(&self, name: &str) -> bool {
        let mut checkers = self.checkers.write().await;
        let original_len = checkers.len();
        checkers.retain(|c| c.name() != name);
        checkers.len() < original_len
    }

    /// Runs all health checks and returns the results.
    pub async fn check_all(&self) -> HealthCheck {
        let checkers = self.checkers.read().await;
        let mut checks = HashMap::new();
        let mut overall_healthy = true;

        for checker in checkers.iter() {
            let healthy = checker.check().await;
            if !healthy {
                overall_healthy = false;
            }
            checks.insert(checker.name().to_string(), if healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            });
        }

        HealthCheck {
            status: if overall_healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            },
            checks,
        }
    }

    /// Returns the number of registered checkers.
    pub async fn len(&self) -> usize {
        self.checkers.read().await.len()
    }

    /// Returns true if there are no registered checkers.
    pub async fn is_empty(&self) -> bool {
        self.checkers.read().await.is_empty()
    }
}

/// Health check server.
///
/// Serves HTTP endpoints for health checks.
pub struct HealthServer {
    registry: HealthRegistry,
    bind_address: String,
    bind_port: u16,
}

impl HealthServer {
    /// Creates a new health server.
    pub fn new(bind_address: String, bind_port: u16) -> Self {
        Self {
            registry: HealthRegistry::new(),
            bind_address,
            bind_port,
        }
    }

    /// Returns the health registry.
    pub fn registry(&self) -> &HealthRegistry {
        &self.registry
    }

    /// Runs the health server.
    pub async fn run(self) -> anyhow::Result<()> {
        let addr = format!("{}:{}", self.bind_address, self.bind_port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("health server listening on {}", addr);

        let registry = self.registry.clone();

        loop {
            let (stream, _) = listener.accept().await?;
            let registry = registry.clone();
            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                let http = hyper::server::conn::http1::Builder::new();
                let service = service_fn(move |req: Request<Incoming>| {
                    handle_request(req, registry.clone())
                });

                let _ = http.serve_connection(io, service).await;
            });
        }
    }

    /// Runs the health server in a background task.
    pub fn spawn(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move {
            self.run().await
        })
    }
}

/// Handle incoming HTTP requests.
async fn handle_request(
    req: Request<Incoming>,
    registry: HealthRegistry,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path();

    let response = match path {
        "/healthz" | "/healthz/live" => {
            Response::builder()
                .status(HttpStatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(Full::new(Bytes::from("ok")))
                .unwrap()
        }
        "/healthz/ready" => {
            let result = registry.check_all().await;
            let status = if result.status == HealthStatus::Healthy {
                HttpStatusCode::OK
            } else {
                HttpStatusCode::SERVICE_UNAVAILABLE
            };
            let body = if result.status == HealthStatus::Healthy {
                "ok"
            } else {
                "not ready"
            };
            Response::builder()
                .status(status)
                .header("Content-Type", "text/plain")
                .body(Full::new(Bytes::from(body)))
                .unwrap()
        }
        "/healthz/deep" => {
            let result = registry.check_all().await;
            let status = if result.status == HealthStatus::Healthy {
                HttpStatusCode::OK
            } else {
                HttpStatusCode::SERVICE_UNAVAILABLE
            };
            let json = serde_json::to_string(&result).unwrap_or_default();
            Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        _ => Response::builder()
            .status(HttpStatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("not found")))
            .unwrap(),
    };

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_registry() {
        let registry = HealthRegistry::new();

        // Add a checker
        let checker = FunctionHealthChecker::new("test", || async { true });
        registry.register(Arc::new(checker)).await;

        assert_eq!(registry.len().await, 1);
        assert!(!registry.is_empty().await);

        // Run checks
        let result = registry.check_all().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.checks.get("test"), Some(&HealthStatus::Healthy));

        // Remove checker
        assert!(registry.unregister("test").await);
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_unhealthy_checker() {
        let registry = HealthRegistry::new();

        let checker = FunctionHealthChecker::new("failing", || async { false });
        registry.register(Arc::new(checker)).await;

        let result = registry.check_all().await;
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.checks.get("failing"), Some(&HealthStatus::Unhealthy));
    }
}
