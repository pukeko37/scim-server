# Installation Guide

Complete installation and setup guide for the SCIM Server crate, covering everything from basic development setup to production deployment.

## Table of Contents

- [System Requirements](#system-requirements)
- [Development Installation](#development-installation)
- [Production Installation](#production-installation)
- [Database Setup](#database-setup)
- [Configuration](#configuration)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)

## System Requirements

### Minimum Requirements
- **Rust**: 1.70.0 or later
- **Memory**: 512MB RAM (development), 2GB+ RAM (production)
- **Disk**: 100MB for development, 1GB+ for production
- **OS**: Linux, macOS, or Windows

### Recommended Requirements
- **Rust**: Latest stable version
- **Memory**: 2GB RAM (development), 8GB+ RAM (production)
- **CPU**: 2+ cores
- **Disk**: SSD storage for better performance

### Optional Dependencies
- **Docker**: For containerized deployment
- **PostgreSQL**: For production database storage
- **Redis**: For caching and session management
- **Kubernetes**: For orchestrated deployment

## Development Installation

### 1. Install Rust

#### Via Rustup (Recommended)
```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Via Package Manager

**macOS (Homebrew)**:
```bash
brew install rust
```

**Ubuntu/Debian**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows**:
Download and run the installer from [rustup.rs](https://rustup.rs/)

### 2. Install Development Tools

```bash
# Essential tools
rustup component add clippy rustfmt

# Development utilities
cargo install cargo-watch
cargo install cargo-expand
cargo install cargo-audit
cargo install cargo-criterion
```

### 3. Create New Project

```bash
# Create a new Rust project
cargo new my-scim-server
cd my-scim-server
```

### 4. Add SCIM Server Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
env_logger = "0.10"

# Optional: Web framework
axum = "0.7"
# or
warp = "0.3"

# Optional: Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }
# or
mongodb = "2.7"

# Optional: Error handling
thiserror = "1.0"
anyhow = "1.0"
```

### 5. Basic Setup Test

Create `src/main.rs`:

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, create_user_resource_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Create provider and server
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    
    // Register user resource handler
    server.register_resource_handler("User", create_user_resource_handler());
    
    println!("✅ SCIM Server setup successful!");
    Ok(())
}
```

### 6. Verify Installation

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Run your application
cargo run
```

Expected output:
```
✅ SCIM Server setup successful!
```

## Production Installation

### 1. Optimized Build

```bash
# Build optimized binary
cargo build --release

# Binary location
ls -la target/release/
```

### 2. System Service Setup

#### Systemd Service (Linux)

Create `/etc/systemd/system/scim-server.service`:

```ini
[Unit]
Description=SCIM Server
After=network.target

[Service]
Type=simple
User=scim
Group=scim
WorkingDirectory=/opt/scim-server
ExecStart=/opt/scim-server/bin/scim-server
Restart=always
RestartSec=5

# Environment variables
Environment=RUST_LOG=info
Environment=DATABASE_URL=postgresql://scim:password@localhost/scim
Environment=BIND_ADDRESS=0.0.0.0
Environment=PORT=3000

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/scim-server/data

[Install]
WantedBy=multi-user.target
```

#### Enable and Start Service

```bash
# Create user
sudo useradd --system --no-create-home scim

# Create directories
sudo mkdir -p /opt/scim-server/{bin,data}
sudo chown -R scim:scim /opt/scim-server

# Install binary
sudo cp target/release/scim-server /opt/scim-server/bin/

# Enable and start service
sudo systemctl enable scim-server
sudo systemctl start scim-server

# Check status
sudo systemctl status scim-server
```

### 3. Reverse Proxy Setup

#### Nginx Configuration

Create `/etc/nginx/sites-available/scim-server`:

```nginx
server {
    listen 80;
    server_name scim.example.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name scim.example.com;
    
    # SSL configuration
    ssl_certificate /etc/ssl/certs/scim.example.com.crt;
    ssl_certificate_key /etc/ssl/private/scim.example.com.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    
    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains";
    
    # Proxy to SCIM server
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # SCIM-specific headers
        proxy_set_header Content-Type application/scim+json;
        
        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }
    
    # Health check endpoint
    location /health {
        proxy_pass http://127.0.0.1:3000/health;
        access_log off;
    }
}
```

Enable the site:
```bash
sudo ln -s /etc/nginx/sites-available/scim-server /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

## Database Setup

### PostgreSQL Setup

#### 1. Install PostgreSQL

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
```

**CentOS/RHEL**:
```bash
sudo yum install postgresql-server postgresql-contrib
sudo postgresql-setup initdb
```

**macOS**:
```bash
brew install postgresql
brew services start postgresql
```

#### 2. Create Database and User

```sql
-- Connect as postgres user
sudo -u postgres psql

-- Create database and user
CREATE DATABASE scim;
CREATE USER scim WITH ENCRYPTED PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE scim TO scim;

-- Grant schema permissions
\c scim
GRANT ALL ON SCHEMA public TO scim;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO scim;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO scim;

-- Exit
\q
```

#### 3. Database Schema

Create the SCIM tables:

```sql
-- Connect to SCIM database
psql -U scim -d scim -h localhost

-- Create main resources table
CREATE TABLE scim_resources (
    id UUID PRIMARY KEY,
    resource_type VARCHAR(50) NOT NULL,
    tenant_id VARCHAR(100) DEFAULT 'default',
    data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    version INTEGER DEFAULT 1
);

-- Create indexes for performance
CREATE INDEX idx_scim_resources_tenant_type ON scim_resources(tenant_id, resource_type);
CREATE INDEX idx_scim_resources_username ON scim_resources USING GIN ((data->>'userName'));
CREATE INDEX idx_scim_resources_email ON scim_resources USING GIN ((data->'emails'));
CREATE INDEX idx_scim_resources_created ON scim_resources(created_at);

-- Create table for tenant configuration
CREATE TABLE tenant_configs (
    tenant_id VARCHAR(100) PRIMARY KEY,
    client_id VARCHAR(100) NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create API keys table (optional)
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(100) REFERENCES tenant_configs(tenant_id),
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    permissions JSONB,
    active BOOLEAN DEFAULT true,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_tenant ON api_keys(tenant_id, active);
```

### MongoDB Setup

#### 1. Install MongoDB

**Ubuntu/Debian**:
```bash
wget -qO - https://www.mongodb.org/static/pgp/server-6.0.asc | sudo apt-key add -
echo "deb [ arch=amd64,arm64 ] https://repo.mongodb.org/apt/ubuntu focal/mongodb-org/6.0 multiverse" | sudo tee /etc/apt/sources.list.d/mongodb-org-6.0.list
sudo apt update
sudo apt install mongodb-org
```

**macOS**:
```bash
brew tap mongodb/brew
brew install mongodb-community
brew services start mongodb-community
```

#### 2. Create Database and Collections

```javascript
// Connect to MongoDB
mongosh

// Switch to SCIM database
use scim

// Create collections with validation
db.createCollection("resources", {
   validator: {
      $jsonSchema: {
         bsonType: "object",
         required: ["id", "resourceType", "tenantId", "data"],
         properties: {
            id: { bsonType: "string" },
            resourceType: { bsonType: "string" },
            tenantId: { bsonType: "string" },
            data: { bsonType: "object" }
         }
      }
   }
})

// Create indexes
db.resources.createIndex({ "tenantId": 1, "resourceType": 1 })
db.resources.createIndex({ "data.userName": 1 })
db.resources.createIndex({ "data.emails.value": 1 })
db.resources.createIndex({ "createdAt": 1 })

// Create tenant configurations collection
db.createCollection("tenantConfigs", {
   validator: {
      $jsonSchema: {
         bsonType: "object",
         required: ["tenantId", "clientId", "config"],
         properties: {
            tenantId: { bsonType: "string" },
            clientId: { bsonType: "string" },
            config: { bsonType: "object" }
         }
      }
   }
})

db.tenantConfigs.createIndex({ "tenantId": 1 }, { unique: true })
```

### Redis Setup (Optional - for caching)

#### 1. Install Redis

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install redis-server
```

**macOS**:
```bash
brew install redis
brew services start redis
```

#### 2. Configure Redis

Edit `/etc/redis/redis.conf`:
```
# Bind to localhost for security
bind 127.0.0.1

# Set password
requirepass your_secure_redis_password

# Persistence settings
save 900 1
save 300 10
save 60 10000

# Memory settings
maxmemory 256mb
maxmemory-policy allkeys-lru
```

Restart Redis:
```bash
sudo systemctl restart redis
```

## Configuration

### Environment Variables

Create a `.env` file for development:

```bash
# Database configuration
DATABASE_URL=postgresql://scim:password@localhost/scim
# or for MongoDB
# MONGODB_URL=mongodb://localhost:27017/scim

# Redis configuration (optional)
REDIS_URL=redis://:password@localhost:6379

# Server configuration
BIND_ADDRESS=127.0.0.1
PORT=3000
WORKERS=4

# Logging configuration
RUST_LOG=info
LOG_FORMAT=json
LOG_FILE=/var/log/scim-server/app.log

# Security configuration
JWT_SECRET=your-256-bit-secret-key-here
SESSION_TIMEOUT=3600
MAX_REQUEST_SIZE=1048576

# Multi-tenancy configuration
ENABLE_MULTI_TENANCY=true
DEFAULT_TENANT=default

# Performance configuration
MAX_CONNECTIONS=100
CONNECTION_TIMEOUT=30
QUERY_TIMEOUT=10

# Feature flags
ENABLE_SCHEMA_VALIDATION=true
ENABLE_AUDIT_LOGGING=true
ENABLE_METRICS=true
```

### Configuration File

Create `config/production.toml`:

```toml
[server]
bind_address = "0.0.0.0"
port = 3000
workers = 8
max_request_size = "1MB"

[database]
url = "postgresql://scim:password@db:5432/scim"
max_connections = 100
connection_timeout = 30
query_timeout = 10
migration_timeout = 300

[redis]
url = "redis://redis:6379"
pool_size = 10
timeout = 5

[security]
jwt_secret_file = "/run/secrets/jwt_secret"
session_timeout = 3600
enable_cors = false
allowed_origins = ["https://app.example.com"]

[multi_tenancy]
enabled = true
default_tenant = "default"
resolver_type = "header"  # or "certificate", "jwt", "static"

[logging]
level = "info"
format = "json"
file = "/var/log/scim-server/app.log"
max_file_size = "100MB"
max_files = 10

[monitoring]
enable_metrics = true
metrics_port = 9090
health_check_path = "/health"
ready_check_path = "/ready"

[features]
schema_validation = true
audit_logging = true
performance_monitoring = true
rate_limiting = true

[performance]
request_timeout = 30
keepalive_timeout = 75
max_concurrent_requests = 1000
```

### Configuration Loading

In your `src/main.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub multi_tenancy: MultiTenancyConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub workers: usize,
    pub max_request_size: String,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let env = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        
        let config = config::Config::builder()
            // Load defaults
            .add_source(config::File::with_name("config/default"))
            // Load environment-specific config
            .add_source(config::File::with_name(&format!("config/{}", env)).required(false))
            // Override with environment variables
            .add_source(config::Environment::with_prefix("SCIM").separator("_"))
            .build()?;
        
        config.try_deserialize()
    }
}
```

## Verification

### 1. Health Check Endpoint

Add health check to your server:

```rust
use axum::{routing::get, Json, Router};
use serde_json::json;

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn ready_check(
    State(server): State<SharedServer>
) -> Result<Json<Value>, StatusCode> {
    // Check database connectivity
    let context = RequestContext::with_generated_id();
    match server.provider.list_resources("User", None, &context).await {
        Ok(_) => Ok(Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

let app = Router::new()
    .route("/health", get(health_check))
    .route("/ready", get(ready_check))
    // ... other routes
    .with_state(server);
```

### 2. Test SCIM Operations

```bash
# Test health endpoint
curl http://localhost:3000/health

# Test user creation
curl -X POST http://localhost:3000/Users \
  -H "Content-Type: application/scim+json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "test.user@example.com",
    "name": {
      "givenName": "Test",
      "familyName": "User"
    },
    "emails": [{
      "value": "test.user@example.com",
      "type": "work",
      "primary": true
    }],
    "active": true
  }'

# Test user listing
curl http://localhost:3000/Users
```

### 3. Verify Database Connectivity

```bash
# PostgreSQL
psql -U scim -d scim -h localhost -c "SELECT COUNT(*) FROM scim_resources;"

# MongoDB
mongosh --eval "db.resources.countDocuments()"

# Redis
redis-cli ping
```

### 4. Check Logging

```bash
# Check application logs
tail -f /var/log/scim-server/app.log

# Check system logs
sudo journalctl -u scim-server -f
```

## Troubleshooting

### Common Issues

#### "Cannot connect to database"

**Symptoms**:
- Server fails to start
- "Connection refused" errors
- Timeout errors

**Solutions**:
```bash
# Check database is running
sudo systemctl status postgresql
# or
sudo systemctl status mongod

# Test connectivity
pg_isready -h localhost -p 5432
# or
mongosh --eval "db.adminCommand('ping')"

# Check firewall
sudo ufw status
sudo netstat -tlnp | grep :5432
```

#### "Permission denied" errors

**Symptoms**:
- Cannot read configuration files
- Cannot write to log directory
- Cannot bind to port

**Solutions**:
```bash
# Check file permissions
ls -la /opt/scim-server/
sudo chown -R scim:scim /opt/scim-server/

# Check port permissions (for ports < 1024)
sudo setcap 'cap_net_bind_service=+ep' /opt/scim-server/bin/scim-server

# Check SELinux (if applicable)
sudo setsebool -P httpd_can_network_connect 1
```

#### "Validation errors" on startup

**Symptoms**:
- Server starts but rejects all requests
- Schema validation failures
- Missing schema files

**Solutions**:
```bash
# Check schema files exist
ls -la schemas/
cat schemas/User.json

# Verify schema format
jq '.' schemas/User.json

# Check schema loading
RUST_LOG=debug cargo run 2>&1 | grep -i schema
```

#### Performance issues

**Symptoms**:
- Slow response times
- High memory usage
- Database connection errors

**Solutions**:
```bash
# Monitor performance
top -p $(pgrep scim-server)
sudo netstat -an | grep :3000

# Check database performance
# PostgreSQL
sudo -u postgres psql -c "SELECT * FROM pg_stat_activity WHERE state = 'active';"

# MongoDB
mongosh --eval "db.currentOp()"

# Optimize configuration
# Increase connection pool size
# Enable connection pooling
# Add database indexes
```

### Docker Installation

#### 1. Using Pre-built Image

```bash
# Pull the official image (when available)
docker pull scim-server:latest

# Or build from source
git clone <repository-url>
cd scim-server
docker build -t scim-server .
```

#### 2. Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  scim-server:
    image: scim-server:latest
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://scim:password@postgres:5432/scim
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      - postgres
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=scim
      - POSTGRES_USER=scim
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U scim"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass password
    volumes:
      - redis_data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/ssl/certs
    depends_on:
      - scim-server
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
```

#### 3. Run with Docker Compose

```bash
# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f scim-server

# Stop services
docker-compose down
```

### Kubernetes Installation

#### 1. Create Kubernetes Manifests

**Namespace** (`k8s/namespace.yaml`):
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: scim-server
```

**ConfigMap** (`k8s/configmap.yaml`):
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: scim-config
  namespace: scim-server
data:
  config.toml: |
    [server]
    bind_address = "0.0.0.0"
    port = 3000
    
    [database]
    url = "postgresql://scim:password@postgres:5432/scim"
    
    [logging]
    level = "info"
    format = "json"
```

**Secret** (`k8s/secret.yaml`):
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: scim-secrets
  namespace: scim-server
type: Opaque
data:
  jwt-secret: <base64-encoded-secret>
  database-password: <base64-encoded-password>
```

**Deployment** (`k8s/deployment.yaml`):
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
        image: scim-server:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: database-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: jwt-secret
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
        volumeMounts:
        - name: config
          mountPath: /app/config
      volumes:
      - name: config
        configMap:
          name: scim-config
```

**Service** (`k8s/service.yaml`):
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
  - protocol: TCP
    port: 80
    targetPort: 3000
  type: ClusterIP
```

#### 2. Deploy to Kubernetes

```bash
# Create namespace
kubectl apply -f k8s/namespace.yaml

# Create secrets
kubectl apply -f k8s/secret.yaml

# Create config
kubectl apply -f k8s/configmap.yaml

# Deploy application
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml

# Check status
kubectl get pods -n scim-server
kubectl logs -f deployment/scim-server -n scim-server
```

## Security Configuration

### TLS/SSL Setup

#### Generate Certificates

```bash
# Self-signed certificate for development
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Let's Encrypt for production
sudo apt install certbot
sudo certbot certonly --nginx -d scim.example.com
```

#### Configure TLS in Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name scim.example.com;
    
    ssl_certificate /etc/letsencrypt/live/scim.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/scim.example.com/privkey.pem;
    
    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers off;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    
    # HSTS
    add_header Strict-Transport-Security "max-age=63072000" always;
    
    location / {
        proxy_pass http://127.0.0.1:3000;
        # ... other proxy settings
    }
}
```

### Firewall Configuration

```bash
# Ubuntu/Debian with UFW
sudo ufw enable
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw deny 3000/tcp  # Block direct access to SCIM server

# CentOS/RHEL with firewalld
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

## Performance Tuning

### Database Optimization

#### PostgreSQL Tuning

```sql
-- Increase connection limits
ALTER SYSTEM SET max_connections = 200;

-- Tune memory settings
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET work_mem = '4MB';

-- Optimize for SCIM workload
ALTER SYSTEM SET random_page_cost = 1.1;  # For SSD
ALTER SYSTEM SET checkpoint_segments = 32;

-- Reload configuration
SELECT pg_reload_conf();
```

#### Add Performance Indexes

```sql
-- Composite indexes for common queries
CREATE INDEX CONCURRENTLY idx_resources_tenant_type_username 
    ON scim_resources(tenant_id, resource_type, (data->>'userName'));

CREATE INDEX CONCURRENTLY idx_resources_tenant_type_active 
    ON scim_resources(tenant_id, resource_type, (data->>'active'));

-- Partial indexes for better performance
CREATE INDEX CONCURRENTLY idx_resources_active_users 
    ON scim_resources(tenant_id, (data->>'userName')) 
    WHERE resource_type = 'User' AND (data->>'active')::boolean = true;
```

### Application Tuning

#### Cargo Configuration

Create `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"

[profile.release-with-debug]
inherits = "release"
debug = true
```

#### Runtime Configuration

```rust
// Optimize Tokio runtime
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .thread_name("scim-worker")
        .thread_stack_size(3 * 1024 * 1024)  // 3MB stack
        .enable_all()
        .build()
        .unwrap();
    
    rt.block_on(async {
        run_server().await
    })
}
```

## Monitoring Setup

### Logging Configuration

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn setup_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "scim_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::