# Monitoring and Observability

This guide covers comprehensive monitoring, metrics, logging, and observability strategies for SCIM Server deployments. Effective observability is crucial for maintaining reliable identity management systems at scale.

## Overview

Observability for SCIM servers encompasses:

- **Metrics Collection** - Performance and business metrics
- **Logging** - Structured application and audit logs
- **Tracing** - Distributed request tracing
- **Health Checks** - Service health and readiness
- **Alerting** - Proactive incident detection
- **Dashboards** - Visual monitoring and analysis

## Metrics Collection

### Built-in Metrics

The SCIM Server library provides comprehensive metrics out of the box:

```rust
use scim_server::metrics::{MetricsConfig, MetricsCollector, PrometheusExporter};

let metrics_config = MetricsConfig::builder()
    .enable_http_metrics(true)
    .enable_business_metrics(true)
    .enable_system_metrics(true)
    .prometheus_endpoint("/metrics")
    .collection_interval_seconds(15)
    .histogram_buckets(vec![0.001, 0.01, 0.1, 1.0, 10.0])
    .build()?;

let metrics_collector = MetricsCollector::new(metrics_config);
let server = ScimServer::new(storage)
    .with_metrics(metrics_collector)
    .await?;
```

### HTTP Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `scim_http_requests_total` | Counter | Total HTTP requests | `method`, `endpoint`, `status_code`, `tenant_id` |
| `scim_http_request_duration_seconds` | Histogram | Request duration | `method`, `endpoint`, `status_code` |
| `scim_http_request_size_bytes` | Histogram | Request body size | `method`, `endpoint` |
| `scim_http_response_size_bytes` | Histogram | Response body size | `method`, `endpoint` |
| `scim_http_requests_in_flight` | Gauge | Concurrent requests | `endpoint` |

### Business Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `scim_users_total` | Gauge | Total users in system | `tenant_id`, `active` |
| `scim_groups_total` | Gauge | Total groups in system | `tenant_id` |
| `scim_operations_total` | Counter | SCIM operations performed | `operation`, `resource_type`, `tenant_id` |
| `scim_bulk_operations_total` | Counter | Bulk operations performed | `tenant_id`, `status` |
| `scim_filter_queries_total` | Counter | Filter queries executed | `complexity`, `tenant_id` |
| `scim_authentication_attempts_total` | Counter | Authentication attempts | `method`, `result`, `tenant_id` |

### System Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `scim_memory_usage_bytes` | Gauge | Memory usage | `type` |
| `scim_cpu_usage_percentage` | Gauge | CPU usage | - |
| `scim_database_connections_active` | Gauge | Active DB connections | `pool` |
| `scim_database_connections_idle` | Gauge | Idle DB connections | `pool` |
| `scim_cache_hits_total` | Counter | Cache hits | `cache_type` |
| `scim_cache_misses_total` | Counter | Cache misses | `cache_type` |

### Custom Metrics

```rust
use scim_server::metrics::{Counter, Histogram, Gauge, MetricRegistry};

pub struct CustomMetrics {
    tenant_provisioning_duration: Histogram,
    active_sessions: Gauge,
    password_reset_requests: Counter,
    data_sync_errors: Counter,
}

impl CustomMetrics {
    pub fn new(registry: &MetricRegistry) -> Self {
        Self {
            tenant_provisioning_duration: registry.register_histogram(
                "scim_tenant_provisioning_duration_seconds",
                "Time to provision new tenant",
                vec!["tenant_type"]
            ),
            active_sessions: registry.register_gauge(
                "scim_active_sessions",
                "Number of active user sessions",
                vec!["tenant_id"]
            ),
            password_reset_requests: registry.register_counter(
                "scim_password_reset_requests_total",
                "Password reset requests",
                vec!["tenant_id", "method"]
            ),
            data_sync_errors: registry.register_counter(
                "scim_data_sync_errors_total",
                "Data synchronization errors",
                vec!["source_system", "error_type"]
            ),
        }
    }
    
    pub fn record_tenant_provisioning(&self, tenant_type: &str, duration: Duration) {
        self.tenant_provisioning_duration
            .with_label_values(&[tenant_type])
            .observe(duration.as_secs_f64());
    }
    
    pub fn update_active_sessions(&self, tenant_id: &str, count: i64) {
        self.active_sessions
            .with_label_values(&[tenant_id])
            .set(count as f64);
    }
}
```

