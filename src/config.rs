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

//! Configuration structures for the controller manager.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Main configuration for the kube-controller-manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerManagerConfig {
    /// Generic configuration applicable to all controllers.
    pub generic: GenericControllerManagerConfig,

    /// KubeCloudShared configuration contains cloud provider related config.
    #[serde(default)]
    pub kube_cloud_shared: KubeCloudSharedConfig,

    /// Endpoint controller configuration.
    #[serde(default)]
    pub endpoint: EndpointControllerConfig,

    /// Replication controller configuration.
    #[serde(default)]
    pub replication: ReplicationControllerConfig,

    /// Pod garbage collector configuration.
    #[serde(default)]
    pub pod_gc: PodGCControllerConfig,

    /// Resource quota controller configuration.
    #[serde(default)]
    pub resource_quota: ResourceQuotaControllerConfig,

    /// Namespace controller configuration.
    #[serde(default)]
    pub namespace: NamespaceControllerConfig,

    /// Service account controller configuration.
    #[serde(default)]
    pub service_account: ServiceAccountControllerConfig,

    /// Garbage collector controller configuration.
    #[serde(default)]
    pub garbage_collector: GarbageCollectorControllerConfig,

    /// DaemonSet controller configuration.
    #[serde(default)]
    pub daemon_set: DaemonSetControllerConfig,

    /// Job controller configuration.
    #[serde(default)]
    pub job: JobControllerConfig,

    /// Deployment controller configuration.
    #[serde(default)]
    pub deployment: DeploymentControllerConfig,

    /// ReplicaSet controller configuration.
    #[serde(default)]
    pub replica_set: ReplicaSetControllerConfig,

    /// Horizontal Pod Autoscaler configuration.
    #[serde(default)]
    pub hpa: HPAControllerConfig,

    /// Disruption (PodDisruptionBudget) controller configuration.
    #[serde(default)]
    pub disruption: DisruptionControllerConfig,

    /// StatefulSet controller configuration.
    #[serde(default)]
    pub stateful_set: StatefulSetControllerConfig,

    /// CronJob controller configuration.
    #[serde(default)]
    pub cron_job: CronJobControllerConfig,

    /// Certificate controllers configuration.
    #[serde(default)]
    pub certificates: CertificatesConfig,

    /// TTL controller configuration.
    #[serde(default)]
    pub ttl: TTLControllerConfig,

    /// Bootstrap controller configuration.
    #[serde(default)]
    pub bootstrap: BootstrapControllerConfig,

    /// Node IPAM controller configuration.
    #[serde(default)]
    pub node_ipam: NodeIPAMControllerConfig,

    /// Node lifecycle controller configuration.
    #[serde(default)]
    pub node_lifecycle: NodeLifecycleControllerConfig,

    /// Persistent volume binder controller configuration.
    #[serde(default)]
    pub pv_binder: PersistentVolumeBinderConfig,

    /// Attach detach controller configuration.
    #[serde(default)]
    pub attach_detach: AttachDetachControllerConfig,

    /// Persistent volume claim protection controller configuration.
    #[serde(default)]
    pub pvc_protection: PVCProtectionControllerConfig,

    /// Persistent volume protection controller configuration.
    #[serde(default)]
    pub pv_protection: PVProtectionControllerConfig,

    /// TTL after finished controller configuration.
    #[serde(default)]
    pub ttl_after_finished: TTLAfterFinishedControllerConfig,

    /// Leader election configuration.
    #[serde(default)]
    pub leader_election: LeaderElectionConfig,

    /// Leader migration configuration.
    #[serde(default)]
    pub leader_migration: LeaderMigrationConfig,

    /// Concurrent syncs for device taint eviction controller.
    #[serde(default = "default_concurrent_syncs")]
    pub concurrent_device_taint_eviction_syncs: i32,
}

