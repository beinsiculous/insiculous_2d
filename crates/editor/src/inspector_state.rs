//! Inspector view state that outlives a frame: which component sections
//! are collapsed, which Advanced disclosures are open, and which colour row
//! owns the colour editor.
//!
//! None of it is scene data — an entity's rows look the same after a reload
//! — so only the collapsed set is persisted, in the editor preferences.

use std::collections::BTreeSet;

use ecs::EntityId;
use glam::Vec4;
use ui::Rect;

/// The colour row the colour editor is open on: which entity, which
/// component by registry type name, and which field of it.
///
/// The component is named, not indexed: a walk-time index moves when an
/// undo removes a component above it, and the editor would then be sitting
/// on whichever colour row took that place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorEditorTarget {
    pub entity: EntityId,
    pub component: String,
    pub field_index: usize,
}

/// An open colour editor: its row, where that row last drew its swatch and
/// what it last showed, whether the row rendered this frame, and the edit
/// the popup made that the row has not taken yet.
#[derive(Debug, Clone)]
struct OpenColorEditor {
    target: ColorEditorTarget,
    anchor: Rect,
    value: Vec4,
    seen: bool,
    pending: Option<Vec4>,
    /// The popup asked to close on the same frame it produced an edit; the
    /// row must take that edit first, so the close waits for the next pass.
    closing: bool,
}

/// Per-inspector view state, held as one field on the editor context.
#[derive(Debug, Default)]
pub struct InspectorState {
    /// Registry type names whose section is collapsed to its header.
    collapsed: BTreeSet<String>,
    /// Registry type names whose Advanced disclosure is open.
    advanced_open: BTreeSet<String>,
    color_editor: Option<OpenColorEditor>,
}

impl InspectorState {
    /// Whether `type_name`'s section is collapsed to its header row.
    pub fn is_collapsed(&self, type_name: &str) -> bool {
        self.collapsed.contains(type_name)
    }

    /// Collapse an expanded section, expand a collapsed one.
    pub fn toggle_collapsed(&mut self, type_name: &str) {
        if !self.collapsed.remove(type_name) {
            self.collapsed.insert(type_name.to_string());
        }
    }

    /// The collapsed type names, sorted — what the preferences persist.
    pub fn collapsed_names(&self) -> Vec<String> {
        self.collapsed.iter().cloned().collect()
    }

    /// Adopt a persisted collapsed set. Names the registry no longer knows
    /// are kept rather than dropped: a component removed from a scene must
    /// still open collapsed when it comes back.
    pub fn set_collapsed_names(&mut self, names: Vec<String>) {
        self.collapsed = names.into_iter().collect();
    }

    /// Whether `type_name`'s Advanced disclosure is open.
    pub fn is_advanced_open(&self, type_name: &str) -> bool {
        self.advanced_open.contains(type_name)
    }

    /// Open a closed Advanced disclosure, close an open one.
    pub fn toggle_advanced(&mut self, type_name: &str) {
        if !self.advanced_open.remove(type_name) {
            self.advanced_open.insert(type_name.to_string());
        }
    }

    /// The colour row the editor is open on, if any.
    pub fn color_editor(&self) -> Option<&ColorEditorTarget> {
        self.color_editor.as_ref().map(|open| &open.target)
    }

    /// Open the colour editor on one row, replacing any open one. The row
    /// has just drawn, so it counts as seen: the popup pass runs before the
    /// panels and would otherwise close it before it ever appeared.
    pub fn open_color_editor(&mut self, target: ColorEditorTarget, anchor: Rect, value: Vec4) {
        self.color_editor = Some(OpenColorEditor {
            target,
            anchor,
            value,
            seen: true,
            pending: None,
            closing: false,
        });
    }

    /// The colour row reporting that it rendered, with where its swatch is
    /// and what it shows.
    pub fn note_color_row(&mut self, anchor: Rect, value: Vec4) {
        if let Some(open) = &mut self.color_editor {
            open.anchor = anchor;
            open.value = value;
            open.seen = true;
        }
    }

    /// Where to draw the popup and what colour to show, for the pass.
    pub fn color_editor_frame(&self) -> Option<(Rect, Vec4)> {
        self.color_editor.as_ref().map(|open| (open.anchor, open.value))
    }

    /// Whether the row that owns the editor rendered since the last pass. A
    /// row that stopped rendering — its entity deselected, its section
    /// collapsed, its component removed, its panel hidden — takes the
    /// editor with it.
    pub fn color_editor_row_was_seen(&self) -> bool {
        self.color_editor.as_ref().is_some_and(|open| open.seen)
    }

    /// Clear the seen mark so the coming frame's walk has to set it again.
    pub fn clear_color_editor_seen(&mut self) {
        if let Some(open) = &mut self.color_editor {
            open.seen = false;
        }
    }

    /// Record the colour the popup edited to, for the row to write.
    pub fn set_color_edit(&mut self, value: Vec4) {
        if let Some(open) = &mut self.color_editor {
            open.pending = Some(value);
            open.value = value;
        }
    }

    /// Take the popup's edit, if this is the row that owns the editor.
    pub fn take_color_edit_for(&mut self, entity: EntityId, component: &str) -> Option<Vec4> {
        let open = self.color_editor.as_mut()?;
        if open.target.entity != entity || open.target.component != component {
            return None;
        }
        open.pending.take()
    }

