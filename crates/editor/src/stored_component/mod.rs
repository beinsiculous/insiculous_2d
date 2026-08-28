//! The editor's component registry — the single source of truth for every
//! component type the editor can capture, restore, add, remove, and inspect.
//!
//! All per-component dispatch (undo/redo capture, add-component popup,
//! read-only inspection) is generated from ONE `editor_component_registry!`
//! invocation below. **To make a new component editor-visible, add one line
//! to that invocation** — no match statements elsewhere need to change.

use ecs::audio_components::{AudioListener, AudioSource};
use ecs::behavior::{Behavior, BehaviorState, EntityTag};
use ecs::hierarchy::GlobalTransform2D;
use ecs::sprite_components::{Name, Sprite, SpriteAnimation};
use ecs::tilemap::Tilemap;
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::{EntityId, World};
use physics::components::{Collider, RigidBody};
use ui::UIContext;

use crate::behavior_editor::edit_behavior;
use crate::commands::{
    CommandHistory, RemoveComponentCommand, SetAudioSourceCommand, SetBehaviorCommand,
    SetColliderCommand, SetEntityTagCommand, SetNameCommand, SetRigidBodyCommand,
    SetSpriteCommand, SetTransformCommand, SetUiButtonCommand, SetUiLabelCommand,
    SetUiPanelCommand,
};
use crate::component_editors::{
    edit_audio_source, edit_collider, edit_entity_tag, edit_name, edit_rigid_body, edit_sprite,
    edit_transform2d,
};
use crate::ui_component_editors::{edit_ui_button, edit_ui_label, edit_ui_panel};
use crate::inspector::{inspect_component, InspectorStyle};
use crate::{EditableFieldStyle, EditableInspector};

/// Expands one component's editable-inspector block for
/// [`edit_all_components`] — dispatched on the edit spec written in the
/// registry: `{ edit <fn> => <SetCommand> }` renders field editors with
/// undo-recorded writeback, `{ readonly }` renders the registry header with
/// a remove button plus the serde-based read-only display.
macro_rules! registry_edit_block {
    // Editable, NOT removable (builtin): the editor fn renders its own header.
    (@fixed $name:ident, $ty:ty, (edit $edit_fn:ident => $cmd:ident),
     $ui:ident, $world:ident, $entity:ident, $history:ident, $x:ident, $width:ident, $y:ident,
     $inspect_style:ident, $field_style:ident, $gap:ident, $idx:ident, $removals:ident,
     $extras:ident) => {
        if let Some(value) = $world.get::<$ty>($entity).cloned() {
            $y += $gap;
            let mut inspector = EditableInspector::new($ui, $x, $y)
                .with_component_index($idx)
                .with_width($width)
                .with_style($field_style.clone());
            let edit = $edit_fn(&mut inspector, &value, &mut *$extras);
            $y = inspector.y();
            crate::component_editors::apply_component_edit($world, $entity, &value, edit, $history, |e, old, new, hint| {
                Box::new($cmd::new(e, old, new, hint))
            });
            $idx += 1;
        }
    };
    // Editable + removable: overlay the [X] at the header the editor fn drew.
    (@removable $name:ident, $ty:ty, (edit $edit_fn:ident => $cmd:ident),
     $ui:ident, $world:ident, $entity:ident, $history:ident, $x:ident, $width:ident, $y:ident,
     $inspect_style:ident, $field_style:ident, $gap:ident, $idx:ident, $removals:ident,
     $extras:ident) => {
        if let Some(value) = $world.get::<$ty>($entity).cloned() {
            $y += $gap;
            let header_y = $y;
            let mut inspector = EditableInspector::new($ui, $x, $y)
                .with_component_index($idx)
                .with_width($width)
                .with_style($field_style.clone());
            let edit = $edit_fn(&mut inspector, &value, &mut *$extras);
            $y = inspector.y();
            if crate::component_editors::remove_button($ui, $idx, $x, header_y, $width) {
                $removals.push(ComponentKind::$name);
            }
            crate::component_editors::apply_component_edit($world, $entity, &value, edit, $history, |e, old, new, hint| {
                Box::new($cmd::new(e, old, new, hint))
            });
            $idx += 1;
        }
    };
    // Read-only + removable: registry header with [X] + serde inspection
    // (components without a field editor yet).
    (@removable $name:ident, $ty:ty, (readonly),
     $ui:ident, $world:ident, $entity:ident, $history:ident, $x:ident, $width:ident, $y:ident,
     $inspect_style:ident, $field_style:ident, $gap:ident, $idx:ident, $removals:ident,
     $extras:ident) => {
        if $world.get::<$ty>($entity).is_some() {
            $y += $gap;
            let mut inspector = EditableInspector::new($ui, $x, $y)
                .with_component_index($idx)
                .with_width($width)
                .with_style($field_style.clone());
            if inspector.header_with_remove(stringify!($name), true) {
                $removals.push(ComponentKind::$name);
            }
            $y = inspector.y();
            if let Some(value) = $world.get::<$ty>($entity) {
                $y = inspect_component($ui, "", value, $x + 16.0, $y, $inspect_style);
            }
            $idx += 1;
        }
    };
}