impl Default for ControllerManagerConfig {
    fn default() -> Self {
        Self {
            generic: GenericControllerManagerConfig::default(),
            kube_cloud_shared: KubeCloudSharedConfig::default(),
            endpoint: EndpointControllerConfig::default(),
            replication: ReplicationControllerConfig::default(),
            pod_gc: PodGCControllerConfig::default(),
            resource_quota: ResourceQuotaControllerConfig::default(),
            namespace: NamespaceControllerConfig::default(),
            service_account: ServiceAccountControllerConfig::default(),
            garbage_collector: GarbageCollectorControllerConfig::default(),
            daemon_set: DaemonSetControllerConfig::default(),
            job: JobControllerConfig::default(),
            deployment: DeploymentControllerConfig::default(),
            replica_set: ReplicaSetControllerConfig::default(),
            hpa: HPAControllerConfig::default(),
            disruption: DisruptionControllerConfig::default(),
            stateful_set: StatefulSetControllerConfig::default(),
            cron_job: CronJobControllerConfig::default(),
            certificates: CertificatesConfig::default(),
            ttl: TTLControllerConfig::default(),
            bootstrap: BootstrapControllerConfig::default(),
            node_ipam: NodeIPAMControllerConfig::default(),
            node_lifecycle: NodeLifecycleControllerConfig::default(),
            pv_binder: PersistentVolumeBinderConfig::default(),
            attach_detach: AttachDetachControllerConfig::default(),
            pvc_protection: PVCProtectionControllerConfig::default(),
            pv_protection: PVProtectionControllerConfig::default(),
            ttl_after_finished: TTLAfterFinishedControllerConfig::default(),
            leader_election: LeaderElectionConfig::default(),
            leader_migration: LeaderMigrationConfig::default(),
            concurrent_device_taint_eviction_syncs: default_concurrent_syncs(),
        }
    }
}

fn default_concurrent_syncs() -> i32 {
    5
}

/// Generic configuration applicable to all controllers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericControllerManagerConfig {
    /// Kubeconfig file for talking to the apiserver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubeconfig: Option<PathBuf>,

    /// Master URL to build a client from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master: Option<String>,

    /// The address to serve metrics.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// The port to serve metrics.
    #[serde(default = "default_metrics_port")]
    pub bind_port: u16,

    /// Enable content_type = application/vnd.kubernetes.protobuf in watched events.
    #[serde(default)]
    pub use_protobuf_content_type: bool,

    /// Enable goroutine profiling via web interface.
    #[serde(default)]
    pub enable_contention_profiling: bool,

    /// Enable profiling via web interface.
    #[serde(default)]
    pub enable_profiling: bool,

    /// Port for pprof.
    #[serde(default = "default_pprof_port")]
    pub pprof_port: u16,

    /// Comma-separated list of controllers to enable.
    /// '*' enables all controllers, 'foo' enables the controller named 'foo',
    /// '-foo' disables the controller named 'foo'.
    #[serde(default = "default_controllers")]
    pub controllers: String,

    /// List of controllers to enable. If empty, all controllers are enabled.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub controllers_enabled: HashSet<String>,

    /// List of controllers to disable.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub controllers_disabled: HashSet<String>,

    /// Minimum resync period for informers.
    #[serde(
        default = "default_min_resync_period",
        with = "humantime_serde"
    )]
    pub min_resync_period: Duration,

    /// The duration between each controller starting.
    #[serde(
        default = "default_controller_start_interval",
        with = "humantime_serde"
    )]
    pub controller_start_interval: Duration,

    /// The maximum time to wait for controllers to stop.
    #[serde(
        default = "default_shutdown_timeout",
        with = "humantime_serde"
    )]
    pub shutdown_timeout: Duration,

    /// The number of concurrent workers for each controller.
    #[serde(default = "default_concurrent_syncs")]
    pub concurrent_syncs: i32,

    /// The client burst QPS.
    #[serde(default = "default_client_burst")]
    pub client_burst: i32,

    /// The client QPS.
    #[serde(default = "default_client_qps")]
    pub client_qps: f32,

    /// The address for the Kubernetes API server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_server: Option<String>,

    /// Namespace to watch for default resources.
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// Port for the health check server.
    #[serde(default = "default_healthz_port")]
    pub healthz_bind_port: u16,

    /// Whether to enable leader election.
    #[serde(default)]
    pub leader_election_enabled: bool,

    /// Whether to enable the insecure diagnostics server.
    #[serde(default = "default_enable_diagnostics")]
    pub enable_diagnostics: bool,

    /// Port for the insecure diagnostics server.
    #[serde(default = "default_diagnostics_port")]
    pub diagnostics_port: u16,
}

