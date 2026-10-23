//! Conductor Demo — two enemy kinds playing one shared part of a song.
//!
//! The hummer plays the melody on the sine bank's one preset, the pinger on an imported
//! sample; both walk the one `melody` part (`Loop`, unquantized), so the kinds alternate
//! through one tune. The demo mints every trigger itself — on one game clock, and only for
//! an enemy inside the camera's view — and ends an enemy's source the frame it leaves the
//! view, so an off-screen enemy neither sounds nor moves the melody on.
//!
//! Controls: Space pauses the game (no attack is minted, the song plays on); P pauses the
//! song (its clock holds and triggers are rejected, the game runs on); M and S mute the
//! music and sfx buses; D despawns the enemy nearest the camera, mid-note; R respawns
//! whatever kind is missing; Escape quits.
//!
//! `--stress spread|burst|saturate` measures dense activity instead: 48 enemies on a grid
//! in view, one attack a second each with their phases spread over the second or all at
//! once, or 40 pingers at once past the sample slot's ceiling. `--seconds N` quits after N
//! seconds. Every five seconds a `conductor=on` line reports the window.
//!
//! Scene file: examples/assets/scenes/conductor_demo.scene.ron; song:
//! examples/assets/music/ai_demo.mid. The asset base is `examples/assets/`.

#[path = "conductor_demo/enemies.rs"]
mod enemies;
#[cfg(test)]
#[path = "conductor_demo/tests.rs"]
mod tests;

use std::path::Path;

use engine_core::prelude::*;
use enemies::{eligible, Enemy, Kind};

/// The demo's asset base: the song's SoundFont and sample paths resolve under it.
pub const ASSET_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets");
const SCENE: &str = "scenes/conductor_demo.scene.ron";
const SONG: &str = "music/ai_demo.mid";
/// Seconds between one kind's attacks in the ordinary scene, and every stress enemy's.
const PERIOD_SECONDS: f32 = 1.0;
const STRESS_PERIOD_SECONDS: f32 = 1.0;
const REPORT_SECONDS: f32 = 5.0;

/// The dense workloads `--stress` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stress {
    /// 48 enemies, phases spread over the second: 48 attempts a second, sustained.
    Spread,
    /// The same 48 at phase 0: 48 attempts on one frame, every second.
    Burst,
    /// 40 pingers on one frame, every second, past the sample slot's ceiling.
    Saturate,
}

/// What the command line asks for.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Settings {
    pub stress: Option<Stress>,
    pub quit_after_seconds: Option<f32>,
}

impl Settings {
    /// Read `--stress <shape>` and `--seconds <n>`; anything else is an error naming it.
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut settings = Settings::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--stress" => {
                    settings.stress = Some(match arguments.next().as_deref() {
                        Some("spread") => Stress::Spread,
                        Some("burst") => Stress::Burst,
                        Some("saturate") => Stress::Saturate,
                        other => return Err(format!("--stress takes spread, burst or saturate, not {other:?}")),
                    })
                }
                "--seconds" => {
                    let value = arguments.next().and_then(|seconds| seconds.parse::<f32>().ok());
                    settings.quit_after_seconds = Some(value.ok_or("--seconds takes a number")?);
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(settings)
    }
}

/// One report window's own counts, beside the conductor's.
#[derive(Debug, Clone, Copy, Default)]
struct Window {
    seconds: f32,
    attempted: u64,
}

/// See the module docs.
pub struct ConductorDemo {
    settings: Settings,
    scene: Option<SceneInstance>,
    enemies: Vec<Enemy>,
    /// The game clock every cadence reads: `delta_time × time_scale`, summed.
    clock: f32,
    elapsed_seconds: f32,
    game_paused: bool,
    song_line: String,
    window: Window,
    attempted_total: u64,
}

