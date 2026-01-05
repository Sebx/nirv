#!/bin/bash

# GitHub Actions Workflow Validation Script
# This script validates the complete workflow integration and configuration

set -e

echo "=== GitHub Actions Workflow Validation ==="
echo "Starting comprehensive workflow validation..."
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Validation counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNING_CHECKS=0

# Function to print status
print_status() {
    local status=$1
    local message=$2
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    case $status in
        "PASS")
            echo -e "${GREEN}✅ PASS${NC}: $message"
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
            ;;
        "FAIL")
            echo -e "${RED}❌ FAIL${NC}: $message"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            ;;
        "WARN")
            echo -e "${YELLOW}⚠️  WARN${NC}: $message"
            WARNING_CHECKS=$((WARNING_CHECKS + 1))
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  INFO${NC}: $message"
            ;;
    esac
}

# Function to check file exists
check_file_exists() {
    local file=$1
    local description=$2
    
    if [ -f "$file" ]; then
        print_status "PASS" "$description exists: $file"
        return 0
    else
        print_status "FAIL" "$description missing: $file"
        return 1
    fi
}

# Function to validate YAML syntax
validate_yaml() {
    local file=$1
    local description=$2
    
    if command -v yq >/dev/null 2>&1; then
        if yq eval '.' "$file" >/dev/null 2>&1; then
            print_status "PASS" "$description has valid YAML syntax"
            return 0
        else
            print_status "FAIL" "$description has invalid YAML syntax"
            return 1
        fi
    else
        print_status "WARN" "yq not available - skipping YAML validation for $description"
        return 0
    fi
}

# Function to check workflow triggers
check_workflow_triggers() {
    local workflow_file=$1
    shift # Remove first argument (workflow_file)
    local expected_triggers=("$@")
    
    print_status "INFO" "Checking workflow triggers in $(basename $workflow_file)"
    
    for trigger in "${expected_triggers[@]}"; do
        if grep -q "$trigger" "$workflow_file"; then
            print_status "PASS" "Trigger '$trigger' found in workflow"
        else
            print_status "FAIL" "Trigger '$trigger' missing from workflow"
        fi
    done
}

# Function to check service containers
check_service_containers() {
    local workflow_file=$1
    local services=("sqlserver" "postgres" "mysql")
    
    print_status "INFO" "Checking database service containers"
    
    for service in "${services[@]}"; do
        if grep -q "$service:" "$workflow_file"; then
            print_status "PASS" "Service container '$service' configured"
            
            # Check health checks
            if grep -A 10 "$service:" "$workflow_file" | grep -q "health-cmd"; then
                print_status "PASS" "Health check configured for '$service'"
            else
                print_status "WARN" "Health check missing for '$service'"
            fi
        else
            print_status "FAIL" "Service container '$service' not configured"
        fi
    done
}

# Function to check caching configuration
check_caching_config() {
    local workflow_file=$1
    local cache_types=("Cargo registry" "Cargo binaries" "build artifacts")
    
    print_status "INFO" "Checking caching configuration"
    
    if grep -q "actions/cache@v3" "$workflow_file"; then
        print_status "PASS" "Cache action configured"
        
        # Check for different cache types
        if grep -q "~/.cargo/registry" "$workflow_file"; then
            print_status "PASS" "Cargo registry cache configured"
        else
            print_status "FAIL" "Cargo registry cache missing"
        fi
        
        if grep -q "~/.cargo/bin" "$workflow_file"; then
            print_status "PASS" "Cargo binaries cache configured"
        else
            print_status "WARN" "Cargo binaries cache missing"
        fi
        
        if grep -q "target/" "$workflow_file"; then
            print_status "PASS" "Build artifacts cache configured"
        else
            print_status "FAIL" "Build artifacts cache missing"
        fi
        
        # Check cache key strategy
        if grep -q "hashFiles.*Cargo.lock" "$workflow_file"; then
            print_status "PASS" "Cache invalidation based on Cargo.lock"
        else
            print_status "FAIL" "Cache invalidation strategy missing"
        fi
        
    else
        print_status "FAIL" "No caching configured"
    fi
}