impl Default for GenericControllerManagerConfig {
    fn default() -> Self {
        Self {
            kubeconfig: None,
            master: None,
            bind_address: default_bind_address(),
            bind_port: default_metrics_port(),
            use_protobuf_content_type: false,
            enable_contention_profiling: false,
            enable_profiling: false,
            pprof_port: default_pprof_port(),
            controllers: default_controllers(),
            controllers_enabled: HashSet::new(),
            controllers_disabled: HashSet::new(),
            min_resync_period: default_min_resync_period(),
            controller_start_interval: default_controller_start_interval(),
            shutdown_timeout: default_shutdown_timeout(),
            concurrent_syncs: default_concurrent_syncs(),
            client_burst: default_client_burst(),
            client_qps: default_client_qps(),
            api_server: None,
            namespace: default_namespace(),
            healthz_bind_port: default_healthz_port(),
            leader_election_enabled: default_leader_election(),
            enable_diagnostics: default_enable_diagnostics(),
            diagnostics_port: default_diagnostics_port(),
        }
    }
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_metrics_port() -> u16 {
    10252
}

fn default_pprof_port() -> u16 {
    10253
}

fn default_controllers() -> String {
    "*".to_string()
}

fn default_min_resync_period() -> Duration {
    Duration::from_secs(12 * 60 * 60) // 12 hours
}

fn default_controller_start_interval() -> Duration {
    Duration::from_millis(100)
}

fn default_shutdown_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_client_burst() -> i32 {
    100
}

fn default_client_qps() -> f32 {
    50.0
}

fn default_namespace() -> String {
    "default".to_string()
}

fn default_healthz_port() -> u16 {
    10257
}

fn default_leader_election() -> bool {
    true
}

fn default_enable_diagnostics() -> bool {
    true
}

fn default_diagnostics_port() -> u16 {
    10256
}

/// Cloud provider related configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KubeCloudSharedConfig {
    /// The cloud provider name.
    #[serde(default)]
    pub cloud_provider: CloudProvider,

    /// CIDR Range for Pods in cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_cidr: Option<String>,

    /// CIDR Range for services in cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_cidr: Option<String>,

    /// Allocate Node CIDRs (for cloud-provider integration).
    #[serde(default)]
    pub allocate_node_cidrs: bool,

    /// CIDR Allocator type.
    #[serde(default)]
    pub cidr_allocator_type: CidrAllocatorType,
}

/// Cloud provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    /// No cloud provider.
    #[default]
    None,
    /// AWS cloud provider.
    Aws,
    /// Azure cloud provider.
    Azure,
    /// GCE cloud provider.
    Gce,
    /// OpenStack cloud provider.
    Openstack,
    /// VSphere cloud provider.
    Vsphere,
}

/// CIDR allocator types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CidrAllocatorType {
    /// Range allocator (default).
    #[default]
    Range,
    /// Cloud provider allocator.
    Cloud,
}

/// Endpoint controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EndpointControllerConfig {
    /// Number of endpoint syncing workers.
    #[serde(default = "default_endpoint_concurrent_syncs")]
    pub concurrent_endpoint_syncs: i32,

    /// Endpoint updates batch period.
    #[serde(
        default = "default_endpoint_batch_period",
        with = "humantime_serde"
    )]
    pub endpoint_updates_batch_period: Duration,
}

fn default_endpoint_concurrent_syncs() -> i32 {
    5
}

