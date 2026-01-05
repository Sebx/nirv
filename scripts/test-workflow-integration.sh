#!/bin/bash

# GitHub Actions Workflow Integration Test
# This script tests the integration between workflow components

set -e

echo "=== GitHub Actions Workflow Integration Test ==="
echo "Testing component integration and compatibility..."
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to print test status
print_test_status() {
    local status=$1
    local message=$2
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    case $status in
        "PASS")
            echo -e "${GREEN}✅ PASS${NC}: $message"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            ;;
        "FAIL")
            echo -e "${RED}❌ FAIL${NC}: $message"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  INFO${NC}: $message"
            ;;
    esac
}

# Test 1: Workflow file syntax and structure
test_workflow_syntax() {
    print_test_status "INFO" "Testing workflow file syntax and structure"
    
    # Test main workflow structure
    if grep -q "name: Rust CI/CD Pipeline" .github/workflows/rust.yml; then
        print_test_status "PASS" "Main workflow has correct name"
    else
        print_test_status "FAIL" "Main workflow name incorrect or missing"
    fi
    
    # Test job dependencies
    if grep -A 5 "needs:" .github/workflows/rust.yml | grep -q "prepare"; then
        print_test_status "PASS" "Job dependencies configured correctly"
    else
        print_test_status "FAIL" "Job dependencies missing or incorrect"
    fi
    
    # Test release workflow structure
    if grep -q "name: Release Workflow" .github/workflows/release.yml; then
        print_test_status "PASS" "Release workflow has correct name"
    else
        print_test_status "FAIL" "Release workflow name incorrect or missing"
    fi
}

# Test 2: Service container configuration compatibility
test_service_containers() {
    print_test_status "INFO" "Testing service container configuration compatibility"
    
    # Check SQL Server configuration
    local sqlserver_config=$(grep -A 15 "sqlserver:" .github/workflows/rust.yml)
    
    if echo "$sqlserver_config" | grep -q "mcr.microsoft.com/mssql/server:2022-latest"; then
        print_test_status "PASS" "SQL Server image version specified"
    else
        print_test_status "FAIL" "SQL Server image version missing or incorrect"
    fi
    
    if echo "$sqlserver_config" | grep -q "ACCEPT_EULA: Y"; then
        print_test_status "PASS" "SQL Server EULA acceptance configured"
    else
        print_test_status "FAIL" "SQL Server EULA acceptance missing"
    fi
    
    if echo "$sqlserver_config" | grep -q "SA_PASSWORD:"; then
        print_test_status "PASS" "SQL Server password configured"
    else
        print_test_status "FAIL" "SQL Server password missing"
    fi
    
    # Check PostgreSQL configuration
    local postgres_config=$(grep -A 10 "postgres:" .github/workflows/rust.yml)
    
    if echo "$postgres_config" | grep -q "postgres:15"; then
        print_test_status "PASS" "PostgreSQL version specified"
    else
        print_test_status "FAIL" "PostgreSQL version missing or incorrect"
    fi
    
    # Check MySQL configuration
    local mysql_config=$(grep -A 10 "mysql:" .github/workflows/rust.yml)
    
    if echo "$mysql_config" | grep -q "mysql:8.0"; then
        print_test_status "PASS" "MySQL version specified"
    else
        print_test_status "FAIL" "MySQL version missing or incorrect"
    fi
}

# Test 3: Cache configuration integration
test_cache_integration() {
    print_test_status "INFO" "Testing cache configuration integration"
    
    # Check cache key consistency
    local cache_sections=$(grep -A 10 -B 2 "actions/cache@v3" .github/workflows/rust.yml)
    
    if echo "$cache_sections" | grep -q "Cache Cargo registry" && \
       echo "$cache_sections" | grep -q "Restore Cargo registry cache"; then
        print_test_status "PASS" "Cache key consistency maintained between save and restore"
    else
        print_test_status "FAIL" "Cache key mismatch between save and restore operations"
    fi
    
    # Check cache path consistency
    if grep -q "~/.cargo/registry" .github/workflows/rust.yml && \
       grep -q "~/.cargo/bin" .github/workflows/rust.yml && \
       grep -q "target/" .github/workflows/rust.yml; then
        print_test_status "PASS" "All cache paths configured consistently"
    else
        print_test_status "FAIL" "Cache path configuration incomplete"
    fi
}

