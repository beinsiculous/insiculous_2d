//! Inline rename: the F2 field on a hierarchy row, and the rule that keeps
//! its keyboard focus honest — focus follows the drawn field.

use ecs::EntityId;

use super::HierarchyPanel;

/// Passes that draw no rows a rename survives before it ends — about half
/// a second at sixty frames: a splitter dragged through zero, even one that
/// pauses there, keeps its rename; a panel left collapsed or hidden does not
/// keep the keyboard for longer than that. Frames, not wall-clock time: a
/// hitch stretches the grace rather than shortening it.
const EMPTY_PASS_GRACE: u8 = 30;

impl HierarchyPanel {
    /// Widget id of an entity's inline rename field — shared by the panel's
    /// render pass and the host's `focus_text_input` call so F2 lands in an
    /// already-focused field.
    pub fn rename_widget_id(entity: EntityId) -> String {
        format!("hierarchy_rename_{}", entity.value())
    }

    /// Enter inline-rename mode for `entity` (the host focuses the field via
    /// `UIContext::focus_text_input` with the same widget id).
    pub fn begin_rename(&mut self, entity: EntityId) {
        self.renaming = Some(entity);
        self.last_rename = Some(entity);
        // The field has not drawn yet; without this grace the first frame
        // would read it as gone.
        self.rename_field_drawn = true;
    }

    /// Leave inline-rename mode without committing (the field's text is
    /// dropped, as Escape does).
    pub fn cancel_rename(&mut self) {
        self.renaming = None;
    }

    /// Row currently in inline-rename mode, if any.
    pub fn renaming(&self) -> Option<EntityId> {
        self.renaming
    }

    /// Focus follows the drawn field: a rename whose field was not drawn
    /// last frame — cancelled from outside, scrolled off the panel, or in a
    /// panel the dock did not render at all — ends, and its keyboard focus
    /// is dropped. `render` runs it first; a frame that does not render the
    /// panel runs it instead, so a hidden panel cannot keep the keyboard.
    pub fn settle_rename_focus(&mut self, ui: &mut ui::UIContext) {
        let drawn_last_frame = std::mem::take(&mut self.rename_field_drawn);
        let any_row_drawn = std::mem::take(&mut self.rows_drawn) > 0;
        let Some(last) = self.last_rename else {
            return;
        };
        if self.renaming == Some(last) {
            if drawn_last_frame {
                self.empty_passes = 0;
                return;
            }
            if !any_row_drawn && self.empty_passes < EMPTY_PASS_GRACE {
                self.empty_passes += 1;
                return;
            }
        }
        self.empty_passes = 0;
        if ui.is_focused(Self::rename_widget_id(last).as_str()) {
            ui.clear_text_focus();
        }
        if self.renaming == Some(last) {
            self.renaming = None;
        }
        self.last_rename = None;
    }
}