/// Category grouping for the add-component popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentCategory {
    Core,
    Rendering,
    Physics,
    Audio,
    Gameplay,
    Ui,
}

impl ComponentCategory {
    /// All categories in display order.
    pub const ALL: [ComponentCategory; 6] = [
        ComponentCategory::Core,
        ComponentCategory::Rendering,
        ComponentCategory::Physics,
        ComponentCategory::Audio,
        ComponentCategory::Gameplay,
        ComponentCategory::Ui,
    ];

    /// Display name for the category header.
    pub fn label(self) -> &'static str {
        match self {
            ComponentCategory::Core => "Core",
            ComponentCategory::Rendering => "Rendering",
            ComponentCategory::Physics => "Physics",
            ComponentCategory::Audio => "Audio",
            ComponentCategory::Gameplay => "Gameplay",
            ComponentCategory::Ui => "UI",
        }
    }
}

/// Generates the editor's component dispatch from a single component list.
///
/// Sections:
/// - `hidden`: captured for undo/redo only (always present on entities,
///   never inspected or removable) — e.g. `GlobalTransform2D`.
/// - `builtin`: captured AND inspected, but never addable/removable —
///   e.g. `Transform2D`.
/// - `removable`: full lifecycle (capture, inspect, add, remove), each
///   tagged with a `ComponentCategory` for the add-component popup.
macro_rules! editor_component_registry {
    (
        hidden:    [ $( $h:ident => $h_ty:ty ),+ $(,)? ],
        builtin:   [ $( $b:ident => $b_ty:ty { $($b_edit:tt)+ } ),+ $(,)? ],
        removable: [ $( $r:ident => $r_ty:ty : $cat:ident { $($r_edit:tt)+ } ),+ $(,)? ] $(,)?
    ) => {
        /// A captured component value for undo/redo storage.
        ///
        /// Typed variants store a cloned concrete component; `Dynamic` holds
        /// a registry-name + serde value for any component the dynamic tier
        /// covers (game-registered types — issue #43).
        #[derive(Debug, Clone)]
        pub enum StoredComponent {
            $( $h($h_ty), )+
            $( $b($b_ty), )+
            $( $r($r_ty), )+
            /// A dynamic-tier component (registered in the ecs global
            /// registry, no typed editor entry): restored via the registry's
            /// insert fn pointer.
            Dynamic { name: String, value: serde_json::Value },
        }

        impl StoredComponent {
            /// Add this stored component to an entity in the world.
            pub fn apply_to(&self, world: &mut World, entity: EntityId) {
                match self {
                    $( Self::$h(c) => { world.add_component(&entity, Clone::clone(c)).ok(); } )+
                    $( Self::$b(c) => { world.add_component(&entity, Clone::clone(c)).ok(); } )+
                    $( Self::$r(c) => { world.add_component(&entity, Clone::clone(c)).ok(); } )+
                    Self::Dynamic { name, value } => dynamic::apply_dynamic(world, entity, name, value),
                }
            }
        }

        /// Capture all known component types from an entity into a `Vec<StoredComponent>`.
        ///
        /// Reads every TYPED registry component first, then every dynamic-tier
        /// component present on the entity (sorted by name). Hierarchy
        /// components (Parent, Children) are deliberately excluded —
        /// hierarchy is managed separately by the command implementations.
        pub fn capture_all_components(world: &World, entity: EntityId) -> Vec<StoredComponent> {
            let mut components = Vec::new();
            $( if let Some(c) = world.get::<$h_ty>(entity) {
                components.push(StoredComponent::$h(Clone::clone(c)));
            } )+
            $( if let Some(c) = world.get::<$b_ty>(entity) {
                components.push(StoredComponent::$b(Clone::clone(c)));
            } )+
            $( if let Some(c) = world.get::<$r_ty>(entity) {
                components.push(StoredComponent::$r(Clone::clone(c)));
            } )+
            dynamic::capture_dynamic_components(world, entity, &mut components);
            components
        }

        /// The `TypeId` of every TYPED registry component (macro entries
        /// only — the dynamic tier's ids come from the global registry).
        pub fn registered_typed_component_type_ids() -> Vec<std::any::TypeId> {
            vec![
                $( std::any::TypeId::of::<$h_ty>(), )+
                $( std::any::TypeId::of::<$b_ty>(), )+
                $( std::any::TypeId::of::<$r_ty>(), )+
            ]
        }

        /// The `TypeId` of every component type this registry captures and
        /// restores — typed entries PLUS the dynamic tier (issue #43).
        /// Snapshot code diffs an entity's actual components
        /// (`World::component_types`) against this set to detect component
        /// types that would be silently lost by a capture/restore round-trip.
        ///
        /// Components that store `EntityId` references need snapshot-reference
        /// auditing (repair or exclusion) before being registered — restored
        /// references to entities that are not part of the same snapshot
        /// dangle. No registered component stores `EntityId` today; dynamic
        /// components are documented as forbidden from storing raw ids.
        pub fn registered_component_type_ids() -> Vec<std::any::TypeId> {
            let mut ids = registered_typed_component_type_ids();
            ids.extend(dynamic::dynamic_component_type_ids());
            ids
        }

        /// The component kinds that can be added to / removed from entities.
        ///
        /// This is THE editor-wide `ComponentKind` — commands, the inspector,
        /// and the add-component popup all share it.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ComponentKind {
            $( $r, )+
        }

        impl ComponentKind {
            /// All removable component kinds, in registry order.
            pub const ALL: &'static [ComponentKind] = &[ $( ComponentKind::$r, )+ ];

            /// Human-readable display name (matches the type name).
            pub fn display_name(self) -> &'static str {
                match self { $( Self::$r => stringify!($r), )+ }
            }

            /// Category for the add-component popup.
            pub fn category(self) -> ComponentCategory {
                match self { $( Self::$r => ComponentCategory::$cat, )+ }
            }

            /// Add a default instance of this component to an entity.
            pub fn add_default(self, world: &mut World, entity: EntityId) {
                match self {
                    $( Self::$r => { world.add_component(&entity, <$r_ty>::default()).ok(); } )+
                }
            }

            /// Capture the current value of this component, if present.
            pub fn capture(self, world: &World, entity: EntityId) -> Option<StoredComponent> {
                match self {
                    $( Self::$r => world.get::<$r_ty>(entity)
                        .map(|c| StoredComponent::$r(Clone::clone(c))), )+
                }
            }

            /// Remove this component from an entity (no-op if absent).
            pub fn remove(self, world: &mut World, entity: EntityId) {
                match self {
                    $( Self::$r => { world.remove_component::<$r_ty>(&entity).ok(); } )+
                }
            }

            /// Whether the entity currently has this component.
            pub fn is_present(self, world: &World, entity: EntityId) -> bool {
                match self {
                    $( Self::$r => world.get::<$r_ty>(entity).is_some(), )+
                }
            }
        }

        /// Render the editable inspector for every present component
        /// (builtin + removable, in registry order): field editors with
        /// undo-recorded writeback via [`apply_component_edit`], remove [X]
        /// buttons (removals executed as commands), and a serde read-only
        /// display for components marked `readonly` in the registry.
        ///
        /// Returns `(next_y, component_count)` — the count feeds the
        /// add-component popup's widget-id offsets.
        #[allow(clippy::too_many_arguments)]
        pub fn edit_all_components(
            ui: &mut UIContext,
            world: &mut World,
            entity: EntityId,
            history: &mut CommandHistory,
            x: f32,
            width: f32,
            mut y: f32,
            inspect_style: &InspectorStyle,
            field_style: &EditableFieldStyle,
            section_gap: f32,
            extras: &mut crate::InspectorExtras<'_>,
        ) -> (f32, usize) {
            let mut component_index: usize = 0;
            let mut removals: Vec<ComponentKind> = Vec::new();

            $( registry_edit_block!(@fixed $b, $b_ty, ($($b_edit)+),
                ui, world, entity, history, x, width, y,
                inspect_style, field_style, section_gap, component_index, removals, extras); )+
            $( registry_edit_block!(@removable $r, $r_ty, ($($r_edit)+),
                ui, world, entity, history, x, width, y,
                inspect_style, field_style, section_gap, component_index, removals, extras); )+

            // Dynamic-tier components render read-only (serde display) with
            // a remove [X] — typed field editors are the opt-in overlay this
            // registry provides, and game types don't have one (issue #43).
            let mut dynamic_removals: Vec<String> = Vec::new();
            y = render_dynamic_edit_blocks(
                ui, world, entity, x, width, y,
                inspect_style, field_style, section_gap,
                &mut component_index, &mut dynamic_removals,
            );

            for kind in &removals {
                let cmd = RemoveComponentCommand::new(entity, *kind);
                history.execute(Box::new(cmd), world);
                log::info!("Removed component: {}", kind.display_name());
            }
            for name in dynamic_removals {
                log::info!("Removed dynamic component: {}", name);
                let cmd = crate::commands::RemoveDynamicComponentCommand::new(entity, name);
                history.execute(Box::new(cmd), world);
            }

            (y, component_index)
        }

        /// Render a read-only inspection of every present inspectable component
        /// (builtin + removable), in registry order. Returns the next Y position.
        pub fn inspect_all_components(
            ui: &mut UIContext,
            world: &World,
            entity: EntityId,
            x: f32,
            mut y: f32,
            style: &InspectorStyle,
            section_gap: f32,
        ) -> f32 {
            $( if let Some(c) = world.get::<$b_ty>(entity) {
                y += section_gap;
                y = inspect_component(ui, stringify!($b), c, x, y, style);
            } )+
            $( if let Some(c) = world.get::<$r_ty>(entity) {
                y += section_gap;
                y = inspect_component(ui, stringify!($r), c, x, y, style);
            } )+
            for name in dynamic::dynamic_components_on(world, entity) {
                if let Some(value) = dynamic::dynamic_value(world, entity, &name) {
                    y += section_gap;
                    y = inspect_component(ui, &name, &value, x, y, style);
                }
            }
            y
        }

        /// Names of every component the command API may `set` (builtin +
        /// removable in registry order, then dynamic-tier names sorted) —
        /// hidden entries excluded, and `Name` excluded too (it is set
        /// through the `rename` verb, which also covers entities without
        /// one). Typed names are borrowed; dynamic names are owned (F2).
        pub fn settable_component_names() -> Vec<std::borrow::Cow<'static, str>> {
            use std::borrow::Cow;
            let mut names: Vec<Cow<'static, str>> = Vec::new();
            $( if stringify!($b) != "Name" { names.push(Cow::Borrowed(stringify!($b))); } )+
            $( names.push(Cow::Borrowed(stringify!($r))); )+
            names.extend(dynamic::dynamic_component_names().into_iter().map(Cow::Owned));
            names
        }

        /// Deserialize a `StoredComponent` from a JSON value by registry
        /// type name — the write-path mirror of `capture_all_values`.
        /// Unknown typed names fall through to the dynamic tier.
        pub fn stored_component_from_json(
            name: &str,
            value: serde_json::Value,
        ) -> Result<StoredComponent, String> {
            match name {
                $( stringify!($b) => serde_json::from_value::<$b_ty>(value)
                    .map(StoredComponent::$b)
                    .map_err(|e| format!("bad {} value: {e}", stringify!($b))), )+
                $( stringify!($r) => serde_json::from_value::<$r_ty>(value)
                    .map(StoredComponent::$r)
                    .map_err(|e| format!("bad {} value: {e}", stringify!($r))), )+
                other => dynamic::stored_dynamic_from_json(other, value),
            }
        }

        /// Capture one present component by registry type name (builtin +
        /// removable; `Ok(None)` = the entity doesn't carry it). Unknown
        /// typed names fall through to the dynamic tier.
        pub fn capture_component_by_name(
            world: &World,
            entity: EntityId,
            name: &str,
        ) -> Result<Option<StoredComponent>, String> {
            match name {
                $( stringify!($b) => Ok(world
                    .get::<$b_ty>(entity)
                    .cloned()
                    .map(StoredComponent::$b)), )+
                $( stringify!($r) => Ok(world
                    .get::<$r_ty>(entity)
                    .cloned()
                    .map(StoredComponent::$r)), )+
                other => dynamic::capture_dynamic_by_name(world, entity, other),
            }
        }

        impl StoredComponent {
            /// The registry type name of this stored value (`&str`, not
            /// `&'static str` — dynamic names are owned).
            pub fn type_name(&self) -> &str {
                match self {
                    $( Self::$h(_) => stringify!($h), )+
                    $( Self::$b(_) => stringify!($b), )+
                    $( Self::$r(_) => stringify!($r), )+
                    Self::Dynamic { name, .. } => name.as_str(),
                }
            }
        }

        /// Capture every present inspectable component (builtin + removable
        /// in registry order, then dynamic-tier components sorted by name)
        /// as `(type_name, serde value)` pairs — the data half of
        /// `inspect_all_components`, consumed by the command API's
        /// `describe` query. A component that fails to serialize contributes
        /// an error string so the result stays total. Hidden registry
        /// entries (GlobalTransform2D, BehaviorState) are internal and not
        /// emitted. `Name` IS a registry component (editable since #32) and
        /// appears here; the `describe` query filters it out because the API
        /// surfaces the name as a top-level entity field instead.
        pub fn capture_all_values(
            world: &World,
            entity: EntityId,
        ) -> Vec<(std::borrow::Cow<'static, str>, serde_json::Value)> {
            use std::borrow::Cow;
            let mut values: Vec<(Cow<'static, str>, serde_json::Value)> = Vec::new();
            $( if let Some(c) = world.get::<$b_ty>(entity) {
                values.push((Cow::Borrowed(stringify!($b)), match crate::inspector::component_value(c) {
                    Ok(v) => v,
                    Err(e) => serde_json::Value::String(format!("!serialize error: {e}")),
                }));
            } )+
            $( if let Some(c) = world.get::<$r_ty>(entity) {
                values.push((Cow::Borrowed(stringify!($r)), match crate::inspector::component_value(c) {
                    Ok(v) => v,
                    Err(e) => serde_json::Value::String(format!("!serialize error: {e}")),
                }));
            } )+
            for name in dynamic::dynamic_components_on(world, entity) {
                if let Some(value) = dynamic::dynamic_value(world, entity, &name) {
                    values.push((Cow::Owned(name), value));
                }
            }
            values
        }
    };
}

