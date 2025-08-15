# Production Deployment

This guide covers deploying SCIM Server in production environments, including infrastructure setup, security considerations, monitoring, and operational best practices.

## Overview

Production deployment of SCIM Server requires careful consideration of:

- **High Availability**: Multiple instances with load balancing
- **Security**: TLS, authentication, and network isolation
- **Performance**: Database optimization and caching
- **Monitoring**: Health checks, metrics, and alerting
- **Scalability**: Horizontal and vertical scaling strategies
- **Data Protection**: Backups, encryption, and compliance

## Infrastructure Architecture

### Recommended Production Architecture

```
                    Internet
                       |
                  [Load Balancer]
                   /     |     \
            [SCIM-1] [SCIM-2] [SCIM-3]
                   \     |     /
                  [Database Cluster]
                       |
                   [Redis Cache]
```

### Container Deployment with Docker

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build optimized release binary
RUN cargo build --release

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false scim

WORKDIR /app
COPY --from=builder /app/target/release/scim-server .

# Set ownership and permissions
RUN chown scim:scim /app/scim-server
USER scim

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

CMD ["./scim-server"]
```

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  scim-server:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://scim:${POSTGRES_PASSWORD}@postgres:5432/scim
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
      - JWT_SECRET=${JWT_SECRET}
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=scim
      - POSTGRES_USER=scim
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U scim"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - scim-server

volumes:
  postgres_data:
  redis_data:
```

### Kubernetes Deployment

**namespace.yaml:**
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: scim-server
```

**configmap.yaml:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: scim-config
  namespace: scim-server
data:
  RUST_LOG: "info"
  SERVER_HOST: "0.0.0.0"
  SERVER_PORT: "3000"
```

**secret.yaml:**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: scim-secrets
  namespace: scim-server
type: Opaque
data:
  DATABASE_URL: <base64-encoded-database-url>
  JWT_SECRET: <base64-encoded-jwt-secret>
  REDIS_URL: <base64-encoded-redis-url>
```

**deployment.yaml:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: scim-server
  namespace: scim-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: scim-server
  template:
    metadata:
      labels:
        app: scim-server
    spec:
      containers:
      - name: scim-server
        image: your-registry/scim-server:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: DATABASE_URL
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: JWT_SECRET
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: REDIS_URL
        envFrom:
        - configMapRef:
            name: scim-config
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
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
```

**service.yaml:**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: scim-server-service
  namespace: scim-server
spec:
  selector:
    app: scim-server
  ports:
  - port: 80
    targetPort: 3000
  type: ClusterIP
```

**ingress.yaml:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: scim-server-ingress
  namespace: scim-server
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/use-regex: "true"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
  - hosts:
    - scim.yourdomain.com
    secretName: scim-tls-secret
  rules:
  - host: scim.yourdomain.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: scim-server-service
            port:
              number: 80
```

## Database Configuration

### PostgreSQL Production Setup

**Connection pooling configuration:**
```rust
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn create_db_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)          // Maximum connections in pool
        .min_connections(5)           // Minimum connections to maintain
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(true)    // Test connections before use
        .connect(database_url)
        .await
}
```

**Database migration script:**
```sql
-- init.sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255) NOT NULL,
    username VARCHAR(255) NOT NULL,
    given_name VARCHAR(255),
    family_name VARCHAR(255),
    email VARCHAR(255),
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INTEGER NOT NULL DEFAULT 1,
    data JSONB NOT NULL,
    
    UNIQUE(tenant_id, username),
    UNIQUE(tenant_id, email)
);

-- Groups table
CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id VARCHAR(255) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INTEGER NOT NULL DEFAULT 1,
    data JSONB NOT NULL,
    
    UNIQUE(tenant_id, display_name)
);

-- Group memberships
CREATE TABLE group_memberships (
    group_id UUID REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY(group_id, user_id)
);

-- Indexes for performance
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_username ON users(tenant_id, username);
CREATE INDEX idx_users_email ON users(tenant_id, email);
CREATE INDEX idx_users_active ON users(tenant_id, active);
CREATE INDEX idx_users_updated_at ON users(updated_at);

CREATE INDEX idx_groups_tenant_id ON groups(tenant_id);
CREATE INDEX idx_groups_display_name ON groups(tenant_id, display_name);

CREATE INDEX idx_group_memberships_user_id ON group_memberships(user_id);

-- JSONB indexes for advanced querying
CREATE INDEX idx_users_data_gin ON users USING GIN(data);
CREATE INDEX idx_groups_data_gin ON groups USING GIN(data);
```