fn default_endpoint_batch_period() -> Duration {
    Duration::from_millis(50)
}

/// Replication controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplicationControllerConfig {
    /// Number of replication controller workers.
    #[serde(default = "default_rc_concurrent_syncs")]
    pub concurrent_rc_syncs: i32,
}

fn default_rc_concurrent_syncs() -> i32 {
    5
}

/// Pod garbage collector configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodGCControllerConfig {
    /// Number of terminated pods to keep.
    #[serde(default = "default_terminated_pod_threshold")]
    pub terminated_pod_gc_threshold: i32,
}

fn default_terminated_pod_threshold() -> i32 {
    12500
}

/// Resource quota controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceQuotaControllerConfig {
    /// Number of resource quota workers.
    #[serde(default = "default_rq_concurrent_syncs")]
    pub concurrent_resource_quota_syncs: i32,

    /// Sync period for resource quota.
    #[serde(
        default = "default_rq_sync_period",
        with = "humantime_serde"
    )]
    pub resource_quota_sync_period: Duration,
}

fn default_rq_concurrent_syncs() -> i32 {
    5
}

fn default_rq_sync_period() -> Duration {
    Duration::from_secs(60)
}

/// Namespace controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceControllerConfig {
    /// Number of namespace workers.
    #[serde(default = "default_ns_concurrent_syncs")]
    pub concurrent_namespace_syncs: i32,

    /// Namespace sync period.
    #[serde(
        default = "default_ns_sync_period",
        with = "humantime_serde"
    )]
    pub namespace_sync_period: Duration,
}

fn default_ns_concurrent_syncs() -> i32 {
    10
}

fn default_ns_sync_period() -> Duration {
    Duration::from_secs(60)
}

/// Service account controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountControllerConfig {
    /// Enable service account token creation.
    #[serde(default = "default_sa_token")]
    pub use_service_account_token: bool,
}

fn default_sa_token() -> bool {
    true
}

/// Garbage collector controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GarbageCollectorControllerConfig {
    /// Number of garbage collector workers.
    #[serde(default = "default_gc_concurrent_syncs")]
    pub concurrent_gc_syncs: i32,

    /// Enable garbage collector.
    #[serde(default = "default_gc_enabled")]
    pub enable_garbage_collector: bool,

    /// Garbage collector ignored resources.
    #[serde(default)]
    pub gc_ignored_resources: Vec<String>,
}

fn default_gc_concurrent_syncs() -> i32 {
    20
}

fn default_gc_enabled() -> bool {
    true
}

/// DaemonSet controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DaemonSetControllerConfig {
    /// Number of DaemonSet workers.
    #[serde(default = "default_ds_concurrent_syncs")]
    pub concurrent_daemon_set_syncs: i32,
}

fn default_ds_concurrent_syncs() -> i32 {
    5
}

/// Job controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobControllerConfig {
    /// Number of Job workers.
    #[serde(default = "default_job_concurrent_syncs")]
    pub concurrent_job_syncs: i32,
}

fn default_job_concurrent_syncs() -> i32 {
    5
}

/// Deployment controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentControllerConfig {
    /// Number of Deployment workers.
    #[serde(default = "default_deploy_concurrent_syncs")]
    pub concurrent_deployment_syncs: i32,
}

fn default_deploy_concurrent_syncs() -> i32 {
    5
}

/// ReplicaSet controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaSetControllerConfig {
    /// Number of ReplicaSet workers.
    #[serde(default = "default_rs_concurrent_syncs")]
    pub concurrent_rs_syncs: i32,
}

fn default_rs_concurrent_syncs() -> i32 {
    5
}

