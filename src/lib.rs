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

//! kube-controller-manager-rust - A Rust implementation of Kubernetes Controller Manager
//!
//! This library provides the core components for building a Kubernetes controller manager:
//! - Controller traits and types
//! - Controller descriptor and registry
//! - Configuration management
//! - Feature gates
//! - Leader election
//! - Health checks

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments)]

pub mod config;
pub mod controller;
pub mod controller_context;
pub mod controller_descriptor;
pub mod controller_manager;
pub mod feature;
pub mod health;
pub mod leader_election;

// Re-export commonly used types
pub use controller::{Controller, ControllerError, FunctionController};
pub use config::ControllerManagerConfig;
pub use controller_context::ControllerContext;
pub use controller_descriptor::{ControllerDescriptor, ControllerRegistry};
pub use controller_manager::ControllerManager;
pub use feature::{FeatureGate, MemoryFeatureGate};

/// Semantic version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default name for the controller manager.
pub const CONTROLLER_MANAGER_NAME: &str = "kube-controller-manager";
