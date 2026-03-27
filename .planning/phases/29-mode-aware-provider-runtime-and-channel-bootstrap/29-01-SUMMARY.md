# Summary 29-01: Persist Bootstrap Outcomes in Setup State

## Completed

- extended `SetupState` with durable `bootstrap_outcomes`
- added typed `SetupBootstrapOutcome` records for provider, runtime, and channel surfaces
- added helper logic that replaces prior outcomes for the same surface instead of accumulating stale bootstrap state
- added a focused unit test to lock the replacement contract

## Result

Onboarding now has one durable place to record what actually happened during bootstrap, so later repair and handoff work can read the same setup truth instead of inferring it from loose files or wizard output.
