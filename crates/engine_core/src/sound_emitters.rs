//! The host's side of scene-authored sound: which `SoundEmitter`s are playing, and the
//! pass that starts, re-mixes and ends them.
//!
//! **The component is data; this table is the runtime.** First sight, the playing
//! instance and the last mix sent live here, so a `SoundEmitter` saves, snapshots and
//! copies as plain data.
//!
//! **The pass runs in the frame tail after the transform hierarchy**, so a child emitter
//! is placed by this frame's `GlobalTransform2D`. **First sight is recorded only while the
//! time scale is live**: the editor and the playground freeze the engine that way outside
//! Play, so Editing records no first sight and starts no spawn sound, and Play's first
//! frame is the first sight. Everything else runs every frame, frozen or not: a paused
//! game's requests are served (a menu's sound plays now, not as a burst on resume), its
//! loops are re-mixed, and a loop whose entity goes while paused still stops. In order: an
//! emitter seen for the first time with `play_on_spawn` plays; the requests drain; every
//! tracked instance is re-mixed; an instance that ended, whose entity is gone or no
//! longer carries the component is stopped and forgotten. A scripted stop in a binding
//! emitter's spawn frame is served before the pass and wins (`stop_binding` marks the
//! entity seen); a recorded emitter's spawn sound starts at first sight and is stopped by
//! the request in the same tail, so nothing is heard but `plays` counts the start.
//!
//! **Two kinds of sound.** A path emitter plays its recorded file through the asset
//! manager's sound tier and the audio manager, one tracked instance an entity: `play` on
//! a one-shot starts another (the previous one is released: it plays out with its mix
//! frozen, still owned until it ends so a world replacement stops it too), on a running
//! loop it does nothing. A binding emitter triggers the playing song's binding on
//! its entity's source and stops by ending that source; the conductor places and voices
//! it, so the table tracks nothing for it.
//!
//! **Spatial** means distance attenuation and pan from the main camera in world units
//! ([`spatial_mix`]); the main camera is the listener.

use std::collections::{BTreeMap, HashMap, HashSet};

use audio::{AudioManager, PlaybackId, SoundSettings};
use ecs::{EntityId, GlobalTransform2D, SoundEmitter, SoundRef, Spatial, Transform2D, World};
use glam::Vec2;

use crate::assets::AssetManager;
use crate::sound_sources::SoundSources;
use crate::RENDER_UNIT;

/// The most play and stop requests one frame holds; past it a request is refused and
/// counted. The pass serves them every frame, so only a game asking hundreds of times in
/// one frame reaches it, and the count is what shows that bug.
pub const EMITTER_REQUEST_CAPACITY: usize = 256;

/// A request waiting for the pass.
#[derive(Debug, Clone)]
enum Request {
    /// `carried` is the emitter and position a script saw when its command applied, used
    /// when the entity is gone by the pass.
    Play { entity: EntityId, carried: Option<(SoundEmitter, Option<Vec2>)> },
    Stop { entity: EntityId },
}

/// A path emitter's tracked instance.
#[derive(Debug, Clone, Copy)]
struct Playing {
    playback: PlaybackId,
    looping: bool,
    last_mix: (f32, f32),
}

/// The emitters the host is playing; lent to a game as `ctx.emitters`. See the module docs.
#[derive(Debug, Default)]
pub struct SoundEmitters {
    playing: BTreeMap<EntityId, Playing>,
    /// One-shots a newer play released from their entry: still owned until they end, so
    /// `clear` stops them with the rest.
    released: Vec<PlaybackId>,
    seen: HashSet<EntityId>,
    requests: Vec<Request>,
    plays: HashMap<EntityId, u64>,
    failed_loads: u64,
    detached_plays: u64,
    refused: u64,
}

