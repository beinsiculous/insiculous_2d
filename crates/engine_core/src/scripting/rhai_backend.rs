//! Rhai scripting engine backend and execution lifecycle.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use rhai::{Engine, Scope, AST};

use ecs::script::ScriptValue;

use super::commands::ScriptCommandsHandle;
use super::param_header::parse_param_header;
use super::rhai_bindings::create_rhai_engine;
use super::runner::{ScriptError, ScriptErrorKind};
use super::view::{ScriptView, SelfView};

/// Compiled Rhai script unit cached across frames.
#[derive(Clone)]
pub struct RhaiCompiledScript {
    pub ast: AST,
    /// Hash of the source the unit was compiled from; a byte-identical source is not recompiled.
    pub source_hash: u64,
    pub header_defaults: BTreeMap<String, ScriptValue>,
    pub has_early_update: bool,
    pub has_update: bool,
}

/// Arguments passed into Rhai hook invocations.
pub struct RhaiCallArgs<'a> {
    pub me: &'a Rc<SelfView>,
    pub view: &'a Rc<ScriptView>,
    pub params: &'a rhai::Map,
    pub commands: &'a ScriptCommandsHandle,
    pub delta_time: f32,
}

/// Rhai execution backend managing engine instance and AST cache.
pub struct RhaiBackend {
    engine: Engine,
    cache: BTreeMap<String, RhaiCompiledScript>,
}

impl Default for RhaiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiBackend {
    /// Create a new Rhai backend with configured engine.
    pub fn new() -> Self {
        Self {
            engine: create_rhai_engine(),
            cache: BTreeMap::new(),
        }
    }

    /// The compiled unit for a path, if any resolve has compiled it.
    pub fn get(&self, source_path: &str) -> Option<&RhaiCompiledScript> {
        self.cache.get(source_path)
    }

    /// Compile a script source, reusing the cached unit when the source is byte-identical.
    pub fn compile(
        &mut self,
        source_path: &str,
        source: &str,
    ) -> Result<&RhaiCompiledScript, ScriptError> {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let source_hash = hasher.finish();
        let unchanged = self
            .cache
            .get(source_path)
            .is_some_and(|cached| cached.source_hash == source_hash);
        if !unchanged {
            let compiled = Self::compile_fresh(&self.engine, source_path, source, source_hash)?;
            self.cache.insert(source_path.to_string(), compiled);
        }
        self.cache.get(source_path).ok_or_else(|| ScriptError {
            file: source_path.to_string(),
            line: 0,
            kind: ScriptErrorKind::Syntax,
            message: "compiled unit vanished from the cache".to_string(),
        })
    }

    fn compile_fresh(
        engine: &Engine,
        source_path: &str,
        source: &str,
        source_hash: u64,
    ) -> Result<RhaiCompiledScript, ScriptError> {
        let header_defaults = parse_param_header(source)?;
        let ast = engine.compile(source).map_err(|e| {
            let line = e.position().line().unwrap_or(0);
            ScriptError {
                file: source_path.to_string(),
                line,
                kind: ScriptErrorKind::Syntax,
                message: e.to_string(),
            }
        })?;

        let mut has_early_update = false;
        let mut has_update = false;
        for fn_def in ast.iter_functions() {
            if fn_def.name == "early_update" {
                has_early_update = true;
            } else if fn_def.name == "update" {
                has_update = true;
            }
        }

        Ok(RhaiCompiledScript {
            ast,
            source_hash,
            header_defaults,
            has_early_update,
            has_update,
        })
    }

    /// Call `early_update` hook if defined in the script.
    pub fn call_early_update(
        &self,
        source_path: &str,
        compiled: &RhaiCompiledScript,
        args: &RhaiCallArgs<'_>,
    ) -> Result<(), ScriptError> {
        if !compiled.has_early_update {
            return Ok(());
        }

        let mut scope = Scope::new();
        self.engine
            .call_fn::<()>(
                &mut scope,
                &compiled.ast,
                "early_update",
                (
                    args.me.clone(),
                    args.view.clone(),
                    args.params.clone(),
                    args.commands.clone(),
                    args.delta_time,
                ),
            )
            .map_err(|e| {
                let line = e.position().line().unwrap_or(0);
                let kind = match &*e {
                    rhai::EvalAltResult::ErrorTooManyOperations(_) => ScriptErrorKind::RunawayLoop,
                    _ => ScriptErrorKind::Runtime,
                };
                ScriptError {
                    file: source_path.to_string(),
                    line,
                    kind,
                    message: e.to_string(),
                }
            })
    }

    /// Call `update` hook if defined in the script.
    pub fn call_update(
        &self,
        source_path: &str,
        compiled: &RhaiCompiledScript,
        args: &RhaiCallArgs<'_>,
    ) -> Result<(), ScriptError> {
        if !compiled.has_update {
            return Ok(());
        }

        let mut scope = Scope::new();
        self.engine
            .call_fn::<()>(
                &mut scope,
                &compiled.ast,
                "update",
                (
                    args.me.clone(),
                    args.view.clone(),
                    args.params.clone(),
                    args.commands.clone(),
                    args.delta_time,
                ),
            )
            .map_err(|e| {
                let line = e.position().line().unwrap_or(0);
                let kind = match &*e {
                    rhai::EvalAltResult::ErrorTooManyOperations(_) => ScriptErrorKind::RunawayLoop,
                    _ => ScriptErrorKind::Runtime,
                };
                ScriptError {
                    file: source_path.to_string(),
                    line,
                    kind,
                    message: e.to_string(),
                }
            })
    }
}
