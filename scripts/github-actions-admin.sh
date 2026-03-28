#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
REPO_DEFAULT="aihxp/OpenRustClaw"
REPO="${GITHUB_REPO:-$REPO_DEFAULT}"

usage() {
  cat <<'EOF'
Usage:
  scripts/github-actions-admin.sh workflows
  scripts/github-actions-admin.sh recent-runs
  scripts/github-actions-admin.sh check-main-ci

Environment:
  GITHUB_REPO              Optional owner/name override (default: aihxp/OpenRustClaw)
  GH_TOKEN                 Optional GitHub token override
  GITHUB_TOKEN             Optional fallback token if no gh keyring token exists
EOF
}

gh_cmd() {
  env -u GITHUB_TOKEN gh "$@"
}

workflows() {
  local payload
  payload=$(mktemp)
  gh_cmd api "repos/${REPO}/actions/workflows" >"$payload"
  python3 - "$payload" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    print(json.dumps(json.load(fh), indent=2))
PY
}

recent_runs() {
  gh_cmd run list --repo "${REPO}" --limit 12 \
    --json databaseId,workflowName,displayTitle,headBranch,status,conclusion,url,createdAt,updatedAt
}

check_main_ci() {
  local runs_json
  runs_json=$(gh_cmd run list --repo "${REPO}" --limit 20 \
    --json workflowName,headBranch,status,conclusion,url,displayTitle,createdAt)
  local payload
  payload=$(mktemp)
  printf '%s\n' "$runs_json" >"$payload"
  python3 - "$payload" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    runs = json.load(fh)
targets = {
    "Shipped Surface CI": None,
    "Shipped Surface E2E Tests": None,
}
for run in runs:
    if run["headBranch"] != "main":
        continue
    name = run["workflowName"]
    if name in targets and targets[name] is None:
        targets[name] = run
missing = [name for name, run in targets.items() if run is None]
if missing:
    raise SystemExit(f"missing recent main runs for: {', '.join(missing)}")
for name, run in targets.items():
    if run["status"] != "completed":
        raise SystemExit(f"{name} still in progress: {run['url']}")
    if run["conclusion"] not in ("success", "skipped"):
        raise SystemExit(f"{name} not green: {run['conclusion']} {run['url']}")
print("main workflow health ok")
for name, run in targets.items():
    print(f"- {name}: {run['conclusion']} ({run['url']})")
PY
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    workflows) workflows ;;
    recent-runs) recent_runs ;;
    check-main-ci) check_main_ci ;;
    *) usage; exit 1 ;;
  esac
}

main "$@"
