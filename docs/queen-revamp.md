
Here's the full phased plan:

---

# Queen AI Implementation — Phased Plan

## Phase 1: Foundation (no behavioral change)

**Goal:** Introduce `QueenTask` component and wire it into the queen entity. Everything still compiles, all existing tests pass, and the game behaves identically.

**Steps:**
- Add `QueenTask` enum to [components/nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:0:0-0:0) with variants: `Idle { timer: f32 }`, `LayingEggs { egg_timer: f32 }`, `Resting`, `Grooming`
- Remove `egg_timer` field from [QueenHunger](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:71:0-80:1) (it belongs to the `LayingEggs` task state now)
- Update all existing code that reads/writes `QueenHunger.egg_timer` — currently only [queen_egg_laying](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:298:0-344:1) in [nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0) — to still compile (temporarily keep [queen_egg_laying](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:298:0-344:1) working by reading from a local timer or the new task)
- Add `QueenTask::Idle { timer: 0.0 }` to the [spawn_queen](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:168:0-198:1) bundle
- **Exit criteria:** `cargo build` clean, `cargo test sim_core::` passes, `cargo test sim_plugin_` passes

## Phase 2: Pure Scoring Logic (testable in isolation)

**Goal:** All queen decision-making logic exists as pure functions in `sim_core/`, fully unit-tested, with zero ECS dependencies.

**Steps:**
- Create `sim_core/queen_scoring.rs`:
  - `QueenScoringInput { satiation, health_frac, brood_count, colony_food_stored, has_queen_chamber }`
  - `QueenTaskScores { lay_eggs, rest, groom, idle }`
  - `QueenTaskChoice { LayingEggs, Resting, Grooming, Idle }`
  - `compute_queen_scores(input) -> QueenTaskScores`
  - `choose_queen_task(scores) -> QueenTaskChoice`
- Scoring rules:
  - **LayingEggs** — high when `satiation > EGG_SATIATION_COST`, lower as `brood_count` grows (diminishing returns), zero when no queen chamber
  - **Resting** — high when `satiation < 0.3` OR `health_frac < 0.4`; scales inversely with satiation
  - **Grooming** — constant moderate baseline (default active behavior)
  - **Idle** — low constant fallback
- Register in [sim_core/mod.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/sim_core/mod.rs:0:0-0:0)
- Add `QueenScoringInputBuilder` to [test_fixtures.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/sim_core/test_fixtures.rs:0:0-0:0)
- **Unit tests:**
  - Laying wins when well-fed + low brood count
  - Laying loses when satiation below egg cost threshold
  - Laying diminishes as brood count rises
  - Resting wins when satiation is critically low
  - Resting wins when health is critically low
  - Grooming wins as default when nothing else scores strongly
  - Grooming beats idle
- **Exit criteria:** `cargo test sim_core::queen_scoring` all green

## Phase 3: Queen AI Plugin (behavioral switchover)

**Goal:** Queen decisions are driven by the scoring system. Egg laying is now gated by the `LayingEggs` task. Old standalone [queen_egg_laying](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:298:0-344:1) system is removed.

**Steps:**
- Create `plugins/queen_ai.rs` with `QueenAiPlugin`
- **`queen_utility_scoring` system:**
  - Queries `(Entity, &QueenHunger, &Health, &MapId, &mut QueenTask), With<Queen>`
  - Only re-evaluates when `QueenTask::Idle { timer }` exceeds a threshold (same pattern as worker `REEVALUATE_INTERVAL`)
  - Also re-evaluates on task completion (other tasks self-terminate to Idle)
  - Reads nest state: brood count from brood query, colony food from map query, queen chamber existence from grid
  - Calls `compute_queen_scores` → `choose_queen_task` → assigns new `QueenTask`
- **`advance_queen_laying_eggs` system:**
  - Only runs for queens with `QueenTask::LayingEggs { egg_timer }`
  - Ticks `egg_timer += dt`; when timer exceeds `QUEEN_EGG_INTERVAL` and satiation ≥ cost: spawn egg, deduct satiation, reset timer
  - If satiation drops below cost mid-task: transition to `Idle { timer: 0.0 }` (triggers re-eval next cycle)
  - Egg spawning logic is moved verbatim from current [queen_egg_laying](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:298:0-344:1) in [nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0)
