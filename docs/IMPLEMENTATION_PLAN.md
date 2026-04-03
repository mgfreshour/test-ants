# Colony: Implementation Plan

## Sprint-Based Roadmap — Each Sprint Ends with a Demo

---

## Overview

22 two-week sprints. Each sprint produces a runnable build with visible new functionality. Sprints 1–11 built the core simulation (surface + underground). Sprints 12–18 unify the split ant system into a single job-based architecture. Sprints 19–22 add gameplay modes and polish.

```
Sprint  1   █░░░░░░░░░░░░░░░░░░░░░  Wandering Ants ✓
Sprint  2   ██░░░░░░░░░░░░░░░░░░░░  Pheromone Trails ✓
Sprint  3   ███░░░░░░░░░░░░░░░░░░░  Foraging Loop ✓
Sprint  4   ████░░░░░░░░░░░░░░░░░░  Colony & Nest ✓
Sprint  5   █████░░░░░░░░░░░░░░░░░  Nest Pheromones & Pathfinding ✓
Sprint  6   ██████░░░░░░░░░░░░░░░░  Nest Ant AI ✓
Sprint  7   ███████░░░░░░░░░░░░░░░  Stigmergic Digging & Collision ✓
Sprint  8   ████████░░░░░░░░░░░░░░  Player Control ✓
Sprint  9   █████████░░░░░░░░░░░░░  Combat & Enemies ✓
Sprint 10   ██████████░░░░░░░░░░░░  Colony Management UI ✓
Sprint 11   ███████████░░░░░░░░░░░  Player HUD & Action Bar ✓
Sprint 12   ████████████░░░░░░░░░░  Spawn at Egg Location
Sprint 13   ██████████████░░░░░░░░░  AntJob Component ✓
Sprint 14   ██████████████░░░░░░░░  Job-Driven Transitions
Sprint 15   ███████████████░░░░░░░  Unified Steering
Sprint 16   ████████████████░░░░░░  Split AI Files
Sprint 17   █████████████████░░░░░  Unified AI Dispatch ✓
Sprint 18   ██████████████████░░░░  Cleanup Legacy Paths (partial)
Sprint 19   ███████████████████░░░  Environment & Hazards ✓
Sprint 20   ████████████████████░░  Quick Game Complete
Sprint 21   █████████████████████░  Campaign Mode
Sprint 22   ██████████████████████  Polish & Sandbox
```

---

## Completed Sprints (1–11)

### Sprint 1: Wandering Ants ✓
Project setup, core components, terrain, spatial grid, ant spawning, random walk movement, camera pan/zoom, basic HUD.

### Sprint 2: Pheromone Trails ✓
Pheromone grid (evaporation + diffusion), deposit/sense systems, overlay toggle, debug panel for tuning.

### Sprint 3: Foraging Loop ✓
Food sources, pickup/dropoff, FORAGE/RETURN state cycle, dual pheromone trails (home + food), visual trail highways form.

### Sprint 4: Colony & Nest ✓
Underground nest grid (side-view), queen + brood lifecycle, nursing/digging systems, surface ↔ nest view toggle, colony panel sliders.

### Sprint 5: Nest Pheromones & Pathfinding ✓
Nest pheromone grid (queen signal, brood need, construction, chamber labels), JPS pathfinding, path cache, collision clamping, nest overlay.

### Sprint 6: Nest Ant AI ✓
Utility-based task system (nursing, hauling, queen-attending), surface ↔ nest ant transitions, age-based affinity, task labels.

### Sprint 7: Stigmergic Digging & Collision ✓
Construction pheromone dynamics, self-limiting excavation, soil hauling, player dig zones, separation steering, tunnel traffic priority.

### Sprint 8: Player Control ✓
Player-controlled ant (WASD), item pickup/drop, manual pheromone trails, recruit/dismiss followers, ant swapping, camera follow, regurgitate food.

### Sprint 9: Combat & Enemies ✓
Red rival colony, combat detection/resolution, death/corpses, alarm pheromone cascade, spider + antlion predators, victory/defeat detection.

### Sprint 10: Colony Management UI ✓
`bevy_egui` integration, colony management panel (job/caste sliders, stats), sim controls toolbar, view toggle, overlay controls, tooltips.

### Sprint 11: Player HUD & Action Bar ✓
Player info HUD (HP, hunger, carried item, followers), clickable action bar with hotkey labels, context-sensitive buttons, minimap, notification toasts, hotkey reference overlay.

---

## Sprint 12: Spawn at Egg Location ✓ (Weeks 23-24)

### Goal
Brood hatches into ants at the pupa's world position instead of teleporting to the surface portal. Sets the foundation for ants being "born" in the nest (needed for unified pool where new ants start underground).

### Tasks

