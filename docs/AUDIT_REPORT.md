# Colony Codebase Audit Report

**Date:** 2026-04-01
**Scope:** Full codebase (~7,800 lines, 50 Rust files)
**Tests:** 47/47 passing, 0 failures
**Clippy:** ~50 unique warnings (115 total including duplicates)

---

## Summary

| Category | Findings |
|---|---|
| Architecture (A) | 7 |
| Correctness (C) | 4 |
| Test Quality (T) | 3 |
| Idioms/Smells (I) | 5 |
| Documentation (D) | 2 |

| Severity | Count |
|---|---|
| CRITICAL | 1 |
| HIGH | 8 |
| MEDIUM | 8 |
| LOW | 4 |

---

## Quick Wins (HIGH severity + trivial/small effort)

| ID | Fix | Effort |
|---|---|---|
| C1 | Replace `f32 ==` with sorted priority list in `choose_task` | small |
| C2 | Add food underflow guards (2 lines) | trivial |
| C3 | Transition to Idle when no larva found (1 line change) | trivial |
| T2 | Add `cargo clippy -- -D warnings` to CI | trivial |
| I2-I4 | Run `cargo clippy --fix` for 11 auto-fixable warnings | trivial |

---

## Architecture (7 findings)

### [A1] HIGH | medium | `nest_task_advance` is a 600-line monolith with O(n^2) queries

- **File:** `src/plugins/nest_ai.rs:548-1149`
- **Problem:** This single function handles all 5 task types across all ants. It takes 8 query parameters (clippy warns `too many arguments`), contains deeply nested match-arms, and performs repeated linear scans over brood/food entities per ant. At 600 lines, it is by far the hardest function to modify without regressions.
- **Fix:** Split into per-task systems (`advance_feed_task`, `advance_dig_task`, etc.) that each take only the queries they need. Pre-index brood/food entities into a `HashMap<Entity, ...>` at the top of the frame to eliminate repeated linear scans.

### [A2] HIGH | medium | `nest_separation_steering` is O(n^2)

- **File:** `src/plugins/nest_ai.rs:1324-1377`
- **Problem:** Collects all nest ant positions into a `Vec`, then iterates every ant against every other ant. With 100+ nest ants, this becomes a measurable per-frame cost.
- **Fix:** Use the same spatial-grid approach that `SpatialGrid` provides for surface ants. Create a `NestSpatialGrid` resource rebuilt each frame, query only nearby neighbors within `separation_radius`.

### [A3] HIGH | medium | Non-deterministic simulation due to 17 `rand::thread_rng()` call sites

- **File:** `src/plugins/nest_ai.rs`, `src/plugins/ant_ai.rs`, `src/plugins/nest.rs`, `src/plugins/terrain.rs`, `src/plugins/combat.rs`
- **Problem:** Every system creates its own `thread_rng()`. This makes simulation runs non-reproducible, blocks replay/snapshot testing, and the prepared `SimRng` trait + `SeededSimRng` in `sim_core/rng.rs` is entirely unused (clippy reports it as dead code).
- **Fix:** Wire `SeededSimRng` as a `Resource` through the app. Replace all `rand::thread_rng()` with the shared seeded RNG. This is the single highest-leverage architectural change — it unlocks deterministic replay, property-based testing, and bug reproduction.

### [A4] MEDIUM | medium | Hardcoded `colony_id: 0` prevents multi-colony support

- **File:** `src/plugins/nest_ai.rs:217`, `src/plugins/nest.rs:85,430`, `src/plugins/ant_ai.rs:85`
- **Problem:** Four call sites hardcode `ColonyMember { colony_id: 0 }`. The architecture doc envisions a red enemy colony, but no second colony can actually spawn because the ID is baked in.
- **Fix:** Introduce a `ColonyId` newtype resource (or derive from the map entity) and flow it through spawn functions. This keeps the codebase honest about multi-colony readiness.