- **`advance_queen_idle` system:**
  - Ticks `Idle { timer }` forward by dt (same as worker idle ticking)
- **`advance_queen_resting` system:**
  - No-op placeholder (queen just sits there; the metabolism effect is Phase 4)
  - Transitions to `Idle` after a rest duration so re-eval happens
- Register: `pub mod queen_ai` in [plugins/mod.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/mod.rs:0:0-0:0), add `QueenAiPlugin` to [main.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/main.rs:0:0-0:0)
- Remove [queen_egg_laying](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:298:0-344:1) system + `QUEEN_EGG_INTERVAL` constant from [nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0); remove from [NestPlugin::build](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:39:4-60:5) system registration
- **Exit criteria:** `cargo build` clean, `timeout 3s cargo run` exits 124, eggs still appear in nest view, queen cycles through tasks

## Phase 4: Resting Metabolism (strategic depth)

**Goal:** Resting is a meaningful state — the queen conserves energy, hunger decays at half rate, which naturally reduces the worker "feed queen" signal.

**Steps:**
- Modify [queen_hunger_decay](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:284:0-296:1) in [nest.rs](cci:7://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest.rs:0:0-0:0):
  - Add `QueenTask` to the query: `Query<(&mut QueenHunger, Option<&QueenTask>), With<Queen>>`
  - When task is `QueenTask::Resting`: multiply `decay_rate` by `0.5`
  - All other tasks: normal decay rate
- Add pure helper in `sim_core/queen_scoring.rs`: `pub fn resting_decay_multiplier() -> f32 { 0.5 }` (testable constant)
- **Feedback loop verification:**
  - `QueenHunger.satiation` still decays (just slower when resting)
  - Workers still read `1.0 - satiation` as `queen_hunger_val` in [nest_utility_scoring](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest_ai.rs:518:0-661:1) — no change needed on worker side
  - Emergent result: resting queen → slower satiation drain → lower `queen_hunger_val` → fewer `AttendQueen` workers dispatched → queen self-regulates food demand
- **Tests:**
  - Pure test: `resting_decay_multiplier()` returns 0.5
  - Scoring test: at satiation=0.2, resting wins over laying
  - Scoring test: at satiation=0.8, laying wins over resting
- **Exit criteria:** Full validation sequence: `cargo build && cargo test sim_core:: && cargo test sim_plugin_ && timeout 3s cargo run`

## Phase 5: Pheromone-Based Hunger Signal (future — emergent communication)

**Goal:** Replace the direct `QueenHunger.satiation` query in worker scoring with a pheromone-based signal. Workers respond to diffused "hungry queen" pheromone rather than telepathically knowing her satiation level. This creates spatial emergent behavior: workers closer to the queen respond first.

> **NOTE:** This phase intentionally keeps the existing direct-query logic as a fallback/test baseline. The switchover is incremental:
> 1. Queen deposits hunger pheromone → workers read it → validate behavior matches
> 2. Only then remove the direct query path

**Steps:**
- **Queen deposits hunger pheromone:**
  - In `queen_ai.rs`, add a `queen_hunger_pheromone_deposit` system
  - Every tick, queen writes `hunger_signal = (1.0 - satiation)` into the nest pheromone grid at her grid position
  - This diffuses naturally through the existing `NestPheromoneGrid` diffusion system
  - Resting queen emits the same signal (she's still hungry, just conserving) — or slightly reduced, TBD
- **Worker reads pheromone instead of direct query:**
  - In [nest_utility_scoring](cci:1://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/plugins/nest_ai.rs:518:0-661:1), replace the `queen_query` lookup of [QueenHunger](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:71:0-80:1) with a read of the hunger pheromone at the worker's current grid position
  - Workers far from the queen see a weaker signal → less likely to attend → closer workers naturally prioritize
  - This replaces the `queen_signal` field already in `NestPheromoneGrid` (or adds a new channel)
- **Validation:**
  - A/B comparison: run with direct-query vs pheromone-based and verify queen still gets fed reliably
  - Edge case: queen chamber is far from food storage — signal must still reach workers
- **Cleanup:** Once validated, remove the direct [QueenHunger](cci:2://file:///Users/mfreshour/src/ant-col-test-split-nest-ai/src/components/nest.rs:71:0-80:1) query from worker scoring

---

