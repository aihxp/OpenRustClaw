# Summary 57-01: Defined the Greenfield Boundary Contract

## What Changed

- Added a canonical planning contract at `.planning/codebase/GREENFIELD.md` that defines the target layer model, legacy containment rules, ranked migration inventory, and the first proving slice.
- Added a contributor-facing `Greenfield Transition` architecture page to the mdBook and linked the broader architecture overview to the active migration strategy.
- Updated development guidance so new work defaults to the greenfield lane, with setup handoff reporting chosen as the first bounded migration target.

## Result

Phase 57 turns “brownfield to greenfield” into an executable architecture contract instead of a rewrite slogan. The repo now has one agreed target shape, one set of containment rules for legacy hotspots, and one concrete proving slice for future migration work.

## Follow-on

- Phase 58 will introduce the new application shell and service interfaces that the selected proving slice can target.
- Phase 59 will migrate setup handoff reporting through that new lane and verify the end-to-end behavior remains stable.
