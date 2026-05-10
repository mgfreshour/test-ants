# Overworld Navigation with Obstacles — Design Options

## Problem Statement

The surface world (2048x2048) currently uses continuous steering with no obstacle awareness. Adding dense obstacles (rocks, logs, water, terrain features) requires ants to navigate around them. The current system (additive force blending + boundary bounce) has no concept of impassable terrain beyond the world edges.

We want something between "open field" and "maze" — scattered obstacles that create corridors, chokepoints, and interesting foraging paths without requiring pixel-perfect pathfinding.

---

## Option A: Hybrid — Grid Collision + Continuous Steering

Keep the current continuous steering system but add a coarse grid overlay for obstacle detection and local avoidance.

### How it works
- Overlay a grid (e.g., 128x128 cells → 16px per cell) on the 2048x2048 surface
- Mark cells as passable/impassable based on obstacle placement
- Each frame, ants raycast 2-3 cells ahead in their movement direction
- If blocked, add a wall-avoidance force (perpendicular to the wall normal) to the steering blend
- No pathfinding — ants navigate reactively using wall-following behavior

### Steering changes
```
target_dir = (fwd * FORWARD_WEIGHT
    + perturbed_fwd * noise_scale
    + pheromone_bias
    + momentum
    + nest_bias
    + wall_avoidance * 1.5)   // <-- new, high priority
    .normalize_or_zero();
```

### Pros
- Minimal refactor — adds one force to existing blend
- Cheap per-frame cost (grid lookups, no heap allocation)
- Organic-looking movement (ants slide along walls, not rigid waypoint snapping)
- Pheromone system unchanged (trails form along discovered routes naturally)
- Works well for scattered obstacles and gentle corridors

### Cons
- Can get stuck in concave obstacles (U-shapes, dead ends) — no global planning
- No guarantee ants reach distant targets efficiently
- Wall-following heuristics need tuning (left-hand vs random choice)
- Doesn't help with "find the only path through a bottleneck"

