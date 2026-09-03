//! Player-aware input settings: the universal mapping layer games consume.
//!
//! [`InputSettings`] holds one [`PlayerBindings`] per local player. Bindings
//! are **device-relative**: a [`PlayerSource::PadButton`] means "this player's
//! assigned pad", and the concrete gamepad id lives in a single field per
//! player — re-pointing a player at another pad never rewrites bindings.
//!
//! Actions use the fixed engine vocabulary [`GameAction`], which is what makes
//! one persisted settings schema serve every game (see
//! [`crate::InputMapping`] for game-private action enums).
//!
//! ```
//! use input::{AxisDirection, GameAction, GamepadAxis, InputEvent, InputHandler, InputSettings, PlayerId};
//! use winit::keyboard::KeyCode;
//!
//! let settings = InputSettings::default_two_player();
//! let mut input = InputHandler::new();
//!
//! // W drives player 1's MoveUp; player 2 is on arrows / pad 1
//! input.queue_event(InputEvent::KeyPressed(KeyCode::KeyW));
//! input.process_queued_events();
//! assert!(settings.is_active(PlayerId::P1, GameAction::MoveUp, &input));
//! assert!(!settings.is_active(PlayerId::P2, GameAction::MoveUp, &input));
//!
//! // Merged digital+analog movement, -1.0..=1.0 (+y = up)
//! assert_eq!(settings.move_y(PlayerId::P1, &input), 1.0);
//! ```

use crate::gamepad::{AxisDirection, GamepadAxis, GamepadButton};
use crate::input_handler::InputHandler;
use crate::input_mapping::{source_was_pressed, GameAction, InputMapping, InputSource};
use crate::pad_layout::STANDARD_PAD_LAYOUT;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// Identifies a local player slot (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u8);

impl PlayerId {
    /// Player 1
    pub const P1: PlayerId = PlayerId(0);
    /// Player 2
    pub const P2: PlayerId = PlayerId(1);

    /// The player's 0-based slot index
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A device-relative input source in a player's bindings.
///
/// Pad sources name no gamepad id — they resolve against the owning
/// [`PlayerBindings`]' assigned pad at query time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlayerSource {
    /// Keyboard key
    Keyboard(KeyCode),
    /// Mouse button
    Mouse(MouseButton),
    /// A button on this player's assigned pad
    PadButton(GamepadButton),
    /// An analog axis on this player's assigned pad, used as a digital input
    PadAxis(GamepadAxis, AxisDirection),
}

impl PlayerSource {
    /// Concrete source on `pad`; keyboard/mouse pass through.
    pub fn on_pad(self, pad: u32) -> InputSource {
        match self {
            PlayerSource::Keyboard(key) => InputSource::Keyboard(key),
            PlayerSource::Mouse(button) => InputSource::Mouse(button),
            PlayerSource::PadButton(button) => InputSource::Gamepad(pad, button),
            PlayerSource::PadAxis(axis, direction) => {
                InputSource::GamepadAxis(pad, axis, direction)
            }
        }
    }
}

/// One player's device assignment and action bindings.
#[derive(Debug, Clone, Default)]
pub struct PlayerBindings {
    /// The gamepad id this player's pad sources resolve against.
    /// `None` makes every `PadButton`/`PadAxis` binding inert.
    pad: Option<u32>,
    /// Action → bound sources
    mapping: InputMapping<GameAction, PlayerSource>,
    /// True when a mutation actually changed state since the last
    /// [`InputSettings::take_dirty`] — drives the engine's save-on-change.
    dirty: bool,
}

impl PlayerBindings {
    /// Create empty bindings with no assigned pad
    pub fn new() -> Self {
        Self::default()
    }

    /// The gamepad id assigned to this player, if any
    pub fn pad(&self) -> Option<u32> {
        self.pad
    }

    /// Assign (or clear) this player's gamepad
    pub fn set_pad(&mut self, pad: Option<u32>) {
        if self.pad != pad {
            self.pad = pad;
            self.dirty = true;
        }
    }

    /// Bind a source to an action. Binding the same pair twice is a no-op.
    pub fn bind(&mut self, action: GameAction, source: PlayerSource) {
        self.dirty |= self.mapping.bind(action, source);
    }

    /// Remove one source from an action's bindings
    pub fn unbind(&mut self, action: GameAction, source: &PlayerSource) {
        self.dirty |= self.mapping.unbind(action, source);
    }

    /// All sources bound to an action
    pub fn bindings(&self, action: GameAction) -> &[PlayerSource] {
        self.mapping.bindings(action)
    }