/// Horizontal Pod Autoscaler configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HPAControllerConfig {
    /// Number of HPA workers.
    #[serde(default = "default_hpa_concurrent_syncs")]
    pub concurrent_horizontal_pod_autoscaler_syncs: i32,

    /// HPA sync period.
    #[serde(
        default = "default_hpa_sync_period",
        with = "humantime_serde"
    )]
    pub horizontal_pod_autoscaler_sync_period: Duration,

    /// HPA downscale stabilization window.
    #[serde(
        default = "default_hpa_downscale_stabilization",
        with = "humantime_serde"
    )]
    pub horizontal_pod_autoscaler_downscale_stabilization_window: Duration,

    /// HPA tolerance.
    #[serde(default = "default_hpa_tolerance")]
    pub horizontal_pod_autoscaler_tolerance: f64,

    /// HPA CPU initialization period.
    #[serde(
        default = "default_hpa_cpu_init_period",
        with = "humantime_serde"
    )]
    pub horizontal_pod_autoscaler_cpu_initialization_period: Duration,

    /// HPA initial readiness delay.
    #[serde(
        default = "default_hpa_init_readiness_delay",
        with = "humantime_serde"
    )]
    pub horizontal_pod_autoscaler_initial_readiness_delay: Duration,
}

fn default_hpa_concurrent_syncs() -> i32 {
    5
}

fn default_hpa_sync_period() -> Duration {
    Duration::from_secs(15)
}

fn default_hpa_downscale_stabilization() -> Duration {
    Duration::from_secs(300)
}

fn default_hpa_tolerance() -> f64 {
    0.1
}

fn default_hpa_cpu_init_period() -> Duration {
    Duration::from_secs(300)
}

fn default_hpa_init_readiness_delay() -> Duration {
    Duration::from_secs(30)
}

/// Disruption (PodDisruptionBudget) controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DisruptionControllerConfig {
    /// Number of disruption workers.
    #[serde(default = "default_disruption_concurrent_syncs")]
    pub concurrent_disruption_syncs: i32,
}

fn default_disruption_concurrent_syncs() -> i32 {
    5
}

/// StatefulSet controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatefulSetControllerConfig {
    /// Number of StatefulSet workers.
    #[serde(default = "default_sts_concurrent_syncs")]
    pub concurrent_stateful_set_syncs: i32,
}

fn default_sts_concurrent_syncs() -> i32 {
    5
}

/// CronJob controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CronJobControllerConfig {
    /// Number of CronJob workers.
    #[serde(default = "default_cj_concurrent_syncs")]
    pub concurrent_cron_job_syncs: i32,
}

fn default_cj_concurrent_syncs() -> i32 {
    5
}

/// Certificate controllers configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertificatesConfig {
    /// Cluster root CA certificate file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_root_ca_cert_file: Option<PathBuf>,

    /// Cluster root CA certificate file for controllers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_root_ca_cert_file_controller: Option<PathBuf>,

    /// Request header CA certificate file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestheader_client_ca_file: Option<PathBuf>,

    /// Request header allowed names.
    #[serde(default)]
    pub requestheader_allowed_names: Vec<String>,

    /// Request header extra headers.
    #[serde(default)]
    pub requestheader_extra_headers_prefixes: Vec<String>,

    /// Request header group headers.
    #[serde(default)]
    pub requestheader_group_headers: Vec<String>,

    /// Request header username headers.
    #[serde(default)]
    pub requestheader_username_headers: Vec<String>,

    /// Client CA file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ca_file: Option<PathBuf>,

    /// Enable the CSR signing controller.
    #[serde(default = "default_csr_signing_enabled")]
    pub enable_signing_controller: bool,

    /// Enable the CSR approving controller.
    #[serde(default = "default_csr_approving_enabled")]
    pub enable_approving_controller: bool,

    /// Enable the CSR clean up controller.
    #[serde(default = "default_csr_cleaning_enabled")]
    pub enable_cleaning_controller: bool,

    /// Cluster signing duration.
    #[serde(
        default = "default_cluster_signing_duration",
        with = "humantime_serde"
    )]
    pub cluster_signing_duration: Duration,
}

fn default_csr_signing_enabled() -> bool {
    true
}

fn default_csr_approving_enabled() -> bool {
    true
}

fn default_csr_cleaning_enabled() -> bool {
    true
}