### [A5] MEDIUM | medium | `ColonyFood` is defined in `plugins::ant_ai` but used across 3 plugins

- **File:** `src/plugins/ant_ai.rs:33-36`
- **Problem:** `ColonyFood` is a cross-cutting resource used by `ant_ai`, `nest_ai`, and `nest`. Defining it in a plugin module that also contains surface AI systems creates a coupling where nest code imports from `plugins::ant_ai`.
- **Fix:** Move `ColonyFood` to `resources/colony.rs` alongside `BehaviorSliders` and `ColonyStats`. This aligns with the existing module boundaries.

### [A6] MEDIUM | small | Duplicated zone expansion logic in two systems

- **File:** `src/plugins/nest_ai.rs:128-163` (`apply_deferred_zone_expansions`) and `src/plugins/nest_ai.rs:1203-1238` (`apply_zone_expansions`)
- **Problem:** These two systems do the same thing (convert a tunnel to a chamber, invalidate path cache, update pheromones, update sprite) but are triggered by different marker components (`ExpandZoneDeferred` vs `ExpandZone`). The bodies are copy-pasted.
- **Fix:** Unify into a single marker component and a single system.

### [A7] LOW | small | `NestGrid::cells` stored as `Vec<Vec<CellType>>` — cache-unfriendly

- **File:** `src/resources/nest.rs:13`
- **Problem:** A 2D `Vec<Vec<T>>` allocates each row separately, fragmenting memory. For a 60x40 grid this is 40 separate heap allocations. `NestPheromoneGrid` correctly uses a flat `Vec<T>` with index math, but `NestGrid` does not.
- **Fix:** Change to `Vec<CellType>` with `y * width + x` indexing (matching `NestPheromoneGrid`). The `get` / `set` methods already abstract access, so callers won't need changes.

---

## Correctness (4 findings)

### [C1] CRITICAL | small | `choose_task` uses `==` to compare `f32` scores — tie-breaking is fragile

- **File:** `src/sim_core/nest_scoring.rs:104-121`
- **Problem:** `choose_task` computes a `max_score` then uses `max_score == scores.feed` to identify the winner. Floating-point equality is unreliable — two different scoring paths can produce the same `f32` bit pattern, and a tiny change to any formula silently changes which task wins in ties. Additionally, the priority order (MoveBrood > Feed > Dig > Haul > Attend > Idle) is implicit in the `if-else` chain, not documented.
- **Fix:** Collect `(score, NestTaskChoice)` pairs into a `Vec`, sort by score descending with a stable tiebreaker on explicit priority, then pick the first. This makes priority explicit and eliminates float-equality hazards.

### [C2] HIGH | small | `colony_food.stored` can go negative

- **File:** `src/plugins/nest_ai.rs:700` and `src/plugins/nest_ai.rs:1009`
- **Problem:** Both `FeedLarva::DeliverFood` and `AttendQueen::FeedQueen` subtract from `colony_food.stored` without checking for underflow. If an ant picks up the last food but a concurrent feeder also subtracts, stored food goes negative. `nest_ant_feeding` correctly checks `< NEST_FEED_COST` but these paths do not.
- **Fix:** Add `if colony_food.stored >= 1.0 { colony_food.stored -= 1.0; }` guards, or clamp to zero after subtraction.

### [C3] MEDIUM | trivial | `FeedLarva::FindLarva` silently proceeds to deliver even when no larva found

- **File:** `src/plugins/nest_ai.rs:686-688`
- **Problem:** When `best` is `None` (no unfed larva found), the code sets `*step = FeedStep::DeliverFood` with `target_larva` still `None`. The `DeliverFood` step then runs `try_insert(BroodFed)` on `None` (harmless) but still despawns the carried food entity and decrements colony food, wasting a food item into the void.
- **Fix:** When no larva is found, transition to `Idle` instead of `DeliverFood`.

### [C4] MEDIUM | trivial | 28 dead-code warnings (unused variants, methods, structs)

