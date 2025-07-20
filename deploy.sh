#!/bin/bash

# Monolith Docker Deployment Script
# Usage: ./deploy.sh [dev|prod] [action]
# Actions: start, stop, restart, logs, status

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
ENVIRONMENT=${1:-dev}
ACTION=${2:-start}

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if Docker is running
check_docker() {
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker first."
        exit 1
    fi
}

# Function to check if docker-compose is available
check_compose() {
    if ! command -v docker-compose >/dev/null 2>&1; then
        print_error "docker-compose is not installed. Please install it first."
        exit 1
    fi
}

# Function to create necessary directories
create_directories() {
    print_status "Creating necessary directories..."
    mkdir -p data logs config
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        mkdir -p config/ssl
    fi
    
    print_success "Directories created successfully"
}

# Function to set up configuration files
setup_config() {
    print_status "Setting up configuration files..."
    
    # Create Redis configuration
    if [ ! -f "config/redis.conf" ]; then
        cat > config/redis.conf << EOF
# Redis configuration for development
port 6379
bind 0.0.0.0
save 900 1
save 300 10
save 60 10000
rdbcompression yes
dbfilename dump.rdb
dir /data
appendonly yes
appendfsync everysec
EOF
        print_status "Created Redis development configuration"
    fi
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        if [ ! -f "config/redis.prod.conf" ]; then
            cat > config/redis.prod.conf << EOF
# Redis configuration for production
port 6379
bind 127.0.0.1
protected-mode yes
save 900 1
save 300 10
save 60 10000
rdbcompression yes
dbfilename dump.rdb
dir /data
appendonly yes
appendfsync everysec
maxmemory 256mb
maxmemory-policy allkeys-lru
EOF
            print_status "Created Redis production configuration"
        fi
        
        # Create Nginx configuration for production
        if [ ! -f "config/nginx.conf" ]; then
            cat > config/nginx.conf << EOF
events {
    worker_connections 1024;
}

http {
    upstream monolith_backend {
        server monolith-web:7080;
    }
    
    server {
        listen 80;
        server_name _;
        
        # Redirect HTTP to HTTPS
        return 301 https://\$server_name\$request_uri;
    }
    
    server {
        listen 443 ssl http2;
        server_name _;
        
        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;
        
        # Security headers
        add_header X-Frame-Options DENY;
        add_header X-Content-Type-Options nosniff;
        add_header X-XSS-Protection "1; mode=block";
        
        location / {
            proxy_pass http://monolith_backend;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto \$scheme;
        }
        
        # Serve static files directly
        location /static/ {
            proxy_pass http://monolith_backend;
            expires 1y;
            add_header Cache-Control "public, immutable";
        }
    }
}
EOF
            print_status "Created Nginx production configuration"
        fi
    fi
    
    # Copy translation config if it doesn't exist
    if [ ! -f "translation-config.toml" ] && [ -f "translation-config.toml.example" ]; then
        cp translation-config.toml.example translation-config.toml
        print_status "Created translation configuration from example"
    fi
    
    print_success "Configuration files set up successfully"
}

# Function to start services
start_services() {
    local compose_file="docker-compose.yml"
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        compose_file="docker-compose.prod.yml"
        print_status "Starting production environment..."
    else
        print_status "Starting development environment..."
    fi
    
    docker-compose -f "$compose_file" up -d
    print_success "Services started successfully"
    
    # Show status
    docker-compose -f "$compose_file" ps
}

# Function to stop services
stop_services() {
    local compose_file="docker-compose.yml"
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        compose_file="docker-compose.prod.yml"
        print_status "Stopping production environment..."
    else
        print_status "Stopping development environment..."
    fi
    
    docker-compose -f "$compose_file" down
    print_success "Services stopped successfully"
}

# Function to restart services
restart_services() {
    stop_services
    sleep 2
    start_services
}

# Function to show logs
show_logs() {
    local compose_file="docker-compose.yml"
    local service=${3:-}
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        compose_file="docker-compose.prod.yml"
    fi
    
    if [ -n "$service" ]; then
        docker-compose -f "$compose_file" logs -f "$service"
    else
        docker-compose -f "$compose_file" logs -f
    fi
}

# Function to show status
show_status() {
    local compose_file="docker-compose.yml"
    
    if [ "$ENVIRONMENT" = "prod" ]; then
        compose_file="docker-compose.prod.yml"
    fi
    
    print_status "Service Status:"
    docker-compose -f "$compose_file" ps
    
    print_status "System Resources:"
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"
}

# Function to show help
show_help() {
    echo "Monolith Docker Deployment Script"
    echo ""
    echo "Usage: $0 [ENVIRONMENT] [ACTION] [SERVICE]"
    echo ""
    echo "ENVIRONMENT:"
    echo "  dev     Development environment (default)"
    echo "  prod    Production environment"
    echo ""
    echo "ACTION:"
    echo "  start   Start services (default)"
    echo "  stop    Stop services"
    echo "  restart Restart services"
    echo "  logs    Show logs"
    echo "  status  Show status"
    echo "  help    Show this help"
    echo ""
    echo "SERVICE (optional for logs):"
    echo "  monolith-web    Application logs"
    echo "  redis          Redis logs"
    echo "  nginx          Nginx logs (prod only)"
    echo ""
    echo "Examples:"
    echo "  $0 dev start"
    echo "  $0 prod logs monolith-web"
    echo "  $0 dev status"
}

# Main execution
main() {
    case "$ACTION" in
        start)
            check_docker
            check_compose
            create_directories
            setup_config
            start_services
            ;;
        stop)
            check_docker
            check_compose
            stop_services
            ;;
        restart)
            check_docker
            check_compose
            restart_services
            ;;
        logs)
            check_docker
            check_compose
            show_logs
            ;;
        status)
            check_docker
            check_compose
            show_status
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown action: $ACTION"
            show_help
            exit 1
            ;;
    esac
}

# Validate environment
if [ "$ENVIRONMENT" != "dev" ] && [ "$ENVIRONMENT" != "prod" ]; then
    print_error "Invalid environment: $ENVIRONMENT. Use 'dev' or 'prod'."
    exit 1
fi

# Run main function
main