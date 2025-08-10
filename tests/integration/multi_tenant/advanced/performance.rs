//! Performance monitoring and statistics for multi-tenant SCIM operations.
//!
//! This module contains the data structures and functionality for monitoring
//! tenant performance, collecting usage statistics, and tracking resource
//! utilization across multiple tenants.

use std::collections::HashMap;

/// Tenant usage statistics
#[derive(Debug)]
pub struct TenantStatistics {
    pub tenant_id: String,
    pub total_resources: usize,
    pub resources_by_type: HashMap<String, usize>,
    pub storage_usage_bytes: u64,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub operations_count: u64,
}

impl TenantStatistics {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            total_resources: 0,
            resources_by_type: HashMap::new(),
            storage_usage_bytes: 0,
            last_activity: None,
            operations_count: 0,
        }
    }

    pub fn with_resource_count(mut self, resource_type: String, count: usize) -> Self {
        self.resources_by_type.insert(resource_type, count);
        self.total_resources += count;
        self
    }

    pub fn with_storage_usage(mut self, bytes: u64) -> Self {
        self.storage_usage_bytes = bytes;
        self
    }

    pub fn with_last_activity(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.last_activity = Some(timestamp);
        self
    }

    pub fn with_operations_count(mut self, count: u64) -> Self {
        self.operations_count = count;
        self
    }

    pub fn increment_operations(&mut self) {
        self.operations_count += 1;
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Some(chrono::Utc::now());
    }
}

/// Performance metrics for monitoring system health
#[derive(Debug)]
pub struct PerformanceMetrics {
    pub tenant_id: String,
    pub response_times: Vec<std::time::Duration>,
    pub throughput_ops_per_second: f64,
    pub memory_usage_bytes: u64,
    pub cache_hit_ratio: f64,
    pub error_rate: f64,
}

impl PerformanceMetrics {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            response_times: Vec::new(),
            throughput_ops_per_second: 0.0,
            memory_usage_bytes: 0,
            cache_hit_ratio: 0.0,
            error_rate: 0.0,
        }
    }

    pub fn record_response_time(&mut self, duration: std::time::Duration) {
        self.response_times.push(duration);
        // Keep only the last 100 response times for memory efficiency
        if self.response_times.len() > 100 {
            self.response_times.remove(0);
        }
    }

    pub fn average_response_time(&self) -> Option<std::time::Duration> {
        if self.response_times.is_empty() {
            None
        } else {
            let total: std::time::Duration = self.response_times.iter().sum();
            Some(total / self.response_times.len() as u32)
        }
    }

    pub fn p95_response_time(&self) -> Option<std::time::Duration> {
        if self.response_times.is_empty() {
            return None;
        }

        let mut sorted_times = self.response_times.clone();
        sorted_times.sort();
        let index = (sorted_times.len() as f64 * 0.95) as usize;
        sorted_times.get(index.min(sorted_times.len() - 1)).copied()
    }
}

/// Resource utilization tracking for capacity planning
#[derive(Debug)]
pub struct ResourceUtilization {
    pub tenant_id: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_io_bytes_per_second: u64,
    pub active_connections: u32,
}

impl ResourceUtilization {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            network_io_bytes_per_second: 0,
            active_connections: 0,
        }
    }

    pub fn is_under_pressure(&self) -> bool {
        self.cpu_usage_percent > 80.0
            || self.memory_usage_percent > 85.0
            || self.disk_usage_percent > 90.0
    }

    pub fn health_score(&self) -> f64 {
        let cpu_score = (100.0 - self.cpu_usage_percent) / 100.0;
        let memory_score = (100.0 - self.memory_usage_percent) / 100.0;
        let disk_score = (100.0 - self.disk_usage_percent) / 100.0;

        (cpu_score + memory_score + disk_score) / 3.0
    }
}