impl ConductorDemo {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            scene: None,
            enemies: Vec::new(),
            clock: 0.0,
            elapsed_seconds: 0.0,
            game_paused: false,
            song_line: String::new(),
            window: Window::default(),
            attempted_total: 0,
        }
    }

    pub fn enemies(&self) -> &[Enemy] {
        &self.enemies
    }

    pub fn enemies_mut(&mut self) -> &mut Vec<Enemy> {
        &mut self.enemies
    }

    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// Attacks minted for an eligible enemy since the start.
    pub fn attempted(&self) -> u64 {
        self.attempted_total
    }

    /// Spawn one `kind` from the scene's prefab at `home`, its cadence joining the clock at
    /// the kind's phase.
    pub fn spawn(&mut self, ctx: &mut GameContext, kind: Kind, home: Vec2, period: f32) -> Option<EntityId> {
        let scene = self.scene.as_ref()?;
        let at = ComponentData::Transform2D { position: (home.x, home.y), rotation: 0.0, scale: (0.5, 0.5) };
        let entity = match scene.spawn_prefab(ctx.world, ctx.assets, kind.prefab(), &[OverrideData::fields(at, &["position"])]) {
            Ok(entity) => entity,
            Err(error) => {
                log::error!("the {} prefab would not spawn: {error}", kind.prefab());
                return None;
            }
        };
        ctx.world.add_component(&entity, Name::new(kind.prefab())).ok();
        self.enemies.push(Enemy::spawned(entity, kind, period, home, self.clock));
        Some(entity)
    }

    /// Remove the enemy nearest the camera, mid-note: its source ends in this frame's tail.
    pub fn despawn_nearest(&mut self, world: &mut World) -> Option<EntityId> {
        let camera = camera_position(world, self.scene.as_ref());
        let index = (0..self.enemies.len()).min_by(|first, second| {
            let distance = |index: &usize| self.enemies[*index].position(self.clock).distance(camera);
            distance(first).total_cmp(&distance(second))
        })?;
        let enemy = self.enemies.remove(index);
        world.remove_entity(&enemy.entity).ok();
        Some(enemy.entity)
    }

    fn respawn_missing(&mut self, ctx: &mut GameContext) {
        for kind in Kind::ALL {
            if self.enemies.iter().any(|enemy| enemy.kind == kind) {
                continue;
            }
            let Some(entity) = self.spawn(ctx, kind, home_of(kind), PERIOD_SECONDS) else { continue };
            if let Some(enemy) = self.enemies.iter_mut().find(|enemy| enemy.entity == entity) {
                // A stress enemy stands on its grid cell, so a respawn keeps the workload.
                enemy.patrols = patrols(kind) && self.settings.stress.is_none();
            }
        }
    }

    fn spawn_stress(&mut self, ctx: &mut GameContext, stress: Stress) {
        let (count, kinds): (usize, &[Kind]) = match stress {
            Stress::Spread | Stress::Burst => (48, &Kind::ALL),
            Stress::Saturate => (40, &[Kind::Pinger]),
        };
        for index in 0..count {
            // A grid of eight columns well inside an 800 × 600 view.
            let home = Vec2::new(-280.0 + 80.0 * (index % 8) as f32, -200.0 + 70.0 * (index / 8) as f32);
            let kind = kinds[index % kinds.len()];
            let Some(entity) = self.spawn(ctx, kind, home, STRESS_PERIOD_SECONDS) else { continue };
            let phase = match stress {
                Stress::Spread => STRESS_PERIOD_SECONDS * index as f32 / count as f32,
                Stress::Burst | Stress::Saturate => 0.0,
            };
            if let Some(enemy) = self.enemies.iter_mut().find(|enemy| enemy.entity == entity) {
                enemy.phase = phase;
                enemy.last_index = enemies::attack_index(self.clock, enemy.period, phase);
            }
        }
    }

    fn handle_keys(&mut self, ctx: &mut GameContext) {
        let input = ctx.input;
        let pressed = |code| input.is_key_just_pressed(code);
        if pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        if pressed(KeyCode::Space) {
            self.game_paused = !self.game_paused;
        }
        if pressed(KeyCode::KeyP) {
            if ctx.audio.is_song_playing() {
                ctx.audio.pause_song();
            } else {
                ctx.audio.resume_song();
            }
        }
        if pressed(KeyCode::KeyM) {
            let music = if ctx.audio.music_volume() > 0.0 { 0.0 } else { 1.0 };
            ctx.audio.set_music_volume(music);
        }
        if pressed(KeyCode::KeyS) {
            let sfx = if ctx.audio.sfx_volume() > 0.0 { 0.0 } else { 1.0 };
            ctx.audio.set_sfx_volume(sfx);
        }
        if pressed(KeyCode::KeyD) {
            self.despawn_nearest(ctx.world);
        }
        if pressed(KeyCode::KeyR) {
            self.respawn_missing(ctx);
        }
    }

    /// Each enemy's frame: its cadence advanced on the game clock, its source ended on the
    /// frame it leaves the view, and its attack minted only while the camera sees it.
    fn run_enemies(&mut self, ctx: &mut GameContext) {
        let camera = camera_position(ctx.world, self.scene.as_ref());
        let window = ctx.window_size;
        for enemy in &mut self.enemies {
            let position = enemy.position(self.clock);
            if enemy.patrols {
                if let Some(transform) = ctx.world.get_mut::<Transform2D>(enemy.entity) {
                    transform.position = position;
                }
            }
            let now_eligible = eligible(position, camera, window);
            if enemy.was_eligible && !now_eligible {
                ctx.sources.end(enemy.entity, ctx.audio);
            }
            enemy.was_eligible = now_eligible;
            if enemy.advance(self.clock) && now_eligible {
                self.window.attempted += 1;
                self.attempted_total += 1;
                let source = ctx.sources.entity(enemy.entity);
                ctx.sources.trigger(ctx.audio, enemy.kind.binding(), source);
            }
        }
    }

    fn report(&mut self, ctx: &mut GameContext) {
        let window = std::mem::take(&mut self.window);
        let Some(mut snapshot) = ctx.audio.song_snapshot() else {
            println!("conductor=off t={:.0}s attempted={}", self.elapsed_seconds, window.attempted);
            return;
        };
        let maxima = ctx.audio.take_song_maxima();
        snapshot.source.render_nanoseconds_longest = maxima.render_nanoseconds_longest;
        snapshot.source.drain_gap_nanoseconds_longest = maxima.drain_gap_nanoseconds_longest;
        println!(
            "{} t={:.0}s stress={:?} attempted={} attempted_total={} refused={} enemies={}",
            snapshot.log_line(),
            self.elapsed_seconds,
            self.settings.stress,
            window.attempted,
            self.attempted_total,
            ctx.sources.refused_triggers(),
            self.enemies.len(),
        );
    }

    fn draw_hud(&self, ctx: &mut GameContext) {
        let lines = [
            self.song_line.clone(),
            format!(
                "game {} · song {} · music {} · sfx {} · enemies {}",
                if self.game_paused { "paused" } else { "running" },
                if ctx.audio.is_song_playing() { "playing" } else { "paused or stopped" },
                if ctx.audio.music_volume() > 0.0 { "on" } else { "muted" },
                if ctx.audio.sfx_volume() > 0.0 { "on" } else { "muted" },
                self.enemies.len(),
            ),
            "Space game pause · P song pause · M/S mute · D despawn · R respawn · Esc quit".to_string(),
        ];
        for (row, line) in lines.iter().enumerate() {
            ctx.ui.label(line, Vec2::new(12.0, 20.0 + 18.0 * row as f32));
        }
    }
}

