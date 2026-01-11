# Validation Guide

This document describes how to verify that the Rust implementation matches the Go kube-controller-manager behavior.

## Test Levels

### Level 1: Unit Tests (✓ Implemented)

Run unit tests for each module:

```bash
cargo test
```

### Level 2: Compatibility Tests (✓ Implemented)

Run tests that verify compatibility with Go version:

```bash
cargo test --test compatibility
```

These tests verify:
- CLI arguments match
- Controller list matches
- Feature gates match
- Default configuration values match
- Health endpoints match

### Level 3: Behavioral Tests (TODO)

Tests that verify the same input produces the same output:

```bash
./tests/behavioral_test.sh
```

### Level 4: Integration Tests (TODO)

Tests running against a real Kubernetes cluster:

```bash
./tests/integration_test.sh
```

## Validation Checklist

### CLI Compatibility

| Feature | Go Version | Rust Version | Status |
|---------|-----------|--------------|--------|
| `--kubeconfig` | ✓ | ✓ | ✓ Compatible |
| `--master` | ✓ | ✓ | ✓ Compatible |
| `--config` | ✓ | ✓ | ✓ Compatible |
| `--controllers` | ✓ | ✓ | ✓ Compatible |
| `--leader-elect` | ✓ | ✓ | ✓ Compatible |
| `--feature-gates` | ✓ | ✓ | ✓ Compatible |
| `--namespace` | ✓ | ✓ | ✓ Compatible |
| `--bind-address` | ✓ | ✓ | ✓ Compatible |
| `--bind-port` | ✓ | ✓ | ✓ Compatible |

### Default Values

| Setting | Go Default | Rust Default | Match? |
|---------|-----------|--------------|--------|
| bind-port | 10252 | 10252 | ✓ |
| healthz-port | 10257 | 10257 | ✓ |
| diagnostics-port | 10256 | 10256 | ✓ |
| min-resync-period | 12h | 12h | ✓ |
| lease-duration | 15s | 15s | ✓ |
| renew-deadline | 10s | 10s | ✓ |
| retry-period | 2s | 2s | ✓ |

### Controllers

All 30+ controllers from Go version are recognized:

| Controller | Known | Implemented | Notes |
|------------|-------|-------------|-------|
| endpoint | ✓ | TODO | |
| endpointslice | ✓ | TODO | |
| deployment | ✓ | TODO | |
| replicaset | ✓ | TODO | |
| statefulset | ✓ | TODO | |
| daemonset | ✓ | TODO | |
| ... | ✓ | TODO | |

### Feature Gates

All Kubernetes feature gates are defined:

| Feature Gate | Known | Default | Match Go? |
|--------------|-------|---------|-----------|
| AllAlpha | ✓ | false | ✓ |
| AllBeta | ✓ | false | ✓ |
| ServiceLBEndpointFinalizers | ✓ | true | ✓ |
| VolumeAttributesClass | ✓ | false | ✓ |
| ... | ✓ | | ✓ |

### Health Endpoints

| Endpoint | Go | Rust | Match? |
|----------|-----|------|--------|
| /healthz | ✓ | ✓ | ✓ |
| /healthz/live | ✓ | ✓ | ✓ |
| /healthz/ready | ✓ | ✓ | ✓ |
| /healthz/deep | ✓ | ✓ | ✓ |

## Side-by-Side Testing

To run both versions side-by-side:

```bash
# Terminal 1: Go version
kube-controller-manager \
  --kubeconfig ~/.kube/config \
  --controllers endpoint \
  --bind-port 10252

# Terminal 2: Rust version
./target/release/kube-controller-manager \
  --kubeconfig ~/.kube/config \
  --controllers endpoint \
  --bind-port 10253

# Terminal 3: Compare
kubectl get endpoints -w
```

## Log Comparison

Compare log outputs:

```bash
# Go version logs
kube-controller-manager --log-level debug 2>&1 | tee go.log

# Rust version logs
./target/release/kube-controller-manager --log-level debug 2>&1 | tee rust.log

# Compare patterns
diff <(grep "starting controller" go.log) <(grep "starting controller" rust.log)
```

## Metrics Comparison

Both versions expose Prometheus metrics:

```bash
# Go metrics
curl http://localhost:10252/metrics

# Rust metrics (when implemented)
curl http://localhost:10253/metrics

# Compare
diff <(curl localhost:10252/metrics) <(curl localhost:10253/metrics)
```

## Next Steps

1. Implement specific controllers (endpoint, deployment, etc.)
2. Add behavioral tests for each controller
3. Add integration tests with kind/minikube
4. Add performance benchmarks
5. Add chaos engineering tests