    /// All (action, sources) pairs, for persistence and inspection
    pub fn all_bindings(&self) -> impl Iterator<Item = (GameAction, &[PlayerSource])> {
        self.mapping.iter()
    }

    /// Resolve a device-relative source to a concrete [`InputSource`].
    /// Pad sources resolve to `None` when no pad is assigned.
    pub fn resolve(&self, source: PlayerSource) -> Option<InputSource> {
        match source {
            PlayerSource::Keyboard(key) => Some(InputSource::Keyboard(key)),
            PlayerSource::Mouse(button) => Some(InputSource::Mouse(button)),
            PlayerSource::PadButton(..) | PlayerSource::PadAxis(..) => {
                self.pad.map(|id| source.on_pad(id))
            }
        }
    }

    /// Resolved sources for an action, filtered by `keep`
    fn resolved_sources<'a>(
        &'a self,
        action: GameAction,
        keep: impl Fn(&PlayerSource) -> bool + 'a,
    ) -> impl Iterator<Item = InputSource> + 'a {
        self.bindings(action)
            .iter()
            .filter(move |s| keep(s))
            .filter_map(|s| self.resolve(*s))
    }

    fn is_active(&self, action: GameAction, input: &InputHandler) -> bool {
        self.resolved_sources(action, |_| true)
            .any(|source| input.is_source_pressed(&source))
    }

    fn is_just_pressed(&self, action: GameAction, input: &InputHandler) -> bool {
        self.resolved_sources(action, |_| true)
            .any(|source| input.is_source_just_pressed(&source))
    }

    fn was_active(&self, action: GameAction, input: &InputHandler) -> bool {
        self.resolved_sources(action, |_| true)
            .any(|source| source_was_pressed(&source, input))
    }

    /// Like `is_active`, but only over digital (non-axis) sources — used by
    /// the merged movement queries so analog granularity isn't flattened to
    /// ±1 by the axis' own threshold binding.
    fn is_active_digital(&self, action: GameAction, input: &InputHandler) -> bool {
        self.resolved_sources(action, |s| !matches!(s, PlayerSource::PadAxis(..)))
            .any(|source| input.is_source_pressed(&source))
    }
}

/// Per-player input settings: device assignment + bindings for every local
/// player, evaluated against an [`InputHandler`]'s device state.
#[derive(Debug, Clone)]
pub struct InputSettings {
    players: Vec<PlayerBindings>,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self::default_two_player()
    }
}

impl InputSettings {
    /// Build settings from explicit per-player bindings. The result starts
    /// clean: construction (e.g. loading a settings file) is not a change
    /// worth re-saving.
    pub fn from_players(players: Vec<PlayerBindings>) -> Self {
        let mut settings = Self { players };
        settings.clear_dirty();
        settings
    }

    /// True if any binding mutation happened since the last call; clears the
    /// flag. The engine polls this each frame to save settings on change.
    pub fn take_dirty(&mut self) -> bool {
        let dirty = self.players.iter().any(|p| p.dirty);
        self.clear_dirty();
        dirty
    }

    /// Re-flag the settings as needing a save (used when a save attempt
    /// failed and should be retried).
    pub fn mark_dirty(&mut self) {
        if let Some(player) = self.players.first_mut() {
            player.dirty = true;
        }
    }

    fn clear_dirty(&mut self) {
        for player in &mut self.players {
            player.dirty = false;
        }
    }

