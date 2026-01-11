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

//! Feature gate support.
//!
//! Feature gates allow Kubernetes to incrementally roll out new features.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Error type for feature gate operations.
#[derive(Debug, thiserror::Error)]
pub enum FeatureGateError {
    #[error("unknown feature gate: {0}")]
    UnknownFeature(String),

    #[error("feature gate {0} is read-only and cannot be modified")]
    ReadOnly(String),

    #[error("invalid value for feature gate {0}: expected bool, got {1}")]
    InvalidValue(String, String),
}

/// Result type for feature gate operations.
pub type Result<T> = std::result::Result<T, FeatureGateError>;

/// A trait for types that can check and manipulate feature gates.
///
/// Feature gates allow Kubernetes to incrementally roll out new features
/// by providing a way to enable/disable functionality at runtime.
pub trait FeatureGate: Send + Sync {
    /// Returns true if the given feature gate is enabled.
    fn enabled(&self, feature: &str) -> bool;

    /// Sets a feature gate to the given value.
    ///
    /// Returns an error if the feature gate doesn't exist or is read-only.
    fn set(&self, feature: &str, enabled: bool) -> Result<()>;

    /// Returns all known feature gates and their current values.
    fn all_features(&self) -> HashMap<String, bool>;

    /// Checks if a feature gate exists.
    fn contains(&self, feature: &str) -> bool;
}

/// Simple in-memory feature gate implementation.
///
/// This implementation stores feature gate state in memory and allows
/// runtime modification of non-read-only features.
#[derive(Debug, Clone)]
pub struct MemoryFeatureGate {
    /// Feature gate states.
    features: Arc<RwLock<HashMap<String, FeatureState>>>,

    /// Known feature gates and their default states.
    known_features: HashMap<String, FeatureState>,
}

/// Internal state of a feature gate.
#[derive(Debug, Clone, Copy)]
struct FeatureState {
    /// Current enabled state.
    enabled: bool,

    /// Whether this feature gate can be modified at runtime.
    read_only: bool,

    /// Whether this feature gate is pre-release (hidden from help).
    pre_release: bool,
}

impl MemoryFeatureGate {
    /// Creates a new feature gate with the given known features.
    ///
    /// # Arguments
    ///
    /// * `features` - Iterator of (name, default_enabled, read_only, pre_release)
    pub fn new(
        features: impl IntoIterator<Item = (&'static str, bool, bool, bool)>,
    ) -> Self {
        let known_features: HashMap<String, FeatureState> = features
            .into_iter()
            .map(|(name, enabled, read_only, pre_release)| {
                (
                    name.to_string(),
                    FeatureState {
                        enabled,
                        read_only,
                        pre_release,
                    },
                )
            })
            .collect();

        let features = known_features
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        Self {
            features: Arc::new(RwLock::new(features)),
            known_features,
        }
    }

    /// Enables a feature gate.
    pub fn enable(&self, feature: &str) -> Result<()> {
        self.set(feature, true)
    }

    /// Disables a feature gate.
    pub fn disable(&self, feature: &str) -> Result<()> {
        self.set(feature, false)
    }

    /// Sets feature gates from a comma-separated list.
    ///
    /// Format: "Feature1=true,Feature2=false"
    pub fn set_from_string(&self, s: &str) -> Result<()> {
        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let (name, value) = part
                .split_once('=')
                .ok_or_else(|| FeatureGateError::InvalidValue(part.to_string(), "missing =".to_string()))?;

            let enabled = value
                .parse::<bool>()
                .map_err(|_| FeatureGateError::InvalidValue(part.to_string(), value.to_string()))?;

            self.set(name, enabled)?;
        }
        Ok(())
    }

    /// Returns all pre-release feature gates.
    pub fn pre_release_features(&self) -> Vec<String> {
        self.known_features
            .iter()
            .filter(|(_, v)| v.pre_release)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Returns all feature gate names.
    pub fn feature_names(&self) -> Vec<String> {
        self.known_features.keys().cloned().collect()
    }
}

impl FeatureGate for MemoryFeatureGate {
    fn enabled(&self, feature: &str) -> bool {
        let features = self.features.read().unwrap();
        features
            .get(feature)
            .map(|f| f.enabled)
            .unwrap_or(false)
    }

