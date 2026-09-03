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
    /// others are selected with it: `Entity: 7` alone, `Entity: 7
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

    use crate::test_support::entity;

    fn order(selection: &Selection) -> Vec<EntityId> {
        selection.selected().collect()
    }

    #[test]
    fn test_selection_keeps_insertion_order_and_re_adding_keeps_the_original_position() {
        let (third, first, second) = (entity(3), entity(1), entity(2));
        let mut selection = Selection::new();

        selection.add(third);
        selection.add(first);
        selection.add(second);
        assert_eq!(order(&selection), vec![third, first, second], "order is by selection, not by id");
        assert_eq!(selection.primary(), Some(third), "the first added is the primary");

        selection.add(first);
        assert_eq!(order(&selection), vec![third, first, second], "a re-add neither duplicates nor moves");
        assert_eq!(selection.len(), 3);

        selection.toggle(first);
        assert_eq!(order(&selection), vec![third, second], "toggle on a selected entity removes it");
        selection.toggle(first);
        assert_eq!(order(&selection), vec![third, second, first], "toggle back appends at the end");
    }

    #[test]
    fn test_removing_the_primary_falls_back_to_the_earliest_remaining_without_reordering() {
        // Four entries so that a swap_remove would be visible: removing the
        // second of [a, b, c, d] must leave [a, c, d], never [a, d, c].
        let (a, b, c, d) = (entity(1), entity(2), entity(3), entity(4));
        let mut selection = Selection::new();
        selection.add(a);
        selection.add(b);
        selection.add(c);
        selection.add(d);

        selection.remove(b);
        assert_eq!(order(&selection), vec![a, c, d], "removal shifts, it never swaps the tail in");
        assert_eq!(selection.primary(), Some(a), "removing a non-primary keeps the primary");

        selection.remove(a);
        assert_eq!(
            selection.primary(),
            Some(c),
            "the earliest remaining entity becomes primary — deterministic, not hash order"
        );
        assert!(!selection.contains(a));

        selection.remove(c);
        selection.remove(d);
        assert!(selection.is_empty());
        assert_eq!(selection.primary(), None, "an emptied selection has no primary");
    }

    #[test]
    fn test_select_replaces_the_selection_in_the_given_order_with_the_first_as_primary() {
        let (previous, e5, e2, e9) = (entity(7), entity(5), entity(2), entity(9));
        let mut selection = Selection::new();
        selection.select(previous);

        selection.select(e5);
        assert_eq!(order(&selection), vec![e5], "a single select clears what was there");
        assert_eq!(selection.primary(), Some(e5));

        selection.select_multiple([e5, e2, e9, e2]);
        assert_eq!(order(&selection), vec![e5, e2, e9], "given order kept, duplicates keep their first slot");
        assert_eq!(selection.primary(), Some(e5), "the first given is the primary");
        assert!(!selection.contains(previous));

        selection.select_multiple(std::iter::empty());
        assert!(selection.is_empty());
        assert_eq!(selection.primary(), None, "an empty multi-select clears the primary too");

        selection.add(e2);
        selection.clear();
        assert!(selection.is_empty());
        assert_eq!(selection.primary(), None);
    }

    #[test]
    fn test_inspector_heading_names_the_primary_and_counts_the_rest_of_a_multi_selection() {
        // The inspector heading is how the user tells WHICH of a
        // multi-selection they are editing.
        let mut selection = Selection::new();
        assert_eq!(selection.inspector_heading(), None, "nothing selected, no heading");

        selection.select(entity(7));
        assert_eq!(selection.inspector_heading().as_deref(), Some("Entity: 7"));

        selection.add(entity(8));
        selection.add(entity(9));
        assert_eq!(selection.inspector_heading().as_deref(), Some("Entity: 7  (1 of 3 selected)"));

        selection.remove(entity(7));
        assert_eq!(
            selection.inspector_heading().as_deref(),
            Some("Entity: 8  (1 of 2 selected)"),
            "the heading follows the primary fallback"
        );
    }
}