## Structured Logging

### Logging Configuration

```rust
use scim_server::logging::{LoggingConfig, LogFormat, LogLevel};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

let logging_config = LoggingConfig::builder()
    .level(LogLevel::Info)
    .format(LogFormat::Json)
    .enable_spans(true)
    .enable_events(true)
    .fields(vec![
        "timestamp".to_string(),
        "level".to_string(),
        "target".to_string(),
        "message".to_string(),
        "tenant_id".to_string(),
        "user_id".to_string(),
        "request_id".to_string(),
        "operation".to_string(),
        "resource_type".to_string(),
        "duration_ms".to_string(),
    ])
    .exclude_fields(vec!["password".to_string(), "token".to_string()])
    .max_log_level_per_module(HashMap::from([
        ("scim_server::auth".to_string(), LogLevel::Debug),
        ("sqlx".to_string(), LogLevel::Warn),
    ]))
    .build()?;

// Initialize structured logging
tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "scim_server=info".into())
    )
    .with(
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
    )
    .init();
```

### Contextual Logging

```rust
use tracing::{info, warn, error, debug, Span};
use tracing_futures::Instrument;

// Create request-scoped spans
#[tracing::instrument(
    name = "create_user",
    skip(user_data),
    fields(
        tenant_id = %tenant_id,
        user_name = %user_data.user_name.as_deref().unwrap_or("unknown"),
        operation = "create_user"
    )
)]
pub async fn create_user(
    tenant_id: &str,
    user_data: CreateUserRequest,
) -> Result<User, ScimError> {
    let span = Span::current();
    
    // Add dynamic fields to the span
    span.record("user_id", &tracing::field::display(&user_data.id));
    
    info!("Starting user creation");
    
    // Validate input
    match validate_user_data(&user_data).await {
        Ok(_) => debug!("User data validation passed"),
        Err(e) => {
            warn!(error = %e, "User data validation failed");
            return Err(ScimError::ValidationError(e));
        }
    }
    
    // Create user in storage
    let start_time = Instant::now();
    let user = match storage.create_user(tenant_id, user_data).await {
        Ok(user) => {
            let duration = start_time.elapsed();
            info!(
                duration_ms = duration.as_millis(),
                user_id = %user.id,
                "User created successfully"
            );
            user
        }
        Err(e) => {
            error!(
                error = %e,
                duration_ms = start_time.elapsed().as_millis(),
                "Failed to create user"
            );
            return Err(e);
        }
    };
    
    // Log business event
    info!(
        event_type = "user_created",
        user_id = %user.id,
        tenant_id = %tenant_id,
        user_name = %user.user_name,
        active = user.active,
        "User creation completed"
    );
    
    Ok(user)
}
```

### Audit Logging

```rust
use scim_server::audit::{AuditLogger, AuditEvent, AuditLevel};
use serde_json::json;

pub struct AuditLogger {
    logger: tracing::Span,
    config: AuditConfig,
}

impl AuditLogger {
    pub async fn log_user_operation(&self, event: UserOperationEvent) {
        let audit_event = AuditEvent {
            timestamp: Utc::now(),
            event_type: "user_operation".to_string(),
            actor: ActorInfo {
                user_id: event.actor_id.clone(),
                session_id: event.session_id.clone(),
                ip_address: event.ip_address,
                user_agent: event.user_agent.clone(),
            },
            resource: ResourceInfo {
                resource_type: "User".to_string(),
                resource_id: event.user_id.clone(),
                tenant_id: event.tenant_id.clone(),
            },
            operation: OperationInfo {
                operation_type: event.operation.clone(),
                method: event.http_method.clone(),
                endpoint: event.endpoint.clone(),
                success: event.success,
                error_message: event.error_message.clone(),
            },
            details: json!({
                "user_id": event.user_id,
                "fields_modified": event.fields_modified,
                "before_values": event.before_values,
                "after_values": event.after_values,
                "request_size": event.request_size,
                "response_size": event.response_size,
            }),
        };
        
        // Log to structured log
        info!(
            target: "audit",
            event_type = %audit_event.event_type,
            actor_id = %audit_event.actor.user_id,
            resource_type = %audit_event.resource.resource_type,
            resource_id = %audit_event.resource.resource_id,
            tenant_id = %audit_event.resource.tenant_id,
            operation = %audit_event.operation.operation_type,
            success = audit_event.operation.success,
            ip_address = %audit_event.actor.ip_address,
            user_agent = %audit_event.actor.user_agent.as_deref().unwrap_or("unknown"),
            "{}", serde_json::to_string(&audit_event.details).unwrap_or_default()
        );
        
        // Send to external audit system
        if let Some(webhook_url) = &self.config.audit_webhook_url {
            self.send_to_webhook(webhook_url, &audit_event).await;
        }
        
        // Store in database for compliance
        if self.config.store_in_database {
            self.store_audit_event(&audit_event).await;
        }
    }
    
    pub async fn log_authentication_event(&self, event: AuthenticationEvent) {
        info!(
            target: "audit.auth",
            event_type = "authentication",
            user_id = %event.user_id.as_deref().unwrap_or("unknown"),
            tenant_id = %event.tenant_id.as_deref().unwrap_or("unknown"),
            auth_method = %event.auth_method,
            success = event.success,
            failure_reason = %event.failure_reason.as_deref().unwrap_or(""),
            ip_address = %event.ip_address,
            user_agent = %event.user_agent.as_deref().unwrap_or("unknown"),
            session_id = %event.session_id.as_deref().unwrap_or(""),
            "Authentication attempt"
        );
        
        // Increment authentication metrics
        if event.success {
            self.metrics.authentication_success.inc();
        } else {
            self.metrics.authentication_failure.inc();
        }
    }
}
```

