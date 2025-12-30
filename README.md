# Rust Service Benchmark (Axum + Postgres + Prometheus + Grafana)

This repository contains a small **Rust** service built with **Axum** that:

- Exposes a simple HTTP API to read an item from **PostgreSQL**.
- Exposes **Prometheus** metrics at `/metrics`.
- Ships with a complete local observability stack via **Docker Compose**:
  - **PostgreSQL** (data store)
  - **Prometheus** (scrapes `/metrics` and cAdvisor)
  - **Grafana** (pre-provisioned datasource + dashboards)
  - **cAdvisor** (container CPU/memory metrics)

The goal is to run a reproducible local benchmark and visualize **RPS, latency p90**, plus **CPU and memory** of the Rust container.

---

## Architecture

**Request flow**

1. Client calls: `GET /api/item/{id}`
2. Service queries Postgres (`items` table)
3. Service returns JSON response
4. Middleware records:
   - total requests (counter)
   - request duration (histogram)
   - in-flight requests (gauge)
5. Prometheus scrapes `/metrics`
6. Grafana renders dashboards from provisioned JSON

---

## Tech stack

- **Rust** (async runtime: **Tokio**)
- **Axum** (HTTP server & routing)
- **SQLx** (PostgreSQL client)
- **Prometheus** Rust client (`prometheus` crate)
- **tracing / tracing-subscriber** (structured logs)
- **tower-http TraceLayer** (HTTP trace logs)
- **Docker Compose** (local stack)
- **Prometheus + Grafana + cAdvisor** (observability)

Key dependencies (from `Cargo.toml`):

- `axum = "0.8.8"`
- `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }`
- `sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "postgres"] }`
- `prometheus = "0.14.0"`
- `tracing`, `tracing-subscriber`
- `tower-http`

---

## Endpoints

- `GET /health`  
  Quick health check.

- `GET /api/item/{id}`  
  Reads an item from Postgres by `id`.  
  Example: `GET /api/item/1`

- `GET /metrics`  
  Prometheus metrics endpoint in text format.

---

## Metrics exposed

The service exports:

- `http_requests_received_total{method,path,code}` (counter)  
- `http_request_duration_seconds_bucket{method,path,code,le}` (histogram buckets)  
- `http_request_duration_seconds_sum{...}` / `_count{...}`  
- `http_requests_in_progress{path}` (gauge)

> Paths are labeled using `MatchedPath` (route templates like `/api/item/{id}`) to avoid high cardinality.

---

## Repository layout (example)

```
bench-local/
  docker-compose.yml
  .env
  db/
    init.sql
  prometheus/
    prometheus.yml
  grafana/
    provisioning/
      datasources/
        datasource.yml
      dashboards/
        dashboards.yml
    dashboards/
      rust-benchmark-dashboard.json
  rust-service/
    Cargo.toml
    src/
      main.rs
      app.rs
      handlers.rs
      state.rs
      metrics.rs
      error.rs
```

---

## Prerequisites

- Docker Desktop (Windows/macOS/Linux)
- Docker Compose v2 (included with Docker Desktop)
- PowerShell (Windows) if you want to use the provided load generator script

---

## Configuration (.env)

This project uses a `.env` file for local configuration.

Example `.env`:

```env
COMPOSE_PROJECT_NAME=bench-local

POSTGRES_USER=postgres
POSTGRES_PASSWORD=change_me
POSTGRES_DB=appdb
POSTGRES_PORT=5432

RUST_PORT=8080
DB_POOL_MAX_CONNECTIONS=10
DB_POOL_MIN_CONNECTIONS=0
DB_CONNECT_TIMEOUT_SECS=5
DB_ACQUIRE_TIMEOUT_SECS=2
RUST_LOG=info

PROMETHEUS_PORT=9090

GRAFANA_PORT=3000
GF_SECURITY_ADMIN_USER=admin
GF_SECURITY_ADMIN_PASSWORD=change_me

CADVISOR_PORT=8082
```

---

## Database initialization

`db/init.sql` creates and seeds the `items` table:

```sql
CREATE TABLE IF NOT EXISTS items (
  id   INT PRIMARY KEY,
  name TEXT NOT NULL
);

INSERT INTO items (id, name)
VALUES (1, 'Hello from Postgres')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;
```

---

## Run everything with Docker Compose

From the repository root (where `docker-compose.yml` lives):

```bash
docker compose up -d --build
```

Check status:

```bash
docker compose ps
```

Follow logs (service):

```bash
docker compose logs -f rust-service
```

---

## URLs

- Rust service:
  - API: `http://localhost:${RUST_PORT}/api/item/1`
  - Metrics: `http://localhost:${RUST_PORT}/metrics`
  - Health: `http://localhost:${RUST_PORT}/health`

