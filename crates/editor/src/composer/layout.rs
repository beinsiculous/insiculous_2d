//! Where everything in the composer panel goes: a pure function of the panel's bounds and
//! the composer's state, so the render, the hit tests and the tests read one geometry.
//!
//! Three modes by width. **Full** puts the sidebar left, the roll in the middle and the
//! note list right, under the transport. **Compact** (below [`COMPACT_BELOW_WIDTH`])
//! narrows the columns and shows one sidebar section at a time behind a row of tabs.
//! **Tabs** (below [`TABS_BELOW_WIDTH`]) shows one area at a time — the roll, the notes or
//! the sidebar — behind a row of area tabs. The native editor's minimum window is wider
//! than both thresholds, so only a narrow browser reaches the other two.

use music_midi::TempoMap;
use ui::Rect;

use super::state::{ComposerFocus, ComposerState};

/// Below this panel width the composer is [`ComposerMode::Compact`].
pub const COMPACT_BELOW_WIDTH: f32 = 720.0;
/// Below this panel width the composer is [`ComposerMode::Tabs`].
pub const TABS_BELOW_WIDTH: f32 = 480.0;
/// The transport row along the panel's top.
pub const TRANSPORT_HEIGHT: f32 = 30.0;
/// A row of section or area tabs.
pub const TAB_STRIP_HEIGHT: f32 = 24.0;
/// The column of key names at the roll's left edge.
pub const KEY_LABEL_WIDTH: f32 = 40.0;
/// One key's row in the roll.
pub const KEY_HEIGHT: f32 = 10.0;
/// MIDI's keys, 0 to 127.
pub const KEY_COUNT: u8 = 128;
const HIGHEST_KEY: f32 = 127.0;

const FULL_SIDEBAR_WIDTH: f32 = 260.0;
const FULL_NOTE_LIST_WIDTH: f32 = 230.0;
const COMPACT_SIDEBAR_WIDTH: f32 = 200.0;
const COMPACT_NOTE_LIST_WIDTH: f32 = 170.0;
/// Beat and bar lines closer than this are thinned out, as the scene grid's LOD thins its
/// cells.
const MINIMUM_LINE_SPACING: f32 = 8.0;
/// The sixteenths drawn inside a beat that is wide enough for them.
const SUBDIVISIONS_PER_BEAT: u64 = 4;
/// A bound on the beats one frame walks, so no zoom can stall a frame.
const MAXIMUM_BEATS_WALKED: usize = 4096;

/// See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerMode {
    Full,
    Compact,
    Tabs,
}

impl ComposerMode {
    pub fn for_width(width: f32) -> Self {
        if width < TABS_BELOW_WIDTH {
            ComposerMode::Tabs
        } else if width < COMPACT_BELOW_WIDTH {
            ComposerMode::Compact
        } else {
            ComposerMode::Full
        }
    }
}

/// The panel cut into its areas; an area the mode hides is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposerLayout {
    pub mode: ComposerMode,
    pub transport: Rect,
    /// The area tabs of [`ComposerMode::Tabs`].
    pub area_tabs: Option<Rect>,
    pub sidebar: Option<Rect>,
    /// The section tabs over the sidebar below the full width.
    pub section_tabs: Option<Rect>,
    pub note_list: Option<Rect>,
    pub roll: Option<PianoRollGeometry>,
}

