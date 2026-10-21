//! The host's side of a song's sources: which `SourceId` each entity, the game and the
//! menu trigger under, and when their activations end.
//!
//! **An activation is an epoch.** One host-wide counter numbers every activation of any
//! source; an entity's first trigger mints its `SourceId` with the next epoch, and every
//! later trigger reuses it, so its notes walk one activation and a shared cursor stays
//! whole. A despawned entity is deactivated by [`SoundSources::reconcile`] in the frame
//! tail and forgotten, so a later entity — or the same id revived by a snapshot restore —
//! starts a new activation the conductor tells apart.
//!
//! **The game and the menu are sources too**, at the top two `u64` ids, which no entity
//! reaches: entity ids count up from zero and are never recycled. They live in the same
//! table and keep one activation until a flush; reconcile never ends them. A sound that
//! must outlive its entity — a brick's explosion — is triggered on [`SoundSources::game`].
//!
//! **A flush ends every activation at once**, for a world replaced wholesale: the counter
//! advances and the conductor refuses every epoch below the advanced value, so nothing
//! minted before it, this frame included, sounds again.

use std::collections::BTreeMap;

use audio::{AudioManager, SourceId};
use ecs::{EntityId, World};

/// The game-owned source's id: the largest `u64`, above every entity's.
pub const GAME_SOURCE_ID: u64 = u64::MAX;
/// The menu's source id: one below the game's.
pub const MENU_SOURCE_ID: u64 = u64::MAX - 1;

/// Who a source belongs to; the table's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Owner {
    Entity(EntityId),
    Game,
    Menu,
}

/// The live sources the host has minted, and the activation counter. Lent to a game as
/// `ctx.sources`; see the module docs.
#[derive(Debug, Default)]
pub struct SoundSources {
    entries: BTreeMap<Owner, SourceId>,
    /// The last epoch issued, to a source or a flush. It never rewinds: at a thousand
    /// activations a second the `u32` lasts 49 days of play.
    epoch: u32,
    refused_triggers: u64,
}

impl SoundSources {
    pub fn new() -> Self {
        Self::default()
    }

    /// The source `entity` triggers under: its live activation, or a new one.
    pub fn entity(&mut self, entity: EntityId) -> SourceId {
        self.live_or_mint(Owner::Entity(entity), entity.value(), entity.generation())
    }

    /// The game's own source, for a sound that outlives whatever caused it.
    pub fn game(&mut self) -> SourceId {
        self.live_or_mint(Owner::Game, GAME_SOURCE_ID, 0)
    }

    /// The menu's source.
    pub fn menu(&mut self) -> SourceId {
        self.live_or_mint(Owner::Menu, MENU_SOURCE_ID, 0)
    }

    fn live_or_mint(&mut self, owner: Owner, id: u64, generation: u64) -> SourceId {
        if let Some(source) = self.entries.get(&owner) {
            return *source;
        }
        self.epoch += 1;
        let source = SourceId {
            id,
            generation,
            epoch: self.epoch,
        };
        self.entries.insert(owner, source);
        source
    }

    /// Trigger `action` for `source`. A refusal — no song, an action the song does not
    /// bind, a full queue — is counted and logged at debug, never an error: a sound is
    /// best-effort. Whether the manager took it.
    pub fn trigger(&mut self, audio: &mut AudioManager, action: &str, source: SourceId) -> bool {
        let Err(error) = audio.trigger(action, source) else {
            return true;
        };
        self.refused_triggers += 1;
        // Powers of two thin a storm of refusals to a handful of lines.
        if self.refused_triggers.is_power_of_two() {
            log::debug!(
                "trigger '{action}' refused ({error}); {} refused so far",
                self.refused_triggers
            );
        }
        false
    }

