#!/bin/bash
# Run cargo with repo-local temp and target directories.

set -euo pipefail

source "$(dirname "$0")/use-local-tmp.sh"

exec cargo "$@"