    fn set(&self, feature: &str, enabled: bool) -> Result<()> {
        // Check if feature exists
        let current = self
            .known_features
            .get(feature)
            .ok_or_else(|| FeatureGateError::UnknownFeature(feature.to_string()))?;

        // Check if read-only
        if current.read_only {
            return Err(FeatureGateError::ReadOnly(feature.to_string()));
        }

        let mut features = self.features.write().unwrap();
        if let Some(f) = features.get_mut(feature) {
            f.enabled = enabled;
        }
        Ok(())
    }

    fn all_features(&self) -> HashMap<String, bool> {
        let features = self.features.read().unwrap();
        features
            .iter()
            .map(|(k, v)| (k.clone(), v.enabled))
            .collect()
    }

    fn contains(&self, feature: &str) -> bool {
        self.known_features.contains_key(feature)
    }
}

/// Kubernetes feature gates.
///
/// These are the known feature gates for Kubernetes.
/// See: https://kubernetes.io/docs/reference/command-line-tools-reference/feature-gates/
pub mod kubernetes {
    /// Returns the default Kubernetes feature gates.
    ///
    /// This is a subset of all available feature gates.
    /// You can add more as needed.
    pub fn default_features() -> &'static [(&'static str, bool, bool, bool)] {
        &[
            // (name, default_enabled, read_only, pre_release)
            //
            // General features
            ("AllAlpha", false, false, true),
            ("AllBeta", false, false, true),
            //
            // Known stable/alpha/beta features
            ("ServiceLBEndpointFinalizers", true, false, false),
            ("VolumeAttributesClass", false, false, true),
            ("SELinuxChangePolicy", false, false, true),
            ("SELinuxMountReadWriteOncePod", false, false, true),
            ("HPAContainerMetrics", false, false, true),
            ("DynamicResourceAllocation", false, false, true),
            ("DRAResourceClassDeviceParams", false, false, true),
            ("DRADeviceTaints", false, false, true),
            ("DRAAdminAccess", false, false, true),
            ("DRAPrioritizedList", false, false, true),
            ("CoordinatedLeaderElection", true, false, false),
            ("EndpointSliceTerminatingCondition", true, false, false),
            ("EndpointSliceNodeNameExpansion", false, false, true),
            ("EphemeralContainers", true, false, false),
            ("RotateKubeletServerCertificate", true, false, false),
            ("StatefulSetStartOrdinal", false, false, true),
            ("DaemonSetUpdateSurge", false, false, true),
            ("LegacyServiceAccountTokenCleanUp", false, false, true),
            ("MultiCIDRServiceAllocator", false, false, true),
            ("ServiceInternalTrafficPolicy", true, false, false),
            ("ServiceTrafficDistribution", false, false, true),
            ("DisableKubeletCloudCredentialProviders", false, false, true),
            ("DisableNodeKubeProxyVersion", false, false, true),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_gate_enable_disable() {
        let fg = MemoryFeatureGate::new([("test-feature", false, false, false)]);

        assert!(!fg.enabled("test-feature"));
        fg.enable("test-feature").unwrap();
        assert!(fg.enabled("test-feature"));
        fg.disable("test-feature").unwrap();
        assert!(!fg.enabled("test-feature"));
    }

    #[test]
    fn test_feature_gate_read_only() {
        let fg = MemoryFeatureGate::new([("test-feature", true, true, false)]);

        assert!(fg.enabled("test-feature"));
        assert!(fg.set("test-feature", false).is_err());
        assert!(fg.enabled("test-feature"));
    }

    #[test]
    fn test_feature_gate_unknown() {
        let fg = MemoryFeatureGate::new([("test-feature", false, false, false)]);

        assert!(!fg.enabled("unknown-feature"));
        assert!(fg.set("unknown-feature", true).is_err());
    }

    #[test]
    fn test_feature_gate_from_string() {
        let fg = MemoryFeatureGate::new([
            ("feature1", false, false, false),
            ("feature2", true, false, false),
        ]);

        fg.set_from_string("feature1=true,feature2=false").unwrap();
        assert!(fg.enabled("feature1"));
        assert!(!fg.enabled("feature2"));
    }
}
