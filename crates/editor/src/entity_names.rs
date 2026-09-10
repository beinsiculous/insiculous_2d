//! The one name an entity shows anywhere in the editor.
//!
//! The hierarchy row, the inspector heading and the command API's `display`
//! field all read from here, so a renamed entity reads the same in all three.

use ecs::sprite_components::{Name, Sprite};
use ecs::{EntityId, World};
use physics::components::RigidBody;

/// The entity's display name.
///
/// Resolution order:
/// 1. `Name` component
/// 2. `Sprite` component → "Sprite (Entity {id})"
/// 3. `RigidBody` component → "RigidBody (Entity {id})"
/// 4. Fallback → "Entity {id}"
pub fn entity_display_name(world: &World, entity: EntityId) -> String {
    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_string();
    }
    if world.get::<Sprite>(entity).is_some() {
        return format!("Sprite (Entity {})", entity.value());
    }
    if world.get::<RigidBody>(entity).is_some() {
        return format!("RigidBody (Entity {})", entity.value());
    }
    format!("Entity {}", entity.value())
}
