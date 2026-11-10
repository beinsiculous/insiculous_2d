//! Scale contracts driven through the editor and the engine's real frame tail.

use glam::Vec2;
use input::{InputEvent, prelude::MouseButton};
use engine_core::{Game, GameConfig, contexts::GameContext, test_support::GameHarness};
use editor::editor_preferences::UiScale;
use ui::{DrawCommand, Rect};
use super::{EditorGame, test_support::{editor_game, DummyGame}};

const WINDOW: Vec2 = Vec2::new(1600.0, 1000.0);
const STEP: f32 = 1.0 / 60.0;

fn harness(percent: u16) -> GameHarness<EditorGame<DummyGame>> {
    let mut harness = GameHarness::new(editor_game(), GameConfig {
        width: WINDOW.x as u32, height: WINDOW.y as u32, ..Default::default()
    });
    harness.step(STEP, &[]);
    harness.game_mut().ui_scale = UiScale::Percent(percent);
    harness.step(STEP, &[]);
    harness
}

fn pointer(position: Vec2) -> InputEvent { InputEvent::MouseMoved(position.x, position.y) }

fn sprite(harness: &mut GameHarness<EditorGame<DummyGame>>) -> ecs::EntityId {
    let world = harness.world_mut();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::default()).unwrap();
    world.add_component(&entity, ecs::sprite_components::Sprite::new(0)).unwrap();
    harness.step(STEP, &[]);
    entity
}

