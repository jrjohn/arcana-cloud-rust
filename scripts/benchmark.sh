#!/bin/bash
# Comprehensive Benchmark Script for Arcana Cloud
# Compares Rust vs Spring Boot architectures

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DURATION=30        # Test duration in seconds
THREADS=4          # Number of threads
CONNECTIONS=50     # Number of connections
RESULTS_DIR="benchmark_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create results directory
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Arcana Cloud Throughput Benchmark - Rust vs Spring Boot   ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Configuration:${NC}"
echo "  Duration: ${DURATION}s"
echo "  Threads: ${THREADS}"
echo "  Connections: ${CONNECTIONS}"
echo ""

# Function to run HTTP benchmark
run_http_benchmark() {
    local name=$1
    local url=$2
    local output_file=$3

    echo -e "${GREEN}Running HTTP benchmark: ${name}${NC}"
    echo "  URL: ${url}"

    # Use wrk for HTTP benchmarking
    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION}s --latency "$url" 2>&1 | tee "$output_file"
    echo ""
}

# Function to run gRPC benchmark
run_grpc_benchmark() {
    local name=$1
    local host=$2
    local proto=$3
    local method=$4
    local data=$5
    local output_file=$6

    echo -e "${GREEN}Running gRPC benchmark: ${name}${NC}"
    echo "  Host: ${host}"
    echo "  Method: ${method}"

    # Use ghz for gRPC benchmarking
    ghz --insecure \
        --proto "$proto" \
        --call "$method" \
        -d "$data" \
        -c ${CONNECTIONS} \
        -n 10000 \
        --format summary \
        "$host" 2>&1 | tee "$output_file"
    echo ""
}

# Function to extract metrics from wrk output
extract_wrk_metrics() {
    local file=$1

    local rps=$(grep "Requests/sec" "$file" | awk '{print $2}')
    local latency_avg=$(grep "Latency" "$file" | head -1 | awk '{print $2}')
    local latency_99=$(grep "99%" "$file" | awk '{print $2}')

    echo "$rps,$latency_avg,$latency_99"
}

# Function to check if service is healthy
check_health() {
    local url=$1
    local name=$2
    local max_retries=10
    local count=0

    echo -e "${YELLOW}Checking health of ${name}...${NC}"

    while [ $count -lt $max_retries ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ ${name} is healthy${NC}"
            return 0
        fi
        count=$((count + 1))
        echo "  Waiting for ${name}... ($count/$max_retries)"
        sleep 2
    done

    echo -e "${RED}✗ ${name} is not responding${NC}"
    return 1
}

# ============================================================================
# RUST BENCHMARKS
# ============================================================================

run_rust_benchmarks() {
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                    RUST ARCHITECTURE BENCHMARKS                ${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Monolithic Mode
    if check_health "http://localhost:8080/health" "Rust Monolithic"; then
        echo ""
        echo -e "${YELLOW}=== Rust Monolithic Mode ===${NC}"

        # Health endpoint (warm-up)
        run_http_benchmark "Rust Monolithic - Health (warmup)" \
            "http://localhost:8080/health" \
            "$RESULTS_DIR/rust_mono_health_warmup.txt"

        # Health endpoint (actual test)
        run_http_benchmark "Rust Monolithic - Health" \
            "http://localhost:8080/health" \
            "$RESULTS_DIR/rust_mono_health_${TIMESTAMP}.txt"

        # API endpoint (if auth required, use health as baseline)
        run_http_benchmark "Rust Monolithic - API Readiness" \
            "http://localhost:8080/api/health/readiness" \
            "$RESULTS_DIR/rust_mono_readiness_${TIMESTAMP}.txt"
    fi

    # K8s Mode (if available)
    if check_health "http://localhost:8081/health" "Rust K8s"; then
        echo ""
        echo -e "${YELLOW}=== Rust K8s Layered Mode ===${NC}"

        run_http_benchmark "Rust K8s - Health (warmup)" \
            "http://localhost:8081/health" \
            "$RESULTS_DIR/rust_k8s_health_warmup.txt"

        run_http_benchmark "Rust K8s - Health" \
            "http://localhost:8081/health" \
            "$RESULTS_DIR/rust_k8s_health_${TIMESTAMP}.txt"
    fi
}

# ============================================================================
# SPRING BOOT BENCHMARKS
# ============================================================================

run_springboot_benchmarks() {
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}                SPRING BOOT ARCHITECTURE BENCHMARKS             ${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Monolithic Mode
    if check_health "http://localhost:8082/actuator/health" "Spring Boot Monolithic"; then
        echo ""
        echo -e "${YELLOW}=== Spring Boot Monolithic Mode ===${NC}"

        # Health endpoint (warm-up - important for JVM)
        run_http_benchmark "Spring Boot Monolithic - Health (warmup)" \
            "http://localhost:8082/actuator/health" \
            "$RESULTS_DIR/springboot_mono_health_warmup.txt"

        # Let JVM warm up more
        sleep 5

        # Health endpoint (actual test)
        run_http_benchmark "Spring Boot Monolithic - Health" \
            "http://localhost:8082/actuator/health" \
            "$RESULTS_DIR/springboot_mono_health_${TIMESTAMP}.txt"
    fi
}

# ============================================================================
# MAIN
# ============================================================================

echo -e "${YELLOW}Starting benchmarks...${NC}"
echo ""

# Run Rust benchmarks
run_rust_benchmarks

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    BENCHMARK COMPLETE                          ${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Results saved to: $RESULTS_DIR/"
echo ""

# List result files
ls -la "$RESULTS_DIR/"*.txt 2>/dev/null || echo "No results yet"