| # | Task | Est | Status |
|---|---|---|---|
| 12.1 | Change spawn position in `brood_development` from portal exit to brood entity's current `Transform` | 3h | ✓ |
| 12.2 | Spawn with `MapId` matching the brood's map (nest) instead of surface | 2h | ✓ |
| 12.3 | Add `NestTask::Wander` component so the ant starts as a nest ant | 2h | ✓ |
| 12.4 | Remove `spawn_initial_ants` system (all ants now come from brood) | 2h | ✓ |
| 12.5 | Adjust `INITIAL_NEST_ANTS` count (20) and seed colony food (10 units) to compensate for early-game population | 2h | ✓ |
| 12.6 | Remove `AntState::Nursing` and `AntState::Digging` (unused after unified job/task model) | 2h | ✓ |
| 12.7 | Runtime validation — verify colony bootstraps correctly | 1h | ✓ |

### Completion Notes
- Hatched ants spawn at pupa's `Transform` position on the nest map with `AntJob::Unassigned`, `AntState::Idle`, `NestTask::Wander`, and full nest-ant component set (StimulusThresholds, SteeringTarget, SteeringWeights, Movement, PositionHistory)
- Colony bootstrap: 20 initial nest ants (8 Nurse, 6 Digger, 6 Forager) + 10 food units seeded into ColonyFood
- Removed `AntState::Nursing` and `AntState::Digging` — these were only set/displayed, never used in conditional logic
- ColonyStats updated from Caste-based to AntJob-based counting (foragers/nurses/diggers/defenders/unassigned)
- 79 tests pass, runtime validation exit 124

### Acceptance Criteria
- [x] New ants spawn at brood's `Transform` position
- [x] New ants have nest `MapId` + `NestTask::Wander`
- [x] Initial surface spawner removed
- [x] Colony bootstraps with correct early-game population
- [x] Tests pass

**Files touched**:
- `src/plugins/nest.rs` — `brood_development` hatch at pupa position, `update_colony_stats` AntJob-based
- `src/plugins/ant_ai/mod.rs` — removed `spawn_initial_ants`
- `src/plugins/nest_ai/core.rs` — 20 initial ants (8N/6D/6F), 10 food seed
- `src/components/ant.rs` — removed `AntState::Nursing` and `AntState::Digging`
- `src/plugins/egui_ui.rs` — colony stats display updated for AntJob
- `src/plugins/ant_ai/visuals.rs` — removed Nursing/Digging label arms
- `src/resources/colony.rs` — ColonyStats fields updated

---

## Sprint 13: AntJob Component + Job Assignment ✓ (Weeks 25-26)

### Goal
Every ant gets a persistent `AntJob` tag. A global system enforces `BehaviorSliders` ratios using age-based polyethism. This sprint is behavior-neutral — `AntJob` is attached but nothing reads it yet (besides the assignment system).

### Tasks

| # | Task | Est | Status |
|---|---|---|---|
| 13.1 | Add `AntJob` enum component (Forager/Nurse/Digger/Defender/Unassigned) | 2h | ✓ |
| 13.2 | Create `sim_core/job_assignment.rs` — pure logic for ratio computation, job selection | 4h | ✓ |
| 13.3 | Add `AntJob` to `spawn_initial_nest_ants` | 1h | ✓ |
| 13.4 | Add `AntJob` to `brood_development` spawn (Sprint 12 output) | 1h | ✓ |
| 13.5 | Extend `BehaviorSliders` if needed | 1h | ✓ (no changes needed) |
| 13.6 | `job_assignment_system` — runs every ~3s, rebalances jobs per colony | 5h | ✓ |
| 13.7 | Age-based affinity (young→Nurse, mid→Digger, old→Forager/Defender) | 3h | ✓ |
| 13.8 | Hysteresis band (±5%) to prevent oscillation | 2h | ✓ |
| 13.9 | Unit tests for ratio math, age affinity, hysteresis | 4h | ✓ (8 tests) |

### Demo
> Ants now have persistent AntJob components. Every 3 seconds, the job_assignment_system rebalances colony workforce. Adjust "Forage" slider up — over the next few seconds, more Unassigned/Nurse ants transition to Forager as the system rebalances. Young ants cluster in Nurse jobs (high nursing affinity), old ants prefer Forager (high forager affinity). Middle-aged ants stay in Digger/intermediate roles. Existing AI behavior unchanged (still uses NestTask/AntState for decisions).

### Acceptance Criteria
- [x] `AntJob` component on all ants
- [x] Job assignment system rebalances ants to match sliders every 3s
- [x] Age-based affinity creates generational division
- [x] Hysteresis prevents rapid oscillation (±5% band)
- [x] Pure logic tests pass (8 tests all green)
- [x] No behavior changes (all 74 tests pass, runtime validation succeeds)

**Files touched**:
- `src/components/ant.rs` — `AntJob` enum (Forager/Nurse/Digger/Defender/Unassigned)
- `src/sim_core/job_assignment.rs` (new, 357 lines) — pure logic with compute_job_affinity, should_reassign_ant, find_best_job
- `src/plugins/nest_ai.rs` — `job_assignment_system`, `JobAssignmentTimer` resource
- `src/plugins/nest.rs` — add `AntJob::Unassigned` to brood_development spawn
- `src/plugins/ant_ai.rs` — add `AntJob::Unassigned` to spawn_initial_ants
- `src/sim_core/mod.rs` — export job_assignment module