## Distributed Tracing

### OpenTelemetry Integration

```rust
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;

pub async fn setup_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OpenTelemetry exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://jaeger:14268/api/traces")
        )
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_sampler(opentelemetry::sdk::trace::Sampler::TraceIdRatioBased(0.1))
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    KeyValue::new("service.name", "scim-server"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", "production"),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    // Create tracing subscriber with OpenTelemetry layer
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("scim_server=info"))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    
    Ok(())
}
```

### Trace Instrumentation

```rust
use tracing::{instrument, Span};
use opentelemetry::trace::{TraceContextExt, Tracer};

#[instrument(
    name = "scim.user.create",
    skip(storage, user_data),
    fields(
        scim.tenant_id = %tenant_id,
        scim.operation = "create",
        scim.resource_type = "User",
        user.name = %user_data.user_name.as_deref().unwrap_or("unknown"),
        otel.kind = "server"
    )
)]
pub async fn create_user_with_tracing(
    storage: &dyn StorageProvider,
    tenant_id: &str,
    user_data: CreateUserRequest,
) -> Result<User, ScimError> {
    let span = Span::current();
    let cx = span.context();
    
    // Add custom attributes
    if let Some(trace_id) = cx.span().span_context().trace_id().to_string() {
        span.record("trace_id", &tracing::field::display(&trace_id));
    }
    
    // Validate user data (child span)
    validate_user_data(&user_data)
        .instrument(tracing::info_span!("scim.validation", validation.type = "user"))
        .await?;
    
    // Check uniqueness (child span)
    check_user_uniqueness(storage, tenant_id, &user_data.user_name)
        .instrument(tracing::info_span!(
            "scim.uniqueness_check",
            db.operation = "select",
            user.name = %user_data.user_name
        ))
        .await?;
    
    // Create user in storage (child span)
    let user = storage
        .create_user(tenant_id, user_data)
        .instrument(tracing::info_span!(
            "scim.storage.create",
            db.operation = "insert",
            db.table = "users"
        ))
        .await?;
    
    // Record successful creation
    span.record("user.id", &tracing::field::display(&user.id));
    span.record("scim.status", "success");
    
    Ok(user)
}
```

## Health Checks

### Health Check Implementation

