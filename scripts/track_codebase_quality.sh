#!/bin/bash
# Track OpenRustClaw Codebase Quality Metrics
# Usage: ./scripts/track_codebase_quality.sh

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║     OpenRustClaw Codebase Quality Tracker v1.0              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Output file
REPORT_FILE="codebase_quality_report.md"
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

# Function to count unwrap occurrences
count_unwraps() {
    local crate=$1
    if [ "$crate" = "workspace" ]; then
        grep -r "\.unwrap()" --include="*.rs" crates/ 2>/dev/null | wc -l
    else
        grep -r "\.unwrap()" --include="*.rs" "crates/$crate/src/" 2>/dev/null | wc -l
    fi
}

# Function to count tests
count_tests() {
    local crate=$1
    if [ "$crate" = "workspace" ]; then
        grep -r "#\[test\]" --include="*.rs" crates/ 2>/dev/null | wc -l
    else
        grep -r "#\[test\]" --include="*.rs" "crates/$crate/src/" 2>/dev/null | wc -l
    fi
}

# Function to count TODOs
count_todos() {
    local crate=$1
    if [ "$crate" = "workspace" ]; then
        grep -r "TODO\|FIXME\|XXX\|HACK" --include="*.rs" crates/ 2>/dev/null | wc -l
    else
        grep -r "TODO\|FIXME\|XXX\|HACK" --include="*.rs" "crates/$crate/src/" 2>/dev/null | wc -l
    fi
}

# Function to count lines of code
count_loc() {
    local crate=$1
    if [ "$crate" = "workspace" ]; then
        find crates/ -name "*.rs" -type f -exec cat {} \; 2>/dev/null | wc -l
    else
        find "crates/$crate/src/" -name "*.rs" -type f -exec cat {} \; 2>/dev/null | wc -l
    fi
}

