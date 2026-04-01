# Simulation Logic Testability Work Plan

## Goal

Create reliable tests for simulation behavior while explicitly **not** testing display/UI logic (sprites, colors, labels, camera, input visuals).

## Scope Boundaries

Test:
- Ant state transitions and movement decisions.
- Food economy and nest economy rules.
- Task assignment and task progression logic.
- Pheromone deposition/decay decision logic.
- Simulation clock and speed semantics.
- Lifecycle rules (brood progression, spawning conditions).

Do not test:
- Sprite color/size updates.
- Text labels (`Text2d`) and HUD behavior.
- Camera movement/zoom behavior.
- Visibility toggling for rendering concerns.
- Input UX details except where input must be translated into simulation commands.

## Priority and Difficulty Key

- **Priority**: `P0` (blocker/foundation), `P1` (high value), `P2` (important follow-up), `P3` (nice-to-have)
- **Difficulty**: `S` (small), `M` (medium), `L` (large), `XL` (very large)

## Ordered Checklist (Refactors First, Then Tests)

1. [x] **W1 (P0/S)** Define simulation-test charter and boundaries in-repo.  
       Why: prevents accidental UI test scope creep and aligns contributors.
2. [x] **W2 (P0/L) - REFACTOR** Create `sim_core` module for pure simulation functions (no `Commands`, `Sprite`, `Text2d`).  
       Why: establishes testable seams for logic currently embedded in ECS systems.
3. [x] **W3 (P0/XL) - REFACTOR** Extract ant decision logic from `ant_ai` into pure functions.  
       Why: enables deterministic tests for foraging/returning/transition behaviors.
4. [x] **W4 (P0/L) - REFACTOR** Extract nest utility scoring from `nest_ai` into pure scoring API.  
       Why: makes task-selection behavior testable with table-driven tests.
5. [x] **W5 (P0/XL) - REFACTOR** Separate task state-machine transitions from ECS mutation in `nest_task_advance`.  
       Why: allows exhaustive sub-step transition tests without world setup overhead.
6. [x] **W6 (P0/M) - REFACTOR** Add deterministic RNG abstraction (`trait SimRng`) and seeded test implementation.  
       Why: removes flakiness from steering/selection tests.
7. [x] **W7 (P1/M)** Introduce test fixtures/builders for simulation state snapshots.  
       Why: reduces test boilerplate and encourages broad test coverage.
8. [x] **W8 (P1/S)** Add unit tests for `SimSpeed`, clock tick, pause semantics, and dt scaling.  
       Why: protects foundational timing assumptions used by all systems.
9. [x] **W9 (P1/M)** Add unit tests for hunger/starvation/food-relief rules.  
       Why: captures critical survival and economy interactions.
10. [x] **W10 (P1/M)** Add unit tests for food pickup/deposit and state transitions.  
        Why: verifies main foraging loop correctness.
11. [x] **W11 (P1/M)** Add unit tests for pheromone decision/deposit logic by ant state.  
        Why: prevents regressions in trail formation behavior.
12. [x] **W12 (P1/M)** Add unit tests for nest utility scoring outcomes under varied colony conditions.  
        Why: ensures assignment logic remains explainable and tunable.
13. [x] **W13 (P1/L)** Add unit tests for nest task progression sub-steps (`Feed`, `MoveBrood`, `Haul`, `Dig`, `Attend`).  
        Why: protects high-complexity behavior chains with many edge cases.
14. [x] **W14 (P2/M)** Add property-style tests for invariants (no invalid states, bounded values, no negative food).  
        Why: catches broad classes of logic bugs early.
15. [x] **W15 (P2/M)** Add integration tests with `bevy::app::App` for minimal end-to-end sim ticks (headless).  
        Why: validates ECS wiring while keeping display assertions out of scope.
16. [x] **W16 (P2/M)** Add regression tests for known bug-prone flows (orphaned returners, portal transitions, dig face selection).  
        Why: protects specific historical failure modes.
17. [x] **W17 (P2/S)** Add CI test partition: fast unit suite + slower integration suite.  
        Why: keeps feedback loop fast and reliable.
18. [ ] **W18 (P3/S)** Add mutation-focused review checklist for simulation PRs.  
        Why: improves long-term test quality and maintainability.

## Refactor Targets Required Before Most Testing

These are the highest-leverage extraction candidates based on current coupling:

1. `src/plugins/ant_ai.rs`
   - Extract pure functions for:
     - hunger update and starvation damage decision.
     - forage steering vector computation.
     - return steering vector computation.
     - boundary bounce response.
     - food pickup/deposit rule decisions.
2. `src/plugins/nest_ai.rs`
   - Extract pure APIs for:
     - utility score computation and winner selection.
     - per-task state transition logic (`FeedStep`, `MoveBroodStep`, `HaulStep`, `DigStep`, `AttendStep`).
     - dig-face scoring and selection.
3. `src/plugins/nest.rs`
   - Extract lifecycle/economy rules:
     - egg-laying eligibility.
     - brood stage transition decision and spawn parameters.
4. `src/plugins/simulation.rs` + `src/resources/simulation.rs`
   - Keep as baseline unit-tested timing logic.

## Milestone Checklist

- [ ] **Milestone 1 (Required refactors complete):** W1-W6
- [ ] **Milestone 2 (Core unit tests complete):** W7-W13
- [ ] **Milestone 3 (Hardening + CI complete):** W14-W17
- [ ] **Milestone 4 (Process guardrails complete):** W18

## Acceptance Criteria Per Phase

### Phase 1 (Foundations)
- [ ] `sim_core` exists and compiles with no rendering/UI dependencies.
- [ ] Deterministic RNG is used by extracted logic.
- [ ] At least one example unit test per extracted API.

### Phase 2 (Core Behavior Coverage)
- [ ] Ant and nest decision logic are mostly exercised through unit tests.
- [ ] New tests avoid asserting on `Sprite`, `Text2d`, `Visibility`, or camera values.
- [ ] Edge cases covered: paused clock, empty food, missing targets, out-of-bounds positions.

### Phase 3 (Integration and Regression)
- [ ] Headless ECS integration tests validate system wiring and resource flow.
- [ ] Regression tests exist for at least three known fragile behavior chains.
- [ ] CI runs simulation tests on every PR.

## Risks and Mitigations

- **Risk:** Refactor changes emergent behavior unintentionally.  
  **Mitigation:** capture baseline behavior with snapshot-style expected decisions before large extractions.

- **Risk:** Tests become brittle due to random behavior.  
  **Mitigation:** force seeded RNG in test paths and isolate stochastic boundaries.

- **Risk:** ECS-heavy tests become slow.  
  **Mitigation:** prioritize pure-unit tests; keep only a small set of integration tests.

## Definition of Done

Simulation logic is considered testable when:
- [ ] Most decision/state transition logic runs through pure functions with deterministic inputs.
- [ ] Unit tests cover critical colony loops (forage, return, nest tasking, lifecycle).
- [ ] Integration tests verify ECS wiring without touching display assertions.
- [ ] New simulation features require test additions as part of PR acceptance.