editor_component_registry! {
    hidden: [
        GlobalTransform2D => GlobalTransform2D,
        BehaviorState     => BehaviorState,
    ],
    builtin: [
        Name        => Name { edit edit_name => SetNameCommand },
        Transform2D => common::Transform2D { edit edit_transform2d => SetTransformCommand },
    ],
    removable: [
        Camera          => common::Camera : Core { readonly },
        Sprite          => Sprite : Rendering { edit edit_sprite => SetSpriteCommand },
        SpriteAnimation => SpriteAnimation : Rendering { readonly },
        Tilemap         => Tilemap : Rendering { readonly },
        RigidBody       => RigidBody : Physics { edit edit_rigid_body => SetRigidBodyCommand },
        Collider        => Collider : Physics { edit edit_collider => SetColliderCommand },
        AudioSource     => AudioSource : Audio { edit edit_audio_source => SetAudioSourceCommand },
        AudioListener   => AudioListener : Audio { readonly },
        Behavior        => Behavior : Gameplay { edit edit_behavior => SetBehaviorCommand },
        EntityTag       => EntityTag : Gameplay { edit edit_entity_tag => SetEntityTagCommand },
        UiLabel         => UiLabel : Ui { edit edit_ui_label => SetUiLabelCommand },
        UiPanel         => UiPanel : Ui { edit edit_ui_panel => SetUiPanelCommand },
        UiButton        => UiButton : Ui { edit edit_ui_button => SetUiButtonCommand },
    ],
}

