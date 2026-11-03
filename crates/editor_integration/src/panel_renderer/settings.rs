//! The External Tools panel: the editor crate's walk drawn over the context's config,
//! with Test and the desktop-only hint decided by the target this build is for.

use editor::EditorContext;

/// Render the settings panel into the panel's content `bounds`. A changed DAW extension
/// list reclassifies the asset browser's entries, so the browser is marked unscanned: its
/// next draw rescans, and a Test before that scans for itself.
pub(super) fn render_settings(editor: &mut EditorContext, ui: &mut ui::UIContext, bounds: common::Rect) {
    let desktop = cfg!(not(target_arch = "wasm32"));
    let extensions_changed =
        editor::settings_panel::render_settings_panel(ui, &editor.theme, &mut editor.settings, &mut editor.external_tools, bounds, desktop);
    if extensions_changed {
        editor.asset_browser.scanned = false;
    }
}
