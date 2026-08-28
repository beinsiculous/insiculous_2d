# Technical Debt: engine_core — LIVE (open items only)

Last audited: June 2026 (July 2026: Game Programming Patterns audit).
Resolved history: root `log_archive.md` § engine_core.

## Game Programming Patterns Audit (July 2026 — closed; history in `log_archive.md`)
(GPP-07 and DRY-010 resolved Jul 13 2026 — runtime `spawn_prefab` + merge-layer helper; GPP-03 closed Jul 13 2026 with game 3's rule-of-three — both promotion rounds in `log_archive.md`.)
- [ ] **GPP-05 (Low, Game Loop):** no render interpolation between fixed physics steps — only act if stutter observed on high-refresh displays.
- [ ] **GPP-L2 (Low, Spatial Partition):** O(n²) tag scans in `behavior_runner.rs:473-496` — grid/sensor-based lookup if chaser/collectible counts grow.

## Low Priority

### [KISS-002] LifecycleManager over-engineered for single-threaded use
- **File:** `lifecycle.rs` — 7 states + 2 lock flags; `Clone` creates *independent* locks (`lifecycle.rs:51-56`). Document Clone semantics or simplify.

### [DRY-010] `merge_components()` duplicates its merge loop
- **File:** `scene_loader.rs:265-299` — same find-and-replace loop written twice. Extract `apply_component_layer()` helper.

### [DRY-003] GameContext constructed twice with identical fields
- **File:** `game.rs:406-418`, `game.rs:600-612`. Macro/helper blocked by split borrows — acceptable to leave, documented.

### [DRY-011] White texture handle `TextureHandle { id: 0 }` hardcoded in multiple places
- **File:** `ui_integration.rs:51,258` (+ similar in `game.rs`). Use `TextureHandle::WHITE`.

### [SIZE-001] File sizes vs 600-line guideline
- `scene_loader.rs`: ~583 non-test lines — extract component factory helpers from the `add_component_to_entity` match.
- `scene_serializer_tests.rs`: 747 lines, all tests (pre-existing; E3/E4 shrank it from 777) — split by component family on the next substantive change.
- `game.rs`: RESOLVED Jul 16 2026 — render tail extracted to child module `game/render.rs` (530 lines remain).
- `behavior_runner.rs`: 763 lines (pre-existing) — split the per-behavior handlers into a `behavior_runner/` directory on the next substantive change.

### [DOC-001] Doc gaps
- `scene_serializer.rs:190` `behavior_to_data()` undocumented; `ui_manager.rs:22` `begin_frame()` undocumented; `contexts.rs:77` `lines` buffer append-vs-replace contract undocumented.
