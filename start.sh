#!/bin/bash

# Quick start script for Monolith development
set -e

echo "🚀 Starting Monolith Web Application..."

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Create necessary directories
mkdir -p data logs

# Copy config if needed
if [ ! -f "translation-config.toml" ] && [ -f "translation-config.toml.example" ]; then
    cp translation-config.toml.example translation-config.toml
    echo "📋 Created translation configuration from example"
fi

# Start services
echo "🔧 Starting services..."
docker-compose up -d

# Wait a moment for services to start
sleep 5

# Show status
echo "📊 Service Status:"
docker-compose ps

echo ""
echo "✅ Monolith is now running!"
echo "🌐 Web Interface: http://localhost:7080"
echo "📚 Library: http://localhost:7080/library"
echo "🔧 Redis Commander: http://localhost:8081 (admin/secret)"
echo ""
echo "📝 To view logs: docker-compose logs -f"
echo "🛑 To stop: docker-compose down"