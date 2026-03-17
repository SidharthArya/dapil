#!/bin/bash

CONCURRENCY=100
REQUESTS=20000
DURATION=3 # Seconds to wait for server to start
PORT=8181

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Dapil Features Benchmark ===${NC}"
echo -e "Requests: $REQUESTS, Concurrency: $CONCURRENCY\n"

run_bench() {
    local mode=$1
    local command=$2
    local workers=$3

    echo -e "${GREEN}Testing ${mode} (${workers} workers)...${NC}"
    
    # Kill any existing process on the port
    fuser -k "${PORT}/tcp" > /dev/null 2>&1 || true
    
    # Start the server
    eval "$command --workers $workers" > /dev/null 2>&1 &
    local pid=$!
    
    # Wait for server to start
    sleep $DURATION
    
    echo "1. Simple Prefixed Route (/api/test)"
    ab -c $CONCURRENCY -n $REQUESTS http://127.0.0.1:${PORT}/api/test | grep "Requests per second"
    
    echo "2. Dependency Injection Route (/api/auth)"
    ab -c $CONCURRENCY -n $REQUESTS http://127.0.0.1:${PORT}/api/auth | grep "Requests per second"

    echo "3. OpenAPI Generation (/openapi.json)"
    ab -c $CONCURRENCY -n $REQUESTS http://127.0.0.1:${PORT}/openapi.json | grep "Requests per second"
    
    # Cleanup
    kill $pid > /dev/null 2>&1 || true
    fuser -k "${PORT}/tcp" > /dev/null 2>&1 || true
    echo ""
}

# Run tests
run_bench "Single-Worker Mode" "python benchmarks/feature_app.py" 1
run_bench "Multi-Worker Mode" "python benchmarks/feature_app.py" 4

echo -e "${BLUE}=== Benchmarks Complete ===${NC}"
