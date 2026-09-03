//! Flat-pool particle manager.
//!
//! Particles live in a fixed-capacity `Vec<Particle>`. The `alive` flag on
//! each slot marks whether the slot is occupied. Spawn walks the pool from a
//! cursor (round-robin); when the pool is full it overwrites the oldest slot,
//! which is the canonical "ring-buffer pool" pattern used by production
//! particle systems.
//!
//! Zero allocations per frame after construction.

use glam::{Vec2, Vec4};

use super::particle::{Particle, ParticleConfig};

/// Default pool capacity. Pong-scale games rarely need more than this.
pub const DEFAULT_CAPACITY: usize = 16_384;

/// Owns the live particle pool.
///
/// The pool is fixed-size; [`spawn_burst`](Self::spawn_burst) overwrites the
/// oldest slots when more particles are emitted than the capacity holds.
/// Use [`with_capacity`](Self::with_capacity) to size the pool to your needs.
pub struct ParticleManager {
    pool: Vec<Particle>,
    /// Cursor into the pool — the next slot considered for spawning.
    cursor: usize,
    /// Cheap LCG seed for direction / lifetime / speed jitter. Avoids pulling
    /// in `rand` as a workspace dep.
    rng_state: u64,
}

impl Default for ParticleManager {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl ParticleManager {
    /// Build a manager with a custom pool capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            pool: vec![Particle::default(); capacity],
            cursor: 0,
            // Seeded with a non-zero constant so behavior is reproducible
            // across runs unless [`reseed`](Self::reseed) is called.
            rng_state: 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// Override the deterministic seed — useful for tests.
    pub fn reseed(&mut self, seed: u64) {
        self.rng_state = seed.max(1);
    }

    /// Total slot count, alive + dead.
    pub fn capacity(&self) -> usize {
        self.pool.len()
    }

    /// Number of currently alive particles.
    pub fn alive_count(&self) -> usize {
        self.pool.iter().filter(|p| p.alive).count()
    }

    /// Mark every slot dead.
    pub fn clear(&mut self) {
        for slot in &mut self.pool {
            slot.alive = false;
        }
        self.cursor = 0;
    }

    /// Spawn a burst at `origin` using the parameters in `config`.
    ///
    /// If `config.count` exceeds the number of free slots in the pool, the
    /// oldest slots are overwritten. This is a deliberate trade — better to
    /// drop a few frames of stale dust than to allocate or stutter.
    pub fn spawn_burst(&mut self, origin: Vec2, config: &ParticleConfig) {
        let half_spread = config.spread_radians * 0.5;
        let base_angle = config.direction.y.atan2(config.direction.x);

        for _ in 0..config.count {
            let slot = self.next_slot();
            let lifetime = self.uniform_range(config.lifetime_range.0, config.lifetime_range.1).max(0.001);
            let speed = self.uniform_range(config.speed_range.0, config.speed_range.1);
            let jitter = self.uniform_range(-half_spread, half_spread);
            let angle = base_angle + jitter;
            let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
            let angular_velocity =
                self.uniform_range(config.angular_velocity_range.0, config.angular_velocity_range.1);

            self.pool[slot] = Particle {
                position: origin,
                velocity,
                acceleration: config.gravity,
                color_start: config.color_start,
                color_end: config.color_end,
                scale_start: config.scale_start,
                scale_end: config.scale_end,
                rotation: 0.0,
                angular_velocity,
                drag: config.drag,
                age: 0.0,
                lifetime,
                emissive: config.emissive,
                texture: config.texture,
                alive: true,
            };
        }
    }

