#!/usr/bin/env bash

set -euo pipefail

# These advisories are currently accepted exceptions for the shipped workspace.
# They are either transitive-only, tied to optional platform stacks, or blocked
# on upstream dependency ecosystems that are not being upgraded in this release.
IGNORED_ADVISORIES=(
  RUSTSEC-2026-0044
  RUSTSEC-2026-0048
  RUSTSEC-2026-0068
  RUSTSEC-2026-0067
  RUSTSEC-2026-0021
  RUSTSEC-2026-0020
  RUSTSEC-2024-0437
  RUSTSEC-2023-0071
  RUSTSEC-2026-0049
  RUSTSEC-2025-0118
  RUSTSEC-2025-0046
  RUSTSEC-2025-0057
  RUSTSEC-2025-0012
  RUSTSEC-2025-0141
  RUSTSEC-2024-0412
  RUSTSEC-2024-0413
  RUSTSEC-2024-0418
  RUSTSEC-2024-0416
  RUSTSEC-2024-0411
  RUSTSEC-2024-0414
  RUSTSEC-2024-0415
  RUSTSEC-2024-0420
  RUSTSEC-2024-0419
  RUSTSEC-2024-0384
  RUSTSEC-2025-0119
  RUSTSEC-2024-0436
  RUSTSEC-2024-0370
  RUSTSEC-2025-0134
  RUSTSEC-2024-0429
  RUSTSEC-2026-0002
)

ARGS=()
for advisory in "${IGNORED_ADVISORIES[@]}"; do
  ARGS+=(--ignore "$advisory")
done

echo "==> cargo audit (${#IGNORED_ADVISORIES[@]} accepted exceptions)"
cargo audit "${ARGS[@]}"