fn default_cluster_signing_duration() -> Duration {
    Duration::from_secs(365 * 24 * 60 * 60) // 1 year
}

/// TTL controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TTLControllerConfig {
    /// Number of TTL workers.
    #[serde(default = "default_ttl_concurrent_syncs")]
    pub concurrent_ttl_syncs: i32,
}

fn default_ttl_concurrent_syncs() -> i32 {
    5
}

/// Bootstrap controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapControllerConfig {
    /// Bootstrap token sign duration.
    #[serde(
        default = "default_bootstrap_token_duration",
        with = "humantime_serde"
    )]
    pub bootstrap_token_sign_duration: Duration,
}

fn default_bootstrap_token_duration() -> Duration {
    Duration::from_secs(60 * 60 * 24 * 365 * 10) // 10 years
}

/// Node IPAM controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeIPAMControllerConfig {
    /// Service CIDR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_cidr: Option<String>,

    /// Secondary service CIDR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_service_cidr: Option<String>,

    /// Node CIDR mask size for IPv4.
    #[serde(default = "default_node_cidr_mask_size_ipv4")]
    pub node_cidr_mask_size_ipv4: i32,

    /// Node CIDR mask size for IPv6.
    #[serde(default = "default_node_cidr_mask_size_ipv6")]
    pub node_cidr_mask_size_ipv6: i32,
}

fn default_node_cidr_mask_size_ipv4() -> i32 {
    24
}

fn default_node_cidr_mask_size_ipv6() -> i32 {
    64
}

/// Node lifecycle controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeLifecycleControllerConfig {
    /// Node monitor period.
    #[serde(
        default = "default_node_monitor_period",
        with = "humantime_serde"
    )]
    pub node_monitor_period: Duration,

    /// Node startup grace period.
    #[serde(
        default = "default_node_startup_grace_period",
        with = "humantime_serde"
    )]
    pub node_startup_grace_period: Duration,

    /// Node monitor grace period.
    #[serde(
        default = "default_node_monitor_grace_period",
        with = "humantime_serde"
    )]
    pub node_monitor_grace_period: Duration,

    /// Node eviction rate.
    #[serde(default = "default_node_eviction_rate")]
    pub node_eviction_rate: f64,

    /// Secondary node eviction rate.
    #[serde(default = "default_secondary_node_eviction_rate")]
    pub secondary_node_eviction_rate: f64,

    /// Large cluster size threshold.
    #[serde(default = "default_large_cluster_threshold")]
    pub large_cluster_size_threshold: i32,

    /// Unhealthy zone threshold.
    #[serde(default = "default_unhealthy_zone_threshold")]
    pub unhealthy_zone_threshold: f64,
}

fn default_node_monitor_period() -> Duration {
    Duration::from_secs(5)
}

fn default_node_startup_grace_period() -> Duration {
    Duration::from_secs(60)
}

fn default_node_monitor_grace_period() -> Duration {
    Duration::from_secs(40)
}

fn default_node_eviction_rate() -> f64 {
    0.1
}

fn default_secondary_node_eviction_rate() -> f64 {
    0.01
}

fn default_large_cluster_threshold() -> i32 {
    500
}

fn default_unhealthy_zone_threshold() -> f64 {
    0.55
}

/// Persistent volume binder controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeBinderConfig {
    /// PV claim binder sync period.
    #[serde(
        default = "default_pv_claim_binder_sync_period",
        with = "humantime_serde"
    )]
    pub pv_claim_binder_sync_period: Duration,

    /// Volume configuration.
    #[serde(default)]
    pub volume_configuration: VolumeConfiguration,
}

fn default_pv_claim_binder_sync_period() -> Duration {
    Duration::from_secs(15)
}

/// Volume configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VolumeConfiguration {
    /// Enable dynamic provisioning.
    #[serde(default = "default_enable_dynamic_provisioning")]
    pub enable_dynamic_provisioning: bool,
}

fn default_enable_dynamic_provisioning() -> bool {
    true
}