# Function to check quality checks
check_quality_checks() {
    local workflow_file=$1
    local quality_checks=("cargo fmt" "cargo clippy" "cargo audit")
    
    print_status "INFO" "Checking code quality enforcement"
    
    for check in "${quality_checks[@]}"; do
        if grep -q "$check" "$workflow_file"; then
            print_status "PASS" "Quality check '$check' configured"
        else
            print_status "FAIL" "Quality check '$check' missing"
        fi
    done
    
    # Check for strict enforcement
    if grep -q "\-D warnings" "$workflow_file"; then
        print_status "PASS" "Strict clippy enforcement configured"
    else
        print_status "WARN" "Clippy warnings not treated as errors"
    fi
    
    # Check for continue-on-error (should not be present for strict enforcement)
    if grep -q "continue-on-error.*true" "$workflow_file"; then
        print_status "WARN" "continue-on-error found - may weaken quality enforcement"
    else
        print_status "PASS" "No continue-on-error found - strict enforcement maintained"
    fi
}

# Function to check parallel execution
check_parallel_execution() {
    local workflow_file=$1
    
    print_status "INFO" "Checking parallel execution configuration"
    
    if grep -q "strategy:" "$workflow_file"; then
        print_status "PASS" "Matrix strategy configured for parallel execution"
        
        if grep -q "fail-fast.*false" "$workflow_file"; then
            print_status "PASS" "fail-fast disabled for comprehensive feedback"
        else
            print_status "WARN" "fail-fast not explicitly disabled"
        fi
        
        if grep -q "max-parallel" "$workflow_file"; then
            print_status "PASS" "max-parallel configured for resource management"
        else
            print_status "WARN" "max-parallel not configured"
        fi
    else
        print_status "FAIL" "No parallel execution strategy found"
    fi
}

# Function to check performance monitoring
check_performance_monitoring() {
    local workflow_file=$1
    
    print_status "INFO" "Checking performance monitoring"
    
    # Check for performance thresholds
    local thresholds=("MAX_PREPARE_TIME" "MAX_QUALITY_TIME" "MAX_TEST_TIME" "MAX_TOTAL_PIPELINE_TIME")
    
    for threshold in "${thresholds[@]}"; do
        if grep -q "$threshold" "$workflow_file"; then
            print_status "PASS" "Performance threshold '$threshold' configured"
        else
            print_status "WARN" "Performance threshold '$threshold' missing"
        fi
    done
    
    # Check for timing measurements
    if grep -q "date +%s" "$workflow_file"; then
        print_status "PASS" "Timing measurements configured"
    else
        print_status "WARN" "No timing measurements found"
    fi
}

# Function to check error handling
check_error_handling() {
    local workflow_file=$1
    
    print_status "INFO" "Checking error handling and retry logic"
    
    # Check for retry logic
    if grep -q "for attempt in" "$workflow_file"; then
        print_status "PASS" "Retry logic implemented"
    else
        print_status "WARN" "No retry logic found"
    fi
    
    # Check for detailed error reporting
    if grep -q "Connection details:" "$workflow_file"; then
        print_status "PASS" "Detailed error reporting configured"
    else
        print_status "WARN" "Basic error reporting may be insufficient"
    fi
    
    # Check for timeout handling
    if grep -q "timeout" "$workflow_file"; then
        print_status "PASS" "Timeout handling configured"
    else
        print_status "WARN" "No explicit timeout handling found"
    fi
}

# Function to validate environment variables
check_environment_variables() {
    local workflow_file=$1
    
    print_status "INFO" "Checking environment variable configuration"
    
    # Database connection strings
    local db_vars=("SQLSERVER_CONNECTION_STRING" "POSTGRES_CONNECTION_STRING" "MYSQL_CONNECTION_STRING")
    
    for var in "${db_vars[@]}"; do
        if grep -q "$var" "$workflow_file"; then
            print_status "PASS" "Environment variable '$var' configured"
        else
            print_status "WARN" "Environment variable '$var' missing"
        fi
    done
    
    # Rust-specific variables
    if grep -q "CARGO_TERM_COLOR" "$workflow_file"; then
        print_status "PASS" "Cargo color output configured"
    fi
    
    if grep -q "RUST_BACKTRACE" "$workflow_file"; then
        print_status "PASS" "Rust backtrace configured"
    fi
}

