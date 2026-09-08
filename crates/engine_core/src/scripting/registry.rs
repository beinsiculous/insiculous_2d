//! Behavior trait and registry for compiled script behaviors.

use std::collections::BTreeMap;

use ecs::script::ScriptValue;

use super::commands::ScriptCommands;
use super::view::{ScriptView, SelfView};

/// Trait implemented by native Rust script behaviors.
pub trait ScriptBehavior: Send + Sync {
    /// Pre-physics update hook (runs before physics step).
    fn early_update(
        &mut self,
        _me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        _out: &mut ScriptCommands,
    ) {
    }

    /// Post-physics update hook (runs after physics step and collision drain).
    fn update(
        &mut self,
        _me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        _out: &mut ScriptCommands,
    ) {
    }
}

/// Specification for a parameter accepted by a native script behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    pub name: &'static str,
    pub default: ScriptValue,
}

/// Static descriptor for a registered native script behavior.
pub struct ScriptDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub params: &'static [ParamSpec],
    pub make: fn() -> Box<dyn ScriptBehavior>,
}

/// Registry of native script behavior descriptors.
#[derive(Default)]
pub struct ScriptRegistry {
    descriptors: BTreeMap<String, ScriptDescriptor>,
}

impl ScriptRegistry {
    /// Create a registry initialized with engine built-in scripts.
    pub fn new() -> Self {
        let mut registry = Self {
            descriptors: BTreeMap::new(),
        };
        registry.register(crate::scripting::builtin::rotate::ROTATE_DESCRIPTOR);
        registry
    }

    /// Register a new script behavior descriptor.
    ///
    /// Panics if a descriptor with the same ID is already registered.
    pub fn register(&mut self, descriptor: ScriptDescriptor) {
        if self.descriptors.contains_key(descriptor.id) {
            panic!("Duplicate script id registered: {}", descriptor.id);
        }
        self.descriptors
            .insert(descriptor.id.to_string(), descriptor);
    }

    /// Look up a descriptor by ID.
    pub fn get(&self, id: &str) -> Option<&ScriptDescriptor> {
        self.descriptors.get(id)
    }

    /// Iterate over all registered descriptors.
    pub fn descriptors(&self) -> impl Iterator<Item = &ScriptDescriptor> {
        self.descriptors.values()
    }
}