- **File:** Multiple — `BiomeType`, `Terrain`, `SimRng`, `ThreadSimRng`, `SeededSimRng`, `DigStep::PickUpSoil/GoToMidden/DropSoil`, etc.
- **Problem:** Dead code is confusing to contributors and makes clippy output noisy, masking real warnings.
- **Fix:** Remove dead variants/structs, or annotate with `#[allow(dead_code)]` + a doc comment explaining planned usage. The `SimRng` trait and `DigStep` soil variants should be kept (they're planned features) but marked explicitly.

---

## Test Quality (3 findings)

### [T1] HIGH | large | Zero tests for any plugin system — 47 tests all in `sim_core` and `resources`

- **File:** `src/plugins/` (0 tests except 2 trivial sim-plugin tests)
- **Problem:** All business logic in `nest_ai.rs` (1528 lines), `ant_ai.rs` (744 lines), `nest.rs` (505 lines), `combat.rs`, `pheromone.rs` — the actual runtime systems — have no tests. The `sim_core` extraction is excellent but only covers pure functions. Systems that compose queries, mutate components, and spawn entities are completely untested. A regression in, say, `portal_transition` or `nest_task_advance` will not be caught.
- **Fix:** Add Bevy `App::new() + MinimalPlugins` integration tests for key systems: portal transitions, food deposit, brood development, dig excavation. The existing `sim_plugin_advances_tick` test demonstrates the pattern. Target the 5 highest-risk systems first.

### [T2] HIGH | medium | No CI step runs `cargo clippy` — 50 warnings go unnoticed

- **File:** `.github/workflows/simulation-tests.yml`
- **Problem:** The CI workflow only runs `cargo test sim_core::` and `cargo test sim_plugin_`. It does not run `clippy`, does not run the full test suite, and does not check `cargo build` for the complete binary (which can fail even when filtered tests pass).
- **Fix:** Add a job that runs `cargo clippy -- -D warnings` and `cargo build`. Also change the integration job to `cargo test` (unfiltered) to catch regressions in `resources::` tests.

### [T3] MEDIUM | small | Test builder (`NestScoringInputBuilder`) doesn't set `brood_need` or `queen_signal`

- **File:** `src/sim_core/test_fixtures.rs`
- **Problem:** The builder defaults `brood_need` and `queen_signal` to 0.0 and provides no setter. Tests that call `.queen(1.0, 1.0)` set `queen_signal` correctly, but there's no way to test scoring under varying `brood_need` without manually constructing the struct, defeating the builder's purpose.
- **Fix:** Add `.brood_need(f32)` and `.queen_signal(f32)` setters to the builder.

---

## Idioms/Smells (5 findings)

### [I1] HIGH | medium | 14 "very complex type" clippy warnings on system signatures

- **File:** `src/plugins/nest_ai.rs`, `src/plugins/ant_ai.rs`
- **Problem:** Bevy system functions with 6-9 query parameters produce type signatures that clippy flags. This isn't just cosmetic — it signals that systems are doing too much. `nest_task_advance` has a signature with 8 query parameters spanning 3 lines.
- **Fix:** For Bevy-specific `Query<...>` types, introduce type aliases. Longer term, decompose systems (see A1).

### [I2] MEDIUM | trivial | 5 `map_or` calls that can be simplified

- **File:** Multiple
- **Problem:** Clippy reports `this map_or can be simplified` — typically `x.map_or(false, |v| ...)` can become `x.is_some_and(|v| ...)`.
- **Fix:** Run `cargo clippy --fix` to auto-apply these.

### [I3] MEDIUM | trivial | 3 collapsible `if` statements

- **File:** Multiple
- **Problem:** Nested `if` blocks that can be joined with `&&`.
- **Fix:** `cargo clippy --fix`.

### [I4] LOW | trivial | 3 unnecessary dereferences

- **File:** Multiple
- **Problem:** `*ref` where Rust auto-deref suffices.
- **Fix:** `cargo clippy --fix`.

### [I5] LOW | trivial | Magic numbers throughout scoring formulas

- **File:** `src/sim_core/nest_scoring.rs:43-100`
- **Problem:** Values like `300.0`, `0.8`, `0.15`, `0.6`, `5.0`, `3.0`, `0.7`, `0.3`, `4.0` appear as raw literals in `compute_scores`. When tuning one formula, it's easy to change the wrong number.
- **Fix:** Extract into named constants on `NestScoringInput` or a `NestScoringConfig` struct. This also makes it trivial to expose them for UI-driven tuning later.

---

## Documentation (2 findings)

### [D1] MEDIUM | small | Architecture doc describes systems and plugins that don't exist

- **File:** `docs/ARCHITECTURE.md`
- **Problem:** The architecture doc references `Environment Plugin`, `Campaign Plugin`, `Predator Plugin`, `Rendering Plugin`, `red_colony_strategy`, `predator_ai`, and many events (`RainStartEvent`, `LawnmowerEvent`, etc.) that are not implemented. There is no indication of what is aspirational vs. implemented. New contributors may spend time searching for code that doesn't exist.
- **Fix:** Add a section header "Implemented" vs. "Planned" to each area, or move planned items to `IMPLEMENTATION_PLAN.md`.

### [D2] LOW | trivial | No top-level README

- **Problem:** There is no `README.md` in the repo root. `CLAUDE.md` and `AGENTS.md` exist but are AI-tool specific. A contributor cloning this repo has no entry point for build instructions, project overview, or how to run the simulation.
- **Fix:** Add a brief `README.md` with: what this is, `cargo run`, key controls (Space, Tab, Period, click), and a link to `docs/ARCHITECTURE.md`.

---

## Guidelines for Future Work

1. **Extract before extending.** Before adding a new nest task type, decompose `nest_task_advance` (A1). Each new task added to the current monolith increases the risk surface quadratically.

2. **Deterministic RNG first.** Before adding multi-colony or AI replay features, wire up `SeededSimRng` (A3). Every subsequent feature will benefit from reproducible runs.

3. **Test the systems, not just the math.** The `sim_core` extraction pattern is excellent — keep pushing logic there. But also write at least one Bevy integration test per plugin system that mutates components (T1). A single `portal_transition` test would have caught the orphaned-returner regression before it needed a dedicated `regressions.rs` module.

4. **One system, one job.** Bevy's parallel scheduler can only parallelize systems with non-overlapping queries. The current `.chain()` in `nest_ai` and `ant_ai` forces all 15+ systems to run sequentially. As you decompose systems, remove chains where ordering isn't strictly necessary to unlock free parallelism.

5. **Keep `components/` dumb.** Components should be data-only with minimal methods (constructors, simple accessors). The `NestTask::label()` and `NestTask::color()` methods are rendering concerns that belong in the UI layer, not on the component.

---

## Automation Recommendations

### 1. CI: Add a clippy gate

```yaml
clippy:
  name: Clippy Lint Gate
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - uses: Swatinem/rust-cache@v2
    - run: cargo clippy -- -D warnings
```

### 2. CI: Run the full test suite, not filtered subsets

Change the integration job from `cargo test sim_plugin_` to `cargo test` so that any new tests are automatically included.

### 3. Pre-commit: `cargo fmt --check`

Add a formatting check to CI and/or a git pre-commit hook. Formatting is currently consistent, but without enforcement it will drift.

### 4. Dependency audit

Add `cargo install cargo-audit && cargo audit` to CI. The project only has 2 direct dependencies (`bevy`, `rand`) but their transitive tree is large (200+ crates). Automated vulnerability scanning is free insurance.

### 5. Dead code enforcement

Once planned features are annotated with `#[allow(dead_code)]`, add `-D dead_code` to the clippy invocation. This prevents new dead code from accumulating silently.
