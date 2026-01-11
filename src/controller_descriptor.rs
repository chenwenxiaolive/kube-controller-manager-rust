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

//! Controller descriptor and registry.
//!
//! This module provides the types for describing and registering controllers.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::controller::{Controller, CancellationToken, Result};
use crate::controller_context::ControllerContext;
use crate::feature::FeatureGate;

/// A builder function that creates a controller instance.
///
/// The function receives:
/// - The shared controller context
/// - A cancellation token for shutdown
///
/// It returns `Ok(Some(controller))` if the controller was successfully created,
/// `Ok(None)` if the controller is disabled (e.g., missing dependencies),
/// or `Err` if there was a fatal error.
pub type ControllerConstructor = Arc<
    dyn Fn(ControllerContext, CancellationToken) -> BoxFuture<'static, Result<Option<Arc<dyn Controller>>>> + Send + Sync,
>;

/// Boxed future for async controller construction.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Descriptor for a controller.
///
/// Contains metadata about a controller and a constructor function to build it.
///
/// # Example
///
/// ```ignore
/// let descriptor = ControllerDescriptor::builder("my-controller")
///     .with_alias("my")
///     .with_alias("controller")
///     .disabled_by_default()
///     .build(constructor);
/// ```
#[derive(Clone)]
pub struct ControllerDescriptor {
    /// Canonical name of the controller.
    name: String,

    /// Alternative names for backward compatibility.
    ///
    /// These should never be removed, only new ones added.
    aliases: Vec<String>,

    /// Feature gates required for this controller to run.
    required_feature_gates: Vec<String>,

    /// Whether this controller is disabled by default.
    disabled_by_default: bool,

    /// Whether this is a cloud provider controller (deprecated).
    is_cloud_provider_controller: bool,

    /// Whether this controller requires special handling during startup.
    requires_special_handling: bool,

    /// Constructor function to build the controller.
    constructor: ControllerConstructor,
}

impl ControllerDescriptor {
    /// Creates a new builder for a controller descriptor.
    pub fn builder(name: impl Into<String>) -> Builder {
        Builder::new(name)
    }

    /// Returns the canonical name of this controller.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns all aliases for this controller.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Returns the feature gates required by this controller.
    pub fn required_feature_gates(&self) -> &[String] {
        &self.required_feature_gates
    }

    /// Returns whether this controller is disabled by default.
    pub fn is_disabled_by_default(&self) -> bool {
        self.disabled_by_default
    }

    /// Returns whether this is a cloud provider controller.
    pub fn is_cloud_provider_controller(&self) -> bool {
        self.is_cloud_provider_controller
    }

    /// Returns whether this controller requires special handling.
    pub fn requires_special_handling(&self) -> bool {
        self.requires_special_handling
    }

    /// Builds a controller instance from this descriptor.
    ///
    /// Returns `Ok(None)` if:
    /// - A required feature gate is not enabled
    /// - This is a cloud provider controller
    ///
    /// Returns `Ok(Some(controller))` if the controller was successfully created.
    pub async fn build_controller(
        &self,
        ctx: ControllerContext,
        cancel: CancellationToken,
        feature_gate: &dyn FeatureGate,
    ) -> Result<Option<Arc<dyn Controller>>> {
        // Check feature gates
        for gate in &self.required_feature_gates {
            if !feature_gate.enabled(gate) {
                tracing::debug!(
                    controller = %self.name,
                    "controller disabled by feature gate: {}",
                    gate
                );
                return Ok(None);
            }
        }

        // Skip cloud provider controllers
        if self.is_cloud_provider_controller {
            tracing::debug!(
                controller = %self.name,
                "skipping cloud provider controller"
            );
            return Ok(None);
        }

        // Build the controller
        (self.constructor)(ctx, cancel).await
    }
}

impl fmt::Debug for ControllerDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControllerDescriptor")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("required_feature_gates", &self.required_feature_gates)
            .field("disabled_by_default", &self.disabled_by_default)
            .field("is_cloud_provider_controller", &self.is_cloud_provider_controller)
            .field("requires_special_handling", &self.requires_special_handling)
            .finish()
    }
}