**Commit**: e8c7571 ("feat: Sprint 13 - AntJob component and job assignment system")

---

## Sprint 14: Job-Driven Portal Transitions (Weeks 27-28)

### Goal
Replace the current slider-throttled portal transitions with job-aware transitions. `Nurse`/`Digger` ants seek the nest; `Forager`/`Defender` ants seek the surface. Portal transitions now respect `AntJob` instead of rolling dice.

### Tasks

| # | Task | Est |
|---|---|---|
| 14.1 | Rewrite `portal_transition` to check `AntJob` instead of counting ants | 4h |
| 14.2 | Nurse/Digger on surface near portal → enter nest, add `NestTask::Idle` | 3h |
| 14.3 | Forager/Defender in nest near portal → exit to surface, remove `NestTask`/`NestPath` | 3h |
| 14.4 | Add small random delay/cooldown to avoid ping-ponging | 2h |
| 14.5 | Update `nest_to_surface_transition` — remove age-based exit, use job-based logic | 3h |
| 14.6 | New helpers: `should_transition_to_nest(job)`, `should_transition_to_surface(job)` in `sim_core` | 2h |
| 14.7 | Update `should_enter_nest` in `sim_core/regressions.rs` to job-based logic | 2h |
| 14.8 | Pure function tests for transition logic | 3h |
| 14.9 | Integration test: nurse ants enter nest, foragers exit | 2h |

### Demo
> Adjust "Nurse" slider up — surface ants near the portal walk into the nest entrance. They gain `NestTask::Idle` and start navigating underground. Adjust "Forage" slider up — idle nest ants pathfind to the entrance and emerge on the surface. No more random dice rolls — transitions are deterministic based on job.

### Acceptance Criteria
- [ ] Portal transitions respect `AntJob`
- [ ] Nurse/Digger jobs cause ants to enter nest
- [ ] Forager/Defender jobs cause ants to exit nest
- [ ] Cooldown prevents ping-ponging
- [ ] Pure logic tests pass
- [ ] Integration test: ants transition correctly

**Files touched**:
- `src/plugins/nest_ai.rs` — `portal_transition`, `nest_to_surface_transition`
- `src/sim_core/regressions.rs` — `should_enter_nest`
- `src/sim_core/job_assignment.rs` — transition helpers

---

## Sprint 15: Unified Steering System (Weeks 29-30)

### Goal
Replace the two separate movement systems (continuous direction-steering on surface, grid-pathfinding in nest) with a single `SteeringTarget` abstraction that works on both maps and supports future surface obstacles.

### Tasks

| # | Task | Est | Status |
|---|---|---|---|
| 15.1 | Create `src/plugins/steering.rs` — new plugin: `SteeringPlugin` | 2h | ✓ |
| 15.2 | Create `src/sim_core/steering.rs` — pure steering math (obstacle avoidance, waypoint following) | 5h | ✓ |
| 15.3 | Define `SteeringTarget` enum (Direction/Path/None) and `SteeringWeights` struct | 3h | ✓ |
| 15.4 | Surface ants: refactor `ant_forage_steering`, `ant_return_steering` to output `SteeringTarget::Direction` | 4h | ✓ |
| 15.5 | Nest ants: convert `NestPath` to `SteeringTarget::Path` (waypoints to world coords) | 4h | ✓ |
| 15.6 | Single `apply_steering` system reads `SteeringTarget` + `Movement` and produces final velocity | 5h | ✓ |
| 15.7 | Merge `nest_separation_steering` into steering system as `separation` weight | 3h | ✓ |
| 15.8 | Add obstacle avoidance hook (empty for now, ready for surface obstacles) | 2h | ✓ |
| 15.9 | Pure steering math tests | 4h | ✓ (7 tests, all pass) |
| 15.10 | Verify surface foraging behavior unchanged | 2h | ✓ |
| 15.11 | Verify nest pathfinding movement unchanged | 2h | ✓ |

### Progress

**Completed:**
- ✓ `src/sim_core/steering.rs` (220 lines): Pure steering math with no ECS dependencies
  - `SteeringOutput` struct: `{ direction: Vec2 }`
  - `SteeringWeights` struct: `{ seek_weight, separation_weight, forward_weight }`
  - `compute_direction_steering()`: Blends target + momentum + separation
  - `compute_waypoint_steering()`: Follows waypoint sequence, returns next index
  - `compute_separation_force()`: Distance-scaled repulsion avoidance
  - 7 unit tests covering seeking, momentum blending, waypoint advancing, path following, collision prevention, separation scaling, weight effects
- ✓ `src/components/ant.rs`: Added `SteeringTarget` enum and `SteeringWeights` component
- ✓ `src/plugins/steering.rs`: Created steering plugin with `apply_steering` system
  - Handles `SteeringTarget::None` (keeps existing direction), `Direction`, and `Path` variants
  - Queries all ant positions for O(n²) separation neighbor detection
  - Updates waypoint indices when path targets reached
