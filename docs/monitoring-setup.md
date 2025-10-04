# Bridge Router Monitoring Setup

This guide explains how to set up Prometheus metrics collection and Grafana dashboards for the Bridge Router application.

## Overview

The monitoring stack includes:
- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **Bridge Router**: Application with built-in metrics endpoint

## Quick Start

### 1. Start the Monitoring Stack

```bash
# Start Prometheus, Grafana, Redis, and PostgreSQL
docker-compose -f docker-compose.monitoring.yml up -d

# Check that all services are running
docker-compose -f docker-compose.monitoring.yml ps
```

### 2. Start the Bridge Router Application

```bash
# Make sure the application is running and exposing metrics
cargo run

# Or if running in production
./target/release/bridge-router
```

### 3. Access the Services

- **Grafana Dashboard**: http://localhost:3000
  - Username: `admin`
  - Password: `admin`
- **Prometheus**: http://localhost:9090
- **Bridge Router**: http://localhost:8080
- **Bridge Router Metrics**: http://localhost:8080/metrics

## Available Metrics

The Bridge Router application exposes the following metrics:

### HTTP Metrics
- `http_requests_total` - Total HTTP requests by method, path, and status
- `http_request_duration_seconds` - HTTP request duration histogram
- `http_requests_errors_total` - HTTP request errors

### Bridge Service Metrics
- `bridge_requests_total` - Bridge quote requests by bridge and status
- `bridge_response_time_seconds` - Bridge response time histogram
- `bridge_errors_total` - Bridge service errors

### Database Metrics
- `database_queries_total` - Database queries by operation and status
- `database_query_duration_seconds` - Database query duration
- `database_connections_active` - Active database connections
- `database_connections_idle` - Idle database connections

### Cache Metrics
- `cache_operations_total` - Cache operations by type
- `cache_hits_total` - Cache hits
- `cache_misses_total` - Cache misses

### Redis Metrics
- `redis_memory_usage_bytes` - Redis memory usage
- `redis_total_keys` - Total Redis keys
- `redis_connected_clients` - Connected Redis clients
- `redis_uptime_seconds` - Redis uptime
- `redis_keyspace_hits_total` - Redis keyspace hits
- `redis_keyspace_misses_total` - Redis keyspace misses
- `redis_evicted_keys_total` - Evicted keys

### Security Metrics
- `security_events_total` - Security events by type and severity
- `security_audits_processed_total` - Security audits processed
- `security_exploits_processed_total` - Security exploits processed

## Grafana Dashboard

The included dashboard provides:

1. **HTTP Request Rate** - Requests per second by method and path
2. **HTTP Request Duration** - 95th and 50th percentile response times
3. **Bridge Request Rate** - Bridge quote requests per second by bridge
4. **Bridge Response Time** - 95th percentile response times by bridge
5. **Database Connections** - Active and idle connection counts
6. **Redis Memory Usage** - Memory consumption over time
7. **Redis Keys** - Total number of keys in Redis
8. **Cache Hit Rate** - Percentage of cache hits vs misses
9. **Security Events** - Security events by type and severity

## Configuration

### Prometheus Configuration

The Prometheus configuration is located in `prometheus/prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'bridge-router'
    static_configs:
      - targets: ['host.docker.internal:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s
```

### Grafana Configuration

- **Datasources**: Automatically configured in `grafana/datasources/prometheus.yml`
- **Dashboards**: Automatically provisioned from `grafana/dashboards/`

## Customization

### Adding New Metrics

1. **Define the metric** in `src/telemetry/metrics.rs`:
```rust
use metrics::{counter, histogram, gauge};

// Counter example
counter!("custom_metric_total", "label" => "value").increment(1);

// Histogram example
histogram!("custom_duration_seconds", "operation" => "custom").record(0.5);

// Gauge example
gauge!("custom_gauge", "status" => "active").set(1.0);
```

2. **Describe the metric** in the `describe_metrics()` function:
```rust
describe_counter!(
    "custom_metric_total",
    Unit::Count,
    "Description of the custom metric"
);
```

3. **Update the Grafana dashboard** to include the new metric.

### Modifying the Dashboard

1. **Export current dashboard**:
   - Go to Grafana → Dashboards → Bridge Router Dashboard
   - Click "Share" → "Export" → "Save to file"

2. **Modify the JSON** and save to `grafana/dashboards/bridge-router-dashboard.json`

3. **Restart Grafana** to apply changes:
```bash
docker-compose -f docker-compose.monitoring.yml restart grafana
```

## Troubleshooting

### Common Issues

1. **Metrics not appearing in Prometheus**:
   - Check that the Bridge Router is running on port 8080
   - Verify the metrics endpoint: `curl http://localhost:8080/metrics`
   - Check Prometheus targets: http://localhost:9090/targets

2. **Dashboard not loading in Grafana**:
   - Check Grafana logs: `docker-compose -f docker-compose.monitoring.yml logs grafana`
   - Verify datasource connection in Grafana: Configuration → Data Sources

3. **High memory usage**:
   - Adjust Prometheus retention: Modify `--storage.tsdb.retention.time` in docker-compose
   - Reduce scrape interval for less critical metrics

### Logs

View logs for troubleshooting:
```bash
# All services
docker-compose -f docker-compose.monitoring.yml logs

# Specific service
docker-compose -f docker-compose.monitoring.yml logs prometheus
docker-compose -f docker-compose.monitoring.yml logs grafana
```

## Production Considerations

### Security
- Change default Grafana admin password
- Use environment variables for sensitive configuration
- Consider using HTTPS for Grafana and Prometheus

### Performance
- Adjust scrape intervals based on your needs
- Configure appropriate retention policies
- Monitor Prometheus and Grafana resource usage

### High Availability
- Consider running Prometheus and Grafana in a cluster
- Set up alerting rules in Prometheus
- Configure Grafana alerting for critical metrics

## Alerting (Optional)

To set up alerting, you can:

1. **Configure Prometheus alerting rules** in `prometheus/alerts.yml`
2. **Set up Grafana alerting** for dashboard panels
3. **Use Alertmanager** for notification routing

Example alert rule:
```yaml
groups:
  - name: bridge-router
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
```

## Cleanup

To stop and remove the monitoring stack:

```bash
# Stop services
docker-compose -f docker-compose.monitoring.yml down

# Remove volumes (WARNING: This will delete all data)
docker-compose -f docker-compose.monitoring.yml down -v
```