### Redis Configuration

**Redis for caching and sessions:**
```yaml
# redis.conf
bind 0.0.0.0
port 6379
timeout 300
tcp-keepalive 60

# Memory management
maxmemory 1gb
maxmemory-policy allkeys-lru

# Persistence
save 900 1
save 300 10
save 60 10000

# Security
requirepass your-redis-password
```

## Security Configuration

### TLS/SSL Setup

**nginx.conf:**
```nginx
upstream scim_backend {
    least_conn;
    server scim-server-1:3000 weight=1 max_fails=3 fail_timeout=30s;
    server scim-server-2:3000 weight=1 max_fails=3 fail_timeout=30s;
    server scim-server-3:3000 weight=1 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    server_name scim.yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name scim.yourdomain.com;

    # SSL configuration
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options nosniff always;
    add_header X-Frame-Options DENY always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req zone=api burst=20 nodelay;

    location / {
        proxy_pass http://scim_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        
        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # Health check endpoint
    location /health {
        access_log off;
        proxy_pass http://scim_backend;
    }
}
```

### Environment Configuration

**Production environment variables:**
```bash
# Server configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=info

# Database
DATABASE_URL=postgresql://scim:${POSTGRES_PASSWORD}@postgres-cluster:5432/scim?sslmode=require
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# Redis
REDIS_URL=redis://:${REDIS_PASSWORD}@redis-cluster:6379
REDIS_MAX_CONNECTIONS=10

# Security
JWT_SECRET=${JWT_SECRET}
ENCRYPTION_KEY=${ENCRYPTION_KEY}
TLS_CERT_PATH=/etc/ssl/certs/scim.crt
TLS_KEY_PATH=/etc/ssl/private/scim.key

# Observability
METRICS_ENABLED=true
TRACING_ENABLED=true
JAEGER_ENDPOINT=http://jaeger:14268/api/traces

# Performance
CACHE_TTL=300
MAX_REQUEST_SIZE=1048576
WORKER_THREADS=4
```

## Monitoring and Observability

### Health Checks

```rust
use axum::{response::Json, http::StatusCode};
use serde_json::json;

#[derive(Clone)]
pub struct HealthChecker {
    db_pool: sqlx::PgPool,
    redis_client: redis::Client,
}

impl HealthChecker {
    pub async fn check_all(&self) -> Result<HealthStatus, HealthError> {
        let mut checks = Vec::new();
        
        // Database health
        let db_health = self.check_database().await;
        checks.push(("database", db_health.is_ok()));
        
        // Redis health
        let redis_health = self.check_redis().await;
        checks.push(("redis", redis_health.is_ok()));
        
        // Memory usage
        let memory_health = self.check_memory().await;
        checks.push(("memory", memory_health.is_ok()));
        
        let all_healthy = checks.iter().all(|(_, healthy)| *healthy);
        
        Ok(HealthStatus {
            status: if all_healthy { "healthy" } else { "unhealthy" },
            checks,
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn check_database(&self) -> Result<(), HealthError> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| HealthError::Database(e.to_string()))?;
        Ok(())
    }
    
    async fn check_redis(&self) -> Result<(), HealthError> {
        let mut conn = self.redis_client
            .get_async_connection()
            .await
            .map_err(|e| HealthError::Redis(e.to_string()))?;
            
        redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| HealthError::Redis(e.to_string()))?;
        Ok(())
    }
    
    async fn check_memory(&self) -> Result<(), HealthError> {
        use sysinfo::{System, SystemExt};
        let mut system = System::new_all();
        system.refresh_memory();
        
        let used_percentage = (system.used_memory() as f64 / system.total_memory() as f64) * 100.0;
        
        if used_percentage > 90.0 {
            return Err(HealthError::Memory(format!("Memory usage too high: {:.1}%", used_percentage)));
        }
        
        Ok(())
    }
}

pub async fn health_endpoint(
    State(health_checker): State<HealthChecker>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match health_checker.check_all().await {
        Ok(status) => {
            let status_code = if status.status == "healthy" {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            
            Ok(Json(json!(status)))
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, Gauge, Registry, Encoder, TextEncoder};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    http_requests_total: Counter,
    http_request_duration: Histogram,
    active_connections: Gauge,
    users_total: Gauge,
    groups_total: Gauge,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Arc::new(Registry::new());
        
        let http_requests_total = Counter::new(
            "http_requests_total",
            "Total number of HTTP requests"
        )?;
        
        let http_request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        )?;
        
        let active_connections = Gauge::new(
            "active_connections",
            "Number of active connections"
        )?;
        
        let users_total = Gauge::new(
            "users_total",
            "Total number of users"
        )?;
        
        let groups_total = Gauge::new(
            "groups_total",
            "Total number of groups"
        )?;
        
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(users_total.clone()))?;
        registry.register(Box::new(groups_total.clone()))?;
        
        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            active_connections,
            users_total,
            groups_total,
        })
    }
    
    pub fn record_request(&self, duration: f64) {
        self.http_requests_total.inc();
        self.http_request_duration.observe(duration);
    }
    
    pub async fn metrics_handler(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
```