/// In the ordinary scene the hummer patrols, leaving the view on each sweep; the pinger stands.
fn patrols(kind: Kind) -> bool {
    kind == Kind::Hummer
}

/// Where the hummer and the pinger stand in the ordinary scene.
fn home_of(kind: Kind) -> Vec2 {
    match kind {
        Kind::Hummer => Vec2::new(-100.0, 80.0),
        Kind::Pinger => Vec2::new(150.0, -80.0),
    }
}

/// The scene camera's position; the origin without one.
fn camera_position(world: &World, scene: Option<&SceneInstance>) -> Vec2 {
    scene
        .and_then(|scene| scene.named_entities.get("camera"))
        .and_then(|camera| world.get::<Transform2D>(*camera))
        .map_or(Vec2::ZERO, |transform| transform.position)
}

impl Game for ConductorDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        let base = ctx.assets.base_path().to_string();
        match SceneLoader::load_and_instantiate(Path::new(&base).join(SCENE), ctx.world, ctx.assets) {
            Ok(instance) => self.scene = Some(instance),
            Err(error) => log::error!("the demo's scene would not load: {error}"),
        }
        self.song_line = match ctx.audio.play_song(Path::new(&base).join(SONG)) {
            Ok(()) => "song: music/ai_demo.mid — hum and ping walk one melody".to_string(),
            Err(error) => {
                log::error!("the demo's song could not play: {error}");
                format!("song: failed — {error}")
            }
        };
        match self.settings.stress {
            Some(stress) => {
                if let Some(scene) = &self.scene {
                    for name in ["Hummer", "Pinger"] {
                        if let Some(entity) = scene.named_entities.get(name) {
                            ctx.world.remove_entity(entity).ok();
                        }
                    }
                }
                self.spawn_stress(ctx, stress);
            }
            None => {
                for kind in Kind::ALL {
                    let entity = self.scene.as_ref().and_then(|scene| scene.named_entities.get(kind.prefab()).copied());
                    if let Some(entity) = entity {
                        let mut enemy = Enemy::spawned(entity, kind, PERIOD_SECONDS, home_of(kind), self.clock);
                        enemy.patrols = patrols(kind);
                        self.enemies.push(enemy);
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.handle_keys(ctx);
        ctx.time_scale = if self.game_paused { 0.0 } else { 1.0 };
        self.clock += ctx.delta_time * ctx.time_scale;
        self.elapsed_seconds += ctx.delta_time;
        self.window.seconds += ctx.delta_time;
        self.run_enemies(ctx);
        if self.window.seconds >= REPORT_SECONDS {
            self.report(ctx);
        }
        if self.settings.quit_after_seconds.is_some_and(|seconds| self.elapsed_seconds >= seconds) {
            ctx.request_exit();
        }
        self.draw_hud(ctx);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let settings = Settings::parse(std::env::args().skip(1))?;
    let config = GameConfig::new("Conductor Demo - Insiculous 2D")
        .with_size(800, 600)
        .with_clear_color(0.08, 0.08, 0.12, 1.0)
        .with_asset_base_path(ASSET_BASE);
    run_game(ConductorDemo::new(settings), config)?;
    Ok(())
}