```rust
use scim_server::health::{HealthChecker, HealthStatus, HealthCheckResult};

pub struct HealthChecker {
    storage: Arc<dyn StorageProvider>,
    auth_service: Arc<dyn AuthService>,
    cache: Option<Arc<dyn CacheProvider>>,
    config: HealthCheckConfig,
}

impl HealthChecker {
    pub async fn check_health(&self) -> HealthCheckResult {
        let mut checks = Vec::new();
        
        // Overall health check
        checks.push(self.check_application_health().await);
        
        // Storage health
        checks.push(self.check_storage_health().await);
        
        // Authentication service health
        checks.push(self.check_auth_service_health().await);
        
        // Cache health (if configured)
        if let Some(cache) = &self.cache {
            checks.push(self.check_cache_health(cache).await);
        }
        
        // External dependencies
        checks.push(self.check_external_dependencies().await);
        
        let overall_status = if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Critical) {
            HealthStatus::Critical
        } else {
            HealthStatus::Degraded
        };
        
        HealthCheckResult {
            status: overall_status,
            timestamp: Utc::now(),
            checks,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.get_uptime_seconds(),
        }
    }
    
    async fn check_storage_health(&self) -> HealthCheck {
        let start_time = Instant::now();
        
        match timeout(Duration::from_secs(5), self.storage.health_check()).await {
            Ok(Ok(_)) => HealthCheck {
                name: "storage".to_string(),
                status: HealthStatus::Healthy,
                response_time_ms: start_time.elapsed().as_millis() as u64,
                message: Some("Storage is responsive".to_string()),
                details: None,
            },
            Ok(Err(e)) => HealthCheck {
                name: "storage".to_string(),
                status: HealthStatus::Critical,
                response_time_ms: start_time.elapsed().as_millis() as u64,
                message: Some(format!("Storage error: {}", e)),
                details: Some(json!({
                    "error_type": "storage_error",
                    "error_details": e.to_string()
                })),
            },
            Err(_) => HealthCheck {
                name: "storage".to_string(),
                status: HealthStatus::Critical,
                response_time_ms: 5000,
                message: Some("Storage health check timeout".to_string()),
                details: Some(json!({
                    "error_type": "timeout",
                    "timeout_seconds": 5
                })),
            },
        }
    }
    
    async fn check_application_health(&self) -> HealthCheck {
        let mut details = serde_json::Map::new();
        
        // Check memory usage
        let memory_usage = self.get_memory_usage();
        details.insert("memory_usage_mb".to_string(), json!(memory_usage));
        
        // Check CPU usage
        let cpu_usage = self.get_cpu_usage().await;
        details.insert("cpu_usage_percent".to_string(), json!(cpu_usage));
        
        // Check active connections
        let active_connections = self.get_active_connections();
        details.insert("active_connections".to_string(), json!(active_connections));
        
        // Determine status based on resource usage
        let status = if memory_usage > 90.0 || cpu_usage > 95.0 {
            HealthStatus::Critical
        } else if memory_usage > 80.0 || cpu_usage > 85.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        HealthCheck {
            name: "application".to_string(),
            status,
            response_time_ms: 0,
            message: Some("Application resource usage check".to_string()),
            details: Some(json!(details)),
        }
    }
}

// Health check endpoints
async fn health_live() -> Result<Json<HealthCheckResult>, StatusCode> {
    // Simple liveness check
    Ok(Json(HealthCheckResult {
        status: HealthStatus::Healthy,
        timestamp: Utc::now(),
        checks: vec![],
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime_seconds(),
    }))
}

async fn health_ready(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<Json<HealthCheckResult>, StatusCode> {
    let result = health_checker.check_health().await;
    
    match result.status {
        HealthStatus::Healthy => Ok(Json(result)),
        HealthStatus::Degraded => {
            warn!("Health check returned degraded status: {:?}", result);
            Ok(Json(result))
        }
        HealthStatus::Critical => {
            error!("Health check returned critical status: {:?}", result);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
```

## Alerting

### Alert Configuration

