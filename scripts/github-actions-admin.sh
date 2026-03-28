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
  scripts/github-actions-admin.sh check-release-binaries [ref]

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

check_release_binaries() {
  local ref_filter="${1:-}"
  local runs_json
  runs_json=$(gh_cmd run list --repo "${REPO}" --workflow release-binaries.yml --limit 20 \
    --json databaseId,workflowName,displayTitle,headBranch,status,conclusion,url,createdAt,event)
  local payload
  payload=$(mktemp)
  printf '%s\n' "$runs_json" >"$payload"
  local run_id
  run_id=$(python3 - "$payload" "$ref_filter" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    runs = json.load(fh)
ref = sys.argv[2]
target = None
for run in runs:
    if ref and run["headBranch"] != ref:
        continue
    target = run
    break
if target is None:
    msg = f"no recent Release Binaries run found for ref {ref}" if ref else "no recent Release Binaries runs found"
    raise SystemExit(msg)
print(target["databaseId"])
PY
)

  local run_json
  run_json=$(gh_cmd run view "$run_id" --repo "${REPO}" \
    --json databaseId,displayTitle,headBranch,event,status,conclusion,url,jobs)
  local run_payload
  run_payload=$(mktemp)
  printf '%s\n' "$run_json" >"$run_payload"
  python3 - "$run_payload" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    run = json.load(fh)
if run["status"] != "completed":
    raise SystemExit(f"Release Binaries still in progress: {run['url']}")
if run["conclusion"] not in ("success", "skipped"):
    raise SystemExit(f"Release Binaries run not green: {run['conclusion']} {run['url']}")
jobs = run.get("jobs") or []
build_jobs = [job for job in jobs if job["name"].startswith("Build ")]
if not build_jobs:
    raise SystemExit(f"Release Binaries run missing build jobs: {run['url']}")
failed_builds = [job for job in build_jobs if job["conclusion"] != "success"]
if failed_builds:
    names = ", ".join(job["name"] for job in failed_builds)
    raise SystemExit(f"Release Binaries build jobs not green: {names} ({run['url']})")
publish_jobs = [job for job in jobs if job["name"] == "Publish GitHub Release Assets"]
is_tag = str(run.get("headBranch", "")).startswith("v")
if is_tag:
    if not publish_jobs:
        raise SystemExit(f"Release Binaries tag run missing publish job: {run['url']}")
    publish = publish_jobs[0]
    if publish["conclusion"] != "success":
        raise SystemExit(f"Release publish job not green: {publish['conclusion']} {run['url']}")
else:
    if publish_jobs:
        publish = publish_jobs[0]
        if publish["conclusion"] not in ("skipped", "success"):
            raise SystemExit(f"Unexpected publish job state on non-tag run: {publish['conclusion']} {run['url']}")
print("release binaries workflow ok")
print(f"- run: {run['displayTitle']} ({run['url']})")
for job in build_jobs:
    print(f"- {job['name']}: {job['conclusion']}")
if publish_jobs:
    print(f"- Publish GitHub Release Assets: {publish_jobs[0]['conclusion']}")
PY
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    workflows) workflows ;;
    recent-runs) recent_runs ;;
    check-main-ci) check_main_ci ;;
    check-release-binaries) shift; check_release_binaries "${1:-}" ;;
    *) usage; exit 1 ;;
  esac
}

main "$@"