/// Lay the panel out. `ticks_per_quarter` is the song's division (a song with none
/// reads as one, so nothing divides by zero).
pub fn compute(bounds: Rect, state: &ComposerState, ticks_per_quarter: u16) -> ComposerLayout {
    let mode = ComposerMode::for_width(bounds.width);
    let transport = Rect::new(bounds.x, bounds.y, bounds.width, TRANSPORT_HEIGHT.min(bounds.height));
    let mut body = Rect::new(
        bounds.x,
        transport.bottom(),
        bounds.width,
        (bounds.height - transport.height).max(0.0),
    );
    let mut layout = ComposerLayout {
        mode,
        transport,
        area_tabs: None,
        sidebar: None,
        section_tabs: None,
        note_list: None,
        roll: None,
    };
    let (sidebar_width, note_list_width) = match mode {
        ComposerMode::Full => (FULL_SIDEBAR_WIDTH, FULL_NOTE_LIST_WIDTH),
        ComposerMode::Compact => (COMPACT_SIDEBAR_WIDTH, COMPACT_NOTE_LIST_WIDTH),
        ComposerMode::Tabs => {
            let tabs = Rect::new(body.x, body.y, body.width, TAB_STRIP_HEIGHT.min(body.height));
            layout.area_tabs = Some(tabs);
            body = Rect::new(body.x, tabs.bottom(), body.width, (body.height - tabs.height).max(0.0));
            match state.tabs_area {
                ComposerFocus::NoteList => layout.note_list = Some(body),
                ComposerFocus::Sidebar => place_sidebar(&mut layout, body),
                _ => layout.roll = Some(PianoRollGeometry::new(body, state, ticks_per_quarter)),
            }
            return layout;
        }
    };
    let sidebar_width = sidebar_width.min(body.width);
    let note_list_width = note_list_width.min((body.width - sidebar_width).max(0.0));
    let sidebar = Rect::new(body.x, body.y, sidebar_width, body.height);
    let note_list = Rect::new(body.right() - note_list_width, body.y, note_list_width, body.height);
    let roll = Rect::new(
        sidebar.right(),
        body.y,
        (note_list.x - sidebar.right()).max(0.0),
        body.height,
    );
    if mode == ComposerMode::Full {
        layout.sidebar = Some(sidebar);
    } else {
        place_sidebar(&mut layout, sidebar);
    }
    layout.note_list = Some(note_list);
    layout.roll = Some(PianoRollGeometry::new(roll, state, ticks_per_quarter));
    layout
}

/// A sidebar showing one section: its tabs on top, the section below.
fn place_sidebar(layout: &mut ComposerLayout, area: Rect) {
    let tabs = Rect::new(area.x, area.y, area.width, TAB_STRIP_HEIGHT.min(area.height));
    layout.section_tabs = Some(tabs);
    layout.sidebar = Some(Rect::new(
        area.x,
        tabs.bottom(),
        area.width,
        (area.height - tabs.height).max(0.0),
    ));
}

/// What a beat line marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatLineKind {
    Bar,
    Beat,
    Subdivision,
}

/// One vertical line of the roll's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatLine {
    pub tick: u64,
    pub x: f32,
    pub kind: BeatLineKind,
}

/// The roll's mapping between screen pixels and ticks and keys: key 127 at the top, tick
/// 0 at the grid's left edge, both moved by the scroll offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PianoRollGeometry {
    pub bounds: Rect,
    /// The column of key names.
    pub keyboard: Rect,
    /// The notes' area, right of the key names.
    pub grid: Rect,
    pub pixels_per_tick: f32,
    pub key_height: f32,
    pub horizontal_offset: f32,
    pub vertical_offset: f32,
}

impl PianoRollGeometry {
    pub fn new(bounds: Rect, state: &ComposerState, ticks_per_quarter: u16) -> Self {
        let keyboard_width = KEY_LABEL_WIDTH.min(bounds.width);
        Self {
            bounds,
            keyboard: Rect::new(bounds.x, bounds.y, keyboard_width, bounds.height),
            grid: Rect::new(
                bounds.x + keyboard_width,
                bounds.y,
                (bounds.width - keyboard_width).max(0.0),
                bounds.height,
            ),
            pixels_per_tick: state.pixels_per_quarter / f32::from(ticks_per_quarter.max(1)),
            key_height: KEY_HEIGHT,
            horizontal_offset: state.roll_horizontal.offset(),
            vertical_offset: state.roll_vertical.offset(),
        }
    }

    /// The height of all 128 keys, what the vertical scroll covers.
    pub fn content_height(&self) -> f32 {
        f32::from(KEY_COUNT) * self.key_height
    }

