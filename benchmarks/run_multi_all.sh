#!/bin/bash

# Configuration
CONCURRENCY=200
REQUESTS=50000
WORKERS=4
DURATION=5 # Seconds to wait for server to start

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Dapil Multi-Worker Benchmark Suite (Workers: $WORKERS) ===${NC}"
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
    pkill -P $pid > /dev/null 2>&1 || true # Kill children
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
    echo ""
}

# 1. Native Rust Benchmarks (Pre-build required)
# Note: Native Rust uses all cores/threads by default.
if [ ! -f "benchmarks/rust_bench/target/release/axum_bench" ]; then
    echo "Building Rust benchmarks..."
    cargo build --release --manifest-path benchmarks/rust_bench/Cargo.toml > /dev/null 2>&1
fi

export TOKIO_WORKER_THREADS=$WORKERS
run_bench "Native Axum (Tokio threads: $WORKERS)" "./benchmarks/rust_bench/target/release/axum_bench" "http://127.0.0.1:8182/" 8182
run_bench "Native Actix (Workes: $WORKERS)" "./benchmarks/rust_bench/target/release/actix_bench" "http://127.0.0.1:8183/" 8183 # Actix uses threads per physical core by default, but we'll see

# 2. Dapil (Extreme + Multi-Worker)
run_bench "Dapil ($WORKERS workers)" "python benchmarks/dapil_multi.py" "http://127.0.0.1:8181/" 8181

# 3. Competitive Frameworks
export PYTHONPATH=$PYTHONPATH:.
export DJANGO_BOLT_WORKERS=$WORKERS
run_bench "Django-Bolt ($WORKERS workers)" "python benchmarks/bolt_app.py" "http://127.0.0.1:8084/" 8084
run_bench "Django (Gunicorn, $WORKERS workers)" "gunicorn benchmarks.django_app:application --bind 127.0.0.1:8001 --workers $WORKERS --worker-class sync" "http://127.0.0.1:8001/" 8001
run_bench "FastAPI (Uvicorn, $WORKERS workers)" "uvicorn benchmarks.fastapi_app:app --host 127.0.0.1 --port 8000 --workers $WORKERS" "http://127.0.0.1:8000/" 8000

echo -e "${BLUE}=== Multi-Worker Benchmarks Complete ===${NC}"
