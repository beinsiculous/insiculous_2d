//! Blackboard resource for script-shared state.

use std::collections::BTreeMap;
use crate::script::ScriptValue;

/// Global blackboard resource storing shared values for scripts.
///
/// Blackboard is a transient World resource cleared at the start of Play;
/// it is never serialized to scene files or captured in snapshots.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Blackboard(pub BTreeMap<String, ScriptValue>);

impl Blackboard {
    /// Create an empty blackboard.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Read a value from the blackboard.
    pub fn get(&self, key: &str) -> Option<&ScriptValue> {
        self.0.get(key)
    }

    /// Set a value in the blackboard.
    pub fn set(&mut self, key: impl Into<String>, value: ScriptValue) {
        self.0.insert(key.into(), value);
    }
}