# Function to check release workflow
check_release_workflow() {
    local release_file=".github/workflows/release.yml"
    
    print_status "INFO" "Validating release workflow"
    
    if [ ! -f "$release_file" ]; then
        print_status "FAIL" "Release workflow file missing"
        return 1
    fi
    
    # Check release triggers
    if grep -q "tags:" "$release_file"; then
        print_status "PASS" "Release triggered by tags"
    else
        print_status "FAIL" "Release tag trigger missing"
    fi
    
    # Check multi-platform builds
    local platforms=("ubuntu-latest" "windows-latest" "macos-latest")
    for platform in "${platforms[@]}"; do
        if grep -q "$platform" "$release_file"; then
            print_status "PASS" "Multi-platform build for '$platform' configured"
        else
            print_status "WARN" "Platform '$platform' not configured for release"
        fi
    done
    
    # Check checksum generation
    if grep -q "sha256sum" "$release_file"; then
        print_status "PASS" "Checksum generation configured"
    else
        print_status "FAIL" "Checksum generation missing"
    fi
    
    # Check draft release creation
    if grep -q "create-release" "$release_file" || grep -q "draft.*true" "$release_file"; then
        print_status "PASS" "Draft release creation configured"
    else
        print_status "WARN" "Draft release creation not found"
    fi
}

# Function to validate project structure
check_project_structure() {
    print_status "INFO" "Validating project structure for CI/CD"
    
    # Check essential files
    local essential_files=(
        "Cargo.toml"
        "Cargo.lock"
        "src/main.rs"
        "README.md"
        "LICENSE"
    )
    
    for file in "${essential_files[@]}"; do
        check_file_exists "$file" "Essential project file"
    done
    
    # Check for test directories
    if [ -d "tests" ]; then
        print_status "PASS" "Integration tests directory exists"
    else
        print_status "WARN" "No integration tests directory found"
    fi
    
    # Check for documentation
    if [ -d "docs" ]; then
        print_status "PASS" "Documentation directory exists"
    else
        print_status "WARN" "No documentation directory found"
    fi
}

# Function to check Rust project configuration
check_rust_config() {
    print_status "INFO" "Validating Rust project configuration"
    
    # Check Cargo.toml structure
    if [ -f "Cargo.toml" ]; then
        if grep -q "\[package\]" "Cargo.toml"; then
            print_status "PASS" "Package section found in Cargo.toml"
        else
            print_status "FAIL" "Package section missing in Cargo.toml"
        fi
        
        if grep -q "version.*=" "Cargo.toml"; then
            print_status "PASS" "Version specified in Cargo.toml"
        else
            print_status "FAIL" "Version missing in Cargo.toml"
        fi
        
        # Check for workspace configuration if applicable
        if grep -q "\[workspace\]" "Cargo.toml"; then
            print_status "INFO" "Workspace configuration detected"
        fi
    fi
    
    # Check for rustfmt configuration
    if [ -f "rustfmt.toml" ] || [ -f ".rustfmt.toml" ]; then
        print_status "PASS" "Rustfmt configuration found"
    else
        print_status "WARN" "No rustfmt configuration found (using defaults)"
    fi
    
    # Check for clippy configuration
    if [ -f "clippy.toml" ] || [ -f ".clippy.toml" ]; then
        print_status "PASS" "Clippy configuration found"
    else
        print_status "WARN" "No clippy configuration found (using defaults)"
    fi
}

# Function to simulate workflow execution
simulate_workflow_steps() {
    print_status "INFO" "Simulating workflow execution steps"
    
    # Check if Rust toolchain is available
    if command -v cargo >/dev/null 2>&1; then
        print_status "PASS" "Cargo available for local testing"
        
        # Test basic cargo commands
        if cargo check --quiet 2>/dev/null; then
            print_status "PASS" "Project compiles successfully"
        else
            print_status "FAIL" "Project compilation failed"
        fi
        
        # Test formatting
        if cargo fmt --check 2>/dev/null; then
            print_status "PASS" "Code formatting is correct"
        else
            print_status "WARN" "Code formatting issues detected"
        fi
        
        # Test clippy (if available)
        if command -v cargo-clippy >/dev/null 2>&1; then
            if cargo clippy --quiet -- -D warnings 2>/dev/null; then
                print_status "PASS" "No clippy warnings detected"
            else
                print_status "WARN" "Clippy warnings detected"
            fi
        else
            print_status "WARN" "Clippy not available for local testing"
        fi
        
    else
        print_status "WARN" "Rust toolchain not available for local validation"
    fi
}