/// Builder for creating [`ControllerDescriptor`] instances.
pub struct Builder {
    name: String,
    aliases: Vec<String>,
    required_feature_gates: Vec<String>,
    disabled_by_default: bool,
    is_cloud_provider_controller: bool,
    requires_special_handling: bool,
}

impl Builder {
    /// Creates a new builder with the given controller name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            required_feature_gates: Vec::new(),
            disabled_by_default: false,
            is_cloud_provider_controller: false,
            requires_special_handling: false,
        }
    }

    /// Adds an alias for this controller.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Adds multiple aliases for this controller.
    pub fn with_aliases(mut self, aliases: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for alias in aliases {
            self.aliases.push(alias.into());
        }
        self
    }

    /// Adds a required feature gate.
    pub fn with_feature_gate(mut self, gate: impl Into<String>) -> Self {
        self.required_feature_gates.push(gate.into());
        self
    }

    /// Adds multiple required feature gates.
    pub fn with_feature_gates(mut self, gates: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for gate in gates {
            self.required_feature_gates.push(gate.into());
        }
        self
    }

    /// Marks this controller as disabled by default.
    pub fn disabled_by_default(mut self) -> Self {
        self.disabled_by_default = true;
        self
    }

    /// Marks this as a cloud provider controller.
    pub fn cloud_provider_controller(mut self) -> Self {
        self.is_cloud_provider_controller = true;
        self
    }

    /// Marks this controller as requiring special handling.
    pub fn requires_special_handling(mut self) -> Self {
        self.requires_special_handling = true;
        self
    }

    /// Builds the descriptor with the given constructor.
    pub fn build(self, constructor: ControllerConstructor) -> ControllerDescriptor {
        ControllerDescriptor {
            name: self.name,
            aliases: self.aliases,
            required_feature_gates: self.required_feature_gates,
            disabled_by_default: self.disabled_by_default,
            is_cloud_provider_controller: self.is_cloud_provider_controller,
            requires_special_handling: self.requires_special_handling,
            constructor,
        }
    }
}

/// Registry of all known controllers.
///
/// This type manages the collection of controller descriptors and provides
/// methods to query and validate them.
#[derive(Debug, Clone)]
pub struct ControllerRegistry {
    /// Map of canonical controller names to their descriptors.
    controllers: HashMap<String, ControllerDescriptor>,

    /// Map of aliases to canonical names.
    alias_map: HashMap<String, String>,
}