    /// The engine's default two-player pairing:
    ///
    /// - **P1** — WASD movement, Space + left mouse = Action1, LeftShift =
    ///   Action2, Escape = Menu, gamepad **0**
    /// - **P2** — arrow-key movement, Enter = Action1, RightShift = Action2,
    ///   Escape = Menu, gamepad **1**
    ///
    /// Both players' pads bind dpad + left stick to movement, A/B/X/Y to
    /// Action1-4, Start to Menu, Select to Select. 2-player works with zero,
    /// one, or two pads connected.
    pub fn default_two_player() -> Self {
        let mut p1 = PlayerBindings::new();
        p1.set_pad(Some(0));
        p1.bind(GameAction::MoveUp, PlayerSource::Keyboard(KeyCode::KeyW));
        p1.bind(GameAction::MoveDown, PlayerSource::Keyboard(KeyCode::KeyS));
        p1.bind(GameAction::MoveLeft, PlayerSource::Keyboard(KeyCode::KeyA));
        p1.bind(GameAction::MoveRight, PlayerSource::Keyboard(KeyCode::KeyD));
        p1.bind(GameAction::Action1, PlayerSource::Keyboard(KeyCode::Space));
        p1.bind(GameAction::Action1, PlayerSource::Mouse(MouseButton::Left));
        p1.bind(GameAction::Action2, PlayerSource::Keyboard(KeyCode::ShiftLeft));
        p1.bind(GameAction::Menu, PlayerSource::Keyboard(KeyCode::Escape));
        bind_standard_pad_layout(&mut p1);

        let mut p2 = PlayerBindings::new();
        p2.set_pad(Some(1));
        p2.bind(GameAction::MoveUp, PlayerSource::Keyboard(KeyCode::ArrowUp));
        p2.bind(GameAction::MoveDown, PlayerSource::Keyboard(KeyCode::ArrowDown));
        p2.bind(GameAction::MoveLeft, PlayerSource::Keyboard(KeyCode::ArrowLeft));
        p2.bind(GameAction::MoveRight, PlayerSource::Keyboard(KeyCode::ArrowRight));
        p2.bind(GameAction::Action1, PlayerSource::Keyboard(KeyCode::Enter));
        p2.bind(GameAction::Action2, PlayerSource::Keyboard(KeyCode::ShiftRight));
        p2.bind(GameAction::Menu, PlayerSource::Keyboard(KeyCode::Escape));
        bind_standard_pad_layout(&mut p2);

        // Defaults are a baseline, not a player change — start clean.
        Self::from_players(vec![p1, p2])
    }

    /// Number of configured player slots
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// A player's bindings, if the slot exists
    pub fn player(&self, player: PlayerId) -> Option<&PlayerBindings> {
        self.players.get(player.index())
    }

    /// Mutable access to a player's bindings, if the slot exists
    pub fn player_mut(&mut self, player: PlayerId) -> Option<&mut PlayerBindings> {
        self.players.get_mut(player.index())
    }

    /// Point a player at a different gamepad (or `None` for no pad)
    pub fn assign_pad(&mut self, player: PlayerId, pad: Option<u32>) {
        if let Some(bindings) = self.player_mut(player) {
            bindings.set_pad(pad);
        }
    }

    /// The gamepad id assigned to a player, if any
    pub fn pad_of(&self, player: PlayerId) -> Option<u32> {
        self.player(player).and_then(|b| b.pad())
    }

    // ================== Action Queries ==================

    /// Check if a player's action is currently active
    pub fn is_active(&self, player: PlayerId, action: GameAction, input: &InputHandler) -> bool {
        self.player(player)
            .is_some_and(|b| b.is_active(action, input))
    }

    /// Check if a player's action became active this frame (strict edge:
    /// pressing a second bound source while one is held does not re-trigger)
    pub fn just_activated(
        &self,
        player: PlayerId,
        action: GameAction,
        input: &InputHandler,
    ) -> bool {
        self.player(player)
            .is_some_and(|b| b.is_just_pressed(action, input) && !b.was_active(action, input))
    }

    /// Check if the action became active this frame for any player
    pub fn just_activated_any(&self, action: GameAction, input: &InputHandler) -> bool {
        (0..self.players.len() as u8)
            .any(|i| self.just_activated(PlayerId(i), action, input))
    }

    // ================== Analog Queries ==================

    /// Raw value of an axis on the player's assigned pad (0.0 without a pad)
    pub(crate) fn axis_value(&self, player: PlayerId, axis: GamepadAxis, input: &InputHandler) -> f32 {
        let Some(pad) = self.pad_of(player) else {
            return 0.0;
        };
        input
            .gamepads()
            .get_gamepad(pad)
            .map(|g| g.axis_value(axis))
            .unwrap_or(0.0)
    }

    /// Horizontal movement in `-1.0..=1.0`: digital MoveLeft/MoveRight
    /// (keys, dpad) merged with the left stick's X axis, clamped.
    pub fn move_x(&self, player: PlayerId, input: &InputHandler) -> f32 {
        self.merged_move(
            player,
            GameAction::MoveLeft,
            GameAction::MoveRight,
            GamepadAxis::LeftStickX,
            input,
        )
    }

    /// Vertical movement in `-1.0..=1.0` with **+1.0 = up**: digital
    /// MoveDown/MoveUp merged with the left stick's Y axis, clamped.
    pub fn move_y(&self, player: PlayerId, input: &InputHandler) -> f32 {
        self.merged_move(
            player,
            GameAction::MoveDown,
            GameAction::MoveUp,
            GamepadAxis::LeftStickY,
            input,
        )
    }

