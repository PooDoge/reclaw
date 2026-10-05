//! When to try again and how long to wait. The delay grows with each attempt and is spread by a random factor so
//! many requests that failed together do not retry together.
use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    time::Duration,
};

/// The wait before retry number `attempt` (0 for the first retry): 0.5 s, 1 s, 2 s ... up to 8 s, scaled by
/// `jitter` in `0.0..=1.0` to between half and the whole of it.
pub fn delay(attempt: u32, jitter: f64) -> Duration {
    let base = Duration::from_millis(500).saturating_mul(1u32 << attempt.min(5));
    base.min(Duration::from_secs(8)).mul_f64(0.5 + 0.5 * jitter.clamp(0.0, 1.0))
}

/// A random number in `0.0..1.0` that needs no crate: the standard library seeds every `RandomState` differently.
pub fn jitter() -> f64 {
    let bits = RandomState::new().build_hasher().finish() >> 11;
    bits as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_double_and_stop_growing() {
        let full: Vec<u64> = (0..8).map(|a| delay(a, 1.0).as_millis() as u64).collect();
        assert_eq!(full, [500, 1000, 2000, 4000, 8000, 8000, 8000, 8000]);
        assert_eq!(delay(0, 0.0), Duration::from_millis(250), "jitter halves it at most");
        assert_eq!(delay(30, 1.0), Duration::from_secs(8), "a huge attempt number cannot overflow");
    }

    #[test]
    fn jitter_stays_in_range_and_varies() {
        let values: Vec<f64> = (0..50).map(|_| jitter()).collect();
        assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
        assert!(values.windows(2).any(|w| w[0] != w[1]), "not constant");
    }
}