```rust
use scim_server::alerting::{AlertManager, AlertRule, AlertSeverity, AlertChannel};

pub struct AlertManager {
    rules: Vec<AlertRule>,
    channels: Vec<AlertChannel>,
    metrics_client: MetricsClient,
}

impl AlertManager {
    pub fn new() -> Self {
        let rules = vec![
            // High error rate
            AlertRule {
                name: "high_error_rate".to_string(),
                description: "HTTP error rate above 5%".to_string(),
                query: "rate(scim_http_requests_total{status_code=~'5..'}[5m]) / rate(scim_http_requests_total[5m]) > 0.05".to_string(),
                severity: AlertSeverity::Critical,
                for_duration: Duration::from_secs(300), // 5 minutes
                labels: HashMap::from([
                    ("service".to_string(), "scim-server".to_string()),
                    ("type".to_string(), "error_rate".to_string()),
                ]),
            },
            
            // High response time
            AlertRule {
                name: "high_response_time".to_string(),
                description: "95th percentile response time above 1 second".to_string(),
                query: "histogram_quantile(0.95, rate(scim_http_request_duration_seconds_bucket[5m])) > 1.0".to_string(),
                severity: AlertSeverity::Warning,
                for_duration: Duration::from_secs(600), // 10 minutes
                labels: HashMap::from([
                    ("service".to_string(), "scim-server".to_string()),
                    ("type".to_string(), "performance".to_string()),
                ]),
            },
            
            // Database connection issues
            AlertRule {
                name: "database_connection_exhaustion".to_string(),
                description: "Database connection pool nearly exhausted".to_string(),
                query: "scim_database_connections_active / (scim_database_connections_active + scim_database_connections_idle) > 0.9".to_string(),
                severity: AlertSeverity::Warning,
                for_duration: Duration::from_secs(120), // 2 minutes
                labels: HashMap::from([
                    ("service".to_string(), "scim-server".to_string()),
                    ("type".to_string(), "database".to_string()),
                ]),
            },
            
            // Authentication failures
            AlertRule {
                name: "high_auth_failure_rate".to_string(),
                description: "Authentication failure rate above 10%".to_string(),
                query: "rate(scim_authentication_attempts_total{result='failure'}[5m]) / rate(scim_authentication_attempts_total[5m]) > 0.1".to_string(),
                severity: AlertSeverity::Critical,
                for_duration: Duration::from_secs(180), // 3 minutes
                labels: HashMap::from([
                    ("service".to_string(), "scim-server".to_string()),
                    ("type".to_string(), "security".to_string()),
                ]),
            },
            
            // Memory usage
            AlertRule {
                name: "high_memory_usage".to_string(),
                description: "Memory usage above 85%".to_string(),
                query: "scim_memory_usage_bytes{type='heap'} / scim_memory_usage_bytes{type='total'} > 0.85".to_string(),
                severity: AlertSeverity::Warning,
                for_duration: Duration::from_secs(600), // 10 minutes
                labels: HashMap::from([
                    ("service".to_string(), "scim-server".to_string()),
                    ("type".to_string(), "resource".to_string()),
                ]),
            },
        ];
        
        let channels = vec![
            AlertChannel::Slack {
                webhook_url: std::env::var("SLACK_WEBHOOK_URL").unwrap(),
                channel: "#alerts".to_string(),
                username: "SCIM Monitor".to_string(),
            },
            AlertChannel::PagerDuty {
                integration_key: std::env::var("PAGERDUTY_INTEGRATION_KEY").unwrap(),
            },
            AlertChannel::Email {
                smtp_server: "smtp.company.com".to_string(),
                recipients: vec![
                    "oncall@company.com".to_string(),
                    "devops@company.com".to_string(),
                ],
            },
        ];
        
        Self {
            rules,
            channels,
            metrics_client: MetricsClient::new(),
        }
    }
    
    pub async fn check_alerts(&self) {
        for rule in &self.rules {
            match self.evaluate_rule(rule).await {
                Ok(Some(alert)) => {
                    info!("Alert triggered: {}", alert.name);
                    self.send_alert(&alert).await;
                }
                Ok(None) => {
                    debug!("Alert rule {} is not triggered", rule.name);
                }
                Err(e) => {
                    error!("Failed to evaluate alert rule {}: {}", rule.name, e);
                }
            }
        }
    }
    
    async fn send_alert(&self, alert: &Alert) {
        for channel in &self.channels {
            match channel {
                AlertChannel::Slack { webhook_url, .. } => {
                    self.send_slack_alert(webhook_url, alert).await;
                }
                AlertChannel::PagerDuty { integration_key } => {
                    if alert.severity == AlertSeverity::Critical {
                        self.send_pagerduty_alert(integration_key, alert).await;
                    }
                }
                AlertChannel::Email { recipients, .. } => {
                    self.send_email_alert(recipients, alert).await;
                }
            }
        }
    }
}
```

## Dashboards

### Grafana Dashboard Configuration

