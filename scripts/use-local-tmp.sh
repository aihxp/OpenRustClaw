#!/bin/bash
# Configure repo-local temp and target directories for heavy Rust workflows.

if [[ -n "${OPENRUSTCLAW_LOCAL_TMP_READY:-}" ]]; then
    return 0 2>/dev/null || exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

export TMPDIR="${TMPDIR:-${REPO_ROOT}/target/rustc-tmp}"
export TMP="${TMP:-${TMPDIR}}"
export TEMP="${TEMP:-${TMPDIR}}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${REPO_ROOT}/target}"

mkdir -p "${TMPDIR}" "${CARGO_TARGET_DIR}"
export OPENRUSTCLAW_LOCAL_TMP_READY=1