pub mod dynamic;

pub use dynamic::available_dynamic_components;

/// Render the read-only edit blocks for every dynamic-tier component on the
/// entity: registry-name header with a remove [X] plus the serde value
/// display. Removals are returned by name for the caller to execute as
/// `RemoveDynamicComponentCommand`s.
#[allow(clippy::too_many_arguments)]
fn render_dynamic_edit_blocks(
    ui: &mut UIContext,
    world: &World,
    entity: EntityId,
    x: f32,
    width: f32,
    mut y: f32,
    inspect_style: &InspectorStyle,
    field_style: &EditableFieldStyle,
    section_gap: f32,
    component_index: &mut usize,
    removals: &mut Vec<String>,
) -> f32 {
    for name in dynamic::dynamic_components_on(world, entity) {
        let Some(value) = dynamic::dynamic_value(world, entity, &name) else {
            continue;
        };
        y += section_gap;
        let mut inspector = EditableInspector::new(ui, x, y)
            .with_component_index(*component_index)
            .with_width(width)
            .with_style(field_style.clone());
        if inspector.header_with_remove(&name, true) {
            removals.push(name.clone());
        }
        y = inspector.y();
        y = inspect_component(ui, "", &value, x + 16.0, y, inspect_style);
        *component_index += 1;
    }
    y
}

/// Restore a set of stored components onto an entity.
pub fn restore_components(world: &mut World, entity: EntityId, components: &[StoredComponent]) {
    for component in components {
        component.apply_to(world, entity);
    }
}

/// Returns the component kinds that are NOT present on the entity
/// (the candidates for the add-component popup).
pub fn available_components(world: &World, entity: EntityId) -> Vec<ComponentKind> {
    ComponentKind::ALL
        .iter()
        .copied()
        .filter(|kind| !kind.is_present(world, entity))
        .collect()
}

/// Returns all component kinds grouped by category, in display order.
/// Categories with no components are omitted.
pub fn categorized_components() -> Vec<(ComponentCategory, Vec<ComponentKind>)> {
    ComponentCategory::ALL
        .iter()
        .map(|&category| {
            let kinds: Vec<ComponentKind> = ComponentKind::ALL
                .iter()
                .copied()
                .filter(|kind| kind.category() == category)
                .collect();
            (category, kinds)
        })
        .filter(|(_, kinds)| !kinds.is_empty())
        .collect()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod dynamic_tests;
