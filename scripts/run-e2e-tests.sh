#!/bin/bash
# OpenRustClaw E2E Test Runner
# Usage: ./scripts/run-e2e-tests.sh [smoke|horizontal|vertical|regression|all]

set -e

source "$(dirname "$0")/use-local-tmp.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default test type
TEST_TYPE="${1:-smoke}"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from project root${NC}"
    exit 1
fi

# Function to print header
print_header() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

# Function to run tests
run_tests() {
    local filter="$1"
    local name="$2"
    
    print_header "Running $name Tests"
    
    if [ -n "$filter" ]; then
        cargo test --test e2e "$filter" --release -- --nocapture
    else
        cargo test --test e2e --release -- --nocapture
    fi
}

# Main execution
case "$TEST_TYPE" in
    smoke|s)
        print_header "SMOKE TESTS - Critical Path Only (~60 seconds)"
        run_tests "smoke" "Smoke"
        ;;
    
    horizontal|h)
        print_header "HORIZONTAL TESTS - Full User Journeys (~5-10 minutes)"
        run_tests "horizontal" "Horizontal"
        ;;
    
    vertical|v)
        print_header "VERTICAL TESTS - Layer-Specific Tests (~3-5 minutes)"
        run_tests "vertical" "Vertical"
        ;;
    
    regression|r)
        print_header "REGRESSION TESTS - Full Comprehensive Suite (~15-20 minutes)"
        run_tests "regression" "Regression"
        ;;
    
    all|a)
        print_header "ALL E2E TESTS - Complete Suite"
        run_tests "" "All"
        ;;
    
    live|l)
        print_header "LIVE PROVIDER TESTS - Requires API Keys"
        if [ -z "$E2E_LIVE" ]; then
            echo -e "${YELLOW}Warning: E2E_LIVE not set. Set to 1 to run live tests.${NC}"
        fi
        run_tests "test_live" "Live Provider"
        ;;
    
    *)
        echo "Usage: $0 [smoke|horizontal|vertical|regression|all|live]"
        echo ""
        echo "Test Types:"
        echo "  smoke      - Critical path tests only (~60 seconds)"
        echo "  horizontal - Full user journeys (~5-10 minutes)"
        echo "  vertical   - Layer-specific tests (~3-5 minutes)"
        echo "  regression - Full comprehensive suite (~15-20 minutes)"
        echo "  all        - Run all E2E tests"
        echo "  live       - Live provider tests (requires API keys)"
        echo ""
        echo "Environment Variables:"
        echo "  E2E_LIVE=1          - Enable live provider tests"
        echo "  E2E_TIMEOUT_SECS    - Test timeout (default: 300)"
        echo "  OPENAI_API_KEY      - OpenAI API key"
        echo "  ANTHROPIC_API_KEY   - Anthropic API key"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ E2E Tests Complete${NC}"