/// The gain and pan of an emitter at `emitter` heard from a main camera at `camera`, in
/// world units (pixels over [`RENDER_UNIT`]): the gain falls from `volume` at the camera
/// to silence at `max_distance` (floored at [`Spatial::MIN_MAX_DISTANCE`]); the pan is
/// the emitter's offset to the side over `max_distance`, clamped to −1..=1. Zoom is
/// ignored. With no main camera the mix is `(volume, 0)`; a non-finite distance is
/// `(0, 0)`, so no NaN reaches a sink.
pub fn spatial_mix(volume: f32, spatial: Spatial, emitter: Vec2, camera: Option<Vec2>) -> (f32, f32) {
    let flat = (finite_or_silent(volume).clamp(0.0, 1.0), 0.0);
    let Some(camera) = camera else {
        return flat;
    };
    let max_distance_world_units = spatial.max_distance.max(Spatial::MIN_MAX_DISTANCE);
    let offset_world_units = (emitter - camera) / RENDER_UNIT;
    let distance_world_units = offset_world_units.length();
    if !distance_world_units.is_finite() {
        return (0.0, 0.0);
    }
    let gain = flat.0 * (1.0 - distance_world_units / max_distance_world_units);
    let pan = offset_world_units.x / max_distance_world_units;
    (finite_or_silent(gain).clamp(0.0, 1.0), finite_or_silent(pan).clamp(-1.0, 1.0))
}

fn finite_or_silent(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// Where `entity` is this frame: its `GlobalTransform2D`, else its `Transform2D`.
pub(crate) fn emitter_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world
        .get::<GlobalTransform2D>(entity)
        .map(|global| global.position)
        .or_else(|| world.get::<Transform2D>(entity).map(|transform| transform.position))
}

/// The gain and pan `emitter` plays at: spatial from the camera, else its volume, centred.
fn mix_for(emitter: &SoundEmitter, position: Option<Vec2>, camera: Option<Vec2>) -> (f32, f32) {
    match (emitter.spatial, position) {
        (Some(spatial), Some(position)) => spatial_mix(emitter.volume, spatial, position, camera),
        _ => spatial_mix(emitter.volume, Spatial::default(), Vec2::ZERO, None),
    }
}

