#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
METADATA_FILE="${ROOT_DIR}/.github/repository-metadata.json"
README_FILE="${ROOT_DIR}/README.md"
DOC_FILE="${ROOT_DIR}/docs/github-repo-admin.md"

usage() {
  cat <<'EOF'
Usage:
  scripts/github-repo-admin.sh show-desired
  scripts/github-repo-admin.sh validate-local
  scripts/github-repo-admin.sh show-live
  scripts/github-repo-admin.sh check-live
  scripts/github-repo-admin.sh apply-live

Environment:
  GITHUB_TOKEN or GH_TOKEN  GitHub token with repo administration access
EOF
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "Missing required file: $path" >&2
    exit 1
  fi
}

json_field() {
  local expr="$1"
  python3 - "$METADATA_FILE" "$expr" <<'PY'
import json, sys
path, expr = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
value = data
for part in expr.split("."):
    value = value[part]
if isinstance(value, list):
    print("\n".join(value))
else:
    print(value)
PY
}

validate_token() {
  local token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
  if [[ -z "$token" ]]; then
    echo "Missing GITHUB_TOKEN/GH_TOKEN. Live GitHub operations are unavailable." >&2
    exit 1
  fi
  local code
  code=$(curl -sS -o /tmp/openrustclaw-github-auth-check.json -w '%{http_code}' \
    -H "Authorization: Bearer ${token}" \
    -H "Accept: application/vnd.github+json" \
    https://api.github.com/user)
  if [[ "$code" != "200" ]]; then
    echo "GitHub token is invalid for live repo operations (HTTP ${code})." >&2
    sed -n '1,40p' /tmp/openrustclaw-github-auth-check.json >&2 || true
    exit 1
  fi
}

api_get() {
  local path="$1"
  curl -sS \
    -H "Authorization: Bearer ${GITHUB_TOKEN:-${GH_TOKEN:-}}" \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com${path}"
}

api_patch() {
  local path="$1"
  local payload="$2"
  curl -sS -X PATCH \
    -H "Authorization: Bearer ${GITHUB_TOKEN:-${GH_TOKEN:-}}" \
    -H "Accept: application/vnd.github+json" \
    -H "Content-Type: application/json" \
    "https://api.github.com${path}" \
    -d "$payload"
}

api_put() {
  local path="$1"
  local payload="$2"
  curl -sS -X PUT \
    -H "Authorization: Bearer ${GITHUB_TOKEN:-${GH_TOKEN:-}}" \
    -H "Accept: application/vnd.github+json" \
    -H "Content-Type: application/vnd.github+json" \
    "https://api.github.com${path}" \
    -d "$payload"
}

show_desired() {
  require_file "$METADATA_FILE"
  cat "$METADATA_FILE"
}

validate_local() {
  require_file "$METADATA_FILE"
  require_file "$README_FILE"
  require_file "$DOC_FILE"

  python3 - "$METADATA_FILE" <<'PY'
import json, re, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
for key in ("repo", "description", "homepage", "topics"):
    if key not in data:
        raise SystemExit(f"Missing metadata key: {key}")
if not data["repo"].count("/") == 1:
    raise SystemExit("repo must be in owner/name format")
if not data["description"].strip():
    raise SystemExit("description must be non-empty")
if not data["homepage"].startswith("https://"):
    raise SystemExit("homepage must be https")
topics = data["topics"]
if not topics:
    raise SystemExit("topics must be non-empty")
for topic in topics:
    if not re.fullmatch(r"[a-z0-9-]+", topic):
        raise SystemExit(f"invalid topic format: {topic}")
print("metadata schema ok")
PY

  rg -n "github-repo-admin.md" "$README_FILE" >/dev/null
  rg -n "repository-metadata.json|github-repo-admin.sh" "$DOC_FILE" >/dev/null
  echo "local repo-admin contract ok"
}

show_live() {
  validate_token
  local repo
  repo=$(json_field repo)
  api_get "/repos/${repo}" | python3 - <<'PY'
import json, sys
repo = json.load(sys.stdin)
print(json.dumps({
    "full_name": repo["full_name"],
    "description": repo["description"],
    "homepage": repo["homepage"],
    "topics": repo.get("topics", []),
}, indent=2))
PY
}

check_live() {
  validate_token
  local repo desired_desc desired_home
  repo=$(json_field repo)
  desired_desc=$(json_field description)
  desired_home=$(json_field homepage)
  local live_repo live_topics
  live_repo=$(mktemp)
  live_topics=$(mktemp)
  api_get "/repos/${repo}" >"$live_repo"
  api_get "/repos/${repo}/topics" >"$live_topics"
  python3 - "$live_repo" "$live_topics" "$METADATA_FILE" <<'PY'
import json, sys
repo_path, topics_path, desired_path = sys.argv[1:4]
with open(repo_path, "r", encoding="utf-8") as fh:
    repo = json.load(fh)
with open(topics_path, "r", encoding="utf-8") as fh:
    live_topics = json.load(fh).get("names", [])
with open(desired_path, "r", encoding="utf-8") as fh:
    desired = json.load(fh)
errors = []
if repo.get("description") != desired["description"]:
    errors.append(f"description drift: {repo.get('description')!r}")
if (repo.get("homepage") or "") != desired["homepage"]:
    errors.append(f"homepage drift: {repo.get('homepage')!r}")
if sorted(live_topics) != sorted(desired["topics"]):
    errors.append(f"topic drift: live={sorted(live_topics)} desired={sorted(desired['topics'])}")
if errors:
    print("\n".join(errors))
    raise SystemExit(1)
print("live GitHub repo surface matches desired metadata")
PY
}

apply_live() {
  validate_token
  local repo desc home topics_json payload
  repo=$(json_field repo)
  desc=$(json_field description)
  home=$(json_field homepage)
  topics_json=$(python3 - "$METADATA_FILE" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
print(json.dumps({"names": data["topics"]}))
PY
)
  payload=$(python3 - "$METADATA_FILE" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
print(json.dumps({
    "name": data["repo"].split("/", 1)[1],
    "description": data["description"],
    "homepage": data["homepage"]
}))
PY
)
  api_patch "/repos/${repo}" "${payload}" >/tmp/openrustclaw-github-repo-update.json
  api_put "/repos/${repo}/topics" "${topics_json}" >/tmp/openrustclaw-github-topics-update.json
  echo "applied desired GitHub repo metadata and topics"
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    show-desired) show_desired ;;
    validate-local) validate_local ;;
    show-live) show_live ;;
    check-live) check_live ;;
    apply-live) apply_live ;;
    *) usage; exit 1 ;;
  esac
}

main "$@"