    pub fn tick_to_x(&self, tick: u64) -> f32 {
        (f64::from(self.grid.x) + tick as f64 * f64::from(self.pixels_per_tick)
            - f64::from(self.horizontal_offset)) as f32
    }

    /// The nearest tick to `x`; negative left of tick 0, so a drag's delta can be.
    pub fn x_to_tick(&self, x: f32) -> i64 {
        let ticks = (f64::from(x) - f64::from(self.grid.x) + f64::from(self.horizontal_offset))
            / f64::from(self.pixels_per_tick).max(f64::MIN_POSITIVE);
        ticks.round() as i64
    }

    /// The top of `key`'s row.
    pub fn pitch_to_y(&self, key: u8) -> f32 {
        self.grid.y + (HIGHEST_KEY - f32::from(key)) * self.key_height - self.vertical_offset
    }

    /// The key whose row holds `y`; outside 0..=127 above or below the keys.
    pub fn y_to_pitch(&self, y: f32) -> i32 {
        let rows_down = ((y - self.grid.y + self.vertical_offset) / self.key_height).floor();
        HIGHEST_KEY as i32 - rows_down as i32
    }

    /// A note's rectangle: at least three pixels wide, so the shortest note can be seen and
    /// grabbed.
    pub fn note_rect(&self, tick: u64, duration_ticks: u64, key: u8) -> Rect {
        let width = (duration_ticks as f32 * self.pixels_per_tick).max(3.0);
        Rect::new(
            self.tick_to_x(tick),
            self.pitch_to_y(key),
            width,
            (self.key_height - 1.0).max(1.0),
        )
    }

    /// The first and last tick the grid shows.
    pub fn visible_ticks(&self) -> (u64, u64) {
        let first = self.x_to_tick(self.grid.x).max(0) as u64;
        let last = self.x_to_tick(self.grid.right()).max(0) as u64;
        (first, last)
    }

    /// The bar, beat and sixteenth lines over the visible ticks, from the song's tempo map
    /// (its meter changes included), thinned where they would crowd: beats narrower than
    /// eight pixels draw no beat lines, a bar line closer than that to the last is left
    /// out, and sixteenths show only inside a beat wide enough for four.
    pub fn beat_lines(&self, tempo_map: &TempoMap) -> Vec<BeatLine> {
        let (first_tick, last_tick) = self.visible_ticks();
        let mut beat = tempo_map.position_at_tick(first_tick).beat;
        let mut lines = Vec::new();
        let mut last_bar_x = f32::NEG_INFINITY;
        let mut last_line_x = f32::NEG_INFINITY;
        for _ in 0..MAXIMUM_BEATS_WALKED {
            let tick = tempo_map.tick_at_beat(beat);
            if tick > last_tick {
                break;
            }
            let next_tick = tempo_map.tick_at_beat(beat + 1).max(tick + 1);
            let x = self.tick_to_x(tick);
            let beat_width = (next_tick - tick) as f32 * self.pixels_per_tick;
            if tempo_map.position_at_tick(tick).beat_in_bar == 0 {
                if x - last_bar_x >= MINIMUM_LINE_SPACING {
                    lines.push(BeatLine { tick, x, kind: BeatLineKind::Bar });
                    last_bar_x = x;
                    last_line_x = x;
                }
            } else if beat_width >= MINIMUM_LINE_SPACING && x - last_line_x >= MINIMUM_LINE_SPACING {
                lines.push(BeatLine { tick, x, kind: BeatLineKind::Beat });
                last_line_x = x;
            }
            if beat_width >= MINIMUM_LINE_SPACING * SUBDIVISIONS_PER_BEAT as f32 {
                for step in 1..SUBDIVISIONS_PER_BEAT {
                    let sub_tick = tick + (next_tick - tick) * step / SUBDIVISIONS_PER_BEAT;
                    lines.push(BeatLine {
                        tick: sub_tick,
                        x: self.tick_to_x(sub_tick),
                        kind: BeatLineKind::Subdivision,
                    });
                }
            }
            beat += 1;
        }
        lines
    }
}