    /// Digital direction (−1/0/+1 from non-axis sources) plus the raw analog
    /// axis, clamped to `-1.0..=1.0`. Axis-threshold bindings are excluded
    /// from the digital half so a half-deflected stick reads as 0.5, not 1.5.
    fn merged_move(
        &self,
        player: PlayerId,
        negative: GameAction,
        positive: GameAction,
        axis: GamepadAxis,
        input: &InputHandler,
    ) -> f32 {
        let Some(bindings) = self.player(player) else {
            return 0.0;
        };
        let digital = (bindings.is_active_digital(positive, input) as i8
            - bindings.is_active_digital(negative, input) as i8) as f32;
        let analog = self.axis_value(player, axis, input);
        (digital + analog).clamp(-1.0, 1.0)
    }
}

/// The shared pad layout both default players get: dpad + left stick →
/// movement, A/B/X/Y → Action1-4, Start → Menu, Select → Select.
fn bind_standard_pad_layout(bindings: &mut PlayerBindings) {
    for &(action, source) in STANDARD_PAD_LAYOUT {
        bindings.bind(action, source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_handler::InputEvent;

    /// Queue `events` and process them, as the engine does at the top of a frame.
    /// (`tests/common/mod.rs` carries the same helper for the integration tests.)
    fn frame(input: &mut InputHandler, events: &[InputEvent]) {
        for event in events {
            input.queue_event(event.clone());
        }
        input.process_queued_events();
    }

    #[test]
    fn test_dirty_tracks_real_binding_changes_only_and_take_dirty_clears_it() {
        // Construction and loading are baselines, not changes worth re-saving
        let mut settings = InputSettings::default_two_player();
        assert!(!settings.take_dirty(), "construction must not trigger a save");
        let mut loaded = InputSettings::from_players(vec![PlayerBindings::new()]);
        assert!(!loaded.take_dirty(), "loading a settings file must not trigger a save");

        // Real changes: a pad reassignment, a new binding, an effective unbind
        settings.assign_pad(PlayerId::P2, Some(3));
        assert!(settings.take_dirty(), "a pad reassignment is a change");
        assert!(!settings.take_dirty(), "take_dirty must clear the flag");
        let p1 = settings.player_mut(PlayerId::P1).expect("P1 slot exists");
        p1.bind(GameAction::Action3, PlayerSource::Keyboard(KeyCode::KeyQ));
        assert!(settings.take_dirty(), "a new binding via player_mut is a change");
        let p1 = settings.player_mut(PlayerId::P1).expect("P1 slot exists");
        p1.unbind(GameAction::Action3, &PlayerSource::Keyboard(KeyCode::KeyQ));
        assert!(settings.take_dirty(), "removing a binding is a change");

        // Redundant mutations are not changes
        settings.assign_pad(PlayerId::P2, Some(3));
        assert!(!settings.take_dirty(), "assigning the same pad is not a change");
        let p1 = settings.player_mut(PlayerId::P1).expect("P1 slot exists");
        p1.bind(GameAction::MoveUp, PlayerSource::Keyboard(KeyCode::KeyW));
        assert!(!settings.take_dirty(), "a duplicate bind is not a change");
        let p1 = settings.player_mut(PlayerId::P1).expect("P1 slot exists");
        p1.unbind(GameAction::Action4, &PlayerSource::Keyboard(KeyCode::KeyZ));
        assert!(!settings.take_dirty(), "unbinding an absent source is not a change");

        // A failed save re-queues itself
        settings.mark_dirty();
        assert!(settings.take_dirty(), "mark_dirty must make the next poll save again");
    }

    #[test]
    fn test_default_pairing_routes_each_device_to_its_player_and_shares_menu() {
        let settings = InputSettings::default_two_player();
        let mut input = InputHandler::new();

        // P1: WASD, Space, left click
        frame(&mut input, &[
            InputEvent::KeyPressed(KeyCode::KeyW),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert!(settings.is_active(PlayerId::P1, GameAction::MoveUp, &input));
        assert!(settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        assert!(!settings.is_active(PlayerId::P2, GameAction::MoveUp, &input));
        assert!(!settings.is_active(PlayerId::P2, GameAction::Action1, &input));
        input.end_frame();

        // P2: arrows, Enter
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::KeyW),
            InputEvent::MouseButtonReleased(MouseButton::Left),
            InputEvent::KeyPressed(KeyCode::ArrowUp),
            InputEvent::KeyPressed(KeyCode::Enter),
        ]);
        assert!(settings.is_active(PlayerId::P2, GameAction::MoveUp, &input));
        assert!(settings.is_active(PlayerId::P2, GameAction::Action1, &input));
        assert!(!settings.is_active(PlayerId::P1, GameAction::MoveUp, &input));
        assert!(!settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        input.end_frame();

        // Pad 0 is P1's, pad 1 is P2's
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::ArrowUp),
            InputEvent::KeyReleased(KeyCode::Enter),
            InputEvent::GamepadButtonPressed(0, GamepadButton::A),
            InputEvent::GamepadButtonPressed(1, GamepadButton::DPadUp),
        ]);
        assert!(settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        assert!(!settings.is_active(PlayerId::P2, GameAction::Action1, &input));
        assert!(settings.is_active(PlayerId::P2, GameAction::MoveUp, &input));
        assert!(!settings.is_active(PlayerId::P1, GameAction::MoveUp, &input));
        input.end_frame();

        // Menu is shared: Escape pauses once per press for either player ...
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        assert!(settings.just_activated_any(GameAction::Menu, &input));
        input.end_frame();
        frame(&mut input, &[]);
        assert!(settings.is_active(PlayerId::P1, GameAction::Menu, &input));
        assert!(
            !settings.just_activated_any(GameAction::Menu, &input),
            "a held Menu key must not re-toggle the pause every frame"
        );
        input.end_frame();

        // ... and so does P2's Start, which P1 never sees
        frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
        input.end_frame();
        frame(&mut input, &[InputEvent::GamepadButtonPressed(1, GamepadButton::Start)]);
        assert!(settings.just_activated_any(GameAction::Menu, &input));
        assert!(!settings.is_active(PlayerId::P1, GameAction::Menu, &input));
    }

    #[test]
    fn test_assign_pad_repoints_pad_sources_only_and_an_unassigned_pad_is_inert() {
        let mut settings = InputSettings::default_two_player();
        let mut input = InputHandler::new();

        // Re-point P1 at pad 3: pad 0 stops driving P1, pad 3 starts, and the
        // stick edge fires once through the pad-relative axis source
        settings.assign_pad(PlayerId::P1, Some(3));
        frame(&mut input, &[InputEvent::GamepadButtonPressed(0, GamepadButton::A)]);
        assert!(!settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        input.end_frame();

        let pad3_a = InputEvent::GamepadButtonPressed(3, GamepadButton::A);
        let pad3_stick_right = InputEvent::GamepadAxisUpdated(3, GamepadAxis::LeftStickX, 0.8);
        frame(&mut input, &[pad3_a, pad3_stick_right]);
        assert!(settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        assert!(settings.just_activated(PlayerId::P1, GameAction::MoveRight, &input));
        input.end_frame();

        frame(&mut input, &[InputEvent::GamepadAxisUpdated(3, GamepadAxis::LeftStickX, 0.9)]);
        assert!(settings.is_active(PlayerId::P1, GameAction::MoveRight, &input));
        assert!(!settings.just_activated(PlayerId::P1, GameAction::MoveRight, &input));
        input.end_frame();

        // The keyboard cluster is untouched by the re-point
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
        assert!(settings.is_active(PlayerId::P1, GameAction::MoveUp, &input));
        input.end_frame();

        // No pad at all: every pad source is inert, the stick contributes no movement
        settings.assign_pad(PlayerId::P1, None);
        let w_up = InputEvent::KeyReleased(KeyCode::KeyW);
        let pad3_stick_full = InputEvent::GamepadAxisUpdated(3, GamepadAxis::LeftStickX, 1.0);
        frame(&mut input, &[w_up, pad3_stick_full]);
        assert!(!settings.is_active(PlayerId::P1, GameAction::Action1, &input));
        assert!(!settings.is_active(PlayerId::P1, GameAction::MoveRight, &input));
        assert_eq!(settings.move_x(PlayerId::P1, &input), 0.0);
    }

    #[test]
    fn test_move_y_merges_digital_and_stick_and_clamps() {
        let settings = InputSettings::default_two_player();
        let mut input = InputHandler::new();

        // Stick alone: analog granularity preserved (not flattened to 1.0 by
        // the stick's own threshold binding)
        frame(&mut input, &[InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickY, 0.6)]);
        assert_eq!(settings.move_y(PlayerId::P1, &input), 0.6);
        input.end_frame();

        // Key + stick together: clamped to 1.0
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
        assert_eq!(settings.move_y(PlayerId::P1, &input), 1.0);
        input.end_frame();

        // Digital down only: -1.0
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::KeyW),
            InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickY, 0.0),
            InputEvent::KeyPressed(KeyCode::KeyS),
        ]);
        assert_eq!(settings.move_y(PlayerId::P1, &input), -1.0);
    }
}