#[test]
fn a_scaled_pointer_selects_the_sprite_it_is_over() {
    let mut harness = harness(150);
    let entity = sprite(&mut harness);
    let position = harness.game().editor.world_to_screen(Vec2::ZERO) * 1.5;
    harness.step(STEP, &[pointer(position), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(harness.game().editor.selection.primary(), Some(entity));
}

#[test]
fn a_scaled_marquee_draws_at_the_pointer_corners_and_selects_what_it_encloses() {
    let mut harness = harness(150);
    let entity = sprite(&mut harness);
    let center = harness.game().editor.world_to_screen(Vec2::ZERO);
    let start = (center - Vec2::new(65.0, 55.0)) * 1.5;
    let end = (center + Vec2::new(65.0, 55.0)) * 1.5;
    harness.step(STEP, &[pointer(start), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[pointer(end)]);
    let expected = Rect::new(start.x, start.y, end.x - start.x, end.y - start.y);
    assert!(harness.ui_commands().iter().any(|command| matches!(command,
        DrawCommand::RectBorder { bounds, .. } if (bounds.center() - expected.center()).length() < 0.001
            && (bounds.width - expected.width).abs() < 0.001 && (bounds.height - expected.height).abs() < 0.001)));
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(harness.game().editor.selection.primary(), Some(entity));
}

#[test]
fn a_scaled_pan_moves_the_camera_by_the_pointer_distance_in_points() {
    let mut harness = harness(150);
    let center = harness.game().editor.world_to_screen(Vec2::ZERO) * 1.5;
    let before = harness.game().editor.viewport.camera_position();
    harness.step(STEP, &[pointer(center), InputEvent::MouseButtonPressed(MouseButton::Middle)]);
    harness.step(STEP, &[pointer(center + Vec2::new(60.0, 30.0))]);
    assert!((harness.game().editor.viewport.camera_position() - before - Vec2::new(-40.0, 20.0)).length() < 0.001);
}

#[test]
fn a_scaled_gizmo_drag_moves_the_entity_with_its_handle_and_undo_restores_it() {
    let mut harness = harness(150);
    let entity = sprite(&mut harness);
    harness.game_mut().editor.selection.select(entity);
    harness.game_mut().editor.set_tool(editor::EditorTool::Move);
    harness.game_mut().editor.view.snap = false;
    let center = harness.game().editor.world_to_screen(Vec2::ZERO) * 1.5;
    harness.step(STEP, &[pointer(center), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[pointer(center + Vec2::new(45.0, 30.0))]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert!((harness.world().get::<common::Transform2D>(entity).unwrap().position - Vec2::new(30.0, -20.0)).length() < 0.001);
    harness.context(|game, context| game.undo_with_feedback(context.world));
    assert_eq!(harness.world().get::<common::Transform2D>(entity).unwrap().position, Vec2::ZERO);
}

#[test]
fn fractional_scissors_and_gpu_centers_share_the_same_snapped_rect() {
    let mut harness = harness(150);
    harness.set_window_size(Vec2::new(1601.0, 1001.0));
    harness.step(STEP, &[]);
    let bounds = harness.game().editor.scene_view_bounds().unwrap();
    let mut camera = common::Camera::default();
    let mut sprites = renderer::sprite::SpriteBatcher::new();
    let mut scissor = None;
    let world = ecs::World::new();
    harness.game_mut().render(&mut engine_core::contexts::RenderContext {
        world: &world, sprites: &mut sprites, camera: &mut camera,
        window_size: Vec2::new(1601.0, 1001.0), ui_commands: &[],
        glyph_textures: &Default::default(), viewport_scissor: &mut scissor,
    });
    let scissor = scissor.unwrap();
    let (left, top) = ((bounds.x * 1.5).floor(), (bounds.y * 1.5).floor());
    assert_eq!(scissor, common::Rect::new(left, top,
        (bounds.right() * 1.5).ceil() - left, (bounds.bottom() * 1.5).ceil() - top));
    let center = camera.world_to_screen(harness.game().editor.viewport.camera_position());
    assert!((center - scissor.center()).length() < 0.001);
}

/// Where the dialog drew `label`, in physical pixels, nudged into the glyphs from the
/// baseline-left origin the text command carries: a press there lands in the button.
fn drawn_label(harness: &GameHarness<EditorGame<DummyGame>>, label: &str) -> Vec2 {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text == label => Some(data.position + Vec2::new(8.0, -4.0)),
        _ => None,
    }).unwrap_or_else(|| panic!("the dialog drew {label}"))
}

fn assert_dialog_and_buttons_inside(harness: &GameHarness<EditorGame<DummyGame>>, labels: &[&str]) {
    let panel = editor::ConfirmDialog::panel_rect(WINDOW / 2.0);
    let physical = Rect::new(panel.x * 2.0, panel.y * 2.0, panel.width * 2.0, panel.height * 2.0);
    assert!(physical.x >= 0.0 && physical.y >= 0.0 && physical.right() <= WINDOW.x && physical.bottom() <= WINDOW.y);
    assert!(harness.ui_commands().iter().any(|command| matches!(command,
        DrawCommand::RectBorder { bounds, .. } if *bounds == physical)));
    for label in labels {
        assert!(harness.ui_commands().iter().any(|command| matches!(command,
            DrawCommand::Text { data, .. } if data.text == *label && physical.contains(data.position))));
    }
}

#[test]
fn the_scene_dialog_is_inside_a_double_scale_window_and_its_cancel_button_works() {
    let mut harness = harness(200);
    harness.game_mut().scene_confirm.pending_action = Some(super::scene_confirm::PendingSceneAction::NewScene);
    harness.step(STEP, &[]);
    assert_dialog_and_buttons_inside(&harness, &["Save", "Discard", "Cancel"]);
    let cancel = drawn_label(&harness, "Cancel");
    harness.step(STEP, &[pointer(cancel), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert!(harness.game().scene_confirm.pending_action.is_none());
}

#[test]
fn the_stop_dialog_is_inside_a_double_scale_window_and_its_cancel_button_works() {
    let mut harness = harness(200);
    harness.game_mut().stop_confirm.pending = true;
    harness.step(STEP, &[]);
    assert_dialog_and_buttons_inside(&harness, &["Keep", "Discard", "Cancel"]);
    let cancel = drawn_label(&harness, "Cancel");
    harness.step(STEP, &[pointer(cancel), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert!(!harness.game().stop_confirm.pending);
}

#[test]
fn the_status_bar_stays_at_the_physical_bottom_and_a_double_scale_window_goes_narrow() {
    let mut harness = harness(200);
    let color = harness.game().editor.theme.status_bar_bg;
    let bounds = harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Rect { bounds, color: drawn, .. } if *drawn == color && bounds.bottom() == WINDOW.y => Some(*bounds),
        _ => None,
    }).expect("status bar at the window bottom");
    assert_eq!(bounds.x, 0.0);
    assert_eq!(bounds.width, WINDOW.x);
    assert!(bounds.y > 0.0);
    harness.set_window_size(Vec2::new(1024.0, 720.0));
    harness.step(STEP, &[]);
    assert!(harness.game().editor.dock_area.is_narrow());
}

#[test]
fn the_selected_scale_persists_and_shortcuts_step_and_reset_it() {
    use input::prelude::KeyCode;
    let mut harness = harness(150);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::ControlLeft)]);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Equal)]);
    assert_eq!(harness.game().ui_scale, UiScale::Percent(175));
    harness.game_mut().save_preferences_now();
    let saved = engine_core::save_store::read(&harness.game().prefs_slot).unwrap().unwrap();
    assert_eq!(editor::EditorPreferences::from_json(&saved).unwrap().ui_scale, UiScale::Percent(175));
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::Equal), InputEvent::KeyPressed(KeyCode::Minus)]);
    assert_eq!(harness.game().ui_scale, UiScale::Percent(150));
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Digit0)]);
    assert_eq!(harness.game().ui_scale, UiScale::FollowDisplay);
}

#[test]
fn a_scale_shortcut_during_a_gizmo_drag_is_refused_and_says_so() {
    use input::prelude::KeyCode;
    let mut harness = harness(150);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::ControlLeft)]);
    harness.game_mut().gizmo_drag = Some(super::gizmo_drag::GizmoDragState {
        entities: Vec::new(), accumulated_rotation: 0.0,
    });
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Equal)]);
    assert_eq!(harness.game().ui_scale, UiScale::Percent(150));
    assert_eq!(harness.game().editor.status_bar.message(), Some("Finish or Escape the gesture first"));
}

