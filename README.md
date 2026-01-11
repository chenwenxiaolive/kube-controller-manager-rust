# kube-controller-manager-rust

A Rust implementation of the Kubernetes controller manager.

This project rewrites the controller management framework from `cmd/kube-controller-manager` in Rust, providing the core orchestration layer for Kubernetes controllers.

## Overview

The kube-controller-manager is a daemon that embeds the core control loops shipped with Kubernetes. In applications of robotics and automation, a control loop is a non-terminating loop that regulates the state of the system. In Kubernetes, a controller is a control loop that watches the shared state of the cluster through the apiserver and makes changes attempting to move the current state towards the desired state.

## Status

This is an early-stage implementation. The framework is functional but individual controllers have not yet been implemented.

## Features

- **Controller Framework**: Core trait-based controller abstraction
- **Configuration**: Comprehensive config system supporting 30+ controller types
- **Feature Gates**: Kubernetes-compatible feature gate system
- **Health Checks**: HTTP endpoints for readiness/liveness probes
- **Leader Election**: File-based leader election for high availability
- **Graceful Shutdown**: Proper cleanup on SIGINT/SIGTERM

## Building

```bash
cargo build --release
```

The binary will be output to `target/release/kube-controller-manager`.

## Running

### Basic Usage

```bash
# Use default kubeconfig
kube-controller-manager

# Specify kubeconfig
kube-controller-manager --kubeconfig ~/.kube/config

# Run with specific namespace
kube-controller-manager --namespace kube-system

# Enable specific controllers
kube-controller-manager --controllers endpoints,deployment

# Enable all controllers except specific ones
kube-controller-manager --controllers '*,-deployment,-daemonset'
```

### Configuration File

You can also use a configuration file:

```bash
kube-controller-manager --config config.yaml
```

Example configuration:

```yaml
generic:
  kubeconfig: ~/.kube/config
  bind_address: 0.0.0.0
  bind_port: 10252
  healthz_bind_port: 10257
  leader_election_enabled: true
  controllers: "*"
```

## Health Check Endpoints

- `/healthz` - Basic liveness check (always returns "ok")
- `/healthz/live` - Liveness probe
- `/healthz/ready` - Readiness probe with component checks
- `/healthz/deep` - Detailed health status in JSON format

## Controller List

The following controllers are recognized (implementation pending):

- `endpoint` - Endpoints controller
- `endpointslice` - EndpointSlice controller
- `deployment` - Deployment controller
- `replicaset` - ReplicaSet controller
- `statefulset` - StatefulSet controller
- `daemonset` - DaemonSet controller
- `job` - Job controller
- `cronjob` - CronJob controller
- `namespace` - Namespace controller
- `node` - Node controller
- `service` - Service controller
- `route` - Route controller
- `garbagecollector` - Garbage collector
- `ttl-after-finished` - TTL controller
- `bootstrapsigner` - Bootstrap signer
- `csrapproving` - CSR approving controller
- `csrsigning` - CSR signing controller
- `csrcleaner` - CSR cleaning controller
- `clusterrole-aggregation` - ClusterRole aggregation
- `pvc-protection` - PVC protection
- `pv-protection` - PV protection
- `ttl` - TTL controller
- `root-ca-cert-publisher` - Root CA certificate publisher
- `ephemeral-volume` - Ephemeral volume controller
- `persistent-volume-binder` - Persistent volume binder
- `attachdetach` - Attach/detach controller
- `nodeipam` - Node IPAM controller
- `node-lifecycle` - Node lifecycle controller
- `resource-quota` - Resource quota controller
- `horizontalpodautoscaling` - HPA controller
- `disruption` - Disruption controller
- `serviceaccount` - ServiceAccount controller
- `serviceaccount-token` - ServiceAccount token controller

## Feature Gates

The following feature gates are supported:

| Feature | Default | Pre-release |
|---------|---------|-------------|
| AllAlpha | false | Yes |
| AllBeta | false | Yes |
| ServiceLBEndpointFinalizers | true | No |
| VolumeAttributesClass | false | Yes |
| SELinuxChangePolicy | false | Yes |
| HPAContainerMetrics | false | Yes |
| DynamicResourceAllocation | false | Yes |
| CoordinatedLeaderElection | true | No |
| EndpointSliceTerminatingCondition | true | No |
| EphemeralContainers | true | No |
| RotateKubeletServerCertificate | true | No |
| ServiceInternalTrafficPolicy | true | No |

Enable feature gates:

```bash
kube-controller-manager --feature-gates "HPAContainerMetrics=true,DynamicResourceAllocation=false"
```

## Development

### Project Structure

```
src/
├── main.rs                  # CLI entry point
├── lib.rs                   # Library root
├── controller.rs            # Controller trait
├── controller_manager.rs    # Main orchestrator
├── controller_context.rs    # Shared context
├── controller_descriptor.rs # Controller registry
├── config.rs                # Configuration types
├── feature.rs               # Feature gate support
├── health.rs                # Health check server
├── leader_election.rs       # Leader election
└── controllers/
    └── mod.rs               # Controller implementations (TODO)
```

### Running Tests

```bash
cargo test
```

### Checking

```bash
cargo check
```

### Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## Dependencies

- [kube-rs](https://github.com/kube-rs/kube) 0.96 - Kubernetes client
- [k8s-openapi](https://github.com/Arnavion/k8s-openapi) 0.23 - Kubernetes API types
- [tokio](https://tokio.rs) 1.40 - Async runtime
- [hyper](https://hyper.rs) 1.0 - HTTP server
- [clap](https://github.com/clap-rs/clap) 4.5 - CLI parsing
- [serde](https://serde.rs) - Serialization
- [tracing](https://tokio.rs/tokio/topics/tracing) - Logging

## License

Apache License 2.0

## Original Go Implementation

This is a rewrite of the [Kubernetes kube-controller-manager](https://github.com/kubernetes/kubernetes/tree/master/cmd/kube-controller-manager) in Go. The original implementation is approximately 9,400 lines of code.

## Contributing

This is a personal learning project. Contributions are welcome but please understand that this is not an official Kubernetes project.

## Acknowledgments

- The Kubernetes community for the original implementation
- The kube-rs team for the excellent Rust Kubernetes client
