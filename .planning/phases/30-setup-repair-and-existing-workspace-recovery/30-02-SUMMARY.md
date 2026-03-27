# Summary 30-02: Derive Repair Steps from Durable Setup State and Doctor

## Completed

- added repair-plan derivation from unfinished setup steps, failed bootstrap outcomes, and doctor diagnostics
- repair now reuses the durable setup-state contract to mark the targeted steps as active
- added unit tests for repair-plan derivation and repair-state preparation

## Result

Setup recovery now uses the same durable setup-state and first-start diagnostic truth as onboarding itself, so repair stays visible and step-oriented instead of becoming a hidden heuristic.
