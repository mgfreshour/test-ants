use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Caste {
    Worker,
    Soldier,
    Queen,
    Drone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AntState {
    Idle,
    Foraging,
    Returning,
    Defending,
    Fighting,
    Fleeing,
    Following,
    Attacking,
}

/// Persistent job assignment. Determines role within the colony.
/// Sprint 13 foundation: attached to all ants, maintained by job_assignment_system.
/// Read by AI systems starting in Sprint 17.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntJob {
    Forager,
    Nurse,
    Digger,
    Defender,
    Unassigned,
}

impl AntJob {
    /// Returns true if this job works underground (in nest).
    pub fn is_underground_job(self) -> bool {
        matches!(self, AntJob::Nurse | AntJob::Digger)
    }

    /// Returns true if this job works on surface.
    pub fn is_surface_job(self) -> bool {
        matches!(self, AntJob::Forager | AntJob::Defender | AntJob::Unassigned)
    }
}

/// Per-ant stimulus response thresholds for nest AI.
/// Lower values = responds more easily to that stimulus type.
#[derive(Component, Debug, Clone, Copy)]
pub struct StimulusThresholds {
    pub feed_larva: f32,
    pub move_brood: f32,
    pub haul_food: f32,
    pub attend_queen: f32,
    pub dig: f32,
}

impl StimulusThresholds {
    /// Create thresholds from job using pure logic in sim_core.
    pub fn from_job(job: AntJob) -> Self {
        let t = crate::sim_core::nest_stimuli::default_thresholds(job);
        Self {
            feed_larva: t.feed_larva,
            move_brood: t.move_brood,
            haul_food: t.haul_food,
            attend_queen: t.attend_queen,
            dig: t.dig,
        }
    }
}


#[derive(Component)]
pub struct Ant {
    pub caste: Caste,
    pub state: AntState,
    pub age: f32,
    pub hunger: f32,
}

#[derive(Component)]
pub struct Movement {
    pub speed: f32,
    pub direction: Vec2,
}

/// Steering target abstraction for unified movement system
#[derive(Component, Debug, Clone)]
pub enum SteeringTarget {
    /// No steering (idle ant)
    None,
    /// Direction-based steering: seek toward pre-computed target direction
    Direction { target: Vec2 },
    /// Path-based steering: follow waypoint sequence
    Path { waypoints: Vec<Vec2>, index: usize },
}

impl Default for SteeringTarget {
    fn default() -> Self {
        Self::None
    }
}

/// Steering behavior weights as a component
#[derive(Component, Debug, Clone, Copy)]
pub struct SteeringWeights {
    pub seek_weight: f32,           // Target seeking strength
    pub separation_weight: f32,     // Collision avoidance strength
    pub forward_weight: f32,        // Momentum preservation
}

impl Default for SteeringWeights {
    fn default() -> Self {
        Self {
            seek_weight: 1.0,
            separation_weight: 0.5,
            forward_weight: 0.6,
        }
    }
}

/// Cooldown timer preventing an ant from using a portal again too soon.
/// Inserted on portal transition, ticked down each frame, removed at zero.
#[derive(Component)]
pub struct PortalCooldown {
    pub remaining: f32,
}

impl PortalCooldown {
    pub const DURATION: f32 = 3.0;

    pub fn new() -> Self {
        Self { remaining: Self::DURATION }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageSource {
    EnemyAnt,
    Spider,
    Player,
    Antlion,
    Pesticide,
    Lawnmower,
    Footstep,
    Starvation,
    Flood,
    QueenStarvation,
}

impl std::fmt::Display for DamageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnemyAnt => write!(f, "enemy ant"),
            Self::Spider => write!(f, "spider"),
            Self::Player => write!(f, "player"),
            Self::Antlion => write!(f, "antlion"),
            Self::Pesticide => write!(f, "pesticide"),
            Self::Lawnmower => write!(f, "lawnmower"),
            Self::Footstep => write!(f, "footstep"),
            Self::Starvation => write!(f, "starvation"),
            Self::Flood => write!(f, "flood"),
            Self::QueenStarvation => write!(f, "queen starvation"),
        }
    }
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub last_damage_source: Option<DamageSource>,
}

