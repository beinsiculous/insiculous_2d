use std::sync::Arc;

use glam::Vec2;

use super::*;

#[test]
fn test_draw_list_rect() {
    let mut list = DrawList::new();
    list.rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::RED);
    assert_eq!(list.len(), 1);

    if let DrawCommand::Rect { bounds, color, corner_radius, .. } = &list.commands()[0] {
        assert_eq!(bounds.width, 100.0);
        assert_eq!(bounds.height, 50.0);
        assert_eq!(*color, Color::RED);
        assert_eq!(*corner_radius, 0.0);
    } else {
        panic!("Expected Rect command");
    }
}

#[test]
fn test_draw_list_rect_rounded() {
    let mut list = DrawList::new();
    list.rect_rounded(Rect::new(0.0, 0.0, 100.0, 50.0), Color::BLUE, 8.0);

    if let DrawCommand::Rect { corner_radius, .. } = &list.commands()[0] {
        assert_eq!(*corner_radius, 8.0);
    } else {
        panic!("Expected Rect command");
    }
}

#[test]
fn test_draw_list_text_placeholder() {
    let mut list = DrawList::new();
    list.text_placeholder("World", Vec2::new(50.0, 60.0), Color::RED, 24.0);

    if let DrawCommand::TextPlaceholder { text, position, font_size, color, .. } = &list.commands()[0] {
        assert_eq!(text, "World");
        assert_eq!(*position, Vec2::new(50.0, 60.0));
        assert_eq!(*font_size, 24.0);
        assert_eq!(*color, Color::RED);
    } else {
        panic!("Expected TextPlaceholder command");
    }
}

#[test]
fn test_draw_list_text_with_data() {
    let mut list = DrawList::new();
    let text_data = TextDrawData {
        text: "Test".to_string(),
        position: Vec2::new(100.0, 200.0),
        color: Color::GREEN,
        font_size: 32.0,
        font_id: 1,
        width: 80.0,
        height: 32.0,
        glyphs: vec![
            GlyphDrawData {
                bitmap: Arc::from([255u8; 16]),
                width: 4,
                height: 4,
                x: 0.0,
                y: 0.0,
                character: 'T',
            },
        ],
    };
    list.text(text_data);

    if let DrawCommand::Text { data, .. } = &list.commands()[0] {
        assert_eq!(data.text, "Test");
        assert_eq!(data.position, Vec2::new(100.0, 200.0));
        assert_eq!(data.glyphs.len(), 1);
        assert_eq!(data.glyphs[0].character, 'T');
    } else {
        panic!("Expected Text command");
    }
}

#[test]
fn test_draw_list_circle() {
    let mut list = DrawList::new();
    list.circle(Vec2::new(50.0, 50.0), 25.0, Color::GREEN);

    if let DrawCommand::Circle { center, radius, color, .. } = &list.commands()[0] {
        assert_eq!(*center, Vec2::new(50.0, 50.0));
        assert_eq!(*radius, 25.0);
        assert_eq!(*color, Color::GREEN);
    } else {
        panic!("Expected Circle command");
    }
}

#[test]
fn test_draw_list_clear() {
    let mut list = DrawList::new();
    list.rect(Rect::default(), Color::RED);
    list.rect(Rect::default(), Color::BLUE);
    assert_eq!(list.len(), 2);

    list.clear();
    assert!(list.is_empty());
}

#[test]
fn test_draw_command_depth() {
    let cmd = DrawCommand::Rect {
        bounds: Rect::default(),
        color: Color::RED,
        corner_radius: 0.0,
        depth: 5.0,
    };
    assert_eq!(cmd.depth(), 5.0);
}

#[test]
fn test_draw_list_depth_ordering() {
    let mut list = DrawList::new();
    list.rect(Rect::default(), Color::RED);
    list.rect(Rect::default(), Color::BLUE);
    list.rect(Rect::default(), Color::GREEN);

    // Each command should have increasing depth
    let depths: Vec<f32> = list.commands().iter().map(|c| c.depth()).collect();
    assert!(depths[0] < depths[1]);
    assert!(depths[1] < depths[2]);
}

