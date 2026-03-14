#!/bin/bash

# Configuration
CONCURRENCY=50
REQUESTS=10000
DURATION=3 # Seconds to wait for server to start

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Dapil Benchmark Suite ===${NC}"
echo -e "Requests: $REQUESTS, Concurrency: $CONCURRENCY\n"

run_bench() {
    local name=$1
    local command=$2
    local url=$3
    local port=$4

    echo -e "${GREEN}Testing $name...${NC}"
    
    # Kill any existing process on the port
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
    
    # Start the server
    eval "$command" > /dev/null 2>&1 &
    local pid=$!
    
    # Wait for server to start
    sleep $DURATION
    
    # Run ab
    ab -n $REQUESTS -c $CONCURRENCY "$url" | grep "Requests per second"
    
    # Cleanup
    kill $pid > /dev/null 2>&1 || true
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
    echo ""
}

# 1. Native Rust Benchmarks (Pre-build required)
if [ ! -f "benchmarks/rust_bench/target/release/axum_bench" ]; then
    echo "Building Rust benchmarks..."
    cargo build --release --manifest-path benchmarks/rust_bench/Cargo.toml > /dev/null 2>&1
fi

run_bench "Native Axum" "./benchmarks/rust_bench/target/release/axum_bench" "http://127.0.0.1:8182/" 8182
run_bench "Native Actix" "./benchmarks/rust_bench/target/release/actix_bench" "http://127.0.0.1:8183/" 8183

# 2. Dapil (Extreme)
run_bench "Dapil" "python examples/hello_world/main.py" "http://127.0.0.1:8181/" 8181

# 3. Competitive Frameworks
run_bench "Django-Bolt" "python benchmarks/bolt_app.py" "http://127.0.0.1:8084/" 8084
run_bench "Django (Gunicorn)" "gunicorn benchmarks.django_app:app --bind 127.0.0.1:8001 --workers 1 --worker-class sync" "http://127.0.0.1:8001/" 8001
run_bench "FastAPI (Uvicorn)" "uvicorn benchmarks.fastapi_app:app --host 127.0.0.1 --port 8000 --workers 1" "http://127.0.0.1:8000/" 8000

echo -e "${BLUE}=== Benchmarks Complete ===${NC}"