```json
{
  "dashboard": {
    "title": "SCIM Server Monitoring",
    "tags": ["scim", "identity", "monitoring"],
    "timezone": "UTC",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(scim_http_requests_total[5m])",
            "legendFormat": "{{method}} {{endpoint}}"
          }
        ],
        "yAxes": [
          {
            "label": "Requests/sec"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(scim_http_requests_total{status_code=~'4..|5..'}[5m]) / rate(scim_http_requests_total[5m])",
            "legendFormat": "Error Rate"
          }
        ],
        "yAxes": [
          {
            "label": "Error Rate (%)",
            "max": 1,
            "min": 0
          }
        ],
        "alert": {
          "conditions": [
            {
              "query": {
                "queryType": "",
                "refId": "A"
              },
              "reducer": {
                "type": "last",
                "params": []
              },
              "evaluator": {
                "params": [0.05],
                "type": "gt"
              }
            }
          ],
          "executionErrorState": "alerting",
          "for": "5m",
          "frequency": "10s",
          "handler": 1,
          "name": "High Error Rate",
          "noDataState": "no_data",
          "notifications": []
        }
      },
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(scim_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "50th percentile"
          },
          {
            "expr": "histogram_quantile(0.95, rate(scim_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          },
          {
            "expr": "histogram_quantile(0.99, rate(scim_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "99th percentile"
          }
        ]
      },
      {
        "title": "Active Users by Tenant",
        "type": "piechart",
        "targets": [
          {
            "expr": "scim_users_total{active='true'}",
            "legendFormat": "{{tenant_id}}"
          }
        ]
      },
      {
        "title": "Database Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "scim_database_connections_active",
            "legendFormat": "Active"
          },
          {
            "expr": "scim_database_connections_idle",
            "legendFormat": "Idle"
          }
        ]
      },
      {
        "title": "Authentication Methods",
        "type": "piechart",
        "targets": [
          {
            "expr": "increase(scim_authentication_attempts_total{result='success'}[1h])",
            "legendFormat": "{{method}}"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(scim_cache_hits_total[5m]) / (rate(scim_cache_hits_total[5m]) + rate(scim_cache_misses_total[5m]))",
            "legendFormat": "Hit Rate"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percentunit",
            "min": 0,
            "max": 1
          }
        }
      }
    ],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "refresh": "30s"
  }
}
```

### Business Intelligence Dashboard

```json
{
  "dashboard": {
    "title": "SCIM Business Metrics",
    "panels": [
      {
        "title": "User Growth by Tenant",
        "type": "graph",
        "targets": [
          {
            "expr": "increase(scim_users_total[24h])",
            "legendFormat": "{{tenant_id}}"
          }
        ]
      },
      {
        "title": "Most Active Operations",
        "type": "table",
        "targets": [
          {
            "expr": "topk(10, increase(scim_operations_total[1h]))",
            "legendFormat": "{{operation}} - {{resource_type}}"
          }
        ]
      },
      {
        "title": "Tenant Resource Usage",
        "type": "heatmap",
        "targets": [
          {
            "expr": "scim_users_total + scim_groups_total",
            "legendFormat": "{{tenant_id}}"
          }
        ]
      }
    ]
  }
}
```

## Log Aggregation and Analysis

### ELK Stack Integration

```yaml
# Filebeat configuration
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/scim-server/*.log
  fields:
    service: scim-server
    environment: production
  fields_under_root: true
  multiline.pattern: '^\d{4}-\d{2}-\d{2}'
  multiline.negate: true
  multiline.match: after

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  template.settings:
    index.number_of_shards: 1
    index.number_of_replicas: 1

processors:
- add_host_metadata:
    when.not.contains.tags: forwarded
- decode_json_fields:
    fields: ["message"]
    target: "json"
    overwrite_keys: true
```

### Logstash Processing

```ruby
# Logstash pipeline configuration
input {
  beats {
    port => 5044
  }
}

filter {
  if [service] == "scim-server" {
    json {
      source => "message"
    }
    
    # Parse timestamp
    date {
      match => [ "timestamp", "ISO8601" ]
    }
    
    # Extract tenant information
    if [tenant_id] {
      mutate {
        add_field => { "[@metadata][tenant]" => "%{tenant_id}" }
      }
    }
    
    # Classify log types
    if [target] == "audit" {
      mutate {
        add_tag => [ "audit" ]
        add_field => { "log_type" => "audit" }
      }
    } else if [level] == "ERROR" {
      mutate {
        add_tag => [ "error" ]
        add_field => { "log_type" => "error" }
      }
    }
    
    # Sanitize sensitive data
    mutate {
      remove_field => [ "password", "token", "authorization" ]
    }
  }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "scim-server-%{+YYYY.MM.dd}"
    template_overwrite => true
    template_pattern => "scim-server-*"
    template => "/etc/logstash/templates/scim-server.json"
  }
}
```