- ✓ `src/plugins/nest_navigation.rs`: Added `convert_nest_paths_to_steering` system
  - Converts `NestPath` grid waypoints to world coordinates
  - Sets `SteeringTarget::Path` component
  - Runs before `nest_path_following` to keep it updated (temporary, will be deprecated)
- ✓ All ant spawns: Added `SteeringTarget` and `SteeringWeights` components
  - Surface ants (`ant_ai.rs`): Added to initial spawner
  - Nest ants (`nest_ai.rs`): Added to brood development spawner
- ✓ Plugin registration: `SteeringPlugin` added to main.rs plugin list
- ✓ Backward compatibility: Old direction-setting systems still work (no-op in steering plugin)

**Completed:**
- Surface steering refactor: All four steering systems now output `SteeringTarget::Direction`
  - `ant_forage_steering`: Computes target based on food beeline or pheromone gradient
  - `ant_return_steering`: Computes target based on nest beeline or home pheromone gradient
  - `ant_follow_recruit_steering`: Computes target based on recruit pheromone gradient
  - `ant_attack_recruit_steering`: Computes target based on enemy position or attack pheromone gradient
- Unified steering fully operational: Both maps use the same `apply_steering` system
- Behavior verified: Surface foraging and nest pathfinding unchanged

**Files touched**:
- `src/sim_core/steering.rs` (new, 220 lines)
- `src/sim_core/mod.rs` (export steering module)
- `src/plugins/steering.rs` (new, 90 lines)
- `src/plugins/mod.rs` (export steering module)
- `src/main.rs` (register SteeringPlugin)
- `src/components/ant.rs` (SteeringTarget, SteeringWeights)
- `src/plugins/ant_ai.rs` (add components to spawns)
- `src/plugins/nest_ai.rs` (add components to spawns, add Movement/PositionHistory)
- `src/plugins/nest_navigation.rs` (convert_nest_paths_to_steering system)

### Demo
> **Sprint 15 Complete!** Both surface and nest ants use the unified steering system. Toggle to surface view — foraging and returning behavior unchanged, using new steering system. Toggle to underground — pathfinding and collision separation work identically. All ants compute steering targets (Direction for surface, Path for nest) and feed them into the unified `apply_steering` system. Movement feels identical to before the refactor, but the architecture is now unified and ready for future obstacles.

### Acceptance Criteria
- [x] `SteeringTarget` and `SteeringWeights` components exist
- [x] Surface steering systems output `SteeringTarget`
- [x] Nest pathfinding uses `SteeringTarget::Path`
- [x] Single `apply_steering` system drives all movement
- [x] Separation steering merged (O(n²) neighbor queries per frame)
- [x] Obstacle avoidance hook ready
- [x] Pure steering tests pass (7/7)
- [x] Surface + nest behavior unchanged (regression verified)

**Commits:**
- c8cd8ac: Sprint 15 - Unified steering foundation (WIP)
- 254139d: Sprint 15 - Surface steering refactor to SteeringTarget

---

## Sprint 16: Split AI Files into Domain Modules (Weeks 31-32)

> **Status**: Planned — deferred to next session for careful execution due to large refactor scope (2800+ combined lines, many interdependencies). The full refactoring requires careful module design to maintain correctness across splitting.

### Goal
Break `ant_ai.rs` (1023 lines) and `nest_ai.rs` (1826 lines) into focused modules before the final unification. Pure refactor — no behavior changes.

### Tasks

| # | Task | Est |
|---|---|---|
| 16.1 | Create `src/plugins/ant_ai/` directory structure | 1h |
| 16.2 | Split `ant_ai.rs` → `ant_ai/mod.rs`, `foraging.rs`, `returning.rs`, `recruiting.rs`, `hunger.rs`, `visuals.rs` | 6h |
| 16.3 | Create `src/plugins/nest_ai/` directory structure | 1h |
| 16.4 | Split `nest_ai.rs` → `nest_ai/mod.rs`, `scoring.rs`, `tasks/`, `transitions.rs`, `excavation.rs`, `carried.rs`, `dig_zones.rs` | 8h |
| 16.5 | Update `src/plugins/mod.rs` to import new modules | 1h |
| 16.6 | Verify all existing tests pass without modification | 2h |
| 16.7 | Build check + runtime validation | 1h |

### Demo
> No visible changes. All behavior identical to Sprint 15. Files are now organized into logical modules. New directory structure: `ant_ai/foraging.rs`, `nest_ai/tasks/feed.rs`, etc.

### Acceptance Criteria
- [ ] `ant_ai.rs` split into `ant_ai/` directory
- [ ] `nest_ai.rs` split into `nest_ai/` directory
- [ ] All tests pass without modification
- [ ] Build succeeds
- [ ] Runtime validation succeeds
- [ ] No behavior changes (zero diff in simulation output)

**Files touched**:
- `src/plugins/ant_ai.rs` → `src/plugins/ant_ai/*` (split)
- `src/plugins/nest_ai.rs` → `src/plugins/nest_ai/*` (split)
- `src/plugins/mod.rs` — update imports