    /// The field index of the editor's row, when it is open on this
    /// entity's block of `component`.
    pub fn color_editor_field(&self, entity: EntityId, component: &str) -> Option<usize> {
        let open = self.color_editor.as_ref()?;
        (open.target.entity == entity && open.target.component == component)
            .then_some(open.target.field_index)
    }

    /// Close the colour editor; `true` when one was open. Every path that
    /// invalidates the target — Escape, a click outside, a different
    /// entity, entering Playing — comes through here.
    pub fn close_color_editor(&mut self) -> bool {
        self.color_editor.take().is_some()
    }

    /// Close once the row has had a walk to take the pending edit: a click
    /// outside the popup commits a focused field on that same press, and
    /// closing at once would drop that commit.
    pub fn close_color_editor_after_edit(&mut self) {
        if let Some(open) = &mut self.color_editor {
            open.closing = true;
        }
    }

    /// Whether a close is waiting on the row's next walk.
    pub fn color_editor_closing(&self) -> bool {
        self.color_editor.as_ref().is_some_and(|open| open.closing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapsed_sections_toggle_round_trip_through_the_persisted_names() {
        let mut state = InspectorState::default();
        assert!(!state.is_collapsed("Sprite"));
        state.toggle_collapsed("Sprite");
        state.toggle_collapsed("Collider");
        assert!(state.is_collapsed("Sprite"));
        assert_eq!(state.collapsed_names(), vec!["Collider".to_string(), "Sprite".to_string()]);

        state.toggle_collapsed("Sprite");
        assert!(!state.is_collapsed("Sprite"), "toggling again expands it");

        let mut restored = InspectorState::default();
        restored.set_collapsed_names(vec!["Sprite".to_string(), "Gone".to_string()]);
        assert!(restored.is_collapsed("Sprite"));
        assert!(
            restored.is_collapsed("Gone"),
            "a name the registry no longer knows is kept, not dropped"
        );
    }

    #[test]
    fn test_the_colour_editor_holds_one_target_and_reports_whether_closing_did_anything() {
        let mut state = InspectorState::default();
        assert!(!state.close_color_editor(), "nothing to close");
        let entity = EntityId::with_generation(4, 1);
        let target = ColorEditorTarget {
            entity,
            component: "Sprite".to_string(),
            field_index: 3,
        };
        let anchor = Rect::new(10.0, 20.0, 20.0, 20.0);
        state.open_color_editor(target.clone(), anchor, Vec4::ONE);

        assert_eq!(state.color_editor(), Some(&target));
        assert_eq!(state.color_editor_field(entity, "Sprite"), Some(3));
        assert_eq!(
            state.color_editor_field(entity, "GridBackdrop"),
            None,
            "another component's colour row is not the target"
        );
        assert_eq!(state.color_editor_frame(), Some((anchor, Vec4::ONE)));
        assert!(state.color_editor_row_was_seen(), "the row that opened it had just drawn");

        assert!(state.close_color_editor(), "closing an open editor reports it");
        assert_eq!(state.color_editor(), None);
    }

    #[test]
    fn test_the_popups_edit_is_taken_by_its_own_row_and_by_no_other() {
        // The popup draws before the panels, so its edit reaches the world
        // later in the same frame, through the row that owns the colour —
        // that row is the only place that knows which component field it is.
        let mut state = InspectorState::default();
        let entity = EntityId::with_generation(7, 1);
        let anchor = Rect::new(0.0, 0.0, 20.0, 20.0);
        state.open_color_editor(
            ColorEditorTarget { entity, component: "Sprite".to_string(), field_index: 3 },
            anchor,
            Vec4::ONE,
        );
        state.set_color_edit(Vec4::new(0.5, 0.5, 0.5, 1.0));

        assert_eq!(state.take_color_edit_for(entity, "GridBackdrop"), None);
        assert_eq!(
            state.take_color_edit_for(EntityId::with_generation(8, 1), "Sprite"),
            None,
            "another entity's Sprite row is not the target either"
        );
        assert_eq!(state.take_color_edit_for(entity, "Sprite"), Some(Vec4::new(0.5, 0.5, 0.5, 1.0)));
        assert_eq!(state.take_color_edit_for(entity, "Sprite"), None, "an edit is written once");
        assert_eq!(
            state.color_editor_frame().map(|(_, value)| value),
            Some(Vec4::new(0.5, 0.5, 0.5, 1.0)),
            "the popup keeps showing what it edited to while the row catches up"
        );
    }

    #[test]
    fn test_a_row_that_stops_rendering_takes_the_editor_with_it() {
        let mut state = InspectorState::default();
        let entity = EntityId::with_generation(2, 1);
        let anchor = Rect::new(4.0, 8.0, 20.0, 20.0);
        state.open_color_editor(
            ColorEditorTarget { entity, component: "Sprite".to_string(), field_index: 3 },
            anchor,
            Vec4::ONE,
        );

        state.clear_color_editor_seen();
        assert!(!state.color_editor_row_was_seen(), "no walk has drawn the row since");

        let moved = Rect::new(4.0, 40.0, 20.0, 20.0);
        state.note_color_row(moved, Vec4::ZERO);
        assert!(state.color_editor_row_was_seen());
        assert_eq!(
            state.color_editor_frame(),
            Some((moved, Vec4::ZERO)),
            "the popup follows its swatch as the panel scrolls"
        );
    }
}
