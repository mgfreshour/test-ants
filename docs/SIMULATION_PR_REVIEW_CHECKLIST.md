# Simulation PR Review Checklist

Use this checklist for any PR that changes simulation behavior.

## 1) Scope and Separation

- [ ] The change targets simulation logic, not display behavior.
- [ ] Rendering/UI assertions are absent from simulation tests.
- [ ] New logic is added to `sim_core` when feasible.

## 2) Determinism and Testability

- [ ] Randomness uses a testable abstraction (`SimRng`) where behavior matters.
- [ ] Time progression is explicit and test-controlled (`dt`, speed multiplier, pause).
- [ ] Tests avoid flaky assumptions (timing races, unordered iteration assumptions).

## 3) State Safety and Invariants

- [ ] State transitions are valid and intentional (no impossible combinations).
- [ ] Bounded values remain bounded (hunger, food, pheromone intensity, etc.).
- [ ] No negative resource amounts can be produced.

## 4) Regression Coverage

- [ ] Known fragile flows touched by this PR have explicit regression tests.
- [ ] New bug fixes include a failing-then-passing regression test.
- [ ] Existing regression tests still reflect current intended behavior.

## 5) Performance and Coupling

- [ ] No unnecessary per-frame allocations or expensive queries were introduced.
- [ ] ECS mutation logic is separated from pure transition/decision logic when practical.
- [ ] Integration tests remain headless and minimal.

## 6) Validation Gates

- [ ] `cargo build` passes.
- [ ] `timeout 3s cargo run` starts without panic (exit 124 expected).
- [ ] Fast simulation unit tests pass.
- [ ] Integration simulation tests pass.