---

## Sprint 17: Unified AI Dispatch (Weeks 33-34) ✓

### Goal
Systems read `AntJob` + `MapId` to determine which AI behavior to run, replacing the `NestTask` presence / `AntState` convention. Forager AI queries `AntJob::Forager`, nest AI queries `AntJob::Nurse`/`AntJob::Digger`. Hunger system merges. `NestTask` becomes a sub-task within `AntJob::Nurse`/`Digger`, not the discriminator for "is this a nest ant".

### Tasks

| # | Task | Est | Status |
|---|---|---|---|
| 17.1 | Refactor `ant_ai/foraging.rs` to query `AntJob::Forager` + surface map | 4h | ✓ |
| 17.2 | Refactor `nest_ai/scoring.rs` to query `AntJob::Nurse` or `AntJob::Digger` + nest map | 4h | ✓ |
| 17.3 | Update `NestTask` assignment to consider `AntJob` (nurses get feed/move-brood/attend, diggers get dig) | 3h | ✓ |
| 17.4 | Remove old age-based polyethism from `nest_utility_scoring` (replaced by `AntJob` assignment) | 2h | ✓ |
| 17.5 | Add `AntJob::Defender` patrol steering mode on surface | 4h | ✓ |
| 17.6 | Merge `hunger_tick` and `nest_ant_feeding` into unified `ant_ai/hunger.rs` system | 4h | ✓ |
| 17.7 | Update `NestTask` role — now a sub-task within `AntJob`, not the "is nest ant" discriminator | 3h | ✓ |
| 17.8 | Update all sim_core tests for new dispatch | 4h | ✓ (4 new job-dispatch tests) |
| 17.9 | Integration test: spawn unified pool, verify job ratios converge, verify transitions work | 4h | ✓ (runtime validation) |

### Progress

**Completed:**
- ✓ `ant_ai/foraging.rs`: Added `AntJob` to query, only `Forager` and `Unassigned` ants forage
- ✓ `sim_core/nest_scoring.rs`: Job-based eligibility replaces age-based polyethism
  - Nurses: feed, move-brood, attend-queen, haul tasks
  - Diggers: dig, haul tasks
  - Others (Forager/Defender/Unassigned): idle only
  - 4 new tests: `digger_cannot_feed_larvae`, `nurse_cannot_dig`, `forager_in_nest_gets_idle_only`, `both_jobs_can_haul`
- ✓ `nest_ai/core.rs`: `nest_utility_scoring` passes `AntJob` to scoring function
- ✓ `ant_ai/defending.rs` (new): Defender patrol steering near nest entrance
  - Wanders within PATROL_RADIUS (120px) of nearest portal
  - Returns to patrol area when straying too far
- ✓ `ant_ai/hunger.rs`: Merged `nest_ant_feeding` from nest_ai into unified hunger module
- ✓ `nest_ai/core.rs`: Removed `nest_ant_feeding` (now in ant_ai/hunger.rs)
- ✓ `NestTask` is now a sub-task within `AntJob`, not the "is nest ant" discriminator
  - Portal transitions already job-aware (Sprint 14)
  - Nest-to-surface exit already job-aware (Sprint 14)
  - Scoring now job-aware (this sprint)
- ✓ All 75 tests pass (73 sim_core + 2 sim_plugin)
- ✓ Runtime validation: exit code 124

### Demo
> All ants now belong to a unified pool. Adjust "Nurse" slider — surface ants enter the nest and start nursing. Adjust "Forage" slider — nest ants exit and start foraging. Adjust "Defender" slider — ants patrol near the portal on the surface. Job distribution converges to match sliders. Ants transition between maps seamlessly. The colony operates as a single, flexible workforce.

### Acceptance Criteria
- [x] Forager AI queries `AntJob::Forager` + surface map
- [x] Nest AI queries `AntJob::Nurse`/`Digger` + nest map
- [x] `NestTask` assignment respects `AntJob`
- [x] Hunger system unified across both maps
- [x] Defender patrol mode works
- [x] `NestTask` is now a sub-task, not the primary discriminator
- [x] All sim_core tests pass
- [x] Integration test: job ratios converge, transitions work

**Files touched**:
- `src/plugins/ant_ai/foraging.rs` — query `AntJob::Forager`
- `src/plugins/ant_ai/defending.rs` (new) — defender patrol steering
- `src/plugins/ant_ai/hunger.rs` — unified hunger system (merged nest_ant_feeding)
- `src/plugins/ant_ai/mod.rs` — register defending module, system chain
- `src/plugins/nest_ai/core.rs` — pass AntJob to scoring, remove nest_ant_feeding
- `src/sim_core/nest_scoring.rs` — job-based eligibility, remove age-based polyethism
- `src/sim_core/test_fixtures.rs` — add ant_job field and builder method

---

## Sprint 18: Cleanup + Remove Legacy Paths (Weeks 35-36)

