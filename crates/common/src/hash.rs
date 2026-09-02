//! Deterministic integer/float hashing for frame-count-driven pseudo-random
//! values (serve angles, spawn jitter) — reproducible across runs, no RNG
//! state to thread through game structs.

/// Simple multiplicative hash (Knuth) for pseudo-random values from a seed
/// such as a frame counter.
pub fn hash_u32(seed: u32) -> u32 {
    seed.wrapping_mul(2654435761)
}

/// Map a seed to a float in `[0, 1)`.
pub fn hash_f32(seed: u32) -> f32 {
    (hash_u32(seed) >> 8) as f32 / 16777216.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_is_deterministic_in_unit_range_and_nearby_seeds_diverge() {
        let seeds = [0u32, 1, 7, 12345, u32::MAX];

        let first_pass: Vec<f32> = seeds.iter().map(|&seed| hash_f32(seed)).collect();
        let second_pass: Vec<f32> = seeds.iter().map(|&seed| hash_f32(seed)).collect();

        assert_eq!(first_pass, second_pass, "same seed must hash identically");
        for (seed, value) in seeds.iter().zip(&first_pass) {
            assert!((0.0..1.0).contains(value), "hash_f32({seed}) = {value} out of [0,1)");
        }
        assert_ne!(hash_f32(1), hash_f32(2), "adjacent seeds must not collide");
        assert_ne!(hash_u32(100), hash_u32(101), "adjacent seeds must not collide");
    }
}
