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

//! kube-controller-manager - Kubernetes Controller Manager in Rust
//!
//! This is a Rust implementation of the Kubernetes controller manager,
//! which embeds the core control loops shipped with Kubernetes.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments)]

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use kube_controller_manager_rust::config::ControllerManagerConfig;
use kube_controller_manager_rust::controller_manager::ControllerManager;
use kube_controller_manager_rust::feature::{self, FeatureGate};

/// Kubernetes Controller Manager
///
/// The controller manager is a daemon that embeds the core control loops
/// shipped with Kubernetes. In applications of robotics and automation,
/// a control loop is a non-terminating loop that regulates the state of
/// the system. In Kubernetes, a controller is a control loop that watches
/// the shared state of the cluster through the apiserver and makes changes
/// attempting to move the current state towards the desired state.
#[derive(Parser, Debug)]
#[command(name = "kube-controller-manager")]
#[command(author = "Kubernetes Authors")]
#[command(version = "0.1.0")]
#[command(about = "Kubernetes Controller Manager", long_about = None)]
struct Args {
    /// Path to the kubeconfig file
    #[arg(long, global = true)]
    kubeconfig: Option<PathBuf>,

    /// Master URL to build a client from
    #[arg(long, global = true)]
    master: Option<String>,

    /// Path to the configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long)]
    log_json: bool,

    /// Comma-separated list of controllers to enable
    ///
    /// '*' enables all controllers, 'foo' enables the controller named 'foo',
    /// '-foo' disables the controller named 'foo'
    #[arg(long)]
    controllers: Option<String>,

    /// The address to serve metrics
    #[arg(long, default_value = "0.0.0.0")]
    bind_address: String,

    /// The port to serve metrics
    #[arg(long, default_value = "10252")]
    bind_port: u16,

    /// Enable leader election
    #[arg(long)]
    leader_elect: bool,

    /// Disable leader election
    #[arg(long, conflicts_with = "leader_elect")]
    leader_elect_disable: bool,

    /// Port for the health check server
    #[arg(long, default_value = "10257")]
    healthz_bind_port: u16,

    /// Port for the insecure diagnostics server
    #[arg(long, default_value = "10256")]
    diagnostics_port: u16,

    /// Enable profiling
    #[arg(long)]
    enable_profiling: bool,

    /// Minimum resync period for informers
    #[arg(long, value_parser = parse_duration)]
    min_resync_period: Option<Duration>,

    /// Feature gates to enable/disable
    ///
    /// Format: "Feature1=true,Feature2=false"
    #[arg(long)]
    feature_gates: Option<String>,

    /// Namespace to watch
    #[arg(long, default_value = "default")]
    namespace: String,
}

fn parse_duration(s: &str) -> anyhow::Result<Duration> {
    humantime::parse_duration(s)
        .map_err(|e| anyhow::anyhow!("invalid duration: {}", e))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level, args.log_json);

    info!("starting kube-controller-manager");

    // Load or create configuration
    let config = load_config(args).await?;

    // Print configuration
    info!(
        "configuration: controllers={}, leader_election={}",
        config.generic.controllers,
        config.generic.leader_election_enabled
    );

    // Create Kubernetes client
    let client = create_client(&config).await?;

    // Create controller manager
    let mut manager = ControllerManager::new(config, client);

    // TODO: Register feature gates from command line
    // if let Some(fg_string) = args.feature_gates {
    //     manager
    //         .feature_gate()
    //         .set_from_string(&fg_string)
    //         .context("failed to set feature gates")?;
    // }

    // TODO: Register controllers
    // register_controllers(&mut manager);

    // Set up signal handling
    let shutdown_token = manager.shutdown_token();
    tokio::spawn(async move {
        wait_for_shutdown().await;
        shutdown_token.cancel();
    });

    // Run the controller manager
    if let Err(e) = manager.run().await {
        error!("controller manager failed: {:#}", e);
        return Err(e.into());
    }

    info!("kube-controller-manager exited successfully");
    Ok(())
}

/// Initializes logging based on the provided level and format.
fn init_logging(level: &str, json: bool) {
    let env_filter = EnvFilter::builder()
        .with_default_directive(level.parse().unwrap())
        .from_env_lossy();

    if json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().pretty())
            .init();
    }
}