### Kibana Visualizations

```json
{
  "objects": [
    {
      "id": "scim-error-analysis",
      "type": "visualization",
      "attributes": {
        "title": "Error Analysis",
        "visState": {
          "type": "histogram",
          "params": {
            "grid": { "categoryLines": false, "style": { "color": "#eee" } },
            "categoryAxes": [{ "id": "CategoryAxis-1", "type": "category", "position": "bottom", "show": true, "style": {}, "scale": { "type": "linear" }, "labels": { "show": true, "truncate": 100 }, "title": {} }],
            "valueAxes": [{ "id": "ValueAxis-1", "name": "LeftAxis-1", "type": "value", "position": "left", "show": true, "style": {}, "scale": { "type": "linear", "mode": "normal" }, "labels": { "show": true, "rotate": 0, "filter": false, "truncate": 100 }, "title": { "text": "Count" } }]
          },
          "aggs": [
            { "id": "1", "enabled": true, "type": "count", "schema": "metric", "params": {} },
            { "id": "2", "enabled": true, "type": "terms", "schema": "segment", "params": { "field": "error_type.keyword", "size": 10, "order": "desc", "orderBy": "1" } }
          ]
        },
        "uiStateJSON": "{}",
        "description": "",
        "version": 1,
        "kibanaSavedObjectMeta": {
          "searchSourceJSON": {
            "index": "scim-server-*",
            "query": {
              "match": { "level": "ERROR" }
            }
          }
        }
      }
    }
  ]
}
```

## Performance Monitoring

### Application Performance Monitoring (APM)

```rust
use scim_server::apm::{ApmAgent, TransactionType, SpanType};

pub struct ApmAgent {
    elastic_apm: elastic_apm::Agent,
    config: ApmConfig,
}

impl ApmAgent {
    pub async fn trace_operation<F, T>(
        &self,
        operation_name: &str,
        transaction_type: TransactionType,
        operation: F,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let transaction = self.elastic_apm.begin_transaction(
            operation_name,
            transaction_type.as_str(),
        );
        
        let start_time = Instant::now();
        let result = operation.await;
        let duration = start_time.elapsed();
        
        match &result {
            Ok(_) => {
                transaction.set_result("success");
                transaction.set_outcome(elastic_apm::Outcome::Success);
            }
            Err(e) => {
                transaction.set_result("error");
                transaction.set_outcome(elastic_apm::Outcome::Failure);
                transaction.capture_error(e);
            }
        }
        
        transaction.set_custom_context("performance", json!({
            "duration_ms": duration.as_millis(),
            "operation": operation_name,
        }));
        
        transaction.end();
        result
    }
    
    pub fn create_span<F, T>(
        &self,
        span_name: &str,
        span_type: SpanType,
        operation: F,
    ) -> T
    where
        F: FnOnce() -> T,
    {
        let span = self.elastic_apm.begin_span(span_name, span_type.as_str());
        let result = operation();
        span.end();
        result
    }
}

// Usage in request handlers
#[axum::debug_handler]
async fn create_user_handler(
    State(app_state): State<AppState>,
    Json(user_data): Json<CreateUserRequest>,
) -> Result<Json<User>, ScimError> {
    app_state.apm.trace_operation(
        "create_user",
        TransactionType::Request,
        async {
            let user = app_state.scim_server
                .create_user(&user_data.tenant_id, user_data)
                .await?;
            Ok(Json(user))
        }
    ).await
}
```

### Database Performance Monitoring

