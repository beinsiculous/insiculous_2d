//! Shared fixtures for the editor crate's tests.
//!
//! Three facts the test files used to restate: an entity under test is a
//! `Transform2D` (plus its global twin and a `Name`), a pickable is an
//! `EntityId` with generation 1, and **a click is two frames** — the press
//! frame activates the widget, `clicked` fires on the RELEASE frame, and
//! that release frame is `Hovered`, not `Active` (the toolbar-reselects-
//! the-sprite footgun). Every harness frame runs at [`WINDOW`] so the
//! layout math in one test file means the same thing in the next.

use common::{Rect, Transform2D};
use ecs::hierarchy::GlobalTransform2D;
use ecs::sprite_components::Name;
use ecs::{EntityId, World};
use glam::Vec2;
use input::prelude::{InputHandler, KeyCode, MouseButton};
use ui::UIContext;

use crate::{DragDropState, InspectorExtras, PickableEntity, SceneViewport};

/// Window size every harness frame runs at; also the viewport bounds of
/// [`test_viewport`].
pub(crate) const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

/// An entity id that never came from a `World` — for selection, picking and
/// hierarchy tests that only need distinct ids.
pub(crate) fn entity(id: u64) -> EntityId {
    EntityId::with_generation(id, 1)
}

/// A world entity at the origin with `Transform2D`, `GlobalTransform2D` and
/// the name `"Test"` — the minimum every Set*/Delete command test edits.
pub(crate) fn setup_entity(world: &mut World) -> EntityId {
    named_entity(world, "Test", Vec2::ZERO)
}

/// A world entity named `name` at `pos`, with `Transform2D` and
/// `GlobalTransform2D`.
pub(crate) fn named_entity(world: &mut World, name: &str, pos: Vec2) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(pos)).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new(name)).ok();
    entity
}

/// A pickable for the synthetic entity `id`, centered at `pos` with the
/// given absolute size and depth.
pub(crate) fn pickable(id: u64, pos: Vec2, size: Vec2, depth: f32) -> PickableEntity {
    PickableEntity::new(entity(id), pos, size, depth)
}

/// A scene viewport whose panel spans the whole [`WINDOW`] from the origin.
pub(crate) fn test_viewport() -> SceneViewport {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, WINDOW.x, WINDOW.y));
    viewport
}

/// The extras an inspector editor needs when no texture is displayed and
/// nothing is being dragged.
pub(crate) fn extras(drag_drop: &mut DragDropState) -> InspectorExtras<'_> {
    InspectorExtras { drag_drop, texture_display: None, warnings: Vec::new() }
}

/// Start the next input frame: last frame's just-pressed / just-released
/// edges and per-frame deltas are cleared, held keys and buttons stay held.
pub(crate) fn next_frame(input: &mut InputHandler) {
    input.update();
}

/// Move the pointer to `pos` and press `button` for the coming frame.
pub(crate) fn press_button(input: &mut InputHandler, button: MouseButton, pos: Vec2) {
    next_frame(input);
    input.mouse_mut().update_position(pos.x, pos.y);
    input.mouse_mut().handle_button_press(button);
}

/// Move the pointer to `pos` and press the left button for the coming frame.
pub(crate) fn press_mouse(input: &mut InputHandler, pos: Vec2) {
    press_button(input, MouseButton::Left, pos);
}

/// Release `button` for the coming frame (the pointer stays put).
pub(crate) fn release_button(input: &mut InputHandler, button: MouseButton) {
    next_frame(input);
    input.mouse_mut().handle_button_release(button);
}

/// Release the left button for the coming frame (the pointer stays put).
pub(crate) fn release_mouse(input: &mut InputHandler) {
    release_button(input, MouseButton::Left);
}

/// Move the pointer to `pos` for the coming frame (a drag frame while a
/// button is held, a hover frame otherwise).
pub(crate) fn move_mouse(input: &mut InputHandler, pos: Vec2) {
    next_frame(input);
    input.mouse_mut().update_position(pos.x, pos.y);
}

/// Run one UI frame at [`WINDOW`] around `body` and return what it returned.
pub(crate) fn frame<T>(
    ui: &mut UIContext,
    input: &InputHandler,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    ui.begin_frame(input, WINDOW);
    let out = body(ui);
    ui.end_frame();
    out
}

/// The PRESS frame of a click at `pos`.
pub(crate) fn press_at<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pos: Vec2,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    press_mouse(input, pos);
    frame(ui, input, body)
}

/// A frame with the pointer moved to `pos`.
pub(crate) fn move_to<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pos: Vec2,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    move_mouse(input, pos);
    frame(ui, input, body)
}

/// The RELEASE frame of a click — the frame `clicked` fires on.
pub(crate) fn release<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    release_mouse(input);
    frame(ui, input, body)
}

/// A full click at `pos`: the press frame then the release frame, running
/// `body` on both. Returns `(press frame output, release frame output)`.
pub(crate) fn click_through<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pos: Vec2,
    mut body: impl FnMut(&mut UIContext) -> T,
) -> (T, T) {
    let press = press_at(ui, input, pos, &mut body);
    let released = release(ui, input, &mut body);
    (press, released)
}

/// A full keystroke of `key`: the PRESS frame (returned) then a RELEASE
/// frame running `body` again, so the release edge is observed and the
/// next frame carries no `just_released`. Modifier keys pressed on `input`
/// beforehand stay held across both frames. Symmetric with
/// [`click_through`].
pub(crate) fn type_key<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    key: KeyCode,
    mut body: impl FnMut(&mut UIContext) -> T,
) -> T {
    next_frame(input);
    input.keyboard_mut().handle_key_press(key);
    let pressed = frame(ui, input, &mut body);
    next_frame(input);
    input.keyboard_mut().handle_key_release(key);
    frame(ui, input, &mut body);
    pressed
}
