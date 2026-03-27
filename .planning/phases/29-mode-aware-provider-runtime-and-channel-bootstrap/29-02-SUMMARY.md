# Summary 29-02: Validate Provider and Runtime Bootstrap During Onboarding

## Completed

- provider setup now refreshes runtime health instead of assuming a provider switch or API key write means success
- onboarding records provider readiness and control-plane lane status into setup state
- control-plane setup now validates the written runtime mode against the actual control registry and the recommended mode for the selected self-hosted deployment path

## Result

Provider and control-plane bootstrap now converge on the same runtime-health and mode-alignment truth that the rest of the product already uses, so setup can leave a surface ready, warning, or blocked for real reasons.