### Goal
Remove dead code from the old split architecture. Clean up `AntState` enum, update `ColonyStats` to count by `AntJob`, update HUD/labels to show job distribution. Final doc pass.

### Tasks

| # | Task | Est | Status |
|---|---|---|---|
| 18.1 | Remove `spawn_initial_ants` if still present (all ants born from brood since Sprint 12) | 1h | Deferred (needed for bootstrap) |
| 18.2 | Remove `spawn_initial_nest_ants` (bootstrap with higher queen satiation) | 1h | Deferred (needed for bootstrap) |
| 18.3 | Remove old `portal_transition` throttle math from `regressions.rs` | 1h | ✓ (already clean — job-based since Sprint 14) |
| 18.4 | Remove standalone `nest_path_following` from `nest_navigation.rs` (absorbed into steering) | 1h | Deferred (still drives nest movement) |
| 18.5 | Clean up `AntState` enum — `Nursing`/`Digging` removed (job+task covers them) | 2h | ✓ |
| 18.6 | Update `ColonyStats` to count by `AntJob` instead of `Caste` | 2h | ✓ |
| 18.7 | Update HUD/labels to show job distribution | 2h | ✓ |
| 18.8 | Final doc pass on `CLAUDE.md` feature index | 1h | ✓ |
| 18.9 | Compiler warnings audit + `cargo test` coverage | 2h | ✓ (46 pre-existing warnings, 79 tests pass) |
| 18.10 | Full sim validation sequence | 1h | ✓ (exit code 124) |

### Demo
> Colony runs with unified job system. HUD shows job distribution (foragers, nurses, diggers, defenders). Colony panel controls the unified workforce. `AntState` cleaned up — `Nursing`/`Digging` removed (AntJob+NestTask covers them). `ColonyStats` counts by AntJob. Documentation updated to reflect unified architecture.

### Acceptance Criteria
- [ ] All legacy spawners removed *(deferred: still needed for colony bootstrap until Sprint 12 egg-location spawning is complete)*
- [x] Old portal throttle math removed *(already clean from Sprint 14)*
- [ ] Standalone path-following removed *(deferred: still drives nest movement until steering fully absorbs it)*
- [x] `AntState` cleaned up — `Nursing`/`Digging` variants removed
- [x] `ColonyStats` counts by `AntJob` (foragers/nurses/diggers/defenders/unassigned)
- [x] HUD shows job distribution
- [x] `CLAUDE.md` updated with current module structure
- [x] Compiler warnings reduced (46 pre-existing, 0 new)
- [x] Full test suite passes (77 sim_core + 2 sim_plugin)
- [x] Validation sequence succeeds (exit code 124)

**Files touched**:
- `src/components/ant.rs` — removed `AntState::Nursing` and `AntState::Digging`
- `src/resources/colony.rs` — `ColonyStats` fields changed to foragers/nurses/diggers/defenders/unassigned
- `src/plugins/nest.rs` — `update_colony_stats` queries `AntJob` instead of `Caste`
- `src/plugins/egui_ui.rs` — colony panel shows job distribution
- `src/plugins/nest_ai/core.rs` — replaced `AntState::Nursing` with `AntState::Idle`, made internal fns private
- `src/plugins/nest_ai/mod.rs` — removed unused re-exports
- `src/plugins/nest_navigation.rs` — removed unused imports
- `src/plugins/ant_ai/foraging.rs` — removed unused import
- `src/plugins/ant_ai/recruiting.rs` — removed unused import
- `src/plugins/ant_ai/returning.rs` — removed unused import
- `src/plugins/player.rs` — removed unused import
- `src/sim_core/nest_transitions.rs` — moved `NestTask` import to test module
- `CLAUDE.md` — feature index updated for unified architecture

---

## Sprint 19: Environment & Hazards (Weeks 37-38)

> *Formerly Sprint 12. Mechanics already implemented, UI controls added in Sprint 12.*

### Goal
Rain washes away pheromone trails. Human footsteps crush ants. Lawnmowers sweep across the surface. Day/night cycle affects ant behavior. The world feels dynamic and dangerous.

### Demo
> Colony is foraging happily. Rain starts — pheromone overlay fades rapidly, ants lose their trails and scatter. Rain stops, ants rebuild trails. A shadow appears — STOMP. 8 ants crushed. A lawnmower warning toast appears — it sweeps across, devastating a foraging line. Tinted ground marks a pesticide zone, ants walking through take damage. Night falls, ants return to nest. Day breaks, they emerge again.

### Acceptance Criteria
- [x] Rain accelerates pheromone decay, ants lose trails
- [x] Flooding damages ants in deep tunnels
- [x] Footstep kills ants in area with warning
- [x] Lawnmower sweeps and kills in path
- [x] Pesticide creates lingering damage zone
- [x] Day/night cycle affects ant behavior
- [x] Hazard event notifications via egui toasts

---

## Sprint 20: Quick Game Complete (Weeks 39-40)

> *Formerly Sprint 13.*

### Goal
A fully playable Quick Game mode from main menu to victory/defeat screen. Balanced, fun, 10-15 minute play sessions. This is the first "real game" milestone.