### Logging Configuration

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    let file_appender = RollingFileAppender::new(
        Rotation::daily(),
        "/var/log/scim-server",
        "app.log"
    );
    
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "scim_server=info,sqlx=warn".into())
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();
    
    Ok(())
}
```

## Performance Optimization

### Application-Level Optimizations

```rust
// Connection pool configuration
let db_pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .connect(&database_url)
    .await?;

// Redis connection pool
let redis_pool = deadpool_redis::Config::from_url(&redis_url)
    .create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

// Implement caching layer
#[derive(Clone)]
pub struct CachedProvider {
    inner: DatabaseProvider,
    cache: deadpool_redis::Pool,
    cache_ttl: Duration,
}

impl CachedProvider {
    async fn get_user_cached(&self, tenant_id: &str, user_id: &str) -> Result<Option<ScimUser>, ProviderError> {
        let cache_key = format!("user:{}:{}", tenant_id, user_id);
        
        // Try cache first
        if let Ok(mut conn) = self.cache.get().await {
            if let Ok(cached) = redis::cmd("GET").arg(&cache_key).query_async::<_, String>(&mut conn).await {
                if let Ok(user) = serde_json::from_str::<ScimUser>(&cached) {
                    return Ok(Some(user));
                }
            }
        }
        
        // Fallback to database
        let user = self.inner.get_user(tenant_id, user_id).await?;
        
        // Cache the result
        if let (Some(ref user), Ok(mut conn)) = (&user, self.cache.get().await) {
            if let Ok(serialized) = serde_json::to_string(user) {
                let _: Result<(), _> = redis::cmd("SETEX")
                    .arg(&cache_key)
                    .arg(self.cache_ttl.as_secs())
                    .arg(serialized)
                    .query_async(&mut conn)
                    .await;
            }
        }
        
        Ok(user)
    }
}
```

### Database Optimization

```sql
-- Analyze query performance
EXPLAIN ANALYZE SELECT * FROM users WHERE tenant_id = 'tenant-1' AND active = true;

-- Create partial indexes for common queries
CREATE INDEX CONCURRENTLY idx_users_active_by_tenant 
ON users(tenant_id, username) 
WHERE active = true;

-- Optimize JSONB queries
CREATE INDEX CONCURRENTLY idx_users_department 
ON users USING GIN ((data->'department'));

-- Partition large tables by tenant
CREATE TABLE users_partitioned (
    LIKE users INCLUDING ALL
) PARTITION BY HASH (tenant_id);

CREATE TABLE users_part_0 PARTITION OF users_partitioned
FOR VALUES WITH (modulus 4, remainder 0);

CREATE TABLE users_part_1 PARTITION OF users_partitioned
FOR VALUES WITH (modulus 4, remainder 1);