/// Loads the configuration from file or command-line arguments.
async fn load_config(mut args: Args) -> anyhow::Result<ControllerManagerConfig> {
    let mut config = if let Some(config_path) = args.config.take() {
        // Load from file
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("failed to read config file: {:?}", config_path))?;

        serde_yaml::from_str::<ControllerManagerConfig>(&content)
            .with_context(|| format!("failed to parse config file: {:?}", config_path))?
    } else {
        ControllerManagerConfig::default()
    };

    // Override with command-line arguments
    if let Some(kubeconfig) = args.kubeconfig {
        config.generic.kubeconfig = Some(kubeconfig);
    }
    if let Some(master) = args.master {
        config.generic.master = Some(master);
    }
    if let Some(controllers) = args.controllers {
        config.generic.controllers = controllers;
    }
    config.generic.bind_address = args.bind_address;
    config.generic.bind_port = args.bind_port;
    config.generic.healthz_bind_port = args.healthz_bind_port;
    config.generic.diagnostics_port = args.diagnostics_port;
    config.generic.enable_profiling = args.enable_profiling;
    config.generic.namespace = args.namespace;

    // Leader election
    if args.leader_elect {
        config.generic.leader_election_enabled = true;
    } else if args.leader_elect_disable {
        config.generic.leader_election_enabled = false;
    }

    if let Some(duration) = args.min_resync_period {
        config.generic.min_resync_period = duration;
    }

    // Parse controllers string into enabled/disabled sets
    parse_controllers(&mut config);

    Ok(config)
}

/// Parses the controllers string into enabled/disabled sets.
fn parse_controllers(config: &mut ControllerManagerConfig) {
    let controllers = &config.generic.controllers;
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();

    if controllers == "*" {
        // All controllers enabled by default
        return;
    }

    for part in controllers.split(',') {
        let part: &str = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(name) = part.strip_prefix('-') {
            disabled.push(name.to_string());
        } else {
            enabled.push(part.to_string());
        }
    }

    config.generic.controllers_enabled = enabled.into_iter().collect();
    config.generic.controllers_disabled = disabled.into_iter().collect();
}

/// Creates a Kubernetes client from the configuration.
async fn create_client(config: &ControllerManagerConfig) -> anyhow::Result<kube::Client> {
    use kube::config::{Kubeconfig, KubeConfigOptions};
    use kube::Config;

    let kube_config = if let Some(kubeconfig_path) = &config.generic.kubeconfig {
        // Load from specified kubeconfig file
        let kubeconfig = Kubeconfig::read_from(kubeconfig_path)
            .with_context(|| format!("failed to read kubeconfig from: {:?}", kubeconfig_path))?;
        Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default())
            .await
            .with_context(|| format!("failed to load kubeconfig from: {:?}", kubeconfig_path))?
    } else if let Some(master_url) = &config.generic.master {
        // Load from master URL
        let uri = master_url.parse::<http::Uri>()
            .with_context(|| format!("invalid master URL: {}", master_url))?;
        Config::new(uri).into()
    } else {
        // Use default kubeconfig
        Config::infer()
            .await
            .context("failed to load kubeconfig")?
    };

    Ok(kube::Client::try_from(kube_config)?)
}

/// Waits for a shutdown signal (SIGINT or SIGTERM).
async fn wait_for_shutdown() {
    use tokio::signal;

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("received SIGTERM, shutting down");
            }
            _ = sigint.recv() => {
                info!("received SIGINT, shutting down");
            }
        }
    }

    #[cfg(windows)]
    {
        let ctrl_c = signal::windows::ctrl_c();
        let ctrl_close = signal::windows::ctrl_close();
        let ctrl_shutdown = signal::windows::ctrl_shutdown();

        tokio::select! {
            _ = ctrl_c => {
                info!("received Ctrl+C, shutting down");
            }
            _ = ctrl_close => {
                info!("received close signal, shutting down");
            }
            _ = ctrl_shutdown => {
                info!("received shutdown signal, shutting down");
            }
        }
    }
}
