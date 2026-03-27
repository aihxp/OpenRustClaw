# Summary 31-01: Print an Explicit CLI Setup Handoff

## Completed

- onboarding completion now derives handoff state from durable setup state rather than only from transient wizard flags
- CLI output now reports `ready`, `blocked`, `degraded`, or in-progress handoff status plus next action, pending steps, blockers, and bootstrap attention items
- shared handoff helpers now live alongside onboarding so later surfaces can reuse the same contract

## Result

The end of onboarding now tells the operator what state the workspace is actually in, not just what the wizard attempted to configure.
