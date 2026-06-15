#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    pub fn choose_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some((self.next_u64() as usize) % len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_rng_is_deterministic() {
        let mut left = SeededRng::new(42);
        let mut right = SeededRng::new(42);

        assert_eq!(left.next_u64(), right.next_u64());
        assert_eq!(left.next_u64(), right.next_u64());
    }

    #[test]
    fn choose_index_handles_empty_and_non_empty_inputs() {
        let mut rng = SeededRng::new(1);

        assert_eq!(rng.choose_index(0), None);
        assert!(rng.choose_index(3).is_some_and(|index| index < 3));
    }
}
