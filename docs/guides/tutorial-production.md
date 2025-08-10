# Tutorial: Production Deployment

This tutorial guides you through deploying a SCIM server to production, covering security, monitoring, scaling, and operational best practices. By the end, you'll have a production-ready SCIM deployment.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Step 1: Environment Setup](#step-1-environment-setup)
- [Step 2: Security Configuration](#step-2-security-configuration)
- [Step 3: Database Setup](#step-3-database-setup)
- [Step 4: Application Configuration](#step-4-application-configuration)
- [Step 5: Monitoring and Logging](#step-5-monitoring-and-logging)
- [Step 6: Load Balancing](#step-6-load-balancing)
- [Step 7: SSL/TLS Configuration](#step-7-ssltls-configuration)
- [Step 8: Deployment](#step-8-deployment)
- [Step 9: Health Checks](#step-9-health-checks)
- [Step 10: Backup and Recovery](#step-10-backup-and-recovery)
- [Scaling Considerations](#scaling-considerations)
- [Troubleshooting](#troubleshooting)
- [Maintenance](#maintenance)

## Overview

Production deployment of a SCIM server requires careful consideration of:

- **Security**: Authentication, authorization, and data protection
- **Reliability**: High availability and fault tolerance
- **Performance**: Efficient resource utilization and response times
- **Monitoring**: Observability and alerting
- **Compliance**: Data governance and audit requirements

This tutorial uses Docker containers with Kubernetes for orchestration, PostgreSQL for persistence, and industry-standard monitoring tools.

## Prerequisites

- Docker and Kubernetes cluster access
- PostgreSQL database
- SSL certificates for HTTPS
- Monitoring infrastructure (Prometheus, Grafana)
- Basic understanding of Kubernetes, Docker, and database administration

## Step 1: Environment Setup

### Directory Structure

Create a production deployment structure:

```
scim-production/
├── k8s/
│   ├── namespace.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   └── hpa.yaml
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
├── config/
│   ├── server.toml
│   └── schemas/
├── scripts/
│   ├── init-db.sql
│   ├── deploy.sh
│   └── backup.sh
└── monitoring/
    ├── prometheus.yml
    └── grafana-dashboard.json
```

### Dockerfile

Create an optimized production Dockerfile:

```dockerfile
# docker/Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY schemas ./schemas

# Build optimized release
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false scim

# Copy binary and assets
COPY --from=builder /app/target/release/scim-server /usr/local/bin/
COPY --from=builder /app/schemas /app/schemas
COPY config/server.toml /app/config/

# Set ownership
RUN chown -R scim:scim /app

USER scim
WORKDIR /app

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

CMD ["scim-server"]
```

## Step 2: Security Configuration

### Authentication and Authorization

Configure OAuth 2.0 / OIDC integration:

```rust
// src/auth/oauth.rs
use scim_server::error::ScimError;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub scope: String,
    pub tenant_id: Option<String>,
}

pub struct AuthenticationMiddleware {
    jwks_uri: String,
    valid_audiences: HashSet<String>,
    valid_issuers: HashSet<String>,
    decoding_key: DecodingKey,
}

impl AuthenticationMiddleware {
    pub async fn new(
        jwks_uri: String,
        valid_audiences: HashSet<String>,
        valid_issuers: HashSet<String>,
    ) -> Result<Self, ScimError> {
        let decoding_key = Self::fetch_jwks(&jwks_uri).await?;
        
        Ok(Self {
            jwks_uri,
            valid_audiences,
            valid_issuers,
            decoding_key,
        })
    }
    
    pub fn validate_token(&self, token: &str) -> Result<Claims, ScimError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&self.valid_audiences);
        validation.set_issuer(&self.valid_issuers);
        
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| ScimError::unauthorized(&format!("Invalid token: {}", e)))?;
        
        Ok(token_data.claims)
    }
    
    async fn fetch_jwks(jwks_uri: &str) -> Result<DecodingKey, ScimError> {
        // Implementation to fetch and parse JWKS
        // This would typically cache keys and refresh periodically
        todo!("Implement JWKS fetching")
    }
}

pub struct AuthorizationMiddleware {
    rbac_provider: Box<dyn RBACProvider>,
}

#[async_trait::async_trait]
pub trait RBACProvider: Send + Sync {
    async fn check_permission(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        tenant_id: Option<&str>,
    ) -> Result<bool, ScimError>;
}
```

### Configuration File

Create a secure production configuration:

```toml
# config/server.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "${DATABASE_URL}"
max_connections = 20
min_connections = 5
connection_timeout = 30
idle_timeout = 600

[auth]
enabled = true
provider = "oauth2"
jwks_uri = "${JWKS_URI}"
valid_audiences = ["scim-api"]
valid_issuers = ["${OAUTH_ISSUER}"]

[security]
rate_limit_enabled = true
rate_limit_requests = 1000
rate_limit_window = 3600
cors_allowed_origins = ["${ALLOWED_ORIGINS}"]
max_request_size = "1MB"

[logging]
level = "info"
format = "json"
access_log = true

[monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_enabled = true

[multi_tenant]
enabled = true
default_tenant = "default"

[cache]
enabled = true
provider = "redis"
url = "${REDIS_URL}"
ttl = 3600
```

## Step 3: Database Setup

### Database Schema

Create production database schema:

```sql
-- scripts/init-db.sql
CREATE DATABASE scim_production;

\c scim_production;

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- Create schemas
CREATE SCHEMA IF NOT EXISTS scim;
CREATE SCHEMA IF NOT EXISTS audit;

-- Users table
CREATE TABLE scim.users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255) NOT NULL,
    external_id VARCHAR(255),
    username VARCHAR(255) NOT NULL,
    data JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(tenant_id, username),
    UNIQUE(tenant_id, external_id)
);

-- Groups table
CREATE TABLE scim.groups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255) NOT NULL,
    external_id VARCHAR(255),
    display_name VARCHAR(255) NOT NULL,
    data JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(tenant_id, display_name),
    UNIQUE(tenant_id, external_id)
);

-- Group memberships
CREATE TABLE scim.group_memberships (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255) NOT NULL,
    group_id UUID NOT NULL REFERENCES scim.groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES scim.users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(tenant_id, group_id, user_id)
);

-- Audit log
CREATE TABLE audit.events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255),
    user_id VARCHAR(255),
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255),
    action VARCHAR(50) NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indices for performance
CREATE INDEX idx_users_tenant_username ON scim.users(tenant_id, username);
CREATE INDEX idx_users_external_id ON scim.users(tenant_id, external_id);
CREATE INDEX idx_groups_tenant_name ON scim.groups(tenant_id, display_name);
CREATE INDEX idx_memberships_group ON scim.group_memberships(group_id);
CREATE INDEX idx_memberships_user ON scim.group_memberships(user_id);
CREATE INDEX idx_audit_tenant_timestamp ON audit.events(tenant_id, timestamp);
CREATE INDEX idx_audit_resource ON audit.events(resource_type, resource_id);

-- GIN indices for JSONB queries
CREATE INDEX idx_users_data_gin ON scim.users USING GIN(data);
CREATE INDEX idx_groups_data_gin ON scim.groups USING GIN(data);

-- Function to update timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for automatic timestamp updates
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON scim.users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_groups_updated_at BEFORE UPDATE ON scim.groups
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Row Level Security
ALTER TABLE scim.users ENABLE ROW LEVEL SECURITY;
ALTER TABLE scim.groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE scim.group_memberships ENABLE ROW LEVEL SECURITY;

-- Create roles
CREATE ROLE scim_app;
GRANT USAGE ON SCHEMA scim TO scim_app;
GRANT USAGE ON SCHEMA audit TO scim_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA scim TO scim_app;
GRANT INSERT ON ALL TABLES IN SCHEMA audit TO scim_app;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA scim TO scim_app;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA audit TO scim_app;
```

### Database Connection Pool

Implement production-ready database provider:

```rust
// src/providers/postgres_provider.rs
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceId};
use scim_server::error::ScimError;
use sqlx::{PgPool, Row, Postgres, Transaction};
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct PostgresProvider {
    pool: PgPool,
    tenant_id: String,
}

impl PostgresProvider {
    pub async fn new(database_url: &str, tenant_id: String) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        
        // Test connection
        sqlx::query("SELECT 1").execute(&pool).await?;
        
        Ok(Self { pool, tenant_id })
    }
    
    async fn with_transaction<F, R>(&self, f: F) -> Result<R, ScimError>
    where
        F: for<'c> FnOnce(&mut Transaction<'c, Postgres>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, ScimError>> + Send + 'c>>,
    {
        let mut tx = self.pool.begin().await
            .map_err(|e| ScimError::internal_server_error(&format!("Failed to start transaction: {}", e)))?;
        
        let result = f(&mut tx).await;
        
        match result {
            Ok(value) => {
                tx.commit().await
                    .map_err(|e| ScimError::internal_server_error(&format!("Failed to commit transaction: {}", e)))?;
                Ok(value)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }
}

#[async_trait]
impl ResourceProvider for PostgresProvider {
    async fn create(&self, resource: Resource) -> Result<Resource, ScimError> {
        self.with_transaction(|tx| Box::pin(async move {
            let id = Uuid::new_v4();
            let data = serde_json::to_value(&resource)?;
            
            sqlx::query(
                "INSERT INTO scim.users (id, tenant_id, username, external_id, data) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(&id)
            .bind(&self.tenant_id)
            .bind(resource.attributes.get("userName").and_then(|v| v.as_str()))
            .bind(resource.attributes.get("externalId").and_then(|v| v.as_str()))
            .bind(&data)
            .execute(tx)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    ScimError::conflict("Resource already exists")
                } else {
                    ScimError::internal_server_error(&format!("Database error: {}", e))
                }
            })?;
            
            let mut created = resource;
            created.id = Some(ResourceId::from(id.to_string()));
            
            Ok(created)
        })).await
    }
    
    async fn get_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, ScimError> {
        let uuid = Uuid::parse_str(id.as_str())
            .map_err(|_| ScimError::invalid_value("Invalid resource ID format"))?;
        
        let row = sqlx::query(
            "SELECT data FROM scim.users WHERE id = $1 AND tenant_id = $2"
        )
        .bind(&uuid)
        .bind(&self.tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        match row {
            Some(row) => {
                let data: Value = row.get("data");
                let resource: Resource = serde_json::from_value(data)?;
                Ok(Some(resource))
            }
            None => Ok(None),
        }
    }
    
    async fn update(&self, id: &ResourceId, resource: Resource) -> Result<Resource, ScimError> {
        self.with_transaction(|tx| Box::pin(async move {
            let uuid = Uuid::parse_str(id.as_str())
                .map_err(|_| ScimError::invalid_value("Invalid resource ID format"))?;
            
            let data = serde_json::to_value(&resource)?;
            
            let result = sqlx::query(
                "UPDATE scim.users 
                 SET data = $1, username = $2, external_id = $3, version = version + 1
                 WHERE id = $4 AND tenant_id = $5"
            )
            .bind(&data)
            .bind(resource.attributes.get("userName").and_then(|v| v.as_str()))
            .bind(resource.attributes.get("externalId").and_then(|v| v.as_str()))
            .bind(&uuid)
            .bind(&self.tenant_id)
            .execute(tx)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
            
            if result.rows_affected() == 0 {
                return Err(ScimError::not_found(&format!("Resource {} not found", id)));
            }
            
            let mut updated = resource;
            updated.id = Some(id.clone());
            
            Ok(updated)
        })).await
    }
    
    async fn delete(&self, id: &ResourceId) -> Result<(), ScimError> {
        let uuid = Uuid::parse_str(id.as_str())
            .map_err(|_| ScimError::invalid_value("Invalid resource ID format"))?;
        
        let result = sqlx::query(
            "DELETE FROM scim.users WHERE id = $1 AND tenant_id = $2"
        )
        .bind(&uuid)
        .bind(&self.tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        if result.rows_affected() == 0 {
            Err(ScimError::not_found(&format!("Resource {} not found", id)))
        } else {
            Ok(())
        }
    }
    
    async fn list(&self, filter: Option<&str>, start_index: usize, count: usize) 
        -> Result<(Vec<Resource>, usize), ScimError> {
        
        let offset = start_index.saturating_sub(1);
        
        // Build query with filters
        let (where_clause, params) = self.build_filter_query(filter)?;
        
        let query = format!(
            "SELECT data FROM scim.users WHERE tenant_id = $1 {} ORDER BY created_at LIMIT $2 OFFSET $3",
            where_clause
        );
        
        let mut query_builder = sqlx::query(&query)
            .bind(&self.tenant_id)
            .bind(count as i64)
            .bind(offset as i64);
            
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        let resources: Result<Vec<Resource>, _> = rows
            .into_iter()
            .map(|row| {
                let data: Value = row.get("data");
                serde_json::from_value(data)
            })
            .collect();
        
        let resources = resources?;
        
        // Get total count
        let count_query = format!(
            "SELECT COUNT(*) FROM scim.users WHERE tenant_id = $1 {}",
            where_clause
        );
        
        let total_count: i64 = sqlx::query_scalar(&count_query)
            .bind(&self.tenant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        Ok((resources, total_count as usize))
    }
}

impl PostgresProvider {
    fn build_filter_query(&self, filter: Option<&str>) -> Result<(String, Vec<String>), ScimError> {
        // Simplified filter implementation
        // In production, implement full SCIM filter parsing
        if let Some(filter_str) = filter {
            if filter_str.contains("userName eq ") {
                let username = filter_str.split("userName eq ")
                    .nth(1)
                    .and_then(|s| s.trim_matches('"').split_whitespace().next())
                    .ok_or_else(|| ScimError::invalid_filter("Invalid userName filter"))?;
                
                return Ok(("AND username = $4".to_string(), vec![username.to_string()]));
            }
        }
        
        Ok(("".to_string(), vec![]))
    }
}
```

## Step 4: Application Configuration

### Kubernetes Configuration

Create Kubernetes manifests:

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: scim-system
  labels:
    name: scim-system
---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: scim-config
  namespace: scim-system
data:
  server.toml: |
    [server]
    host = "0.0.0.0"
    port = 8080
    workers = 4
    
    [auth]
    enabled = true
    provider = "oauth2"
    
    [security]
    rate_limit_enabled = true
    rate_limit_requests = 1000
    rate_limit_window = 3600
    max_request_size = "1MB"
    
    [logging]
    level = "info"
    format = "json"
    access_log = true
    
    [monitoring]
    metrics_enabled = true
    metrics_port = 9090
    health_check_enabled = true
---
# k8s/secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: scim-secrets
  namespace: scim-system
type: Opaque
data:
  database-url: <base64-encoded-database-url>
  jwks-uri: <base64-encoded-jwks-uri>
  oauth-issuer: <base64-encoded-oauth-issuer>
  redis-url: <base64-encoded-redis-url>
---
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: scim-server
  namespace: scim-system
  labels:
    app: scim-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: scim-server
  template:
    metadata:
      labels:
        app: scim-server
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: scim-server
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: scim-server
        image: your-registry/scim-server:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: database-url
        - name: JWKS_URI
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: jwks-uri
        - name: OAUTH_ISSUER
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: oauth-issuer
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: redis-url
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
      volumes:
      - name: config
        configMap:
          name: scim-config
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - scim-server
              topologyKey: kubernetes.io/hostname
---
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: scim-server
  namespace: scim-system
  labels:
    app: scim-server
spec:
  type: ClusterIP
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
  selector:
    app: scim-server
---
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: scim-server
  namespace: scim-system
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/force-ssl-redirect: "true"
    nginx.ingress.kubernetes.io/backend-protocol: "HTTP"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
  - hosts:
    - scim.yourdomain.com
    secretName: scim-tls
  rules:
  - host: scim.yourdomain.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: scim-server
            port:
              number: 80
---
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: scim-server
  namespace: scim-system
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: scim-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## Step 5: Monitoring and Logging

### Prometheus Configuration

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "scim_alerts.yml"

scrape_configs:
  - job_name: 'scim-server'
    kubernetes_sd_configs:
    - role: pod
      namespaces:
        names:
        - scim-system
    relabel_configs:
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
      action: keep
      regex: true
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
      action: replace
      target_label: __metrics_path__
      regex: (.+)
    - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
      action: replace
      regex: ([^:]+)(?::\d+)?;(\d+)
      replacement: $1:$2
      target_label: __address__

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093
```

### Alert Rules

```yaml
# monitoring/scim_alerts.yml
groups:
- name: scim-server
  rules:
  - alert: SCIMServerDown
    expr: up{job="scim-server"} == 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "SCIM server is down"
      description: "SCIM server has been down for more than 5 minutes"

  - alert: SCIMHighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.1
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High error rate in SCIM server"
      description: "Error rate is {{ $value | humanizePercentage }}"

  - alert: SCIMHighLatency
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1.0
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High latency in SCIM server"
      description: "95th percentile latency is {{ $value }}s"

  - alert: SCIMDatabaseConnections
    expr: database_connections_active / database_connections_max > 0.8
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High database connection usage"
      description: "Database connection pool is {{ $value | humanizePercentage }} full"
```

### Application Metrics

Add metrics collection to your SCIM server:

```rust
// src/metrics.rs
use prometheus::{
    Counter, Histogram, Gauge, Registry, Opts, HistogramOpts,
    register_counter_with_registry, register_histogram_with_registry,
    register_gauge_with_registry,
};
use std::sync::Arc;

pub struct Metrics {
    pub http_requests_total: Counter,
    pub http_request_duration: Histogram,
    pub database_connections_active: Gauge,
    pub database_connections_max: Gauge,
    pub active_sessions: Gauge,
    pub registry: Registry,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let http_requests_total = register_counter_with_registry!(
            Opts::new("http_requests_total", "Total number of HTTP requests"),
            registry
        )?;
        
        let http_request_duration