impl Default for ControllerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            controllers: HashMap::new(),
            alias_map: HashMap::new(),
        }
    }

    /// Registers a controller descriptor.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The descriptor name is empty
    /// - A controller with the same name is already registered
    /// - An alias conflicts with another controller name or alias
    pub fn register(&mut self, descriptor: ControllerDescriptor) -> &mut Self {
        let name = descriptor.name();

        if name.is_empty() {
            panic!("controller name cannot be empty");
        }

        if self.controllers.contains_key(name) {
            panic!("controller {:?} is already registered", name);
        }

        // Check alias conflicts
        for alias in descriptor.aliases() {
            if self.controllers.contains_key(alias) {
                panic!(
                    "alias {:?} conflicts with an existing controller name",
                    alias
                );
            }
            if let Some(existing) = self.alias_map.get(alias) {
                panic!(
                    "alias {:?} is already used by controller {:?}",
                    alias, existing
                );
            }
        }

        // Register aliases
        for alias in descriptor.aliases() {
            self.alias_map.insert(alias.clone(), name.to_string());
        }

        self.controllers.insert(name.to_string(), descriptor);
        self
    }

    /// Returns the descriptor for the given name or alias.
    pub fn get(&self, name: &str) -> Option<&ControllerDescriptor> {
        // First try as canonical name
        if let Some(d) = self.controllers.get(name) {
            return Some(d);
        }

        // Then try as alias
        if let Some(canonical) = self.alias_map.get(name) {
            return self.controllers.get(canonical);
        }

        None
    }

    /// Returns all canonical controller names.
    pub fn controller_names(&self) -> Vec<String> {
        self.controllers.keys().cloned().collect()
    }

    /// Returns all controller descriptors.
    pub fn controllers(&self) -> impl Iterator<Item = &ControllerDescriptor> {
        self.controllers.values()
    }

    /// Returns controllers that are disabled by default.
    pub fn disabled_by_default(&self) -> Vec<String> {
        self.controllers
            .values()
            .filter(|d| d.is_disabled_by_default())
            .map(|d| d.name().to_string())
            .collect()
    }

    /// Returns the alias map.
    pub fn alias_map(&self) -> &HashMap<String, String> {
        &self.alias_map
    }

    /// Returns true if the given name is a known alias.
    pub fn is_alias(&self, name: &str) -> bool {
        self.alias_map.contains_key(name)
    }

    /// Resolves an alias to its canonical name.
    pub fn resolve_alias(&self, name: &str) -> Option<&str> {
        self.alias_map.get(name).map(|s| s.as_str())
    }

    /// Validates the registry for consistency.
    ///
    /// Returns an error if any inconsistencies are found.
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::new();

        // Check that all aliases point to valid controllers
        for (alias, canonical) in &self.alias_map {
            if !self.controllers.contains_key(canonical) {
                return Err(anyhow::anyhow!(
                    "alias {:?} points to non-existent controller {:?}",
                    alias,
                    canonical
                )
                .into());
            }
            seen.insert(alias.clone());
        }

        // Check for alias-name conflicts
        for name in self.controllers.keys() {
            if seen.contains(name) {
                return Err(anyhow::anyhow!(
                    "controller name {:?} conflicts with an alias",
                    name
                )
                .into());
            }
        }

        Ok(())
    }
}

/// Extension trait for converting anyhow errors to controller errors.
trait IntoControllerError {
    fn into_controller_error(self) -> crate::controller::ControllerError;
}

impl IntoControllerError for anyhow::Error {
    fn into_controller_error(self) -> crate::controller::ControllerError {
        crate::controller::ControllerError::Runtime {
            name: "registry".to_string(),
            source: self.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::ControllerError;
    use std::pin::Pin;

    #[tokio::test]
    async fn test_registry_basic() {
        let mut registry = ControllerRegistry::new();

        let constructor: ControllerConstructor = Arc::new(|_, _| {
            Box::pin(async { Ok::<_, ControllerError>(None) })
                as Pin<Box<dyn Future<Output = _> + Send>>
        });
        let descriptor = ControllerDescriptor::builder("test-controller")
            .build(constructor);

        registry.register(descriptor);
        assert!(registry.get("test-controller").is_some());
    }

    #[tokio::test]
    #[should_panic(expected = "controller")]
    async fn test_registry_duplicate_panics() {
        let mut registry = ControllerRegistry::new();

        let constructor: ControllerConstructor = Arc::new(|_, _| {
            Box::pin(async { Ok::<_, ControllerError>(None) })
                as Pin<Box<dyn Future<Output = _> + Send>>
        });
        let descriptor = ControllerDescriptor::builder("test-controller")
            .build(constructor);

        registry.register(descriptor.clone());
        registry.register(descriptor);
    }

    #[tokio::test]
    async fn test_alias_resolution() {
        let mut registry = ControllerRegistry::new();

        let constructor: ControllerConstructor = Arc::new(|_, _| {
            Box::pin(async { Ok::<_, ControllerError>(None) })
                as Pin<Box<dyn Future<Output = _> + Send>>
        });
        let descriptor = ControllerDescriptor::builder("test-controller")
            .with_alias("test")
            .with_alias("tc")
            .build(constructor);

        registry.register(descriptor);

        assert_eq!(registry.resolve_alias("test"), Some("test-controller"));
        assert_eq!(registry.resolve_alias("tc"), Some("test-controller"));
        assert!(registry.resolve_alias("unknown").is_none());
    }
}