impl SoundEmitters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask the next pass to play `entity`'s emitter, carrying the emitter and position as
    /// they are now: a one-shot still sounds there if the entity is gone by the tail (a
    /// pickup's sound, played then despawned in one update), a loop is refused then.
    /// Refused and counted if the entity has no emitter.
    pub fn play(&mut self, entity: EntityId, world: &World) {
        let carried = world
            .get::<SoundEmitter>(entity)
            .cloned()
            .map(|emitter| (emitter, emitter_position(world, entity)));
        self.push(Request::Play { entity, carried });
    }

    /// Ask the next pass to stop `entity`'s emitter: its tracked instance, or a binding's
    /// source. Acts on the table whether or not the component is still there.
    pub fn stop(&mut self, entity: EntityId) {
        self.push(Request::Stop { entity });
    }

    /// A script's play request, carrying what the script saw so a one-shot can outlive
    /// its entity. Scripting sits behind `physics`, and only it carries anything.
    #[cfg(feature = "physics")]
    pub(crate) fn hand_on_play(&mut self, entity: EntityId, carried: Option<(SoundEmitter, Option<Vec2>)>) {
        self.push(Request::Play { entity, carried });
    }

    fn push(&mut self, request: Request) {
        if self.requests.len() >= EMITTER_REQUEST_CAPACITY {
            let (Request::Play { entity, .. } | Request::Stop { entity }) = request;
            self.refuse(entity, "the request queue is full");
            return;
        }
        self.requests.push(request);
    }

    fn refuse(&mut self, entity: EntityId, reason: &str) {
        self.refused += 1;
        // Powers of two thin a storm of refusals to a handful of lines.
        if self.refused.is_power_of_two() {
            log::debug!("sound emitter request for {entity:?} refused ({reason}); {} refused so far", self.refused);
        }
    }

    /// The instance `entity`'s path emitter is playing, while the table tracks one.
    pub fn playing(&self, entity: EntityId) -> Option<PlaybackId> {
        self.playing.get(&entity).map(|playing| playing.playback)
    }

    /// Instances started for `entity`, both kinds, kept after each one ends; forgotten
    /// with the entity and by a world replacement (`clear`), so a count never crosses
    /// from one world into the next.
    pub fn plays(&self, entity: EntityId) -> u64 {
        self.plays.get(&entity).copied().unwrap_or(0)
    }

    /// The gain and pan last sent for `entity`'s tracked instance.
    pub fn last_mix(&self, entity: EntityId) -> Option<(f32, f32)> {
        self.playing.get(&entity).map(|playing| playing.last_mix)
    }

    /// Plays whose recorded sound could not be loaded.
    pub fn failed_loads(&self) -> u64 {
        self.failed_loads
    }

    /// One-shots a script played on an entity that was gone by the pass, played where the
    /// entity was and not tracked.
    pub fn detached_plays(&self) -> u64 {
        self.detached_plays
    }

    /// Requests refused: no emitter, a loop on a gone entity, a full queue.
    pub fn refused(&self) -> u64 {
        self.refused
    }

    /// One-shots released from their entry and still playing.
    pub fn released(&self) -> usize {
        self.released.len()
    }

    /// A binding emitter's stop, served in order with the triggers: ends the entity's
    /// source and counts the entity as seen, so a stop in its spawn frame is not undone
    /// by `play_on_spawn` in the pass that follows.
    pub(crate) fn stop_binding(&mut self, entity: EntityId, sources: &mut SoundSources, audio: &mut AudioManager) {
        self.seen.insert(entity);
        sources.end(entity, audio);
    }

    /// Trigger `entity`'s binding emitter on its source. Whether the song took it.
    pub(crate) fn play_binding(
        &mut self,
        entity: EntityId,
        binding: &str,
        sources: &mut SoundSources,
        audio: &mut AudioManager,
    ) -> bool {
        let source = sources.entity(entity);
        let accepted = sources.trigger(audio, binding, source);
        if accepted {
            *self.plays.entry(entity).or_default() += 1;
        }
        accepted
    }

    /// One frame's pass; see the module docs. The frame tail runs it after the hierarchy
    /// every frame, `live` while the time scale is not zero: first sight is recorded only
    /// then; the requests, the re-mix and the eviction run regardless.
    pub fn update(
        &mut self,
        world: &World,
        assets: &mut AssetManager,
        audio: &mut AudioManager,
        sources: &mut SoundSources,
        live: bool,
    ) {
        let camera = crate::render_manager::main_camera_pose(world).map(|(position, _)| position);
        let mut context = PassContext { world, assets, audio, sources, camera };

        if live {
            let mut emitters = world.query_entities::<ecs::Single<SoundEmitter>>();
            emitters.sort();
            for entity in emitters {
                if !self.seen.insert(entity) {
                    continue;
                }
                if world.get::<SoundEmitter>(entity).is_some_and(|emitter| emitter.play_on_spawn) {
                    self.serve_play(entity, None, &mut context);
                }
            }
        }

        for request in std::mem::take(&mut self.requests) {
            match request {
                Request::Play { entity, carried } => self.serve_play(entity, carried, &mut context),
                Request::Stop { entity } => self.serve_stop(entity, &mut context),
            }
        }

        self.refresh_and_evict(&mut context);
        // An entity whose component was removed is forgotten too, so a `SoundEmitter`
        // added to it later spawns again.
        self.seen
            .retain(|entity| world.validate_entity(entity).is_ok() && world.get::<SoundEmitter>(*entity).is_some());
        self.plays.retain(|entity, _| world.validate_entity(entity).is_ok());
    }

    /// Stop every owned instance — the tracked ones and the released one-shots still
    /// playing — and forget the table, the play counts and the waiting requests, for a
    /// world replaced wholesale: a snapshot restore revives the same ids, and a count from
    /// before the replacement would read as this world's. A binding emitter's source goes
    /// with the sources' flush.
    pub fn clear(&mut self, audio: &mut AudioManager) {
        for playing in self.playing.values() {
            audio.stop_playback(playing.playback);
        }
        for playback in self.released.drain(..) {
            audio.stop_playback(playback);
        }
        self.playing.clear();
        self.seen.clear();
        self.plays.clear();
        self.requests.clear();
    }

    fn serve_play(
        &mut self,
        entity: EntityId,
        carried: Option<(SoundEmitter, Option<Vec2>)>,
        context: &mut PassContext,
    ) {
        let world = context.world;
        if world.validate_entity(&entity).is_err() {
            self.play_detached(entity, carried, context);
            return;
        }
        let Some(emitter) = world.get::<SoundEmitter>(entity) else {
            self.refuse(entity, "the entity has no SoundEmitter");
            return;
        };
        match &emitter.sound {
            SoundRef::Binding(binding) => {
                self.play_binding(entity, binding, context.sources, context.audio);
            }
            SoundRef::Path(path) => {
                if let Some(previous) = self.playing.get(&entity).copied() {
                    if previous.looping {
                        if emitter.looping && context.audio.is_playing(previous.playback) {
                            return;
                        }
                        context.audio.stop_playback(previous.playback);
                    } else if context.audio.is_playing(previous.playback) {
                        // A one-shot's previous instance plays out, released but still
                        // owned, so a world replacement can stop it; a loop's never would.
                        self.released.push(previous.playback);
                    }
                    // The entry leaves with its instance: a start that fails below must
                    // not leave the released one tracked as well.
                    self.playing.remove(&entity);
                }
                let mix = mix_for(emitter, emitter_position(world, entity), context.camera);
                if let Some(playback) = self.start_path(path, emitter.looping, mix, context) {
                    *self.plays.entry(entity).or_default() += 1;
                    self.playing.insert(entity, Playing { playback, looping: emitter.looping, last_mix: mix });
                }
            }
        }
    }

    /// A script's play on an entity gone by the pass: a recorded one-shot plays where the
    /// entity was, untracked; a loop belongs to a living owner and is refused.
    fn play_detached(&mut self, entity: EntityId, carried: Option<(SoundEmitter, Option<Vec2>)>, context: &mut PassContext) {
        let Some((emitter, position)) = carried else {
            self.refuse(entity, "the entity is gone");
            return;
        };
        let SoundRef::Path(path) = &emitter.sound else {
            self.refuse(entity, "a binding emitter's entity is gone");
            return;
        };
        if emitter.looping {
            self.refuse(entity, "a loop on an entity that is gone");
            return;
        }
        let mix = mix_for(&emitter, position, context.camera);
        if let Some(playback) = self.start_path(path, false, mix, context) {
            // Owned like a released one-shot, so a world replacement stops it too.
            self.released.push(playback);
            self.detached_plays += 1;
        }
    }

    fn start_path(&mut self, path: &str, looping: bool, mix: (f32, f32), context: &mut PassContext) -> Option<PlaybackId> {
        let Ok(handle) = context.assets.resolve_sound(path, context.audio) else {
            self.failed_loads += 1;
            return None;
        };
        let settings = SoundSettings::new().with_volume(mix.0).with_pan(mix.1).with_looping(looping);
        match context.audio.play_with_settings(&handle, settings) {
            Ok(playback) => Some(playback),
            Err(error) => {
                log::debug!("sound '{path}' did not start: {error}");
                None
            }
        }
    }

    fn serve_stop(&mut self, entity: EntityId, context: &mut PassContext) {
        if let Some(playing) = self.playing.remove(&entity) {
            context.audio.stop_playback(playing.playback);
        }
        if context.world.get::<SoundEmitter>(entity).is_some_and(|emitter| emitter.sound.is_binding()) {
            context.sources.end(entity, context.audio);
        }
    }

    /// Re-mix every tracked instance, sending only a changed mix; stop and forget one
    /// whose sound ended, whose entity is gone or no longer carries the component, or
    /// whose component now names a binding.
    fn refresh_and_evict(&mut self, context: &mut PassContext) {
        let world = context.world;
        let camera = context.camera;
        let audio = &mut *context.audio;
        self.released.retain(|playback| audio.is_playing(*playback));
        self.playing.retain(|entity, playing| {
            let still_wanted = world.validate_entity(entity).is_ok() && audio.is_playing(playing.playback);
            let emitter = world.get::<SoundEmitter>(*entity);
            let Some(emitter) = emitter.filter(|emitter| still_wanted && !emitter.sound.is_binding()) else {
                audio.stop_playback(playing.playback);
                return false;
            };
            let mix = mix_for(emitter, emitter_position(world, *entity), camera);
            if mix != playing.last_mix {
                audio.set_playback_mix(playing.playback, mix.0, mix.1);
                playing.last_mix = mix;
            }
            true
        });
    }
}

