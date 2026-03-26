# MVP Release Checklist

Use this checklist before promoting an OpenRustClaw MVP release candidate.

## 1. Security Posture

- Review `openrustclaw security audit`.
- Review `/control/security/posture` or the `Security Posture` panel in `/control/ui`.
- Confirm `[security].require_auth = true`.
- Confirm `[security].origin_validation = true` with explicit `gateway.allowed_origins`.
- Confirm the control plane is protected through `security.control_api_token_env` or `security.trusted_proxy_token_env`.
- Confirm the runtime vault path is present and operator-managed secrets are not relying on ad hoc local shell state alone.
- Confirm skill-signing posture is explicit: either a configured verifying key for external skill verification or a documented reason why the current release candidate is not shipping that lane.

## 2. Runtime and Recovery

- Review `openrustclaw runtime services install-status`.
- Review `openrustclaw runtime health`.
- Review `openrustclaw runtime upgrade-plan --config config/default.toml`.
- Review `openrustclaw runtime rollback-plan --config config/default.toml --artifact <path>` for the current candidate artifact or rollback reference.
- Confirm `/control/runtime/operator-ops` or the `Operator Ops Summary` panel in `/control/ui` shows a coherent managed-service, lock, reload, and recovery posture.
- Take a workspace snapshot with `openrustclaw runtime backup` before release promotion.

## 3. Observability

- Confirm `GET /health`, `GET /health/ready`, and `GET /metrics` succeed on the candidate runtime.
- Review recent runtime logs and recent runtime events in `/control/ui`.
- Confirm the expected Prometheus metrics surface includes gateway, security, and tool or channel metrics relevant to the shipped release.
- If OTLP tracing is enabled in the target environment, confirm spans arrive from a live runtime session.

## 4. Artifacts and Budgets

- Build the release artifact with `scripts/build-release-artifacts.sh --target <triple>`.
- Run `scripts/check-runtime-budgets.sh`.
- Confirm the produced tarball and `.sha256` checksum exist for the target runtime.
- Confirm the candidate binary path used by `openrustclaw runtime self-update-plan --artifact <path>` matches the artifact being promoted.

## 5. Verification Bundle

- Run the targeted security posture checks:
  - `cargo test -p openrustclaw-cli posture_summary_reports_release_critical_fields -- --nocapture`
  - `cargo test -p openrustclaw-integration-tests security_posture_summary_surfaces_release_critical_fields -- --nocapture`
- Run the selected E2E release-signal checks:
  - `cargo test -p openrustclaw-e2e-tests test_origin_validation_rejects_invalid -- --nocapture`
  - `cargo test -p openrustclaw-e2e-tests smoke_gateway_metrics_endpoint -- --nocapture`
- Run the bundled gate:
  - `scripts/run-release-gate.sh`

## Exit Criteria

Release promotion is allowed only when:

- security posture is reviewed and any remaining warnings are consciously accepted
- runtime recovery guidance is present and understood
- observability endpoints and operator surfaces are functioning
- release budgets pass
- the verification bundle passes on the candidate build