    /// End and forget every entity source whose entity is gone. A deactivation the
    /// manager cannot send yet waits in the manager, which retries it from `update()`.
    pub fn reconcile(&mut self, world: &World, audio: &mut AudioManager) {
        self.entries.retain(|owner, source| {
            let Owner::Entity(entity) = owner else {
                return true;
            };
            if world.validate_entity(entity).is_ok() {
                return true;
            }
            audio.deactivate_source(*source);
            false
        });
    }

    /// End every activation, for a world replaced wholesale, and forget every source: the
    /// game and the menu re-mint on their next call. Returns the flush's epoch, the floor
    /// below which the conductor refuses every activation.
    pub fn flush(&mut self, audio: &mut AudioManager) -> u32 {
        self.epoch += 1;
        audio.stop_all_sources(self.epoch);
        self.entries.clear();
        self.epoch
    }

    /// Triggers the manager refused, here or through the frame tail. The conductor's own
    /// `rejected_triggers` counts only what reached it.
    pub fn refused_triggers(&self) -> u64 {
        self.refused_triggers
    }

    /// The live source `entity` triggers under, without minting one.
    pub fn source_of(&self, entity: EntityId) -> Option<SourceId> {
        self.entries.get(&Owner::Entity(entity)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entity_keeps_one_activation_until_it_is_gone_and_a_new_entity_takes_the_next_epoch() {
        let mut world = World::new();
        let mut audio = AudioManager::disabled();
        let mut sources = SoundSources::new();
        let bat = world.create_entity();
        let first = sources.entity(bat);
        assert_eq!(sources.entity(bat), first, "a second trigger reuses the activation");
        assert_eq!((first.id, first.generation), (bat.value(), bat.generation()));

        world.remove_entity(&bat).ok();
        sources.reconcile(&world, &mut audio);
        assert_eq!(sources.source_of(bat), None, "the reconcile forgets a despawned entity");
        let ball = world.create_entity();
        assert!(sources.entity(ball).epoch > first.epoch, "every activation takes a new epoch");
    }

    #[test]
    fn the_game_and_menu_sources_sit_above_every_entity_and_survive_the_reconcile() {
        let mut world = World::new();
        let mut audio = AudioManager::disabled();
        let mut sources = SoundSources::new();
        let entity = world.create_entity();
        let (game, menu) = (sources.game(), sources.menu());
        assert!(game.id > menu.id && menu.id > entity.value());

        world.clear();
        sources.reconcile(&world, &mut audio);
        assert_eq!((sources.game(), sources.menu()), (game, menu), "validation never ends them");
    }

    #[test]
    fn a_flush_raises_the_floor_above_this_frames_epoch_and_the_fixed_sources_re_mint_once() {
        let mut world = World::new();
        let mut audio = AudioManager::disabled();
        let mut sources = SoundSources::new();
        let entity = world.create_entity();
        let minted_this_frame = sources.entity(entity);
        let menu_before = sources.menu();

        let floor = sources.flush(&mut audio);
        assert!(minted_this_frame.epoch < floor && menu_before.epoch < floor);
        assert_eq!(sources.source_of(entity), None, "a flush forgets every source");

        let menu_after = sources.menu();
        assert!(menu_after.epoch >= floor, "the conductor admits an epoch at the floor");
        assert_eq!(sources.menu(), menu_after, "one re-mint, never a fresh epoch per call");
        assert!(sources.entity(entity).epoch > menu_after.epoch);
    }

    #[test]
    fn the_counter_only_rises_and_every_refused_trigger_is_counted() {
        let mut audio = AudioManager::disabled();
        let mut sources = SoundSources::new();
        let mut epochs = vec![sources.game().epoch];
        epochs.push(sources.flush(&mut audio));
        epochs.push(sources.game().epoch);
        epochs.push(sources.flush(&mut audio));
        assert!(epochs.windows(2).all(|pair| pair[0] < pair[1]), "{epochs:?}");

        let game = sources.game();
        assert!(!sources.trigger(&mut audio, "hit", game), "no song refuses the trigger");
        assert!(!sources.trigger(&mut audio, "hit", game));
        assert_eq!(sources.refused_triggers(), 2);
    }
}
