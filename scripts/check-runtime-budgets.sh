#!/bin/bash
# Check bounded binary/runtime budgets for the shipped CLI.

set -euo pipefail

if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: run from the repository root" >&2
    exit 1
fi

MAX_BINARY_SIZE_BYTES="${MAX_BINARY_SIZE_BYTES:-83886080}"
MAX_STARTUP_LATENCY_MS="${MAX_STARTUP_LATENCY_MS:-1500}"
MAX_IDLE_RSS_KB="${MAX_IDLE_RSS_KB:-262144}"
STARTUP_TIMEOUT_SECS="${STARTUP_TIMEOUT_SECS:-20}"

BIN_PATH="target/release/openrustclaw"

if [[ ! -x "$BIN_PATH" || "${FORCE_REBUILD:-0}" == "1" ]]; then
    echo "[INFO] Building release binary"
    cargo build -p openrustclaw-cli --release --bin openrustclaw --quiet
else
    echo "[INFO] Reusing existing release binary at ${BIN_PATH}"
fi

binary_size_bytes="$(stat -c%s "$BIN_PATH")"
start_ns="$(date +%s%N)"
"$BIN_PATH" --help >/dev/null
end_ns="$(date +%s%N)"
startup_latency_ms="$(((end_ns - start_ns) / 1000000))"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
cp -R config "$tmpdir/"

port=""
for candidate in $(shuf -i 30000-45000 -n 20); do
    if ! ss -ltn "sport = :${candidate}" | tail -n +2 | grep -q .; then
        port="$candidate"
        break
    fi
done

if [[ -z "$port" ]]; then
    echo "Failed to select a free TCP port for runtime budget checks" >&2
    exit 1
fi

perl -0pi -e "s/port = 18789/port = ${port}/" "$tmpdir/config/default.toml"

echo "[INFO] Measuring startup latency and idle RSS on port ${port}"
(
    cd "$tmpdir"
    "$OLDPWD/$BIN_PATH" start --config config/default.toml >"$tmpdir/runtime.out" 2>"$tmpdir/runtime.err"
) &
runtime_pid=$!

bind_detected=false
for _ in $(seq 1 $((STARTUP_TIMEOUT_SECS * 10))); do
    if ! ps -p "$runtime_pid" >/dev/null 2>&1; then
        break
    fi
    if ss -ltn "sport = :${port}" | tail -n +2 | grep -q .; then
        bind_detected=true
        break
    fi
    sleep 0.1
done

if [[ "$bind_detected" != true ]]; then
    echo "Runtime failed to bind within ${STARTUP_TIMEOUT_SECS}s" >&2
    sed -n '1,120p' "$tmpdir/runtime.err" >&2 || true
    kill "$runtime_pid" >/dev/null 2>&1 || true
    wait "$runtime_pid" >/dev/null 2>&1 || true
    exit 1
fi

idle_rss_kb="$(ps -o rss= -p "$runtime_pid" | tr -d ' ')"
kill "$runtime_pid" >/dev/null 2>&1 || true
wait "$runtime_pid" >/dev/null 2>&1 || true

echo "[INFO] release_binary_size_bytes=${binary_size_bytes}"
echo "[INFO] cli_startup_latency_ms=${startup_latency_ms}"
echo "[INFO] runtime_idle_rss_kb=${idle_rss_kb}"

if (( binary_size_bytes > MAX_BINARY_SIZE_BYTES )); then
    echo "Binary size budget exceeded: ${binary_size_bytes} > ${MAX_BINARY_SIZE_BYTES}" >&2
    exit 1
fi

if (( startup_latency_ms > MAX_STARTUP_LATENCY_MS )); then
    echo "Startup latency budget exceeded: ${startup_latency_ms}ms > ${MAX_STARTUP_LATENCY_MS}ms" >&2
    exit 1
fi

if (( idle_rss_kb > MAX_IDLE_RSS_KB )); then
    echo "Idle RSS budget exceeded: ${idle_rss_kb}KB > ${MAX_IDLE_RSS_KB}KB" >&2
    exit 1
fi

echo "[INFO] Runtime budget checks passed"
