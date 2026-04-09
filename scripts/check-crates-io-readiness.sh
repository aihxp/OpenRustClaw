#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/use-local-tmp.sh"

CRATE="${1:-openrustclaw-core}"

echo "==> Cargo metadata"
cargo metadata --no-deps --format-version 1 >/dev/null

echo "==> Package ${CRATE}"
cargo package -p "${CRATE}" --allow-dirty

echo "==> Rustdoc ${CRATE}"
cargo doc -p "${CRATE}" --no-deps

echo "==> Docs.rs-style rustdoc ${CRATE}"
RUSTDOCFLAGS="--cfg docsrs" cargo doc -p "${CRATE}" --no-deps

echo "==> Publish dry-run ${CRATE}"
cargo publish -p "${CRATE}" --dry-run --allow-dirty