#[test]
fn a_scale_shortcut_during_a_pan_is_refused_and_the_release_steps_it() {
    use input::prelude::KeyCode;
    let mut harness = harness(150);
    let center = harness.game().editor.world_to_screen(Vec2::ZERO) * 1.5;
    harness.step(STEP, &[pointer(center), InputEvent::MouseButtonPressed(MouseButton::Middle)]);
    assert!(harness.game().editor.viewport_input.is_panning());
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::ControlLeft)]);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Equal)]);
    assert_eq!(harness.game().ui_scale, UiScale::Percent(150));
    assert_eq!(harness.game().editor.status_bar.message(), Some("Finish or Escape the gesture first"));
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Middle), InputEvent::KeyReleased(KeyCode::Equal)]);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Equal)]);
    assert_eq!(harness.game().ui_scale, UiScale::Percent(175));
}

#[derive(Default)]
struct HudGame { seen_scale: f32, seen_window: Vec2 }
impl Game for HudGame {
    fn update(&mut self, context: &mut GameContext) {
        self.seen_scale = context.ui.pixels_per_point();
        self.seen_window = context.window_size;
        context.ui.label_styled("Inner HUD", Vec2::new(400.0, 200.0), ui::Color::WHITE, 14.0);
    }
}

#[test]
fn the_inner_hud_and_bottom_right_scene_button_stay_physical_at_editor_scale() {
    let mut game = EditorGame::new(HudGame::default());
    game.prefs_slot = editor_game().prefs_slot;
    let mut harness = GameHarness::new(game, GameConfig { width: 1600, height: 1000, ..Default::default() });
    harness.step(STEP, &[]);
    harness.game_mut().ui_scale = UiScale::Percent(150);
    harness.step(STEP, &[]);
    let center = harness.game().editor.scene_view_bounds().unwrap().center() * 1.5;
    let size = Vec2::new(120.0, 32.0);
    let entity = harness.world_mut().create_entity();
    harness.world_mut().add_component(&entity, ecs::UiButton {
        text: "Tail button".into(), id: "tail".into(), anchor: ecs::UiAnchor::BottomRight,
        offset: center - WINDOW + size / 2.0, size, ..Default::default()
    }).unwrap();
    harness.context(|game, context| super::test_support::press_play(game, context.world));
    harness.step(STEP, &[]);
    assert_eq!(harness.game().inner.seen_scale, 1.0);
    assert_eq!(harness.game().inner.seen_window, WINDOW);
    for label in ["Inner HUD", "Tail button"] {
        assert!(harness.ui_commands().iter().any(|command| matches!(command,
            DrawCommand::Text { data, .. } if data.text == label && data.font_size == 14.0)));
    }
    let clip = harness.game().editor.scene_view_bounds().unwrap();
    let physical_clip = Rect::new(clip.x * 1.5, clip.y * 1.5, clip.width * 1.5, clip.height * 1.5);
    assert!(harness.ui_commands().iter().any(|command| matches!(command,
        DrawCommand::PushClipRect { bounds } if *bounds == physical_clip)));
    harness.step(STEP, &[pointer(center), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    harness.step(STEP, &[]);
    assert!(harness.world().read_events::<engine_core::UiButtonPressed>().iter().any(|event| event.id == "tail"));
    harness.context(|_, context| assert_eq!(context.ui.pixels_per_point(), 1.5));
}

#[test]
fn control_plus_and_minus_resize_editor_text_from_the_main_keyboard_and_number_pad() {
    use input::prelude::KeyCode;
    for (increase, decrease, shifted) in [
        (KeyCode::Equal, KeyCode::Minus, true),
        (KeyCode::NumpadAdd, KeyCode::NumpadSubtract, false),
    ] {
        let mut harness = harness(100);
        let body_size = harness.game().editor.theme.fonts.body;
        harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::ControlLeft)]);
        if shifted { harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::ShiftLeft)]); }
        harness.step(STEP, &[InputEvent::KeyPressed(increase)]);
        assert_eq!(harness.game().ui_scale, UiScale::Percent(125));
        harness.step(STEP, &[InputEvent::KeyReleased(increase), InputEvent::KeyReleased(KeyCode::ShiftLeft)]);
        assert!(harness.ui_commands().iter().any(|command| matches!(command,
            DrawCommand::Text { data, .. } if data.text == "Hierarchy" && data.font_size == body_size * 1.25)));
        harness.step(STEP, &[InputEvent::KeyPressed(decrease)]);
        assert_eq!(harness.game().ui_scale, UiScale::Percent(100));
        harness.step(STEP, &[InputEvent::KeyReleased(decrease)]);
        assert!(harness.ui_commands().iter().any(|command| matches!(command,
            DrawCommand::Text { data, .. } if data.text == "Hierarchy" && data.font_size == body_size)));
    }
}
