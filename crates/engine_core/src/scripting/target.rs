//! The entity a script command names, and how a phase's commands resolve it.

use std::collections::HashMap;

use ecs::sprite_components::Name;
use ecs::{EntityId, World};

use super::runner::{ScriptError, ScriptErrorKind, ScriptErrors};
use super::view::SelfView;

/// Target of a script command: an explicit entity ID or a resolved name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// Target by explicit entity ID.
    Entity(EntityId),
    /// Target by entity name component.
    Named(String),
}

impl From<&SelfView> for Target {
    fn from(me: &SelfView) -> Self {
        Target::Entity(me.entity)
    }
}

impl From<EntityId> for Target {
    fn from(id: EntityId) -> Self {
        Target::Entity(id)
    }
}

impl From<&str> for Target {
    fn from(name: &str) -> Self {
        Target::Named(name.to_string())
    }
}

impl From<String> for Target {
    fn from(name: String) -> Self {
        Target::Named(name)
    }
}

/// The world's names, read once when a phase's commands apply, so every command in the
/// phase resolves against the world as it stood before any of them.
pub(crate) struct TargetResolver {
    name_to_entities: HashMap<String, Vec<EntityId>>,
}

impl TargetResolver {
    pub(crate) fn new(world: &World) -> Self {
        let mut name_to_entities: HashMap<String, Vec<EntityId>> = HashMap::new();
        for entity in world.entities() {
            if let Some(name) = world.get::<Name>(entity) {
                name_to_entities
                    .entry(name.as_str().to_string())
                    .or_default()
                    .push(entity);
            }
        }
        Self { name_to_entities }
    }

    /// The live entity `target` names. A missing or ambiguous name is reported in
    /// `errors`; a dead id is quietly `None`.
    pub(crate) fn resolve(
        &self,
        world: &World,
        target: &Target,
        errors: &mut ScriptErrors,
    ) -> Option<EntityId> {
        match target {
            Target::Entity(id) => {
                if world.validate_entity(id).is_ok() {
                    Some(*id)
                } else {
                    None
                }
            }
            Target::Named(name) => match self.name_to_entities.get(name) {
                None => {
                    // The name is the dedupe key: one report per missing name per Play.
                    errors.push(ScriptError {
                        file: name.clone(),
                        line: 0,
                        kind: ScriptErrorKind::MissingTarget,
                        message: format!("Target entity named '{name}' not found"),
                    });
                    None
                }
                Some(matches) if matches.len() > 1 => {
                    errors.push(ScriptError {
                        file: name.clone(),
                        line: 0,
                        kind: ScriptErrorKind::AmbiguousTarget,
                        message: format!(
                            "Target entity named '{name}' is ambiguous ({} entities match)",
                            matches.len()
                        ),
                    });
                    None
                }
                Some(matches) => Some(matches[0]),
            },
        }
    }
}