- Prometheus: `http://localhost:${PROMETHEUS_PORT}`
- Grafana: `http://localhost:${GRAFANA_PORT}`
- cAdvisor: `http://localhost:${CADVISOR_PORT}`

Grafana default login (from `.env`):

- user: `${GF_SECURITY_ADMIN_USER}` (default `admin`)
- password: `${GF_SECURITY_ADMIN_PASSWORD}` (default `change_me`)

---

## Confirm Prometheus is scraping metrics

1. Open Prometheus UI: `http://localhost:${PROMETHEUS_PORT}`
2. Go to: **Status → Targets**
3. You should see:
   - `job="rust"` → target `rust-service:8080` **UP**
   - `job="cadvisor"` → target `cadvisor:8080` **UP**

Also try a quick query in Prometheus:

- `up{job="rust"}` → should be `1`
- `http_requests_received_total` → should show counters after you generate traffic

Your `prometheus.yml` looks like this:

```yaml
global:
  scrape_interval: 1s
  evaluation_interval: 1s

scrape_configs:
  - job_name: "rust"
    metrics_path: /metrics
    static_configs:
      - targets: ["rust-service:8080"]

  - job_name: "cadvisor"
    static_configs:
      - targets: ["cadvisor:8080"]
```

---

## Grafana provisioning (datasource + dashboards)

Grafana loads files from the container path `/etc/grafana/...` which is mapped via Docker volumes:

```yaml
grafana:
  volumes:
    - ./grafana/provisioning:/etc/grafana/provisioning:ro
    - ./grafana/dashboards:/etc/grafana/dashboards:ro
```

### Datasource provisioning

`grafana/provisioning/datasources/datasource.yml` (example):

```yaml
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
```

### Dashboard provisioning

`grafana/provisioning/dashboards/dashboards.yml`:

```yaml
apiVersion: 1
providers:
  - name: "bench"
    folder: "Bench"
    type: file
    options:
      path: /etc/grafana/dashboards
```

Any `*.json` dashboard you place in `grafana/dashboards/` will be loaded automatically into the **Bench** folder.

---

## Generate load (60s, 12 workers)

Use this PowerShell script to generate continuous traffic for 60 seconds:

```powershell
$baseUrl = "http://localhost:8080"
$seconds = 60
$workers = 12
$end = (Get-Date).AddSeconds($seconds)

$script = {
  param($baseUrl, $end)
  while((Get-Date) -lt $end){
    $id = Get-Random -Minimum 1 -Maximum 2000
    try { Invoke-WebRequest -UseBasicParsing "$baseUrl/api/item/$id" -TimeoutSec 2 | Out-Null } catch {}
  }
}

$jobs = 1..$workers | ForEach-Object { Start-Job -ScriptBlock $script -ArgumentList $baseUrl, $end }
Write-Host "Generating load for $seconds s with $workers workers..."
Wait-Job $jobs | Out-Null
Remove-Job $jobs
Write-Host "Done."
```

Then open Grafana and watch the panels update.

---

## Recommended PromQL queries

### RPS (Requests per second)

```promql
sum(rate(http_requests_received_total{job="rust", path="/api/item/{id}"}[10s]))
```

### p90 latency (seconds)

```promql
histogram_quantile(
  0.90,
  sum by (le) (
    rate(http_request_duration_seconds_bucket{job="rust", path="/api/item/{id}"}[10s])
  )
)
```

### Container CPU usage (cores)

```promql
sum(rate(container_cpu_usage_seconds_total{container_label_com_docker_compose_service="rust-service"}[10s]))
```

To display as percent, multiply by 100 and divide by available CPU cores (optional).

### Container memory (bytes)

```promql
container_memory_working_set_bytes{container_label_com_docker_compose_service="rust-service"}
```

> Note: labels can vary depending on your Docker/cAdvisor setup. If you don’t see `container_label_com_docker_compose_service`, search in Prometheus for `container_memory_working_set_bytes` and inspect labels to match the right container.

---

## Troubleshooting

### Grafana panels show 0 for CPU/memory

Most commonly this is a label mismatch. In Prometheus:

1. Search `container_memory_working_set_bytes`
2. Click a sample series
3. Copy the correct label set and adjust your Grafana query accordingly

Example alternative selectors:

```promql
container_memory_working_set_bytes{container_label_com_docker_compose_project="bench-local"}
```

or filter by `name` / `container` label depending on what your cAdvisor exposes.

### No metrics at `/metrics`

- Verify the Rust service is listening:
  - `docker compose logs -f rust-service`
- Test locally:
  - `Invoke-WebRequest http://localhost:8080/metrics -UseBasicParsing`

### Prometheus target is DOWN

- Confirm network name and service name are correct (`rust-service:8080`, `cadvisor:8080`)
- Check container logs:
  - `docker compose logs prometheus`
  - `docker compose logs cadvisor`

---