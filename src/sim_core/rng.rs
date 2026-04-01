use rand::rngs::{StdRng, ThreadRng};
use rand::{Rng, SeedableRng};

pub trait SimRng {
    fn unit_f32(&mut self) -> f32;
    fn range_f32(&mut self, min: f32, max: f32) -> f32;
}

pub struct ThreadSimRng {
    inner: ThreadRng,
}

impl Default for ThreadSimRng {
    fn default() -> Self {
        Self {
            inner: rand::thread_rng(),
        }
    }
}

impl SimRng for ThreadSimRng {
    fn unit_f32(&mut self) -> f32 {
        self.inner.gen::<f32>()
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        self.inner.gen_range(min..max)
    }
}

pub struct SeededSimRng {
    inner: StdRng,
}

impl SeededSimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
        }
    }
}

impl SimRng for SeededSimRng {
    fn unit_f32(&mut self) -> f32 {
        self.inner.gen::<f32>()
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        self.inner.gen_range(min..max)
    }
}

#[cfg(test)]
mod tests {
    use super::{SeededSimRng, SimRng};

    #[test]
    fn seeded_rng_is_deterministic() {
        let mut a = SeededSimRng::from_seed(42);
        let mut b = SeededSimRng::from_seed(42);

        for _ in 0..10 {
            assert_eq!(a.unit_f32(), b.unit_f32());
            assert_eq!(a.range_f32(-3.0, 7.0), b.range_f32(-3.0, 7.0));
        }
    }
}
