use crate::components::ant::AntJob;

/// Input for job assignment decisions.
#[derive(Debug, Clone)]
pub struct JobAssignmentInput {
    pub total_ants: usize,
    pub target_ratios: JobRatios,
    pub current_assignments: JobCounts,
}

/// Target job ratios from BehaviorSliders (sum should be close to 1.0).
#[derive(Debug, Clone, Copy)]
pub struct JobRatios {
    pub forage: f32,
    pub nurse: f32,
    pub dig: f32,
    pub defend: f32,
}

/// Current job distribution counts.
#[derive(Debug, Clone, Copy, Default)]
pub struct JobCounts {
    pub forager: usize,
    pub nurse: usize,
    pub digger: usize,
    pub defender: usize,
    pub unassigned: usize,
}

impl JobCounts {
    pub fn total(&self) -> usize {
        self.forager + self.nurse + self.digger + self.defender + self.unassigned
    }

    pub fn assigned(&self) -> usize {
        self.forager + self.nurse + self.digger + self.defender
    }

    pub fn get_count(&self, job: AntJob) -> usize {
        match job {
            AntJob::Forager => self.forager,
            AntJob::Nurse => self.nurse,
            AntJob::Digger => self.digger,
            AntJob::Defender => self.defender,
            AntJob::Unassigned => self.unassigned,
        }
    }
}

/// Age-based affinity weights for different jobs.
#[derive(Debug, Clone, Copy)]
pub struct JobAffinityWeights {
    pub forager: f32,
    pub nurse: f32,
    pub digger: f32,
    pub defender: f32,
}

/// Compute job affinity weights based on ant age.
///
/// Age ranges from 0 to ~300 seconds. Young ants prefer nursing,
/// middle-aged ants prefer digging, old ants prefer foraging.
pub fn compute_job_affinity(age: f32) -> JobAffinityWeights {
    let age_frac = (age / 300.0).clamp(0.0, 1.0);

    // Young (0-90s): nursing_affinity = 1.0, forager = 0.2
    // Middle (90-210s): digging_affinity = 1.2, nursing/forager = 0.6
    // Old (210-300s): forager = 1.0, nursing = 0.2
    let nursing_affinity = 1.0 - age_frac * 0.8;
    let forager_affinity = 0.2 + age_frac * 0.8;
    let digging_affinity = if age_frac > 0.15 && age_frac < 0.6 {
        1.2
    } else {
        0.4
    };
    let defender_affinity = 0.5; // Flat, no age preference

    JobAffinityWeights {
        forager: forager_affinity,
        nurse: nursing_affinity,
        digger: digging_affinity,
        defender: defender_affinity,
    }
}

/// Determine if an ant should be reassigned to a different job.
///
/// Uses hysteresis to prevent rapid oscillation: only reassign if the current
/// job's target ratio is outside the `[current_fraction - margin, current_fraction + margin]` band.
///
/// Returns `Some(new_job)` if reassignment is needed, `None` if the ant should keep its job.
pub fn should_reassign_ant(
    current_job: AntJob,
    age: f32,
    input: &JobAssignmentInput,
    hysteresis_margin: f32,
) -> Option<AntJob> {
    if input.total_ants == 0 {
        return None;
    }

    let affinity = compute_job_affinity(age);
    let counts = input.current_assignments;

    // Unassigned ants are always reassigned (they should not stay unassigned)
    if current_job == AntJob::Unassigned {
        // Find best job based on affinity and deficit
        return find_best_job(affinity, counts, input.target_ratios, input.total_ants)
            .filter(|job| *job != AntJob::Unassigned);
    }

    // For assigned ants, check if they're in their job's hysteresis band
    let target_frac = match current_job {
        AntJob::Forager => input.target_ratios.forage,
        AntJob::Nurse => input.target_ratios.nurse,
        AntJob::Digger => input.target_ratios.dig,
        AntJob::Defender => input.target_ratios.defend,
        AntJob::Unassigned => 0.0, // unreachable, handled above
    };

    let current_count = counts.get_count(current_job);
    let current_frac = current_count as f32 / input.total_ants as f32;

    let lower_band = (target_frac - hysteresis_margin).max(0.0);
    let upper_band = (target_frac + hysteresis_margin).min(1.0);

    // If current job is within hysteresis band, keep the ant in its current job
    if current_frac >= lower_band && current_frac <= upper_band {
        return None;
    }

    // Outside hysteresis band - find best job
    find_best_job(affinity, counts, input.target_ratios, input.total_ants)
        .filter(|job| *job != current_job)
}