#[derive(Component)]
pub struct ColonyMember {
    pub colony_id: u32,
}

/// Ring buffer of recent positions so ants avoid backtracking.
/// Stores the last N positions and exposes an anti-backtrack direction.
const HISTORY_LEN: usize = 16;

#[derive(Component)]
pub struct PositionHistory {
    buf: [Vec2; HISTORY_LEN],
    idx: usize,
    count: usize,
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self {
            buf: [Vec2::ZERO; HISTORY_LEN],
            idx: 0,
            count: 0,
        }
    }
}

impl PositionHistory {
    pub fn record(&mut self, pos: Vec2) {
        self.buf[self.idx] = pos;
        self.idx = (self.idx + 1) % HISTORY_LEN;
        if self.count < HISTORY_LEN {
            self.count += 1;
        }
    }

    /// Returns a unit vector pointing AWAY from the centroid of recent positions,
    /// giving the ant forward momentum and preventing U-turns.
    pub fn anti_backtrack(&self, current_pos: Vec2) -> Vec2 {
        if self.count < 2 {
            return Vec2::ZERO;
        }
        let mut centroid = Vec2::ZERO;
        for i in 0..self.count {
            centroid += self.buf[i];
        }
        centroid /= self.count as f32;

        let away = current_pos - centroid;
        if away.length_squared() > 0.1 {
            away.normalize()
        } else {
            Vec2::ZERO
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.idx = 0;
    }
}

#[derive(Component)]
pub struct CarriedItem {
    pub food_amount: f32,
}

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct Follower;

/// Marker component for ants currently in the nest (underground).
#[derive(Component)]
pub struct Underground;

/// What trail, if any, this ant is currently following.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrailSense {
    #[default]
    Searching,
    FollowingFood,
    FollowingHome,
    FollowingAlarm,
    FollowingTrail,
    FollowingAttack,
    BeelineFood,
    BeelineNest,
}

impl Ant {
    pub fn new_worker() -> Self {
        Self {
            caste: Caste::Worker,
            state: AntState::Foraging,
            age: 0.0,
            hunger: 0.0,
        }
    }

    pub fn new_soldier() -> Self {
        Self {
            caste: Caste::Soldier,
            state: AntState::Defending,
            age: 0.0,
            hunger: 0.0,
        }
    }
}

impl Movement {
    pub fn with_random_direction(speed: f32, rng: &mut impl rand::Rng) -> Self {
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        Self {
            speed,
            direction: Vec2::new(angle.cos(), angle.sin()),
        }
    }
}

impl Health {
    pub fn apply_damage(&mut self, amount: f32, source: DamageSource) {
        self.current = (self.current - amount).max(0.0);
        self.last_damage_source = Some(source);
    }

    pub fn worker() -> Self {
        Self { current: 10.0, max: 10.0, last_damage_source: None }
    }

    pub fn soldier() -> Self {
        Self { current: 25.0, max: 25.0, last_damage_source: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_damage_records_source() {
        let mut h = Health::worker();
        assert_eq!(h.last_damage_source, None);

        h.apply_damage(3.0, DamageSource::Spider);
        assert_eq!(h.current, 7.0);
        assert_eq!(h.last_damage_source, Some(DamageSource::Spider));

        h.apply_damage(2.0, DamageSource::Starvation);
        assert_eq!(h.current, 5.0);
        assert_eq!(h.last_damage_source, Some(DamageSource::Starvation));
    }

    #[test]
    fn apply_damage_clamps_at_zero() {
        let mut h = Health::worker();
        h.apply_damage(999.0, DamageSource::Lawnmower);
        assert_eq!(h.current, 0.0);
        assert_eq!(h.last_damage_source, Some(DamageSource::Lawnmower));
    }

    #[test]
    fn damage_source_display() {
        assert_eq!(DamageSource::EnemyAnt.to_string(), "enemy ant");
        assert_eq!(DamageSource::QueenStarvation.to_string(), "queen starvation");
    }
}
