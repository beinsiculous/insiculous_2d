//! The two-enemy demo's rules, pure so its tests can hold them: when an enemy attacks,
//! whether the camera lets it, and where the hummer's patrol puts it.
//!
//! **One game clock.** Every cadence reads the demo's own clock — the sum of
//! `delta_time × time_scale` — so a paused game mints nothing and every frame rate hears
//! the same rhythm. An enemy of period `p` and phase `φ` attacks when
//! `floor((clock − φ) / p)` grows past the index it holds; the index starts at the value
//! its spawn sees, so a new or respawned enemy waits for its kind's next slot, and it
//! advances every frame whether or not the enemy may sound, so an enemy back from ten
//! periods off-screen attacks at its next slot rather than on its first eligible frame. A
//! slow frame mints one attack, never a burst.

use glam::Vec2;

/// The two kinds, one shared part between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Plays the melody on the sine bank's one preset.
    Hummer,
    /// Plays it on the imported sample.
    Pinger,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Hummer, Kind::Pinger];

    /// The song's binding this kind triggers.
    pub fn binding(self) -> &'static str {
        match self {
            Kind::Hummer => "hum",
            Kind::Pinger => "ping",
        }
    }

    /// The scene's prefab, and the entity's name.
    pub fn prefab(self) -> &'static str {
        match self {
            Kind::Hummer => "Hummer",
            Kind::Pinger => "Pinger",
        }
    }

    /// The kind's phase in a cadence of `period`: the hummer on the period, the pinger half
    /// a period on, so the two alternate at any frame rate.
    pub fn phase(self, period: f32) -> f32 {
        match self {
            Kind::Hummer => 0.0,
            Kind::Pinger => period / 2.0,
        }
    }
}

/// Which attack slot `clock` is in: negative before the phase is reached.
pub fn attack_index(clock: f32, period: f32, phase: f32) -> i64 {
    ((clock - phase) / period).floor() as i64
}

/// The slot an attack falls in this frame, if it has grown past `last_index`.
pub fn attack_due(clock: f32, period: f32, phase: f32, last_index: i64) -> Option<i64> {
    let index = attack_index(clock, period, phase);
    (index > last_index).then_some(index)
}

/// Whether an enemy at `position` is inside the view of a camera at `camera` over a
/// `window` of that many world units: the game's policy, since a binding is never spatial.
pub fn eligible(position: Vec2, camera: Vec2, window: Vec2) -> bool {
    let offset = (position - camera).abs();
    offset.x <= window.x / 2.0 && offset.y <= window.y / 2.0
}

/// How far either side of its home the hummer patrols, and how long one sweep takes:
/// far enough to leave an 800-wide view for a few seconds of each sweep.
pub const PATROL_HALF_SPAN: f32 = 450.0;
pub const PATROL_SECONDS: f32 = 12.0;

/// The hummer's x at `clock` on its patrol about `home_x`.
pub fn patrol_x(home_x: f32, clock: f32) -> f32 {
    home_x + PATROL_HALF_SPAN * (std::f32::consts::TAU * clock / PATROL_SECONDS).sin()
}

/// One enemy the demo minted triggers for.
#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    pub entity: ecs::EntityId,
    pub kind: Kind,
    pub period: f32,
    pub phase: f32,
    pub last_index: i64,
    pub was_eligible: bool,
    /// Where it stands when it does not patrol, and the centre of its patrol when it does.
    pub home: Vec2,
    pub patrols: bool,
}

impl Enemy {
    /// An enemy spawned at `clock`: it holds the slot it was born in, so it attacks first
    /// at its kind's next one.
    pub fn spawned(entity: ecs::EntityId, kind: Kind, period: f32, home: Vec2, clock: f32) -> Self {
        let phase = kind.phase(period);
        Self {
            entity,
            kind,
            period,
            phase,
            last_index: attack_index(clock, period, phase),
            was_eligible: false,
            home,
            patrols: false,
        }
    }

    /// Advance the cadence to `clock`: whether an attack falls this frame.
    pub fn advance(&mut self, clock: f32) -> bool {
        match attack_due(clock, self.period, self.phase, self.last_index) {
            Some(index) => {
                self.last_index = index;
                true
            }
            None => false,
        }
    }

    pub fn position(&self, clock: f32) -> Vec2 {
        if self.patrols {
            Vec2::new(patrol_x(self.home.x, clock), self.home.y)
        } else {
            self.home
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enemy(kind: Kind, clock: f32) -> Enemy {
        Enemy::spawned(ecs::EntityId::new(), kind, 1.0, Vec2::ZERO, clock)
    }

    #[test]
    fn the_pinger_attacks_at_half_a_period_the_hummer_at_one_and_then_they_alternate() {
        let (mut hummer, mut pinger) = (enemy(Kind::Hummer, 0.0), enemy(Kind::Pinger, 0.0));
        let mut heard = Vec::new();
        for frame in 1..=150 {
            let clock = frame as f32 / 60.0;
            if hummer.advance(clock) {
                heard.push((frame, Kind::Hummer));
            }
            if pinger.advance(clock) {
                heard.push((frame, Kind::Pinger));
            }
        }
        assert_eq!(heard, vec![(30, Kind::Pinger), (60, Kind::Hummer), (90, Kind::Pinger), (120, Kind::Hummer), (150, Kind::Pinger)]);
    }

    #[test]
    fn a_slow_frame_mints_one_attack_and_a_hitch_across_both_slots_lands_both_kinds() {
        let mut hummer = enemy(Kind::Hummer, 0.0);
        assert!(hummer.advance(3.5), "three slots passed in one frame");
        assert!(!hummer.advance(3.6), "and they were one attack, not a burst");

        let (mut hummer, mut pinger) = (enemy(Kind::Hummer, 0.9), enemy(Kind::Pinger, 0.9));
        assert!(hummer.advance(1.6) && pinger.advance(1.6), "0.9 s to 1.6 s crosses 1.0 and 1.5");
    }

    #[test]
    fn a_respawned_enemy_waits_for_its_kinds_next_slot_and_the_clock_runs_while_it_is_off_screen() {
        let mut pinger = enemy(Kind::Pinger, 2.2);
        assert!(!pinger.advance(2.4), "born after the 1.5 slot, it waits");
        assert!(pinger.advance(2.5), "for 2.5, the pinger's phase, not the hummer's 3.0");

        for tenth in 26..=126 {
            pinger.advance(tenth as f32 / 10.0);
        }
        assert!(!pinger.advance(12.65), "ten periods off-screen advanced it to 12.5");
        assert!(pinger.advance(13.5), "so it attacks at its next slot");
    }

    #[test]
    fn eligibility_is_the_view_around_the_camera() {
        let window = Vec2::new(800.0, 600.0);
        assert!(eligible(Vec2::new(399.0, 0.0), Vec2::ZERO, window));
        assert!(!eligible(Vec2::new(401.0, 0.0), Vec2::ZERO, window));
        assert!(eligible(Vec2::new(401.0, 0.0), Vec2::new(10.0, 0.0), window));
        assert!(!eligible(Vec2::new(0.0, -301.0), Vec2::ZERO, window));
    }

    #[test]
    fn the_hummer_patrol_leaves_an_eight_hundred_wide_view_for_part_of_each_sweep() {
        let window = Vec2::new(800.0, 600.0);
        let outside = (0..120)
            .filter(|tenth| !eligible(Vec2::new(patrol_x(-100.0, *tenth as f32 / 10.0), 0.0), Vec2::ZERO, window))
            .count();
        assert!(outside > 0 && outside < 60, "{outside} tenths of 120 off-screen");
    }
}
