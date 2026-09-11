//! What the UI asks the pointer to look like.
//!
//! A widget that wants a particular pointer shape — a dock edge asking for
//! the resize cursor while the pointer is over it — records the request on
//! the context instead of reaching for a window it does not own. The host
//! reads [`UIContext::requested_cursor`] once at the end of the frame and
//! asks the platform, which is also what keeps this crate free of a
//! windowing dependency.

use super::UIContext;

/// The pointer shapes the UI can ask for — this crate's own vocabulary,
/// mapped to the platform's by whichever host owns a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorIcon {
    /// The platform's ordinary arrow.
    #[default]
    Default,
    /// A vertical edge: drag left or right.
    ColResize,
    /// A horizontal edge: drag up or down.
    RowResize,
    /// A clickable thing that is not a text field.
    Pointer,
    /// A text caret.
    Text,
    /// Something pickable that can be dragged.
    Grab,
    /// A drag in progress.
    Grabbing,
}

impl UIContext {
    /// Ask the pointer to take `icon`'s shape. The last request of a frame
    /// wins: the pointer rests over one thing at a time.
    pub fn request_cursor(&mut self, icon: CursorIcon) {
        self.requested_cursor = icon;
    }

    /// The shape this frame asked for — [`CursorIcon::Default`] when no
    /// widget asked for anything.
    pub fn requested_cursor(&self) -> CursorIcon {
        self.requested_cursor
    }
}