#[test]
fn test_overlay_commands_render_above_base_band() {
    let floating_base = UiLayer::Floating.depth_base();
    let mut list = DrawList::new();
    list.rect(Rect::default(), Color::RED); // content band
    list.begin_overlay();
    list.rect(Rect::default(), Color::BLUE); // floating band
    list.rect(Rect::default(), Color::GREEN); // floating band
    list.end_overlay();
    list.rect(Rect::default(), Color::RED); // content band again

    list.flush_layers();
    let depths: Vec<f32> = list.commands().iter().map(|c| c.depth()).collect();
    // Flushed order: content, content, floating, floating.
    assert!(depths[0] < floating_base, "content command stays in its band");
    assert!(depths[1] < floating_base, "end_overlay returns to content band");
    assert!(depths[1] > depths[0], "content band stays monotonic");
    assert!(depths[2] >= floating_base, "overlay command is elevated");
    assert!(depths[3] > depths[2], "floating band stays monotonic");
}

#[test]
fn test_clear_resets_overlay_mode() {
    let mut list = DrawList::new();
    list.begin_overlay();
    assert!(list.is_overlay());

    list.clear();
    assert!(!list.is_overlay());

    list.rect(Rect::default(), Color::RED);
    assert!(list.commands()[0].depth() < UiLayer::PanelChrome.depth_base());
}

#[test]
fn test_draw_list_clip_rect() {
    let mut list = DrawList::new();
    let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);

    list.push_clip_rect(bounds);
    list.rect(Rect::new(20.0, 20.0, 50.0, 50.0), Color::RED);
    list.pop_clip_rect();

    assert_eq!(list.len(), 3);

    // First command should be PushClipRect
    if let DrawCommand::PushClipRect { bounds: clip_bounds } = &list.commands()[0] {
        assert_eq!(clip_bounds.x, 10.0);
    } else {
        panic!("Expected PushClipRect");
    }

    // Last command should be PopClipRect
    assert!(matches!(list.commands()[2], DrawCommand::PopClipRect));
}

// ==================== UiLayer bands ====================

#[test]
fn test_layers_flush_in_enum_order() {
    // Submission order deliberately scrambled: DragGhost first, then
    // PanelChrome, then Content — the flushed stream must come out
    // Content, PanelChrome, DragGhost.
    let mut list = DrawList::new();
    list.push_layer(UiLayer::DragGhost);
    list.circle(Vec2::ZERO, 1.0, Color::RED);
    list.pop_layer();
    list.push_layer(UiLayer::PanelChrome);
    list.line(Vec2::ZERO, Vec2::ONE, Color::GREEN, 1.0);
    list.pop_layer();
    list.rect(Rect::default(), Color::BLUE);

    list.flush_layers();
    assert!(matches!(list.commands()[0], DrawCommand::Rect { .. }), "Content first");
    assert!(matches!(list.commands()[1], DrawCommand::Line { .. }), "PanelChrome second");
    assert!(matches!(list.commands()[2], DrawCommand::Circle { .. }), "DragGhost last");
}

#[test]
fn test_layer_depths_are_banded() {
    let mut list = DrawList::new();
    for layer in UiLayer::ALL {
        list.push_layer(layer);
        list.rect(Rect::default(), Color::RED);
        list.pop_layer();
    }
    list.flush_layers();

    for (cmd, layer) in list.commands().iter().zip(UiLayer::ALL) {
        let depth = cmd.depth();
        assert!(
            depth >= layer.depth_base() && depth < layer.depth_base() + 15.0,
            "{layer:?} command depth {depth} outside its band"
        );
    }
    // Content must be the lowest band — everything else overlays it.
    assert!(UiLayer::ALL[1..].iter().all(|l| l.depth_base() > UiLayer::Content.depth_base()));
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

#[test]
fn test_push_pop_layer_nest() {
    let mut list = DrawList::new();
    list.push_layer(UiLayer::Modal);
    assert_eq!(list.current_layer(), UiLayer::Modal);
    list.push_layer(UiLayer::Tooltip);
    assert_eq!(list.current_layer(), UiLayer::Tooltip);
    list.pop_layer();
    assert_eq!(list.current_layer(), UiLayer::Modal, "nested pop returns to outer layer");
    list.pop_layer();
    assert_eq!(list.current_layer(), UiLayer::Content);
    list.pop_layer(); // extra pop is a safe no-op
    assert_eq!(list.current_layer(), UiLayer::Content);
}

#[test]
fn test_flush_is_idempotent_and_clear_resets_stack() {
    let mut list = DrawList::new();
    list.push_layer(UiLayer::Floating);
    list.rect(Rect::default(), Color::RED);
    // (deliberately un-popped — clear must recover)

    list.flush_layers();
    let after_first = list.commands().len();
    list.flush_layers();
    assert_eq!(list.commands().len(), after_first, "flush is idempotent");

    list.clear();
    assert!(list.is_empty());
    assert_eq!(list.current_layer(), UiLayer::Content, "clear resets the layer stack");
}
