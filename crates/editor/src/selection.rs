//! Entity selection management for the editor.
//!
//! The Selection struct tracks which entities are currently selected in the
//! editor and provides methods for manipulating the selection.
//!
//! Ordering contract: the selection is insertion-ordered (`IndexSet`), so
//! `selected()` always iterates in the order entities were selected and the
//! primary falls back deterministically — the inspector, gizmo pivot, and
//! undo child ordering must never depend on run-to-run hash order.

use ecs::EntityId;
use indexmap::IndexSet;

/// Manages the current entity selection in the editor.
///
/// Supports single and multi-selection of entities, with methods for
/// common selection operations like toggle, add, remove, and clear.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Currently selected entities, in the order they were selected.
    /// CAUTION: bare `IndexSet::remove` is `swap_remove` and breaks insertion
    /// order — always use `shift_remove` here.
    selected: IndexSet<EntityId>,
    /// Primary selection (the "focus" entity for property editing)
    primary: Option<EntityId>,
}

impl Selection {
    /// Create a new empty selection.
    pub fn new() -> Self {
        Self {
            selected: IndexSet::new(),
            primary: None,
        }
    }

    /// Check if the selection is empty.
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// Get the number of selected entities.
    pub fn len(&self) -> usize {
        self.selected.len()
    }

    /// Check if an entity is selected.
    pub fn contains(&self, entity: EntityId) -> bool {
        self.selected.contains(&entity)
    }

    /// Get the primary selected entity (for property editing).
    pub fn primary(&self) -> Option<EntityId> {
        self.primary
    }

    /// Get all selected entities, in the order they were selected.
    pub fn selected(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.selected.iter().copied()
    }

    /// Select a single entity, clearing any previous selection.
    pub fn select(&mut self, entity: EntityId) {
        self.selected.clear();
        self.selected.insert(entity);
        self.primary = Some(entity);
    }

    /// Add an entity to the current selection (multi-select).
    ///
    /// Re-adding an already-selected entity keeps its original position.
    pub fn add(&mut self, entity: EntityId) {
        self.selected.insert(entity);
        if self.primary.is_none() {
            self.primary = Some(entity);
        }
    }

    /// Remove an entity from the selection. If it was the primary, the
    /// earliest remaining selected entity becomes the new primary.
    pub fn remove(&mut self, entity: EntityId) {
        self.selected.shift_remove(&entity);
        if self.primary == Some(entity) {
            self.primary = self.selected.first().copied();
        }
    }

    /// Toggle an entity's selection state.
    pub fn toggle(&mut self, entity: EntityId) {
        if self.selected.contains(&entity) {
            self.remove(entity);
        } else {
            self.add(entity);
        }
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.selected.clear();
        self.primary = None;
    }

    /// Select multiple entities, clearing any previous selection.
    ///
    /// Order is preserved (duplicates keep their first position) and the
    /// first given entity becomes the primary; empty input clears everything.
    pub fn select_multiple(&mut self, entities: impl IntoIterator<Item = EntityId>) {
        self.selected.clear();
        self.selected.extend(entities);
        self.primary = self.selected.first().copied();
    }

    /// The inspector's heading for the primary entity, saying how many
    /// others are selected with it (#51): `Entity: 7` alone, `Entity: 7
    /// (1 of 5 selected)` in a multi-selection.
    pub fn inspector_heading(&self) -> Option<String> {
        let primary = self.primary()?;
        Some(if self.len() > 1 {
            format!("Entity: {}  (1 of {} selected)", primary.value(), self.len())
        } else {
            format!("Entity: {}", primary.value())
        })
    }

    /// Set the primary selection (must be in the current selection).
    pub fn set_primary(&mut self, entity: EntityId) {
        if self.selected.contains(&entity) {
            self.primary = Some(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u64) -> EntityId {
        EntityId::with_generation(id, 1)
    }

    #[test]
    fn test_inspector_heading_counts_the_rest_of_a_multi_selection() {
        let mut selection = Selection::new();
        assert_eq!(selection.inspector_heading(), None);
        selection.select(entity(7));
        assert_eq!(selection.inspector_heading().as_deref(), Some("Entity: 7"));
        selection.add(entity(8));
        selection.add(entity(9));
        assert_eq!(
            selection.inspector_heading().as_deref(),
            Some("Entity: 7  (1 of 3 selected)")
        );
    }

    #[test]
    fn test_selection_new() {
        let selection = Selection::new();
        assert!(selection.is_empty());
        assert_eq!(selection.len(), 0);
        assert!(selection.primary().is_none());
    }

    #[test]
    fn test_selection_select() {
        let mut selection = Selection::new();
        let e1 = entity(1);

        selection.select(e1);

        assert!(!selection.is_empty());
        assert_eq!(selection.len(), 1);
        assert!(selection.contains(e1));
        assert_eq!(selection.primary(), Some(e1));
    }

    #[test]
    fn test_selection_select_clears_previous() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.select(e1);
        selection.select(e2);

        assert_eq!(selection.len(), 1);
        assert!(!selection.contains(e1));
        assert!(selection.contains(e2));
        assert_eq!(selection.primary(), Some(e2));
    }

    #[test]
    fn test_selection_add() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);