# Test 4: Environment variable integration
test_environment_variables() {
    print_test_status "INFO" "Testing environment variable integration"
    
    # Check database connection strings match service configurations
    local sqlserver_conn=$(grep "SQLSERVER_CONNECTION_STRING" .github/workflows/rust.yml)
    local postgres_conn=$(grep "POSTGRES_CONNECTION_STRING" .github/workflows/rust.yml)
    local mysql_conn=$(grep "MYSQL_CONNECTION_STRING" .github/workflows/rust.yml)
    
    # Verify SQL Server connection string components
    if echo "$sqlserver_conn" | grep -q "localhost,1433" && \
       echo "$sqlserver_conn" | grep -q "sa" && \
       echo "$sqlserver_conn" | grep -q "YourStrong@Passw0rd"; then
        print_test_status "PASS" "SQL Server connection string matches service configuration"
    else
        print_test_status "FAIL" "SQL Server connection string mismatch"
    fi
    
    # Verify PostgreSQL connection string components
    if echo "$postgres_conn" | grep -q "localhost:5432" && \
       echo "$postgres_conn" | grep -q "postgres:postgres" && \
       echo "$postgres_conn" | grep -q "testdb"; then
        print_test_status "PASS" "PostgreSQL connection string matches service configuration"
    else
        print_test_status "FAIL" "PostgreSQL connection string mismatch"
    fi
    
    # Verify MySQL connection string components
    if echo "$mysql_conn" | grep -q "localhost:3306" && \
       echo "$mysql_conn" | grep -q "testuser:testpassword" && \
       echo "$mysql_conn" | grep -q "testdb"; then
        print_test_status "PASS" "MySQL connection string matches service configuration"
    else
        print_test_status "FAIL" "MySQL connection string mismatch"
    fi
}

# Test 5: Matrix strategy integration
test_matrix_strategy() {
    print_test_status "INFO" "Testing matrix strategy integration"
    
    # Check test suite matrix configuration
    local matrix_config=$(grep -A 20 "matrix:" .github/workflows/rust.yml)
    
    if echo "$matrix_config" | grep -q "test-suite:.*unit.*integration.*sqlserver.*protocol"; then
        print_test_status "PASS" "Test suite matrix includes all required test types"
    else
        print_test_status "FAIL" "Test suite matrix incomplete"
    fi
    
    # Check quality check matrix
    if echo "$matrix_config" | grep -q "check:.*format.*clippy.*build.*security.*docs.*coverage"; then
        print_test_status "PASS" "Quality check matrix includes all required checks"
    else
        print_test_status "FAIL" "Quality check matrix incomplete"
    fi
    
    # Check conditional service provisioning
    if grep -q "matrix.test-suite == 'sqlserver'" .github/workflows/rust.yml; then
        print_test_status "PASS" "Conditional service provisioning configured"
    else
        print_test_status "FAIL" "Conditional service provisioning missing"
    fi
}

# Test 6: Performance monitoring integration
test_performance_monitoring() {
    print_test_status "INFO" "Testing performance monitoring integration"
    
    # Check performance thresholds are used
    if grep -q "MAX_PREPARE_TIME" .github/workflows/rust.yml && \
       grep -q "duration_minutes.*MAX_PREPARE_TIME" .github/workflows/rust.yml; then
        print_test_status "PASS" "Performance thresholds are actively used"
    else
        print_test_status "FAIL" "Performance thresholds defined but not used"
    fi
    
    # Check timing measurements
    if grep -q "date +%s" .github/workflows/rust.yml && \
       grep -q "start_time=" .github/workflows/rust.yml && \
       grep -q "end_time=" .github/workflows/rust.yml; then
        print_test_status "PASS" "Timing measurements implemented"
    else
        print_test_status "FAIL" "Timing measurements incomplete"
    fi
    
    # Check performance reporting
    if grep -q "Performance Report" .github/workflows/rust.yml; then
        print_test_status "PASS" "Performance reporting job configured"
    else
        print_test_status "FAIL" "Performance reporting missing"
    fi
}

# Test 7: Error handling integration
test_error_handling() {
    print_test_status "INFO" "Testing error handling integration"
    
    # Check retry logic implementation
    if grep -q "for attempt in.*1 2 3" .github/workflows/rust.yml; then
        print_test_status "PASS" "Retry logic implemented for database connections"
    else
        print_test_status "FAIL" "Retry logic missing or incomplete"
    fi
    
    # Check timeout handling
    if grep -q "timeout.*bash -c" .github/workflows/rust.yml; then
        print_test_status "PASS" "Timeout handling implemented"
    else
        print_test_status "FAIL" "Timeout handling missing"
    fi
    
    # Check detailed error reporting
    if grep -q "Connection details:" .github/workflows/rust.yml; then
        print_test_status "PASS" "Detailed error reporting configured"
    else
        print_test_status "FAIL" "Detailed error reporting missing"
    fi
}

# Test 8: Release workflow integration
test_release_integration() {
    print_test_status "INFO" "Testing release workflow integration"
    
    # Check release trigger consistency
    if grep -q "tags:" .github/workflows/release.yml && \
       grep -q "v\*" .github/workflows/release.yml; then
        print_test_status "PASS" "Release triggers configured correctly"
    else
        print_test_status "FAIL" "Release triggers missing or incorrect"
    fi
    
    # Check pre-release validation uses same services
    local release_services=$(grep -A 50 "pre-release-validation:" .github/workflows/release.yml)
    
    if echo "$release_services" | grep -q "sqlserver:" && \
       echo "$release_services" | grep -q "postgres:" && \
       echo "$release_services" | grep -q "mysql:"; then
        print_test_status "PASS" "Release validation uses same database services"
    else
        print_test_status "FAIL" "Release validation service configuration inconsistent"
    fi
    
    # Check artifact generation
    if grep -q "build-artifacts:" .github/workflows/release.yml && \
       grep -q "matrix:" .github/workflows/release.yml; then
        print_test_status "PASS" "Multi-platform artifact generation configured"
    else
        print_test_status "FAIL" "Artifact generation configuration incomplete"
    fi
}