-- Regular maintenance
CREATE OR REPLACE FUNCTION maintain_database()
RETURNS void AS $$
BEGIN
    -- Update statistics
    ANALYZE;
    
    -- Rebuild indexes if needed
    REINDEX INDEX CONCURRENTLY idx_users_tenant_id;
    
    -- Clean up old data
    DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;

-- Schedule maintenance
SELECT cron.schedule('database-maintenance', '0 2 * * 0', 'SELECT maintain_database();');
```

## Backup and Disaster Recovery

### Database Backup Strategy

```bash
#!/bin/bash
# backup.sh

set -e

BACKUP_DIR="/backups"
DB_NAME="scim"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/scim_backup_${TIMESTAMP}.sql"

# Create backup directory
mkdir -p $BACKUP_DIR

# Perform backup
pg_dump \
    --host=$DB_HOST \
    --port=$DB_PORT \
    --username=$DB_USER \
    --dbname=$DB_NAME \
    --format=custom \
    --compress=9 \
    --file=$BACKUP_FILE

# Encrypt backup
gpg --symmetric --cipher-algo AES256 --output $BACKUP_FILE.gpg $BACKUP_FILE
rm $BACKUP_FILE

# Upload to S3
aws s3 cp $BACKUP_FILE.gpg s3://scim-backups/daily/

# Clean up old local backups (keep 7 days)
find $BACKUP_DIR -name "scim_backup_*.sql.gpg" -mtime +7 -delete

# Clean up old S3 backups (keep 30 days)
aws s3 ls s3://scim-backups/daily/ | grep "scim_backup_" | head -n -30 | awk '{print $4}' | xargs -I {} aws s3 rm s3://scim-backups/daily/{}
```

### Automated Restore Testing

```bash
#!/bin/bash
# test-restore.sh

BACKUP_FILE=$1
TEST_DB="scim_restore_test"

# Create test database
createdb $TEST_DB

# Restore backup
pg_restore \
    --host=$DB_HOST \
    --port=$DB_PORT \
    --username=$DB_USER \
    --dbname=$TEST_DB \
    --clean \
    --if-exists \
    $BACKUP_FILE

# Run validation queries
psql -d $TEST_DB -c "SELECT COUNT(*) FROM users;"
psql -d $TEST_DB -c "SELECT COUNT(*) FROM groups;"

# Clean up
dropdb $TEST_DB

echo "Restore test completed successfully"
```

## Deployment Pipeline

### CI/CD with GitHub Actions

**.github/workflows/deploy.yml:**
```yaml
name: Deploy to Production

on:
  push:
    branches: [main]
    tags: ['v*']

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - uses: actions-rs/cargo@v1
      with:
        command: test

  build-and-push:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
    - name: Checkout
      uses: actions/checkout@v4
      
    - name: Log in to Registry
      uses: docker/login-action@v3
      with:
        registry: ${{ env.REGISTRY }}
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
        
    - name: Extract metadata
      id: meta
      uses: docker/metadata-action@v5
      with:
        images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
        
    - name: Build and push
      uses: docker/build-push-action@v5
      with:
        context: .
        push: true
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}

  deploy:
    needs: build-and-push
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
    - name: Deploy to Kubernetes
      uses: azure/k8s-deploy@v1
      with:
        manifests: |
          k8s/deployment.yaml
          k8s/service.yaml
          k8s/ingress.yaml
        images: |
          ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:main
        kubectl-version: 'latest'
```

## Security Best Practices

### Runtime Security

```yaml
# Pod Security Policy
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: scim-server-psp
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
    - ALL
  volumes:
    - 'configMap'
    - 'emptyDir'
    - 'projected'
    - 'secret'
    - 'downwardAPI'
    - 'persistentVolumeClaim'
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'RunAsAny'
  fsGroup:
    rule: 'RunAsAny'
```

### Network Security

```yaml
# Network Policy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: scim-server-netpol
  namespace: scim-server
spec:
  podSelector:
    matchLabels:
      app: scim-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 3000
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: database
    ports:
    - protocol: TCP
      port: 5432
  - to:
    - namespaceSelector:
        matchLabels:
          name: redis
    ports:
    - protocol: TCP
      port: 6379
```

This comprehensive production deployment guide covers all aspects of running SCIM Server at scale with enterprise-grade reliability, security, and observability.