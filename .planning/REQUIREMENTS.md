# Requirements: v1.39 Native Product E2E Verification and Greenfield Repairs

**Started:** 2026-03-29
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery planning roadmap baseline:** `8/8` milestones shipped, or `100%`
**Native-delivery implementation roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-product E2E roadmap baseline before shipment:** `0/1`, or `0%`
**Native-product E2E roadmap target after shipment:** `1/1`, or `100%`

## Scope

This milestone starts the first native-product E2E queue by running full end-to-end verification across the shipped product paths, recording failures with explicit ownership, applying any required repairs through greenfield-native surfaces first, and revalidating the product truthfully after repair.

## Milestone Requirements

- [ ] **E2E-01**: The milestone verifies the shipped product end to end across the real CLI, control, gateway, MCP, runtime-host, and operator paths.
- [ ] **E2E-02**: The milestone records a failure and ownership matrix that distinguishes app-lane, native-delivery, infrastructure, and bounded legacy exceptions.
- [ ] **E2E-03**: Any repairs found during end-to-end verification land through greenfield-native ownership first instead of reviving legacy command-local logic.
- [ ] **E2E-04**: The milestone closes with revalidation and a truthful operator-facing exit report covering what works, what was repaired, and what remains bounded.

## Future Requirements

- The completed `18/18`, `6/6`, `8/8`, and `6/6` denominators must remain closed and must not be reinterpreted as E2E completion percentages.
- Repairs that surface during E2E verification should prefer `openrustclaw-app`, native delivery crates, or explicit infrastructure adapters before any change to legacy command ownership.
- Any remaining legacy exception after repair must stay explicit and bounded instead of becoming a silent catch-all fix surface.
- Follow-on work after `v1.39` must use the native-product E2E roadmap explicitly instead of reopening completed architecture queues.

## Out of Scope

- Reopening the completed historical, adapter-only, planning, or implementation denominators with a new meaning
- Cosmetic documentation churn unrelated to verification, repair evidence, or operator truth
- Large new feature work unrelated to E2E verification or repair
- Treating compile-only success as proof that the full product works end to end

## Traceability

- `E2E-01` -> Phase 161
- `E2E-02` -> Phase 162
- `E2E-03` -> Phase 163
- `E2E-04` -> Phase 164
