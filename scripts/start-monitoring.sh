#!/bin/bash

# Bridge Router Monitoring Stack Startup Script

set -e

echo "🚀 Starting Bridge Router Monitoring Stack..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose is not installed. Please install docker-compose first."
    exit 1
fi

# Create necessary directories
echo "📁 Creating directories..."
mkdir -p grafana/dashboards
mkdir -p grafana/datasources
mkdir -p prometheus

# Start the monitoring stack
echo "🐳 Starting Docker containers..."
docker-compose -f docker-compose.monitoring.yml up -d

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 10

# Check service health
echo "🔍 Checking service health..."

# Check Prometheus
if curl -s http://localhost:9090/-/healthy > /dev/null; then
    echo "✅ Prometheus is healthy"
else
    echo "⚠️  Prometheus may not be ready yet"
fi

# Check Grafana
if curl -s http://localhost:3000/api/health > /dev/null; then
    echo "✅ Grafana is healthy"
else
    echo "⚠️  Grafana may not be ready yet"
fi

# Check Redis
if docker exec bridge-router-redis redis-cli ping > /dev/null 2>&1; then
    echo "✅ Redis is healthy"
else
    echo "⚠️  Redis may not be ready yet"
fi

# Check PostgreSQL
if docker exec bridge-router-postgres pg_isready -U bridge_router > /dev/null 2>&1; then
    echo "✅ PostgreSQL is healthy"
else
    echo "⚠️  PostgreSQL may not be ready yet"
fi

echo ""
echo "🎉 Monitoring stack is starting up!"
echo ""
echo "📊 Access your services:"
echo "   • Grafana Dashboard: http://localhost:3000 (admin/admin)"
echo "   • Prometheus:        http://localhost:9090"
echo "   • Bridge Router:     http://localhost:8080"
echo "   • Bridge Metrics:    http://localhost:8080/metrics"
echo ""
echo "📋 Useful commands:"
echo "   • View logs:         docker-compose -f docker-compose.monitoring.yml logs -f"
echo "   • Stop services:     docker-compose -f docker-compose.monitoring.yml down"
echo "   • Restart services:  docker-compose -f docker-compose.monitoring.yml restart"
echo ""
echo "🔧 Next steps:"
echo "   1. Start your Bridge Router application: cargo run"
echo "   2. Open Grafana and import the Bridge Router dashboard"
echo "   3. Check Prometheus targets to ensure metrics are being collected"
echo ""
echo "📖 For more information, see docs/monitoring-setup.md"