# Function to check documentation completeness
check_documentation() {
    print_status "INFO" "Checking workflow documentation"
    
    # Check for workflow documentation
    if [ -f "docs/github-actions-workflow.md" ]; then
        print_status "PASS" "Workflow documentation exists"
    else
        print_status "WARN" "Workflow documentation missing"
    fi
    
    # Check for troubleshooting guide
    if [ -f "docs/github-actions-troubleshooting.md" ]; then
        print_status "PASS" "Troubleshooting guide exists"
    else
        print_status "WARN" "Troubleshooting guide missing"
    fi
    
    # Check README for CI/CD information
    if [ -f "README.md" ]; then
        if grep -qi "github.*actions\|ci.*cd\|workflow" "README.md"; then
            print_status "PASS" "README contains CI/CD information"
        else
            print_status "WARN" "README lacks CI/CD documentation"
        fi
    fi
}

# Main validation function
main() {
    echo "Starting GitHub Actions workflow validation..."
    echo "Timestamp: $(date)"
    echo ""
    
    # 1. Check workflow files exist
    echo "=== 1. Workflow File Validation ==="
    check_file_exists ".github/workflows/rust.yml" "Main CI/CD workflow"
    check_file_exists ".github/workflows/release.yml" "Release workflow"
    echo ""
    
    # 2. Validate YAML syntax
    echo "=== 2. YAML Syntax Validation ==="
    if [ -f ".github/workflows/rust.yml" ]; then
        validate_yaml ".github/workflows/rust.yml" "Main workflow"
    fi
    if [ -f ".github/workflows/release.yml" ]; then
        validate_yaml ".github/workflows/release.yml" "Release workflow"
    fi
    echo ""
    
    # 3. Check project structure
    echo "=== 3. Project Structure Validation ==="
    check_project_structure
    echo ""
    
    # 4. Check Rust configuration
    echo "=== 4. Rust Configuration Validation ==="
    check_rust_config
    echo ""
    
    # 5. Validate main workflow components
    if [ -f ".github/workflows/rust.yml" ]; then
        echo "=== 5. Main Workflow Component Validation ==="
        check_workflow_triggers ".github/workflows/rust.yml" "push:" "pull_request:"
        check_service_containers ".github/workflows/rust.yml"
        check_caching_config ".github/workflows/rust.yml"
        check_quality_checks ".github/workflows/rust.yml"
        check_parallel_execution ".github/workflows/rust.yml"
        check_performance_monitoring ".github/workflows/rust.yml"
        check_error_handling ".github/workflows/rust.yml"
        check_environment_variables ".github/workflows/rust.yml"
        echo ""
    fi
    
    # 6. Validate release workflow
    echo "=== 6. Release Workflow Validation ==="
    check_release_workflow
    echo ""
    
    # 7. Check documentation
    echo "=== 7. Documentation Validation ==="
    check_documentation
    echo ""
    
    # 8. Simulate workflow execution
    echo "=== 8. Workflow Simulation ==="
    simulate_workflow_steps
    echo ""
    
    # 9. Generate summary report
    echo "=== 9. Validation Summary ==="
    echo ""
    echo "Total checks performed: $TOTAL_CHECKS"
    echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
    echo -e "Failed: ${RED}$FAILED_CHECKS${NC}"
    echo -e "Warnings: ${YELLOW}$WARNING_CHECKS${NC}"
    echo ""
    
    # Calculate success rate
    if [ $TOTAL_CHECKS -gt 0 ]; then
        SUCCESS_RATE=$(( (PASSED_CHECKS * 100) / TOTAL_CHECKS ))
        echo "Success rate: $SUCCESS_RATE%"
        
        if [ $FAILED_CHECKS -eq 0 ]; then
            echo -e "${GREEN}✅ VALIDATION PASSED${NC}: All critical checks passed"
            if [ $WARNING_CHECKS -gt 0 ]; then
                echo -e "${YELLOW}⚠️  Note: $WARNING_CHECKS warnings detected - review recommended${NC}"
            fi
            exit 0
        else
            echo -e "${RED}❌ VALIDATION FAILED${NC}: $FAILED_CHECKS critical issues detected"
            echo "Please address the failed checks before proceeding"
            exit 1
        fi
    else
        echo -e "${RED}❌ VALIDATION ERROR${NC}: No checks were performed"
        exit 1
    fi
}

# Run main function
main "$@"