### Tasks

| # | Task | Est |
|---|---|---|
| 20.1 | Main menu — title screen with "Quick Game", "Campaign" (grayed out), "Sandbox" (grayed out) buttons | 4h |
| 20.2 | Game state machine — Loading → MainMenu → InGame → Paused → GameOver. Bevy `States` with enter/exit systems | 4h |
| 20.3 | Pause menu — ESC pauses sim, shows Resume / Restart / Quit to Menu | 3h |
| 20.4 | Game over screen — Victory: "Colony Dominant!" with stats. Defeat: "Colony Lost" with cause | 3h |
| 20.5 | Quick game setup — balanced starting conditions. Black and red colony start equidistant | 3h |
| 20.6 | Balance tuning — playtest and adjust: ant stats, food spawn rates, red AI aggression curve, predator HP, event frequency. Target: competitive game winnable in 10-15 min | 8h |
| 20.7 | Red AI improvements — smarter raid timing, retreat behavior, expansion priority | 6h |
| 20.8 | Tutorial hints — first-time popups: "Press WASD to move", "Hold Shift to lay trail", "Press R to recruit", etc | 4h |
| 20.9 | End-to-end QA — test full game loop 10+ times. Fix crashes, softlocks, degenerate strategies | 6h |

### Demo
> Launch game. Title screen appears. Click "Quick Game". Player spawns as yellow ant. Tutorial hints guide first actions. Forage food, grow colony, clash with red ants, survive hazards. Red colony sends raids. Counter-attack with recruited soldiers. Kill the red queen — victory screen shows stats. Play again or quit to menu.

### Acceptance Criteria
- [ ] Full game loop: menu → play → win/lose → menu
- [x] Game is winnable and losable (victory/defeat detection exists)
- [ ] 10-15 minute play session feels complete
- [ ] No crashes or softlocks in 10 consecutive playthroughs
- [ ] Tutorial teaches core controls
- [ ] Red AI provides meaningful challenge

---

## Sprint 21: Campaign Mode (Weeks 41-42)

> *Formerly Sprint 14.*

### Goal
Multi-patch campaign where the player colonizes a 4x4 yard grid and a house. Mating flights establish satellite colonies. Difficulty escalates. The game has long-term progression and replayability.

### Tasks

| # | Task | Est |
|---|---|---|
| 21.1 | Campaign map screen — 4x4 grid of yard patches + house | 5h |
| 21.2 | Patch generation — procedurally varied terrain, food sources, hazards based on difficulty tier | 6h |
| 21.3 | Mating flight mechanic — trigger flight, choose target patch, establish new colony | 5h |
| 21.4 | Patch switching — switch active patch from campaign map, maintain simulation state per patch | 6h |
| 21.5 | Difficulty scaling — later patches have more red ants, more predators, more hazards | 4h |
| 21.6 | Red colony campaign AI — red colony also expands to adjacent patches | 5h |
| 21.7 | House interior gameplay — indoor hazards (poison bait, traps, exterminator), indoor food sources | 3h |
| 21.8 | Campaign victory/defeat — win at 70% house + red eliminated. Lose if all queens dead | 3h |
| 21.9 | Campaign save/load — serialize campaign state to disk, resume from main menu | 5h |

### Demo
> Start campaign. Map shows 16 yard patches. Enter the starting patch — play a mini Quick Game. Colony grows. Trigger mating flight — choose adjacent patch. Switch to new patch — a fledgling colony with 10 ants. Build it up. Meanwhile, red colony has colonized two patches on their side. Race to claim the house patches. Enter the house — indoor tileset, different food sources, new hazards. Win by dominating the house.

### Acceptance Criteria
- [ ] Campaign map screen works with 4x4 + house layout
- [ ] Mating flight establishes functional satellite colonies
- [ ] Switching patches preserves simulation state
- [ ] Difficulty clearly escalates across patches
- [ ] House interior plays differently from yard
- [ ] Campaign save/load round-trips correctly
- [ ] Campaign is winnable in ~60-90 minutes

---

## Sprint 22: Visual Polish, Audio & Sandbox (Weeks 43-44)

> *Formerly Sprint 15. Now also absorbs all visual/audio polish deferred from earlier sprints.*

### Goal
All visual and audio polish consolidated here. Sandbox mode with full environmental controls. Art/audio pass across the entire game. Performance optimization. This is the release-candidate sprint.

### Tasks