        assert_eq!(selection.len(), 2);
        assert!(selection.contains(e1));
        assert!(selection.contains(e2));
        // Primary should be the first added
        assert_eq!(selection.primary(), Some(e1));
    }

    #[test]
    fn test_selection_remove() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);
        selection.remove(e1);

        assert_eq!(selection.len(), 1);
        assert!(!selection.contains(e1));
        assert!(selection.contains(e2));
    }

    #[test]
    fn test_selection_remove_primary_updates() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);
        assert_eq!(selection.primary(), Some(e1));

        selection.remove(e1);
        // Primary should update to remaining entity
        assert_eq!(selection.primary(), Some(e2));
    }

    #[test]
    fn test_selection_toggle() {
        let mut selection = Selection::new();
        let e1 = entity(1);

        selection.toggle(e1);
        assert!(selection.contains(e1));

        selection.toggle(e1);
        assert!(!selection.contains(e1));
    }

    #[test]
    fn test_selection_clear() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);
        selection.clear();

        assert!(selection.is_empty());
        assert!(selection.primary().is_none());
    }

    #[test]
    fn test_selection_select_multiple() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);
        let e3 = entity(3);

        selection.select(e3); // Previous selection
        selection.select_multiple([e1, e2]);

        assert_eq!(selection.len(), 2);
        assert!(selection.contains(e1));
        assert!(selection.contains(e2));
        assert!(!selection.contains(e3));
    }

    #[test]
    fn test_selection_set_primary() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);
        selection.set_primary(e2);

        assert_eq!(selection.primary(), Some(e2));
    }

    #[test]
    fn test_selection_set_primary_must_be_selected() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.select(e1);
        selection.set_primary(e2); // e2 is not selected

        // Primary should remain e1
        assert_eq!(selection.primary(), Some(e1));
    }

    #[test]
    fn test_selected_iterates_in_insertion_order() {
        let mut selection = Selection::new();
        let (e3, e1, e2) = (entity(3), entity(1), entity(2));

        selection.add(e3);
        selection.add(e1);
        selection.add(e2);

        let selected: Vec<_> = selection.selected().collect();
        assert_eq!(selected, vec![e3, e1, e2]);
    }

    #[test]
    fn test_add_already_selected_keeps_position_and_len() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);
        selection.add(e1); // re-add must not duplicate or move e1

        assert_eq!(selection.len(), 2);
        let selected: Vec<_> = selection.selected().collect();
        assert_eq!(selected, vec![e1, e2]);
        assert_eq!(selection.primary(), Some(e1));
    }

    #[test]
    fn test_remove_primary_falls_back_to_earliest_remaining() {
        let mut selection = Selection::new();
        let (e1, e2, e3) = (entity(1), entity(2), entity(3));

        selection.add(e1);
        selection.add(e2);
        selection.add(e3);
        assert_eq!(selection.primary(), Some(e1));

        selection.remove(e1);
        // Deterministic: the earliest remaining selected entity, not an
        // arbitrary hash-order survivor.
        assert_eq!(selection.primary(), Some(e2));
    }

    #[test]
    fn test_select_multiple_primary_is_first_given() {
        let mut selection = Selection::new();
        let (e5, e2, e9) = (entity(5), entity(2), entity(9));

        selection.select_multiple([e5, e2, e9]);

        assert_eq!(selection.primary(), Some(e5));
        let selected: Vec<_> = selection.selected().collect();
        assert_eq!(selected, vec![e5, e2, e9]);
    }

    #[test]
    fn test_select_multiple_empty_clears_primary() {
        let mut selection = Selection::new();
        selection.select(entity(1));

        selection.select_multiple(std::iter::empty());

        assert!(selection.is_empty());
        assert_eq!(selection.primary(), None);
    }

    #[test]
    fn test_select_multiple_dedupes_keeping_first_position() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.select_multiple([e1, e2, e1]);

        assert_eq!(selection.len(), 2);
        let selected: Vec<_> = selection.selected().collect();
        assert_eq!(selected, vec![e1, e2]);
        assert_eq!(selection.primary(), Some(e1));
    }

    #[test]
    fn test_selection_iterator() {
        let mut selection = Selection::new();
        let e1 = entity(1);
        let e2 = entity(2);

        selection.add(e1);
        selection.add(e2);

        let selected: Vec<_> = selection.selected().collect();
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&e1));
        assert!(selected.contains(&e2));
    }
}