```rust
use sqlx::{query, Pool, Postgres};
use tracing::{instrument, Span};

pub struct DatabaseMonitor {
    pool: Pool<Postgres>,
    metrics: DatabaseMetrics,
}

impl DatabaseMonitor {
    #[instrument(skip(self, query_str, params))]
    pub async fn execute_monitored_query<T>(
        &self,
        query_str: &str,
        params: &[&dyn sqlx::Type<Postgres>],
    ) -> Result<T, sqlx::Error>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        let span = Span::current();
        let start_time = Instant::now();
        
        // Record query details (sanitized)
        span.record("db.statement", &sanitize_query(query_str));
        span.record("db.operation", &extract_operation(query_str));
        
        // Execute query
        let result = sqlx::query_as::<_, T>(query_str)
            .fetch_all(&self.pool)
            .await;
        
        let duration = start_time.elapsed();
        
        // Record metrics
        match &result {
            Ok(rows) => {
                span.record("db.rows_affected", &rows.len());
                span.record("db.duration_ms", &duration.as_millis());
                self.metrics.query_duration.observe(duration.as_secs_f64());
                self.metrics.query_success.inc();
            }
            Err(e) => {
                span.record("db.error", &e.to_string());
                span.record("db.duration_ms", &duration.as_millis());
                self.metrics.query_errors.inc();
                warn!("Database query failed: {}", e);
            }
        }
        
        // Alert on slow queries
        if duration > Duration::from_millis(1000) {
            warn!(
                duration_ms = duration.as_millis(),
                query = sanitize_query(query_str),
                "Slow database query detected"
            );
        }
        
        result
    }
}

fn sanitize_query(query: &str) -> String {
    // Remove sensitive data from query for logging
    query
        .replace(|c: char| c.is_numeric(), "?")
        .replace("'.*?'", "'?'")
}
```

## Observability Best Practices

### Correlation and Context

```rust
use uuid::Uuid;
use tracing_subscriber::layer::SubscriberExt;

// Request correlation
#[derive(Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
}

impl RequestContext {
    pub fn new(headers: &HeaderMap, remote_addr: SocketAddr) -> Self {
        let request_id = headers
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);
        
        Self {
            request_id,
            tenant_id: headers.get("x-tenant-id")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            user_id: None, // Set after authentication
            session_id: None, // Set after authentication
            ip_address: remote_addr.ip(),
            user_agent: headers.get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
        }
    }
    
    pub fn create_span(&self, operation: &str) -> tracing::Span {
        tracing::info_span!(
            "scim_operation",
            operation = operation,
            request_id = %self.request_id,
            tenant_id = %self.tenant_id.as_deref().unwrap_or("unknown"),
            user_id = %self.user_id.as_deref().unwrap_or("anonymous"),
            ip_address = %self.ip_address,
        )
    }
}

// Middleware for request correlation
pub async fn correlation_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request<Body>,
    next: Next<Body>,
) -> Response<Body> {
    let context = RequestContext::new(req.headers(), addr);
    
    // Add context to request extensions
    req.extensions_mut().insert(context.clone());
    
    // Create request span
    let span = context.create_span("http_request");
    let _guard = span.enter();
    
    // Add request ID to response headers
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&context.request_id.to_string()).unwrap(),
    );
    
    response
}
```

### Error Tracking Integration

```rust
use sentry::{ClientOptions, integrations::tracing::EventFilter};

pub fn setup_error_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = sentry::init((
        std::env::var("SENTRY_DSN")?,
        ClientOptions {
            release: Some(env!("CARGO_PKG_VERSION").into()),
            environment: Some(std::env::var("ENVIRONMENT").unwrap_or("unknown".into()).into()),
            sample_rate: 1.0,
            traces_sample_rate: 0.1,
            ..Default::default()
        },
    ));
    
    // Configure tracing integration
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("scim_server=info"))
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer().event_filter(|md| {
            match md.level() {
                &tracing::Level::ERROR => EventFilter::Event,
                &tracing::Level::WARN => EventFilter::Breadcrumb,
                _ => EventFilter::Ignore,
            }
        }))
        .init();
    
    Ok(())
}

// Custom error reporting
pub async fn report_error(
    error: &dyn std::error::Error,
    context: &RequestContext,
    additional_data: Option<serde_json::Value>,
) {
    sentry::with_scope(|scope| {
        scope.set_tag("request_id", &context.request_id.to_string());
        scope.set_tag("tenant_id", context.tenant_id.as_deref().unwrap_or("unknown"));
        scope.set_user(Some(sentry::User {
            id: context.user_id.clone(),
            ip_address: Some(context.ip_address.to_string()),
            ..Default::default()
        }));
        
        if let Some(data) = additional_data {
            scope.set_context("additional_data", sentry::protocol::Context::Other(data.into()));
        }
        
        sentry::capture_error(error);
    });
}
```

This comprehensive monitoring and observability guide provides the foundation for operating SCIM servers with full visibility into performance, errors, and business metrics. Regular review and tuning of monitoring configurations ensure optimal system health and rapid incident response.