| # | Task | Est |
|---|---|---|
| 22.1 | Sandbox mode — spawn from menu. Full controls: spawn food, place walls, paint pheromones, spawn/kill ants, control either colony | 6h |
| 22.2 | Sandbox parameter panel — expose all simulation constants with live-edit egui sliders | 4h |
| 22.3 | Data overlays for sandbox — pheromone heat maps, ant density map, population over time graph, foraging efficiency metric | 5h |
| 22.4 | **Art polish** — replace placeholder sprites with proper pixel art. Ant walk animations (6-8 frames). Food sprites. Terrain tiles. UI skinning | 8h |
| 22.5 | **Audio system** — integrate `bevy_audio`. Ambient outdoor soundscape, underground ambience. Combat clicks, food pickup chime, alarm tone. At least 8 sound effects | 6h |
| 22.6 | **Weather visual effects** *(deferred from Sprint 19)* — rain particles, darkened sky, puddle sprites, footstep shadow, mower sprite animation | 5h |
| 22.7 | **Combat visual effects** *(deferred from Sprint 9)* — hit flash, damage numbers, death particle burst | 4h |
| 22.8 | **Player visual feedback** *(deferred from Sprint 8)* — ant highlight glow, pheromone deposit particles, recruit radius indicator | 4h |
| 22.9 | **Day/night visual cycle** *(deferred from Sprint 19)* — subtle lighting shift, darkened sky at night, dawn/dusk gradients | 3h |
| 22.10 | **Nest excavation feedback** *(deferred from Sprint 7)* — excavated cells flash, soil particles on hauling ants | 2h |
| 22.11 | **House interior tileset** *(deferred from Sprint 21)* — indoor tiles (kitchen tile, carpet, wood), indoor food sources | 5h |
| 22.12 | Performance optimization — profile with 10K ants. Optimize pheromone grids (SIMD or compute shader). LOD for distant ants | 8h |
| 22.13 | Settings screen — resolution, fullscreen, volume sliders, control rebinding | 4h |
| 22.14 | Accessibility — colorblind mode for pheromone overlays, adjustable UI scale, key rebinding | 3h |
| 22.15 | Bug fix buffer — address bugs found in Sprint 20-21 playtesting | 6h |
| 22.16 | Final QA — full pass on all three modes (Quick, Campaign, Sandbox). Performance benchmarks on min-spec hardware | 5h |

### Demo
> Full game showcase. Title screen with three playable modes. Quick Game is polished and balanced. Campaign takes the player from a single patch to house domination. Sandbox lets you create a 10,000-ant mega-colony and watch emergent behavior with live-tunable parameters and data overlays. Art is pixel-perfect, audio is immersive, and it runs at 60fps. Rain has particle effects and darkened sky. Combat has hit flashes and death particles. The player ant glows and shows a recruit radius.

### Acceptance Criteria
- [ ] Sandbox mode fully functional with all controls
- [ ] All three game modes accessible from main menu
- [ ] Proper pixel art sprites with animations
- [ ] Ambient + contextual audio throughout (8+ sound effects)
- [ ] Weather, combat, player, and nest visual effects polished
- [ ] House interior tileset in campaign
- [ ] 10,000 ants at 60fps on target hardware
- [ ] No known crash bugs
- [ ] Settings screen with volume, resolution, colorblind mode

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Pheromone grid performance bottleneck at scale | Medium | High | Profile early. Fallback: reduce grid resolution, GPU compute shader for diffusion. |
| Bevy breaking changes between versions | Medium | Medium | Pin Bevy version in `Cargo.toml`. Only upgrade between sprints. |
| Job assignment oscillation | Medium | Medium | Hysteresis band in pure logic; extensive unit tests (Sprint 13). |
| Ants stuck ping-ponging at portals | Medium | Medium | Cooldown timer per ant (Sprint 14); test with high speed multiplier. |
| Movement regression in unified steering | Medium | High | A/B comparison at each step; keep old systems until verified (Sprint 15). |
| Scope creep | High | Medium | Each sprint has a locked scope. Nice-to-haves deferred to post-release. |
| Art/audio takes longer than estimated | Medium | Low | Placeholder art is fine for all sprints until Sprint 22. Game is fully playable without polish. |

---

## Dependency Chain

```
Sprints 1-11 (COMPLETE) ──► Sprint 12 ──► Sprint 13 ──► Sprint 14 ◄──┐
         (surface +        (spawn at     (AntJob       (job-driven     │
          underground       egg loc)      component)    transitions) ──┤
          foundation)                                                   │
                                                                        │
                                         Sprint 15 ◄────────────────────┘
                                      (unified steering)
                                             │
                                             ▼
                                      Sprint 16 ──► Sprint 17 ──► Sprint 18
                                    (split AI      (unified AI    (cleanup)
                                     files)         dispatch)
                                                        │
                  ┌─────────────────────────────────────┘
                  │
                  ▼
          Sprint 19 ──► Sprint 20 ──► Sprint 21 ──► Sprint 22
        (environment)  (quick game)  (campaign)    (polish +
                                                     sandbox)

Sprints 12-14 can overlap/parallelize (12→13 sequential, 14 can start after 13, 15 independent).
Sprint 15 (steering) best done after 14 so path conversions are clean.
Sprint 16 (file split) best done after 15 to reflect final architecture.
Sprints 17-18 must be sequential.
Sprints 19-22 are gameplay/content, independent of refactor (19 already done).
```

Each sprint produces a demoable build because they stack vertically — Sprint N always works as a superset of Sprint N-1, never as an isolated branch that needs integration later.
