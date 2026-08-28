# Technical Debt: ecs — LIVE (open items only)

Last audited: February 2026 (July 2026: Game Programming Patterns audit).
Resolved history: root `log_archive.md` § ecs.

## Game Programming Patterns Audit (July 2026 — closed; history in `log_archive.md`)
(GPP-04 + SRP-003 resolved Jul 13 2026 — dirty-flagged transform propagation, see `log_archive.md`.)
- [ ] **GPP-02 (Decision of record, Data Locality):** `ComponentStore` = `HashMap<EntityId, Box<dyn Component>>` is the accepted simplicity tradeoff. Future path: dense `Vec<T>` columns / archetype storage + bitset queries (see Future Enhancements below). **Trigger to revisit:** profiling shows component access dominating a frame, or games routinely exceed ~a few thousand live entities.
- [ ] **GPP-L1 (Low):** `world.entities()` allocates a `Vec<EntityId>` per call in hot paths — prefer `entity_ids()` iterator.
- [ ] **GPP-L12 (Low):** `EventBus` single-frame lifetime — document the emit-before-read contract.

## Open Items (pre-July-2026 audits)

### [DRY-001] Repeated entity existence checks — Low
- **File:** `world.rs:216-284` — `if !self.entities.contains_key(entity_id)` repeated 7+ times.
- **Fix:** `ensure_entity_exists()` helper. (Explicit checks aid debugging, hence Low.)

### [DRY-003] Duplicate matrix computation in GlobalTransform2D — Low
- **File:** `hierarchy.rs:143-201` — sin/cos computed multiple times in `matrix()`, `mul_transform()`, `transform_point()`.
- **Fix:** cache sin/cos or extract rotation application helper.

### [DRY-004] Repeated builder pattern in audio_components.rs — Low
- **File:** `audio_components.rs` — three components with nearly identical `with_volume()` clamping.
- **Fix:** `VolumeControl` trait with default impl, or helper fn. (Overlaps common DRY-002.)

---

## Future Enhancements (Not Technical Debt)

- **Cache-friendly storage (gated on the GPP-02 trigger above).** When the trigger fires, evaluate the cheaper step FIRST: a **sparse-set layout** (dense `Vec<T>` per component type + entity→index map) gets the cache contiguity without archetype-migration complexity — and migration cost matters here, because the editor adds/removes components constantly (inspector, undo/redo), a workload archetypes punish and HashMaps tolerate. Full archetype storage (typed columns, migration on add/remove, drop handling) only if sparse sets measurably aren't enough. Decision of record Jul 13 2026.
- **System scheduling:** dependency graph, parallel execution, system groups.
- **Component introspection:** reflection, dynamic add/remove by name (pairs with GPP-16), editor integration.
- **Memory pooling** for entity/component allocations.

## Metrics

| Metric | Value |
|--------|-------|
| Test coverage | 187 tests (100% pass rate) |
| `#[allow(dead_code)]` | 0 |
| High priority open | 0 |
| Medium priority open | 0 |
| Low priority open | 5 |
