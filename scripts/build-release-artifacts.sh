#!/bin/bash
# Build and package release artifacts for the OpenRustClaw CLI.

set -euo pipefail

if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: run from the repository root" >&2
    exit 1
fi

TARGET=""
OUTPUT_ROOT="dist"
BIN_NAME="openrustclaw"
PACKAGE_NAME="openrustclaw"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --output)
            OUTPUT_ROOT="$2"
            shift 2
            ;;
        -h|--help)
            cat <<'EOF'
Build and package release artifacts for the OpenRustClaw CLI.

Usage:
  scripts/build-release-artifacts.sh [--target <triple>] [--output <dir>]

Examples:
  scripts/build-release-artifacts.sh
  scripts/build-release-artifacts.sh --target aarch64-unknown-linux-gnu
  scripts/build-release-artifacts.sh --target x86_64-apple-darwin --output dist
EOF
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

if [[ -z "$TARGET" ]]; then
    TARGET="$(rustc -vV | sed -n 's/^host: //p')"
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$VERSION" ]]; then
    echo "Failed to detect workspace version from Cargo.toml" >&2
    exit 1
fi

BUILD_ARGS=(-p openrustclaw-cli --release --bin "$BIN_NAME")
if [[ -n "$TARGET" ]]; then
    BUILD_ARGS+=(--target "$TARGET")
fi

echo "[INFO] Building ${PACKAGE_NAME} ${VERSION} for ${TARGET}"
cargo build "${BUILD_ARGS[@]}"

BIN_PATH="target/${TARGET}/release/${BIN_NAME}"
if [[ "$TARGET" == "$(rustc -vV | sed -n 's/^host: //p')" ]]; then
    if [[ -f "target/release/${BIN_NAME}" ]]; then
        BIN_PATH="target/release/${BIN_NAME}"
    fi
fi

if [[ ! -f "$BIN_PATH" ]]; then
    echo "Built binary not found at ${BIN_PATH}" >&2
    exit 1
fi

STAGING_ROOT="${OUTPUT_ROOT}/${PACKAGE_NAME}-${VERSION}-${TARGET}"
ARCHIVE_PATH="${OUTPUT_ROOT}/${PACKAGE_NAME}-${VERSION}-${TARGET}.tar.gz"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"

rm -rf "$STAGING_ROOT"
mkdir -p "$STAGING_ROOT"

cp "$BIN_PATH" "${STAGING_ROOT}/${BIN_NAME}"
cp README.md "${STAGING_ROOT}/README.md"
if [[ -f "LICENSE" ]]; then
    cp LICENSE "${STAGING_ROOT}/LICENSE"
fi

tar -C "$OUTPUT_ROOT" -czf "$ARCHIVE_PATH" "$(basename "$STAGING_ROOT")"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$ARCHIVE_PATH" > "$CHECKSUM_PATH"
else
    shasum -a 256 "$ARCHIVE_PATH" > "$CHECKSUM_PATH"
fi

echo "[INFO] Artifact: ${ARCHIVE_PATH}"
echo "[INFO] Checksum: ${CHECKSUM_PATH}"