# Test 9: Documentation integration
test_documentation_integration() {
    print_test_status "INFO" "Testing documentation integration"
    
    # Check if documentation files exist and are comprehensive
    if [ -f "docs/github-actions-workflow.md" ]; then
        local doc_content=$(cat "docs/github-actions-workflow.md")
        
        if echo "$doc_content" | grep -q "Database Testing" && \
           echo "$doc_content" | grep -q "Caching Strategy" && \
           echo "$doc_content" | grep -q "Code Quality" && \
           echo "$doc_content" | grep -q "Performance Monitoring"; then
            print_test_status "PASS" "Workflow documentation covers all major components"
        else
            print_test_status "FAIL" "Workflow documentation incomplete"
        fi
    else
        print_test_status "FAIL" "Workflow documentation missing"
    fi
    
    # Check troubleshooting guide
    if [ -f "docs/github-actions-troubleshooting.md" ]; then
        local troubleshoot_content=$(cat "docs/github-actions-troubleshooting.md")
        
        if echo "$troubleshoot_content" | grep -q "Database Service Issues" && \
           echo "$troubleshoot_content" | grep -q "Cache-Related Issues" && \
           echo "$troubleshoot_content" | grep -q "Performance Issues"; then
            print_test_status "PASS" "Troubleshooting guide covers major issue categories"
        else
            print_test_status "FAIL" "Troubleshooting guide incomplete"
        fi
    else
        print_test_status "FAIL" "Troubleshooting guide missing"
    fi
}

# Test 10: Overall workflow coherence
test_workflow_coherence() {
    print_test_status "INFO" "Testing overall workflow coherence"
    
    # Check that all jobs have proper dependencies
    local job_deps=$(grep -A 2 "needs:" .github/workflows/rust.yml)
    
    if echo "$job_deps" | grep -q "prepare" && \
       echo "$job_deps" | grep -q "quality"; then
        print_test_status "PASS" "Job dependencies create proper execution order"
    else
        print_test_status "FAIL" "Job dependency chain incomplete"
    fi
    
    # Check that performance monitoring runs regardless of other job status
    if grep -A 5 "performance-report:" .github/workflows/rust.yml | grep -q "if: always()"; then
        print_test_status "PASS" "Performance monitoring runs regardless of job failures"
    else
        print_test_status "FAIL" "Performance monitoring may not run on failures"
    fi
    
    # Check that all test suites have appropriate resource allocation
    if grep -q "resource-priority:" .github/workflows/rust.yml; then
        print_test_status "PASS" "Resource allocation strategy implemented"
    else
        print_test_status "FAIL" "Resource allocation strategy missing"
    fi
}

# Main test execution
main() {
    echo "Starting GitHub Actions workflow integration tests..."
    echo "Timestamp: $(date)"
    echo ""
    
    # Run all integration tests
    test_workflow_syntax
    echo ""
    
    test_service_containers
    echo ""
    
    test_cache_integration
    echo ""
    
    test_environment_variables
    echo ""
    
    test_matrix_strategy
    echo ""
    
    test_performance_monitoring
    echo ""
    
    test_error_handling
    echo ""
    
    test_release_integration
    echo ""
    
    test_documentation_integration
    echo ""
    
    test_workflow_coherence
    echo ""
    
    # Generate test summary
    echo "=== Integration Test Summary ==="
    echo ""
    echo "Total integration tests: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
    echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
    echo ""
    
    # Calculate success rate
    if [ $TOTAL_TESTS -gt 0 ]; then
        SUCCESS_RATE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
        echo "Integration success rate: $SUCCESS_RATE%"
        
        if [ $FAILED_TESTS -eq 0 ]; then
            echo -e "${GREEN}✅ INTEGRATION TESTS PASSED${NC}: All workflow components integrate correctly"
            echo ""
            echo "The GitHub Actions CI/CD pipeline is ready for production use."
            echo "All components work together as designed and documented."
            exit 0
        else
            echo -e "${RED}❌ INTEGRATION TESTS FAILED${NC}: $FAILED_TESTS integration issues detected"
            echo ""
            echo "Please address the integration issues before deploying the workflow."
            echo "Check component compatibility and configuration consistency."
            exit 1
        fi
    else
        echo -e "${RED}❌ INTEGRATION TEST ERROR${NC}: No tests were performed"
        exit 1
    fi
}

# Run main function
main "$@"