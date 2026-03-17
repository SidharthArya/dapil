#!/bin/bash

CONCURRENCY=100
REQUESTS=20000
DURATION=3 # Seconds to wait for server to start

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Comprehensive Framework Benchmark Suite ===${NC}"
echo -e "Requests: $REQUESTS, Concurrency: $CONCURRENCY\n"

# Only build Rust benchmarks if they don't exist
if [ ! -f "benchmarks/rust_bench/target/release/axum_bench" ]; then
    echo "Building Rust benchmarks..."
    cargo build --release --manifest-path benchmarks/rust_bench/Cargo.toml > /dev/null 2>&1
fi

run_bench() {
    local name=$1
    local command=$2
    local url=$3
    local port=$4

    # Kill any existing process on the port
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
    
    # Start the server
    eval "$command" > /dev/null 2>&1 &
    local pid=$!
    
    # Wait for server to start
    sleep $DURATION
    
    # Run ab and extract requests per second
    local rps=$(ab -n $REQUESTS -c $CONCURRENCY "$url" 2>/dev/null | grep "Requests per second" | awk '{print $4}')
    printf "%-25s | %10s req/sec\n" "$name" "${rps:-FAILED}"
    
    # Cleanup
    kill $pid > /dev/null 2>&1 || true
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
}

run_worker_suite() {
    local w=$1
    echo -e "${BLUE}=== Testing with $w Worker(s) ===${NC}"
    printf "%-25s | %10s \n" "Framework" "Requests/sec"
    echo "--------------------------|-------------------"
    
    # 1. Dapil API (Features)
    run_bench "Dapil" "python benchmarks/feature_app.py --workers $w" "http://127.0.0.1:8181/api/test" 8181

    # 2. Django-Bolt
    run_bench "Django-Bolt" "python benchmarks/bolt_app.py --workers $w" "http://127.0.0.1:8084/" 8084

    # 3. FastAPI (Uvicorn)
    run_bench "FastAPI (Uvicorn)" "uvicorn benchmarks.fastapi_app:app --host 127.0.0.1 --port 8082 --workers $w" "http://127.0.0.1:8082/" 8082

    # 4. Django (Gunicorn)
    run_bench "Django (Gunicorn)" "gunicorn benchmarks.django_app:app --bind 127.0.0.1:8001 --workers $w --worker-class sync" "http://127.0.0.1:8001/" 8001

    # 5. Native Axum (Rust)
    run_bench "Native Axum" "TOKIO_WORKER_THREADS=$w ./benchmarks/rust_bench/target/release/axum_bench" "http://127.0.0.1:8182/" 8182

    # 6. Native Actix (Rust)
    run_bench "Native Actix" "ACTIX_WORKERS=$w ./benchmarks/rust_bench/target/release/actix_bench" "http://127.0.0.1:8183/" 8183

    echo ""
}

# Run both 1 worker and 4 worker suites
run_worker_suite 1
run_worker_suite 4

echo -e "${GREEN}=== Benchmarks Complete ===${NC}"