### Stuck mitigation
- Track time-since-progress; if stuck > N seconds, pick random new direction
- Pheromones naturally guide followers away from dead ends (scouts that get stuck don't deposit strong trails)

---

## Option B: Grid Pathfinding (Like the Nest)

Replace continuous surface steering with a grid-based system identical to the underground nest.

### How it works
- Surface gets a `SurfaceGrid` resource (e.g., 128x128 or 256x256 cells)
- Ants pathfind with A* to their target (food, nest, pheromone peak)
- Path converted to waypoints → fed into existing `SteeringTarget::Path` system
- Exploration uses random walkable neighbor selection rather than angle perturbation

### Architecture
- Reuse `NestPathCache` pattern with generation-based invalidation
- Foraging without a target: pick random walkable cell within expanding radius
- Returning: pathfind to nest portal cell
- Following pheromone: pathfind to highest-gradient neighboring cell (re-evaluated periodically)

### Pros
- Guarantees reachability — ants never get stuck
- Handles arbitrarily complex obstacle layouts (mazes, narrow passages)
- Already proven in nest system — code reuse
- Deterministic, testable
- Easy to add terrain costs (mud = slow, road = fast)

### Cons
- Loses organic/natural movement feel (waypoint-to-waypoint is rigid)
- A* cost with many ants: 200 ants × re-pathing every few seconds = hundreds of A* queries/sec
- Exploration feels artificial — random target selection lacks the correlated random walk aesthetic
- Pheromone gradient following becomes awkward (pathfind to a grid cell vs smooth steering)
- Path cache invalidation needed if obstacles change dynamically

### Cost mitigation
- Hierarchical pathfinding (HPA*) or flow fields for common destinations (nest)
- Path smoothing (Catmull-Rom or string-pulling) to reduce rigidity
- Stagger re-pathing across frames (10-20 ants per frame)

---

## Option C: Flow Fields for Common Targets + Local Steering

Precompute flow fields (one per destination) that give each cell a "best direction" toward the target. Ants sample the flow field as a steering force.

### How it works
- Grid overlay (128x128 or 256x256)
- Compute flow fields for key destinations:
  - "To nest" (always maintained)
  - "To food source X" (computed when discovered, invalidated when depleted)
- Foraging ants blend: `flow_field_sample * weight + noise + pheromone + separation`
- Exploring ants (no target): use only noise + separation (ignore flow fields)
- Flow field = Dijkstra from destination → each cell stores best-direction vector

### Architecture
```rust
struct SurfaceFlowField {
    grid: Vec<Vec<Vec2>>,  // direction at each cell
    generation: u32,
}
```
- Recompute when obstacles change or food appears/depletes
- One BFS/Dijkstra per field (~16K cells for 128x128) = ~1ms each

### Pros
- O(1) per-ant per-frame — just sample the grid cell's direction
- Handles any obstacle layout perfectly (precomputed)
- Returning ants never get stuck (flow field guarantees convergence)
- Natural-looking movement (flow + noise = smooth curves around obstacles)
- Scales to thousands of ants with zero per-ant pathfinding cost
- Combines beautifully with existing steering system (just another force)

### Cons
- Memory: each flow field = 128×128×8 bytes = ~128KB (fine for a few fields)
- Recomputation cost when food appears/depletes (~1ms per field, can amortize)
- Doesn't handle per-ant unique targets well (e.g., "this specific ant's recruit target")
- Exploring ants still need local obstacle avoidance (flow field only helps goal-directed movement)
- More complex to implement than Option A

---

## Option D: Steering + Navmesh (Continuous Space with Precomputed Regions)

Use a navigation mesh that decomposes walkable space into convex polygons. Ants pathfind at the polygon level, then steer freely within each polygon.

### How it works
- Decompose surface into convex walkable regions (navmesh generation)
- When an ant needs a distant target, find polygon path (fast graph search)
- Within current polygon, steer freely with existing noise/pheromone system
- At polygon edge, transition to next polygon in path

### Pros
- True continuous movement within regions — most natural-looking
- Polygon-level pathfinding is very fast (small graph)
- Supports arbitrary obstacle shapes (not grid-aligned)
- Can represent large open areas as single polygons (efficient)

### Cons
- Navmesh generation is complex (need a decomposition algorithm or library)
- Harder to modify dynamically (if obstacles change)
- Overkill for grid-aligned obstacles
- No established Bevy navmesh crate at production quality
- Pheromone system (already grid-based) doesn't align naturally with polygon regions
- Significantly more implementation effort than other options

---

## Option E: Potential Fields (Repulsive Obstacles + Attractive Goals)

Every obstacle emits a repulsive potential; goals emit attractive potential. Ants follow the gradient.

### How it works
- Precompute a scalar potential field over the grid: obstacles = high values, goals = low values
- Each frame, ant samples gradient at its position → steering force
- Multiple potential fields overlaid (obstacle avoidance + nest attraction + food attraction)

### Pros
- Elegant, biologically plausible
- Smooth, natural-looking paths
- Trivially parallelizable
- Works with existing force-blending architecture

### Cons
- Local minima — ants get trapped between equally-repulsive obstacles
- Classic "narrow passage" problem (potential cancels at chokepoints)
- Must be combined with escape heuristics (random walk when stuck)
- Essentially equivalent to flow fields for single targets, but worse for multi-obstacle scenarios

---

## Recommendation

**Option C (Flow Fields) + Option A (Local Wall Avoidance)** as a combined approach:

| Behavior | Mechanism |
|----------|-----------|
| Exploring (no target) | Current steering + local wall avoidance (Option A) |
| Returning to nest | Sample "to-nest" flow field as steering force |
| Going to known food | Sample food-specific flow field |
| Following pheromone | Current pheromone gradient + local wall avoidance |
| Recruited/attacking | Flow field to target colony position |

### Why this combination
1. **Exploration stays organic** — noise + wall avoidance produces natural ant-like meandering around obstacles without needing a target
2. **Goal-directed movement is guaranteed** — flow fields ensure ants always reach the nest/food regardless of obstacle layout
3. **Scales perfectly** — flow field lookup is O(1) per ant; no per-ant pathfinding
4. **Pheromone system unchanged** — pheromone gradient following already works cell-by-cell; add wall avoidance to prevent walking into walls
5. **Incremental implementation** — start with wall avoidance only (works for scattered obstacles), add flow fields later for complex layouts
6. **Reuses existing architecture** — flow field direction feeds into `SteeringTarget::Direction` like any other force

### Implementation order
1. Add `SurfaceGrid` resource with obstacle cells
2. Add wall-avoidance force to foraging/returning/defending steering
3. Add "to-nest" flow field; returning ants sample it
4. Add per-food flow fields as food is discovered
5. Tune: flow field weight vs pheromone weight vs noise

### Grid sizing
- **128x128** (16px cells): good default — obstacles are 16px minimum, matches pheromone grid granularity
- **256x256** (8px cells): if we want finer obstacles, but 4× memory/compute for flow fields
- Match the pheromone grid resolution for simplicity
