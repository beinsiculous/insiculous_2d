//! Typed event bus for loose-coupled communication between systems.
//!
//! Events are emitted during a frame and readable by any system during
//! the same frame. Call `flush_events()` at the end of each frame to
//! clear all queues.
//!
//! # Example
//! ```
//! use ecs::World;
//!
//! #[derive(Debug, Clone)]
//! struct CoinCollected { entity_id: u64, value: u32 }
//!
//! let mut world = World::new();
//! world.emit_event(CoinCollected { entity_id: 1, value: 10 });
//! world.emit_event(CoinCollected { entity_id: 2, value: 5 });
//!
//! for event in world.read_events::<CoinCollected>() {
//!     println!("Entity {} collected {} coins", event.entity_id, event.value);
//! }
//! # assert_eq!(world.read_events::<CoinCollected>().len(), 2);
//!
//! world.flush_events(); // Call at end of frame
//! # assert!(world.read_events::<CoinCollected>().is_empty());
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Trait object interface for type-erased event queue operations.
trait EventQueueOps: Send + Sync {
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A typed queue holding events of a single type.
struct TypedEventQueue<E: Send + Sync + 'static> {
    events: Vec<E>,
}

impl<E: Send + Sync + 'static> TypedEventQueue<E> {
    fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl<E: Send + Sync + 'static> EventQueueOps for TypedEventQueue<E> {
    fn clear(&mut self) {
        self.events.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An event bus that stores per-type event queues.
///
/// Events are emitted with `emit()`, read with `read()`, and
/// cleared each frame with `flush()`.
pub struct EventBus {
    queues: HashMap<TypeId, Box<dyn EventQueueOps>>,
}

impl EventBus {
    /// Create an empty event bus.
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }

    /// Emit an event. It will be readable until the next `flush()`.
    pub fn emit<E: Send + Sync + 'static>(&mut self, event: E) {
        let type_id = TypeId::of::<E>();
        let queue = self.queues
            .entry(type_id)
            .or_insert_with(|| Box::new(TypedEventQueue::<E>::new()));

        let typed = queue
            .as_any_mut()
            .downcast_mut::<TypedEventQueue<E>>()
            .expect("event queue type mismatch");
        typed.events.push(event);
    }

    /// Read all events of type `E` emitted since the last flush.
    /// Returns an empty slice if no events of this type exist.
    pub fn read<E: Send + Sync + 'static>(&self) -> &[E] {
        let type_id = TypeId::of::<E>();
        self.queues
            .get(&type_id)
            .and_then(|queue| {
                queue
                    .as_any()
                    .downcast_ref::<TypedEventQueue<E>>()
                    .map(|typed| typed.events.as_slice())
            })
            .unwrap_or(&[])
    }

    /// Clear all event queues. Call this at the end of each frame.
    pub fn flush(&mut self) {
        for queue in self.queues.values_mut() {
            queue.clear();
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct CoinCollected {
        value: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct PlayerDied {
        player_id: u64,
    }

    #[test]
    fn test_emit_and_read_events() {
        let mut bus = EventBus::new();
        assert!(bus.read::<CoinCollected>().is_empty(), "a type nobody emitted reads as empty");

        bus.emit(CoinCollected { value: 10 });
        bus.emit(PlayerDied { player_id: 1 });
        bus.emit(CoinCollected { value: 5 });

        assert_eq!(
            bus.read::<CoinCollected>(),
            [CoinCollected { value: 10 }, CoinCollected { value: 5 }],
            "events come back in emission order"
        );
        assert_eq!(
            bus.read::<PlayerDied>(),
            [PlayerDied { player_id: 1 }],
            "each event type has its own queue"
        );
    }

    #[test]
    fn test_events_readable_multiple_times_before_flush() {
        let mut bus = EventBus::new();
        bus.emit(CoinCollected { value: 10 });

        // Two consumers in one frame both see the event: reading never drains.
        assert_eq!(bus.read::<CoinCollected>().len(), 1);
        assert_eq!(bus.read::<CoinCollected>().len(), 1);
    }

    #[test]
    fn test_flush_clears_all_events() {
        let mut bus = EventBus::new();
        bus.emit(CoinCollected { value: 10 });
        bus.emit(PlayerDied { player_id: 1 });

        bus.flush();

        assert!(bus.read::<CoinCollected>().is_empty());
        assert!(bus.read::<PlayerDied>().is_empty());

        // The next frame starts fresh on the same bus.
        bus.emit(CoinCollected { value: 2 });
        assert_eq!(bus.read::<CoinCollected>(), [CoinCollected { value: 2 }]);
    }
}
