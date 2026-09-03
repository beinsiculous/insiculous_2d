//! Contract tests for [`DrawList`]: the `UiLayer` z-bands (flush order and
//! depth bands), the layer-stack lifecycle, clip pairs, and the guard that
//! an elevated layer physically escapes a Content clip pair.

use glam::Vec2;

use super::*;

#[test]
fn test_layers_flush_in_enum_order_inside_banded_monotonic_depths() {
    // Submission order deliberately scrambled — DragGhost, PanelChrome,
    // then Content — with two commands per band so the within-band order
    // is observable too.
    let mut list = DrawList::new();
    list.push_layer(UiLayer::DragGhost);
    list.circle(Vec2::ZERO, 1.0, Color::RED);
    list.circle(Vec2::ONE, 2.0, Color::RED);
    list.pop_layer();
    list.push_layer(UiLayer::PanelChrome);
    list.line(Vec2::ZERO, Vec2::ONE, Color::GREEN, 1.0);
    list.pop_layer();
    list.rect(Rect::default(), Color::BLUE);
    list.rect(Rect::default(), Color::BLUE);

    list.flush_layers();

    let commands = list.commands();
    assert!(matches!(commands[0], DrawCommand::Rect { .. }), "Content first");
    assert!(matches!(commands[1], DrawCommand::Rect { .. }));
    assert!(matches!(commands[2], DrawCommand::Line { .. }), "PanelChrome second");
    assert!(matches!(commands[3], DrawCommand::Circle { .. }), "DragGhost last");
    let depths: Vec<f32> = commands.iter().map(DrawCommand::depth).collect();
    assert!(depths.windows(2).all(|pair| pair[0] < pair[1]), "depth is monotonic across the flushed stream: {depths:?}");
    let in_band = |depth: f32, layer: UiLayer| depth >= layer.depth_base() && depth < layer.depth_base() + 15.0;
    assert!(in_band(depths[0], UiLayer::Content) && in_band(depths[1], UiLayer::Content));
    assert!(in_band(depths[2], UiLayer::PanelChrome));
    assert!(in_band(depths[3], UiLayer::DragGhost) && in_band(depths[4], UiLayer::DragGhost));
    assert!(
        UiLayer::ALL.windows(2).all(|pair| pair[0].depth_base() < pair[1].depth_base()),
        "bands rise in enum order, Content lowest"
    );
}

#[test]
fn test_layer_stack_nests_pops_safely_and_clear_resets_it() {
    let mut list = DrawList::new();
    list.push_layer(UiLayer::Modal);
    list.push_layer(UiLayer::Tooltip);
    assert_eq!(list.current_layer(), UiLayer::Tooltip);
    list.pop_layer();
    assert_eq!(list.current_layer(), UiLayer::Modal, "a nested pop returns to the outer layer");
    list.pop_layer();
    list.pop_layer(); // extra pop is a safe no-op
    assert_eq!(list.current_layer(), UiLayer::Content);

    // begin_overlay is Floating sugar.
    list.begin_overlay();
    assert_eq!(list.current_layer(), UiLayer::Floating);
    list.rect(Rect::default(), Color::RED);
    // (deliberately un-popped — flush and clear must both cope)

    list.flush_layers();
    let after_first = list.commands().len();
    list.flush_layers();
    assert_eq!(list.commands().len(), after_first, "flush is idempotent");

    list.clear();
    assert!(list.is_empty());
    assert_eq!(list.current_layer(), UiLayer::Content, "clear resets the layer stack");
    list.rect(Rect::default(), Color::RED);
    assert!(list.commands()[0].depth() < UiLayer::PanelChrome.depth_base(), "records in Content again");
}

#[test]
fn test_clip_push_pop_brackets_the_draw_with_a_matched_pair_carrying_the_bounds() {
    let mut list = DrawList::new();
    let clip = Rect::new(10.0, 10.0, 100.0, 100.0);
    let drawn = Rect::new(20.0, 20.0, 50.0, 50.0);

    list.push_clip_rect(clip);
    list.rect(drawn, Color::RED);
    list.pop_clip_rect();

    let commands = list.commands();
    assert_eq!(commands.len(), 3);
    assert!(matches!(commands[0], DrawCommand::PushClipRect { bounds } if bounds == clip), "{:?}", commands[0]);
    assert!(matches!(commands[1], DrawCommand::Rect { bounds, .. } if bounds == drawn), "{:?}", commands[1]);
    assert!(matches!(commands[2], DrawCommand::PopClipRect));
}

#[test]
fn test_elevated_layer_escapes_content_clip_pair() {
    // The add-component-popup bug in miniature: a Floating command
    // recorded INSIDE a Content clip pair must flush AFTER PopClipRect,
    // physically escaping the clip.
    let mut list = DrawList::new();
    list.push_clip_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
    list.push_layer(UiLayer::Floating);
    list.rect(Rect::new(100.0, 100.0, 50.0, 50.0), Color::RED);
    list.pop_layer();
    list.pop_clip_rect();

    list.flush_layers();

    let popup_pos = list
        .commands()
        .iter()
        .position(|c| matches!(c, DrawCommand::Rect { .. }))
        .expect("popup rect present");
    let pop_pos = list
        .commands()
        .iter()
        .position(|c| matches!(c, DrawCommand::PopClipRect))
        .expect("PopClipRect present");
    assert!(popup_pos > pop_pos, "Floating command flushed outside the clip pair");
}
