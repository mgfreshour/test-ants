# Simulation Test Charter

## Purpose

Define what we test in the simulation layer, what we explicitly do not test, and the standards for adding new simulation behavior safely.

## Core Principle

Prioritize testing deterministic simulation rules and state transitions. Avoid testing presentation details.

## In Scope

- Ant behavior rules (state transitions, movement decisions, carry/drop behavior).
- Nest behavior rules (utility scoring, task selection, task progression).
- Food and brood economy logic.
- Pheromone logic (decision, deposit, and decay rules where applicable).
- Simulation timing semantics (`paused`, `speed multiplier`, `dt` handling).
- Safety invariants (bounded values, no impossible states).

## Out of Scope

- Sprite appearance and visual styling.
- Text labels and HUD output.
- Camera behavior and map-view presentation.
- Visibility/render ordering details.
- Input UX presentation details not tied to simulation state transitions.

## Test Levels

1. **Unit tests (primary)**
   - Pure functions and extracted decision/state transition logic.
   - Deterministic input/output and table-driven scenarios.
2. **Headless integration tests (secondary)**
   - Minimal ECS wiring checks that simulation systems interact correctly.
   - No assertions on rendering-specific components.
3. **Regression tests (targeted)**
   - Add tests for each fixed simulation bug to prevent recurrence.

## Determinism Requirements

- Simulation logic under test must be deterministic.
- Randomness must be injected via a testable interface (seeded in tests).
- Time progression must be controlled (`dt` and sim speed are explicit inputs).

## Contribution Requirements

When adding or changing simulation behavior:

- Add or update tests in the same change.
- Keep new logic in pure, testable functions when possible.
- If logic remains in ECS systems, provide a clear extraction plan in follow-up tasks.
- Avoid adding test assertions for display concerns.

## Exit Criteria For "Simulation Is Testable"

- Critical simulation loops are covered by unit tests.
- Refactored decision logic is isolated from display concerns.
- A small headless integration suite validates ECS wiring.
- CI runs simulation tests on every PR.