# Function to check if crate has documentation
check_docs() {
    local crate=$1
    if [ -f "crates/$crate/README.md" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Function to get build status
get_build_status() {
    echo "Checking build status..."
    if cargo check --workspace 2>/dev/null; then
        echo "PASS"
    else
        echo "FAIL"
    fi
}

# Function to get clippy warnings
get_clippy_warnings() {
    cargo clippy --workspace --message-format=short 2>&1 | grep "^warning:" | wc -l
}

# Generate report
echo "Generating report..."
{
    echo "# OpenRustClaw Codebase Quality Report"
    echo ""
    echo "**Generated:** $TIMESTAMP"
    echo ""
    echo "## 📊 Overall Metrics"
    echo ""
    echo "| Metric | Value | Status |"
    echo "|--------|-------|--------|"
    
    TOTAL_UNWRAP=$(count_unwraps workspace)
    TOTAL_TESTS=$(count_tests workspace)
    TOTAL_TODOS=$(count_todos workspace)
    TOTAL_LOC=$(count_loc workspace)
    
    # Build status
    BUILD_STATUS=$(cargo check --workspace 2>&1 >/dev/null && echo "✅ PASS" || echo "❌ FAIL")
    echo "| Build | - | $BUILD_STATUS |"
    
    # Unwrap count
    if [ $TOTAL_UNWRAP -lt 50 ]; then
        UNWRAP_STATUS="✅"
    elif [ $TOTAL_UNWRAP -lt 200 ]; then
        UNWRAP_STATUS="⚠️"
    else
        UNWRAP_STATUS="❌"
    fi
    echo "| Total unwrap() | $TOTAL_UNWRAP | $UNWRAP_STATUS |"
    
    # Test count
    echo "| Total tests | $TOTAL_TESTS | - |"
    
    # TODO count
    if [ $TOTAL_TODOS -lt 20 ]; then
        TODO_STATUS="✅"
    elif [ $TOTAL_TODOS -lt 50 ]; then
        TODO_STATUS="⚠️"
    else
        TODO_STATUS="❌"
    fi
    echo "| TODO/FIXME count | $TOTAL_TODOS | $TODO_STATUS |"
    
    # Lines of code
    echo "| Lines of code | $TOTAL_LOC | - |"
    
    echo ""
    echo "## 📦 Per-Crate Breakdown"
    echo ""
    echo "| Crate | Lines | unwrap() | Tests | TODOs | README |"
    echo "|-------|-------|----------|-------|-------|--------|"
    
    for crate in core security channels gateway agent distributed mobile wasm; do
        if [ -d "crates/$crate" ]; then
            LOC=$(count_loc $crate)
            UNWRAP=$(count_unwraps $crate)
            TESTS=$(count_tests $crate)
            TODOS=$(count_todos $crate)
            DOCS=$(check_docs $crate)
            echo "| $crate | $LOC | $UNWRAP | $TESTS | $TODOS | $DOCS |"
        fi
    done
    
    echo ""
    echo "## 🎯 Scoring Guide"
    echo ""
    echo "### Current vs Target (10/10)"
    echo ""
    echo "| Category | Current | Target | Gap |"
    echo "|----------|---------|--------|-----|"
    
    # Architecture (subjective - would need manual review)
    echo "| Architecture | ~8/10 | 10/10 | 2 |"
    
    # Code Quality (based on unwrap count)
    if [ $TOTAL_UNWRAP -lt 50 ]; then
        CODE_QUALITY="9/10"
    elif [ $TOTAL_UNWRAP -lt 100 ]; then
        CODE_QUALITY="7/10"
    elif [ $TOTAL_UNWRAP -lt 300 ]; then
        CODE_QUALITY="5/10"
    else
        CODE_QUALITY="3/10"
    fi
    echo "| Code Quality | $CODE_QUALITY | 10/10 | - |"
    
    # Security (subjective)
    echo "| Security | ~8/10 | 10/10 | 2 |"
    
    # Testing (based on test density)
    TEST_RATIO=$((TOTAL_TESTS * 1000 / TOTAL_LOC))
    if [ $TEST_RATIO -gt 50 ]; then
        TESTING="9/10"
    elif [ $TEST_RATIO -gt 30 ]; then
        TESTING="7/10"
    else
        TESTING="5/10"
    fi
    echo "| Testing | $TESTING | 10/10 | - |"
    
    # Documentation (based on README presence)
    DOCS_CRATES=0
    TOTAL_CRATES=0
    for crate in core security channels gateway agent distributed mobile wasm; do
        if [ -d "crates/$crate" ]; then
            TOTAL_CRATES=$((TOTAL_CRATES + 1))
            if [ -f "crates/$crate/README.md" ]; then
                DOCS_CRATES=$((DOCS_CRATES + 1))
            fi
        fi
    done
    DOC_PCT=$((DOCS_CRATES * 100 / TOTAL_CRATES))
    if [ $DOC_PCT -eq 100 ]; then
        DOCUMENTATION="9/10"
    elif [ $DOC_PCT -gt 50 ]; then
        DOCUMENTATION="7/10"
    else
        DOCUMENTATION="5/10"
    fi
    echo "| Documentation | $DOCUMENTATION | 10/10 | - |"
    
    # Build Status
    if [ "$BUILD_STATUS" = "✅ PASS" ]; then
        BUILD_SCORE="10/10"
    else
        BUILD_SCORE="3/10"
    fi
    echo "| Build Status | $BUILD_SCORE | 10/10 | - |"
    
    echo ""
    echo "## 🔍 Detailed Findings"
    echo ""
    
    # Top unwrap offenders
    echo "### Top Files by unwrap() Count"
    echo ""
    echo "| File | Count |"
    echo "|------|-------|"
    
    for file in $(find crates/ -name "*.rs" -type f 2>/dev/null); do
        count=$(grep -c "\.unwrap()" "$file" 2>/dev/null || echo 0)
        if [ "$count" -gt 5 ]; then
            echo "| $file | $count |"
        fi
    done | sort -t'|' -k3 -nr | head -20
    
    echo ""
    echo "### Security TODOs"
    echo ""
    grep -rn "TODO.*secur\|TODO.*auth\|TODO.*valid" --include="*.rs" crates/ 2>/dev/null | head -10 || echo "None found"
    
    echo ""
    echo "### Architecture TODOs"
    echo ""
    grep -rn "TODO.*impl\|TODO.*implement\|TODO.*fix" --include="*.rs" crates/ 2>/dev/null | head -10 || echo "None found"
    
    echo ""
    echo "---"
    echo ""
    echo "*Report generated by OpenRustClaw Quality Tracker*"
    
} > "$REPORT_FILE"

echo ""
echo "✅ Report generated: $REPORT_FILE"
echo ""
echo "Quick summary:"
echo "  - Total unwrap(): $TOTAL_UNWRAP"
echo "  - Total tests: $TOTAL_TESTS"
echo "  - Total TODOs: $TOTAL_TODOS"
echo "  - Lines of code: $TOTAL_LOC"
echo ""

# Print a preview
echo "Report preview:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
head -50 "$REPORT_FILE"
echo "..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