/// What one pass lends its steps.
struct PassContext<'a> {
    world: &'a World,
    assets: &'a mut AssetManager,
    audio: &'a mut AudioManager,
    sources: &'a mut SoundSources,
    camera: Option<Vec2>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT_THE_CAMERA: Vec2 = Vec2::ZERO;

    fn spatial(max_distance: f32) -> Spatial {
        Spatial { max_distance }
    }

    #[test]
    fn the_gain_falls_to_silence_at_max_distance_and_halves_at_half_of_it() {
        let eight_units_up = Vec2::new(0.0, 8.0 * RENDER_UNIT);
        let four_units_up = Vec2::new(0.0, 4.0 * RENDER_UNIT);
        assert_eq!(spatial_mix(0.8, spatial(8.0), eight_units_up, Some(AT_THE_CAMERA)), (0.0, 0.0));
        assert_eq!(spatial_mix(0.8, spatial(8.0), four_units_up, Some(AT_THE_CAMERA)), (0.4, 0.0));
        assert_eq!(
            spatial_mix(0.8, spatial(8.0), Vec2::new(0.0, 20.0 * RENDER_UNIT), Some(AT_THE_CAMERA)),
            (0.0, 0.0),
            "beyond max_distance is silent, never negative"
        );
    }

    #[test]
    fn half_of_max_distance_to_the_right_pans_halfway_right() {
        let camera = Vec2::new(-200.0, 100.0);
        let right = camera + Vec2::new(4.0 * RENDER_UNIT, 0.0);
        let left = camera - Vec2::new(4.0 * RENDER_UNIT, 0.0);
        assert_eq!(spatial_mix(1.0, spatial(8.0), right, Some(camera)), (0.5, 0.5));
        assert_eq!(spatial_mix(1.0, spatial(8.0), left, Some(camera)), (0.5, -0.5));
    }

    #[test]
    fn with_no_main_camera_the_emitter_plays_flat_at_its_volume() {
        assert_eq!(spatial_mix(0.6, spatial(8.0), Vec2::new(900.0, 0.0), None), (0.6, 0.0));
    }

    #[test]
    fn a_zero_max_distance_is_floored_and_a_non_finite_distance_is_silent() {
        assert_eq!(spatial_mix(1.0, spatial(0.0), AT_THE_CAMERA, Some(AT_THE_CAMERA)), (1.0, 0.0));
        assert_eq!(spatial_mix(1.0, spatial(0.0), Vec2::new(RENDER_UNIT, 0.0), Some(AT_THE_CAMERA)), (0.0, 1.0));
        assert_eq!(spatial_mix(1.0, spatial(8.0), Vec2::new(f32::INFINITY, 0.0), Some(AT_THE_CAMERA)), (0.0, 0.0));
        assert_eq!(spatial_mix(1.0, spatial(8.0), Vec2::new(f32::NAN, 0.0), Some(AT_THE_CAMERA)), (0.0, 0.0));
    }
}