/// Attach detach controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttachDetachControllerConfig {
    /// Reconciler sync loop period.
    #[serde(
        default = "default_reconciler_sync_loop_period",
        with = "humantime_serde"
    )]
    pub reconciler_sync_loop_period: Duration,

    /// Disable attach detach reconciler sync.
    #[serde(default)]
    pub disable_attach_detach_reconciler_sync: bool,

    /// Disable force detach on timeout.
    #[serde(default)]
    pub disable_force_detach_on_timeout: bool,
}

fn default_reconciler_sync_loop_period() -> Duration {
    Duration::from_secs(60)
}

/// Persistent volume claim protection controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PVCProtectionControllerConfig {
    /// Enable PVC protection.
    #[serde(default = "default_pvc_protection")]
    pub enable_pvc_protection: bool,
}

fn default_pvc_protection() -> bool {
    true
}

/// Persistent volume protection controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PVProtectionControllerConfig {
    /// Enable PV protection.
    #[serde(default = "default_pv_protection")]
    pub enable_pv_protection: bool,
}

fn default_pv_protection() -> bool {
    true
}

/// TTL after finished controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TTLAfterFinishedControllerConfig {
    /// Number of TTL after finished workers.
    #[serde(default = "default_ttl_after_finished_concurrent_syncs")]
    pub concurrent_ttl_syncs: i32,
}

fn default_ttl_after_finished_concurrent_syncs() -> i32 {
    5
}

/// Leader election configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeaderElectionConfig {
    /// Enable leader election.
    #[serde(default)]
    pub leader_elect: bool,

    /// Resource lock type.
    #[serde(default = "default_resource_lock")]
    pub resource_lock: String,

    /// Resource namespace.
    #[serde(default = "default_resource_namespace")]
    pub resource_namespace: String,

    /// Resource name.
    #[serde(default = "default_resource_name")]
    pub resource_name: String,

    /// Lease duration.
    #[serde(
        default = "default_lease_duration",
        with = "humantime_serde"
    )]
    pub lease_duration: Duration,

    /// Renew deadline.
    #[serde(
        default = "default_renew_deadline",
        with = "humantime_serde"
    )]
    pub renew_deadline: Duration,

    /// Retry period.
    #[serde(
        default = "default_retry_period",
        with = "humantime_serde"
    )]
    pub retry_period: Duration,
}

fn default_resource_lock() -> String {
    "leases".to_string()
}

fn default_resource_namespace() -> String {
    "kube-system".to_string()
}

fn default_resource_name() -> String {
    "kube-controller-manager".to_string()
}

fn default_lease_duration() -> Duration {
    Duration::from_secs(15)
}

fn default_renew_deadline() -> Duration {
    Duration::from_secs(10)
}

fn default_retry_period() -> Duration {
    Duration::from_secs(2)
}

/// Leader migration configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeaderMigrationConfig {
    /// Enable leader migration.
    #[serde(default)]
    pub enabled: bool,

    /// Resource lock type.
    #[serde(default = "default_migration_resource_lock")]
    pub resource_lock: String,

    /// Resource name.
    #[serde(default = "default_migration_resource_name")]
    pub leader_name: String,
}

fn default_migration_resource_lock() -> String {
    "leases".to_string()
}

fn default_migration_resource_name() -> String {
    "kube-controller-manager-migration".to_string()
}

/// Module for duration serialization/deserialization with human-readable format.
mod humantime_serde {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&humantime::format_duration(*duration).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        humantime::parse_duration(&s)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ControllerManagerConfig::default();
        assert_eq!(config.generic.controllers, "*");
    }

    #[test]
    fn test_deserialize_basic_config() {
        let yaml = r#"
generic:
  controllers: "deployment,service"
  minResyncPeriod: 10m
  shutdownTimeout: 60s
"#;

        let config: ControllerManagerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.generic.controllers, "deployment,service");
        assert_eq!(config.generic.min_resync_period, Duration::from_secs(600));
    }
}