    /// Advance every alive particle by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        for p in &mut self.pool {
            if !p.alive {
                continue;
            }
            p.age += dt;
            if p.age >= p.lifetime {
                p.alive = false;
                continue;
            }
            // Symplectic Euler: integrate velocity then position. Stable
            // enough for cosmetic particles even at large dt.
            p.velocity += p.acceleration * dt;
            if p.drag > 0.0 {
                // Exponential damping: v *= exp(-drag * dt). Use a cheap
                // 2-term approximation since dt is small.
                let decay = (1.0 - p.drag * dt).max(0.0);
                p.velocity *= decay;
            }
            p.position += p.velocity * dt;
            p.rotation += p.angular_velocity * dt;
        }
    }

    /// Iterate alive particles in pool order.
    pub fn iter_alive(&self) -> impl Iterator<Item = &Particle> {
        self.pool.iter().filter(|p| p.alive)
    }

    /// Interpolated color of an in-flight particle. Snapshot — does not
    /// mutate. Returned color is what the renderer should tint with.
    pub fn current_color(p: &Particle) -> Vec4 {
        let t = p.t();
        p.color_start.lerp(p.color_end, t)
    }

    /// Interpolated scale of an in-flight particle.
    pub fn current_scale(p: &Particle) -> f32 {
        let t = p.t();
        p.scale_start + (p.scale_end - p.scale_start) * t
    }

    /// Walk the pool finding a dead slot. Wraps; if the entire pool is alive
    /// the cursor's slot is reused (overwriting the oldest).
    fn next_slot(&mut self) -> usize {
        let cap = self.pool.len();
        for _ in 0..cap {
            let idx = self.cursor;
            self.cursor = (self.cursor + 1) % cap;
            if !self.pool[idx].alive {
                return idx;
            }
        }
        // Pool is fully alive — overwrite at the cursor position.
        let idx = self.cursor;
        self.cursor = (self.cursor + 1) % cap;
        idx
    }

    /// xorshift64* — fast, deterministic, fine for cosmetic randomness.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn uniform_range(&mut self, min: f32, max: f32) -> f32 {
        if (max - min).abs() < f32::EPSILON {
            return min;
        }
        // 24 bits of mantissa is plenty for direction/speed jitter.
        let bits = (self.next_u64() >> 40) as u32;
        let unit = bits as f32 / (1u32 << 24) as f32;
        min + unit * (max - min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn overfull_pool_overwrites_the_oldest_and_never_allocates() {
        // Ten single-particle bursts into a four-slot pool: the survivors
        // are the LAST four spawned (origins 6..=9), the pool stays at its
        // capacity, and `clear` frees every slot.
        let mut m = ParticleManager::with_capacity(4);
        let cfg = ParticleConfig::burst(1).with_lifetime(10.0, 10.0).with_speed(0.0, 0.0);

        for i in 0..10 {
            m.spawn_burst(Vec2::new(i as f32, 0.0), &cfg);
        }

        assert_eq!(m.capacity(), 4);
        assert_eq!(m.alive_count(), 4);
        let mut origins: Vec<f32> = m.iter_alive().map(|p| p.position.x).collect();
        origins.sort_by(f32::total_cmp);
        assert_eq!(origins, [6.0, 7.0, 8.0, 9.0], "the oldest four were overwritten");

        m.clear();
        assert_eq!(m.alive_count(), 0);
    }

    #[test]
    fn particles_integrate_velocity_gravity_and_age_until_they_die_and_free_their_slot() {
        let mut m = ParticleManager::with_capacity(8);
        let cfg = ParticleConfig::burst(1)
            .with_lifetime(1.0, 1.0)
            .with_speed(100.0, 100.0)
            .with_direction(Vec2::X, 0.0)
            .with_gravity(Vec2::new(0.0, -100.0))
            .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 0.0, 1.0, 1.0));
        m.spawn_burst(Vec2::ZERO, &cfg);

        m.step(0.5);

        let p = m.iter_alive().next().expect("alive at half-life");
        // 100 units/s along X for 0.5 s; gravity pulled it below the origin.
        assert!((p.position.x - 50.0).abs() < 0.5, "x: {}", p.position.x);
        assert!(p.position.y < 0.0, "expected y < 0 from gravity, got {}", p.position.y);
        // Half-way through its life the tint is an equal mix of start and end.
        let c = ParticleManager::current_color(p);
        assert!((c.x - 0.5).abs() < 1e-3 && (c.z - 0.5).abs() < 1e-3, "tint {c}");

        m.step(0.6);
        assert_eq!(m.alive_count(), 0, "all expired after the full lifetime");
        m.spawn_burst(Vec2::ZERO, &ParticleConfig::burst(5).with_lifetime(0.1, 0.1));
        assert_eq!(m.alive_count(), 5, "dead slots are reused");
    }

    #[test]
    fn direction_spread_stays_within_cone() {
        let mut m = ParticleManager::with_capacity(256);
        m.reseed(42);
        let cfg = ParticleConfig::burst(200)
            .with_speed(100.0, 100.0)
            .with_direction(Vec2::Y, std::f32::consts::FRAC_PI_2); // 90° full cone

        m.spawn_burst(Vec2::ZERO, &cfg);

        // With direction = +Y and ±45°, every particle flies upward.
        assert_eq!(m.alive_count(), 200);
        for p in m.iter_alive() {
            assert!(p.velocity.y > 0.0, "expected upward y, got {:?}", p.velocity);
        }
    }
}
