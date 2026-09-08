//! Game scripting subsystem providing native and Rhai runtime environments.

pub mod builtin;
pub mod commands;
pub mod param_header;
pub mod registry;
pub mod rhai_backend;
pub mod rhai_bindings;
pub mod runner;
pub mod view;

#[cfg(test)]
mod tests;

pub use builtin::rotate::ROTATE_DESCRIPTOR;
pub use commands::{ScriptCommand, ScriptCommands, ScriptCommandsHandle, Target};
pub use param_header::{check_source, parse_param_header};
pub use registry::{ParamSpec, ScriptBehavior, ScriptDescriptor, ScriptRegistry};
pub use rhai_backend::{RhaiBackend, RhaiCompiledScript};
pub use rhai_bindings::create_rhai_engine;
pub use runner::{ScriptError, ScriptErrorKind, ScriptErrors, ScriptRunner};
pub use view::{EntityState, ScriptCollision, ScriptView, SelfView};
