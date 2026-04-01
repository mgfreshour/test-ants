use crate::sim_core::nest_scoring::NestScoringInput;

pub struct NestScoringInputBuilder {
    input: NestScoringInput,
}

impl NestScoringInputBuilder {
    pub fn new() -> Self {
        Self {
            input: NestScoringInput {
                unfed_larvae: 0,
                unrelocated_brood: 0,
                has_food: false,
                colony_food_stored: 0.0,
                has_queen: false,
                queen_hunger: 0.0,
                brood_need: 0.0,
                queen_signal: 0.0,
                nearest_face_construction: 0.0,
                has_dig_faces: false,
                has_player_dig_zones: false,
                expansion_need: 0.0,
                current_diggers: 0,
                current_movers: 0,
                current_queen_attendants: 0,
                ant_age: 120.0,
            },
        }
    }

    pub fn larvae(mut self, unfed_larvae: usize) -> Self {
        self.input.unfed_larvae = unfed_larvae;
        self
    }

    pub fn food(mut self, stored: f32) -> Self {
        self.input.colony_food_stored = stored;
        self.input.has_food = stored > 0.5;
        self
    }

    pub fn queen(mut self, hunger: f32, signal: f32) -> Self {
        self.input.has_queen = true;
        self.input.queen_hunger = hunger;
        self.input.queen_signal = signal;
        self
    }

    pub fn move_brood(mut self, count: usize) -> Self {
        self.input.unrelocated_brood = count;
        self
    }

    pub fn dig_front(mut self, construction: f32, player_zone: bool) -> Self {
        self.input.has_dig_faces = true;
        self.input.nearest_face_construction = construction;
        self.input.has_player_dig_zones = player_zone;
        self
    }

    pub fn age(mut self, ant_age: f32) -> Self {
        self.input.ant_age = ant_age;
        self
    }

    pub fn mover_load(mut self, current_movers: usize) -> Self {
        self.input.current_movers = current_movers;
        self
    }

    pub fn build(self) -> NestScoringInput {
        self.input
    }
}