/// Find the best job for an ant given affinity and current counts.
/// Returns the best job even if there's no deficit (for unassigned ants).
fn find_best_job(
    affinity: JobAffinityWeights,
    counts: JobCounts,
    target_ratios: JobRatios,
    total_ants: usize,
) -> Option<AntJob> {
    let job_scores = [
        (AntJob::Forager, affinity.forager, target_ratios.forage),
        (AntJob::Nurse, affinity.nurse, target_ratios.nurse),
        (AntJob::Digger, affinity.digger, target_ratios.dig),
        (AntJob::Defender, affinity.defender, target_ratios.defend),
    ];

    let mut best_job = None;
    let mut best_score = -1.0;

    for (job, affinity_weight, target_ratio) in &job_scores {
        let job_count = counts.get_count(*job);
        let job_frac = job_count as f32 / total_ants as f32;

        // Deficit: how far below target are we? (0 if already at or above target)
        let deficit = (target_ratio - job_frac).max(0.0);

        // Score = affinity * deficit.
        // If there's a deficit, high affinity + deficit = good job.
        // If no deficit exists (all at target), still pick the job with highest affinity.
        let score = if deficit > 0.0001 {
            affinity_weight * deficit // Prefer jobs with deficit + high affinity
        } else {
            affinity_weight * 0.1 // Small baseline so affinity still matters when no deficit
        };

        if score > best_score {
            best_score = score;
            best_job = Some(*job);
        }
    }

    best_job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_affinity_young_prefers_nursing() {
        let affinity = compute_job_affinity(30.0);
        assert!(affinity.nurse > 0.8);
        assert!(affinity.forager < 0.4);
    }

    #[test]
    fn age_affinity_old_prefers_foraging() {
        let affinity = compute_job_affinity(270.0);
        assert!(affinity.forager > 0.8);
        assert!(affinity.nurse < 0.4);
    }

    #[test]
    fn age_affinity_middle_prefers_digging() {
        let affinity = compute_job_affinity(150.0);
        assert!(affinity.digger > 1.0);
    }

    #[test]
    fn hysteresis_prevents_oscillation() {
        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.6,
                nurse: 0.2,
                dig: 0.1,
                defend: 0.1,
            },
            current_assignments: JobCounts {
                forager: 58,
                nurse: 20,
                digger: 10,
                defender: 10,
                unassigned: 2,
            },
        };
        // Current forager = 58%, target = 60%, within 5% margin → no change
        let result = should_reassign_ant(AntJob::Forager, 150.0, &input, 0.05);
        assert_eq!(result, None);
    }

    #[test]
    fn reassignment_when_outside_hysteresis_band() {
        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.6,
                nurse: 0.2,
                dig: 0.1,
                defend: 0.1,
            },
            current_assignments: JobCounts {
                forager: 45,
                nurse: 20,
                digger: 10,
                defender: 10,
                unassigned: 15,
            },
        };
        // Current forager = 45%, target = 60%, outside 5% margin (55-65%)
        // Old ant has high forager affinity, so best_job is Forager.
        // But current_job is already Forager, so returns None (no change needed).
        let result = should_reassign_ant(AntJob::Forager, 250.0, &input, 0.05);
        assert_eq!(result, None); // Old ant already in best job for them
    }

    #[test]
    fn unassigned_always_reassigned() {
        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.6,
                nurse: 0.2,
                dig: 0.1,
                defend: 0.1,
            },
            current_assignments: JobCounts {
                forager: 60,
                nurse: 20,
                digger: 10,
                defender: 10,
                unassigned: 0,
            },
        };
        // Unassigned should always be reassigned (even if all ratios are met)
        let result = should_reassign_ant(AntJob::Unassigned, 100.0, &input, 0.05);
        assert!(result.is_some());
    }

    #[test]
    fn young_ant_reassigns_away_from_forager() {
        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.5,
                nurse: 0.4,
                dig: 0.05,
                defend: 0.05,
            },
            current_assignments: JobCounts {
                forager: 50,
                nurse: 30,
                digger: 10,
                defender: 10,
                unassigned: 0,
            },
        };
        // Young ant as forager with forager deficit
        // Forager: 50% current, 50% target, outside 5% band (45-55%? no, exactly at 50%)
        // Let me recalc: band is [50 - 5, 50 + 5] = [45, 55], so 50% IS in band
        // So no reassignment happens. Let me modify the test to actually trigger reassignment.

        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.7,    // Increased forager target
                nurse: 0.2,
                dig: 0.05,
                defend: 0.05,
            },
            current_assignments: JobCounts {
                forager: 50,    // Currently only 50%, target is 70%
                nurse: 35,
                digger: 10,
                defender: 5,
                unassigned: 0,
            },
        };
        // Young ant (high nurse affinity) as forager when forager is at 50% < 70% target
        // Forager has deficit (70 - 50 = 20%). Nurse has no deficit (35 vs 20 target).
        // But young ant has high nurse affinity. However, nurse has NO deficit, so score is baseline.
        // Forager score = young_forager_affinity * 0.20. Nurse score = young_nurse_affinity * 0.0.
        // Actually forager ALSO has no deficit. Let me recalc input target ratios must sum to ~1.0.

        let input = JobAssignmentInput {
            total_ants: 100,
            target_ratios: JobRatios {
                forage: 0.4,
                nurse: 0.5,     // Nurse target is higher
                dig: 0.05,
                defend: 0.05,
            },
            current_assignments: JobCounts {
                forager: 50,    // Currently 50% but target is only 40%
                nurse: 30,      // Currently 30%, target is 50% (deficit = 20%)
                digger: 10,
                defender: 10,
                unassigned: 0,
            },
        };
        // Young ant as forager:
        // - Forager is OVER (50 vs 40 target), no deficit
        // - Nurse has deficit (50 vs 30 = 20%)
        // - Young ant has high nurse affinity
        // Expected: reassign to Nurse (has deficit + high affinity)
        let result = should_reassign_ant(AntJob::Forager, 20.0, &input, 0.05);
        assert_eq!(result, Some(AntJob::Nurse));
    }

    #[test]
    fn job_counts_total_and_assigned() {
        let counts = JobCounts {
            forager: 50,
            nurse: 20,
            digger: 10,
            defender: 5,
            unassigned: 15,
        };
        assert_eq!(counts.total(), 100);
        assert_eq!(counts.assigned(), 85);
    }
}
