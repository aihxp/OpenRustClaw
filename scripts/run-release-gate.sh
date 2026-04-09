#!/bin/bash
# Run the MVP release gate against the shipped security, observability, and runtime-budget checks.

set -euo pipefail

source "$(dirname "$0")/use-local-tmp.sh"

if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: run from the repository root" >&2
    exit 1
fi

echo "[1/5] CLI security posture summary"
cargo test -p openrustclaw-cli posture_summary_reports_release_critical_fields -- --nocapture

echo "[2/5] Integration security posture summary"
cargo test -p openrustclaw-integration-tests security_posture_summary_surfaces_release_critical_fields -- --nocapture

echo "[3/5] E2E origin validation"
cargo test -p openrustclaw-e2e-tests test_origin_validation_rejects_invalid -- --nocapture

echo "[4/5] E2E metrics smoke"
cargo test -p openrustclaw-e2e-tests smoke_gateway_metrics_endpoint -- --nocapture

echo "[5/5] Runtime budgets"
bash scripts/check-runtime-budgets.sh

echo "[INFO] MVP release gate passed"
