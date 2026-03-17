#!/bin/bash

CONCURRENCY=100
REQUESTS=20000
DURATION=3 # Seconds to wait for server to start

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

if [ -z "$1" ]; then
    echo -e "${RED}Error: Please specify a framework to benchmark.${NC}"
    echo "Usage: ./benchmarks/run_isolated.sh <framework_name> [workers]"
    echo "Available frameworks: dapil, dapil-old, bolt, fastapi, django, axum, actix"
    exit 1
fi

FRAMEWORK=$1
WORKERS=${2:-1}

echo -e "${BLUE}=== Isolated Benchmark Suite ===${NC}"
echo -e "Requests: $REQUESTS, Concurrency: $CONCURRENCY, Workers: $WORKERS\n"

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
    local rps=$(ab -n $REQUESTS -c $CONCURRENCY "$url" 2>/dev/null | grep "Requests per second" | awk '{print $4}')
    echo -e "Result: ${rps:-FAILED} req/sec\n"
    
    # Cleanup
    kill $pid > /dev/null 2>&1 || true
    fuser -k "${port}/tcp" > /dev/null 2>&1 || true
}

case $FRAMEWORK in
    dapil)
        run_bench "Dapil (Features)" "python benchmarks/feature_app.py --workers $WORKERS" "http://127.0.0.1:8181/api/test" 8181
        ;;
    dapil-old)
        run_bench "Dapil (Hello World)" "python examples/hello_world/main.py" "http://127.0.0.1:8181/" 8181
        ;;
    bolt)
        run_bench "Django-Bolt" "python benchmarks/bolt_app.py --workers $WORKERS" "http://127.0.0.1:8084/" 8084
        ;;
    fastapi)
        run_bench "FastAPI (Uvicorn)" "uvicorn benchmarks.fastapi_app:app --host 127.0.0.1 --port 8082 --workers $WORKERS" "http://127.0.0.1:8082/" 8082
        ;;
    django)
        run_bench "Django (Gunicorn)" "gunicorn benchmarks.django_app:app --bind 127.0.0.1:8001 --workers $WORKERS --worker-class sync" "http://127.0.0.1:8001/" 8001
        ;;
    axum)
        if [ ! -f "benchmarks/rust_bench/target/release/axum_bench" ]; then
            cargo build --release --manifest-path benchmarks/rust_bench/Cargo.toml > /dev/null 2>&1
        fi
        run_bench "Native Axum" "TOKIO_WORKER_THREADS=$WORKERS ./benchmarks/rust_bench/target/release/axum_bench" "http://127.0.0.1:8182/" 8182
        ;;
    actix)
        if [ ! -f "benchmarks/rust_bench/target/release/actix_bench" ]; then
            cargo build --release --manifest-path benchmarks/rust_bench/Cargo.toml > /dev/null 2>&1
        fi
        run_bench "Native Actix" "ACTIX_WORKERS=$WORKERS ./benchmarks/rust_bench/target/release/actix_bench" "http://127.0.0.1:8183/" 8183
        ;;
    *)
        echo "Unknown framework: $FRAMEWORK"
        ;;
esac
