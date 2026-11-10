//! Synchronous page coordination for file mutations inside an editor frame.

#[cfg(any(target_arch = "wasm32", test))]
use editor_integration::{FileMutation, MutationPhase};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static FILE_MUTATION_GUARD: std::cell::RefCell<Option<js_sys::Function>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Register the page's synchronous file guard. It receives JSON and returns an empty
/// string to proceed, or a refusal to show in the editor's prompt.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_set_file_mutation_guard(function: js_sys::Function) {
    FILE_MUTATION_GUARD.with(|guard| *guard.borrow_mut() = Some(function));
}

#[cfg(any(target_arch = "wasm32", test))]
fn mutation_json(mutation: &FileMutation) -> String {
    let phase = match mutation.phase {
        MutationPhase::Prepare => "prepare",
        MutationPhase::Done => "done",
        MutationPhase::Failed => "failed",
    };
    // The editor's root is the project's assets directory; bridge reads and writes use
    // the containing project root, so the open script must receive that same path here.
    let asset_directory = crate::projects::project_asset_base("");
    let project_path = |path: &str| format!("{}/{path}", asset_directory.trim_start_matches('/'));
    let removed: Vec<_> = mutation.removed.iter().map(|path| project_path(path)).collect();
    let renamed: Vec<_> = mutation.renamed.iter().map(|(old, new)| (project_path(old), project_path(new))).collect();
    serde_json::json!({
        "phase": phase,
        "removed": removed,
        "renamed": renamed,
    }).to_string()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn guard_file_mutation(mutation: &FileMutation) -> Result<(), String> {
    let function = FILE_MUTATION_GUARD.with(|guard| guard.borrow().clone());
    let Some(function) = function else { return Ok(()) };
    let result = function.call1(&JsValue::UNDEFINED, &JsValue::from_str(&mutation_json(mutation)))
        .map_err(|error| format!("the scripts panel could not prepare the file change: {error:?}"))?;
    let refusal = result.as_string()
        .ok_or_else(|| "the scripts panel's file guard did not return text".to_string())?;
    if refusal.is_empty() { Ok(()) } else { Err(refusal) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_guard_json_carries_each_phase_and_both_rename_paths() {
        for (phase, name) in [
            (MutationPhase::Prepare, "prepare"),
            (MutationPhase::Done, "done"),
            (MutationPhase::Failed, "failed"),
        ] {
            let mutation = FileMutation {
                phase,
                removed: vec!["scripts/old.rhai".into()],
                renamed: vec![("scripts/from.rhai".into(), "scripts/to.rhai".into())],
            };
            let payload: serde_json::Value = serde_json::from_str(&mutation_json(&mutation)).unwrap();
            assert_eq!(payload["phase"], name);
            assert_eq!(payload["removed"][0], "assets/scripts/old.rhai");
            assert_eq!(payload["renamed"][0], serde_json::json!([
                "assets/scripts/from.rhai", "assets/scripts/to.rhai"
            ]));
        }
    }
}
