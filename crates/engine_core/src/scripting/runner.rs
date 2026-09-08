//! Script runner coordinating script instances, execution phases, and error tracking.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use glam::Vec2;
use input::{GameAction, InputHandler, InputSettings, PlayerId};
use physics::{CollisionData, PhysicsSystem};

use common::Transform2D;
use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::Name;
use ecs::{EntityId, World};

use super::commands::{current_velocity, ScriptCommands, ScriptCommandsHandle};
use super::registry::{ScriptBehavior, ScriptRegistry};
use super::rhai_backend::RhaiBackend;
use super::view::{EntityState, ScriptCollision, ScriptView, SelfView};

/// Classification of script execution and compilation errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptErrorKind {
    Syntax,
    Runtime,
    Header,
    MissingTarget,
    AmbiguousTarget,
    UnresolvedScript,
    RunawayLoop,
}

/// One error originating from script compilation or runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub file: String,
    pub line: usize,
    pub kind: ScriptErrorKind,
    pub message: String,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(
                f,
                "{}:{}: [{:?}] {}",
                self.file, self.line, self.kind, self.message
            )
        } else {
            write!(f, "{}: [{:?}] {}", self.file, self.kind, self.message)
        }
    }
}

/// Deduplicated collection of script errors accumulated per Play session.
#[derive(Debug, Default, Clone)]
pub struct ScriptErrors {
    errors: Vec<ScriptError>,
    seen: HashSet<(String, usize, ScriptErrorKind)>,
}

impl ScriptErrors {
    /// Create an empty errors collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error, deduplicating by (file, line, kind); returns whether it was new,
    /// so a caller can log once for what the list reports once.
    pub fn push(&mut self, error: ScriptError) -> bool {
        let key = (error.file.clone(), error.line, error.kind.clone());
        let is_new = self.seen.insert(key);
        if is_new {
            self.errors.push(error);
        }
        is_new
    }

    /// Read-only slice of recorded errors.
    pub fn as_slice(&self) -> &[ScriptError] {
        &self.errors
    }

    /// Clear all recorded errors.
    pub fn clear(&mut self) {
        self.errors.clear();
        self.seen.clear();
    }
}

enum InstanceKind {
    Native(Box<dyn ScriptBehavior>),
    Rhai,
}

struct ScriptInstance {
    script_id: String,
    source_path: String,
    kind: InstanceKind,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Phase {
    EarlyUpdate,
    Update,
}

struct PhaseContext<'a> {
    phase: Phase,
    input: &'a InputHandler,
    players: &'a InputSettings,
    delta_time: f32,
    collisions: Vec<ScriptCollision>,
}

/// Central script runner managing script lifecycles and phase dispatch.
pub struct ScriptRunner {
    registry: ScriptRegistry,
    instances: BTreeMap<(EntityId, usize), ScriptInstance>,
    rhai: RhaiBackend,
    frames_run: u32,
    errors: ScriptErrors,
    asset_base: String,
    quarantined: HashSet<(EntityId, usize)>,
    /// Refs that failed to resolve this Play, with the identity they failed under: retrying
    /// twice a frame would re-read a missing file forever, but an edited ref gets a new try.
    failed: HashMap<(EntityId, usize), (String, String)>,
}

impl Default for ScriptRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner {
    /// Create a new runner with built-in scripts registered.
    pub fn new() -> Self {
        Self {
            registry: ScriptRegistry::new(),
            instances: BTreeMap::new(),
            rhai: RhaiBackend::new(),
            frames_run: 0,
            errors: ScriptErrors::new(),
            asset_base: String::new(),
            quarantined: HashSet::new(),
            failed: HashMap::new(),
        }
    }

    /// Immutable reference to the script registry.
    pub fn registry(&self) -> &ScriptRegistry {
        &self.registry
    }

    /// Mutable reference to the script registry.
    pub fn registry_mut(&mut self) -> &mut ScriptRegistry {
        &mut self.registry
    }

    /// Slice of errors recorded during the current Play session.
    pub fn errors(&self) -> &[ScriptError] {
        self.errors.as_slice()
    }

    /// Number of frames successfully stepped by this runner.
    pub fn frames_run(&self) -> u32 {
        self.frames_run
    }

    /// Reset all runtime state for a new Play session and install a fresh blackboard.
    pub fn reset(&mut self, world: &mut World, asset_base: &str) {
        self.asset_base = asset_base.to_string();
        self.instances.clear();
        self.frames_run = 0;
        self.errors.clear();
        self.quarantined.clear();
        self.failed.clear();
        world.insert_resource(Blackboard::new());
    }

    /// The VFS key of a script: the asset base joined with the project-relative path, or
    /// the bare path when no base was recorded (headless tests).
    fn resolve_source_path(&self, source_path: &str) -> String {
        if self.asset_base.is_empty() {
            source_path.to_string()
        } else {
            format!("{}/{}", self.asset_base, source_path)
        }
    }

    fn prune_and_resolve(&mut self, world: &World) {
        let dead_keys: Vec<(EntityId, usize)> = self
            .instances
            .iter()
            .filter_map(|((entity, index), instance)| {
                if world.validate_entity(entity).is_err() {
                    return Some((*entity, *index));
                }
                let scripts = world.get::<Scripts>(*entity)?;
                let script_ref = scripts.0.get(*index)?;
                if script_ref.script_id != instance.script_id
                    || script_ref.source_path != instance.source_path
                {
                    return Some((*entity, *index));
                }
                None
            })
            .collect();

        for key in dead_keys {
            self.instances.remove(&key);
        }
        self.failed.retain(|(entity, _), _| world.validate_entity(entity).is_ok());

        let mut pending_refs: Vec<(EntityId, usize, ScriptRef)> = Vec::new();
        for entity in world.entities() {
            let Some(scripts) = world.get::<Scripts>(entity) else { continue };
            for (index, script_ref) in scripts.0.iter().enumerate() {
                let key = (entity, index);
                let failed_as_is = self.failed.get(&key).is_some_and(|(script_id, source_path)| {
                    script_id == &script_ref.script_id && source_path == &script_ref.source_path
                });
                if self.instances.contains_key(&key) || self.quarantined.contains(&key) || failed_as_is {
                    continue;
                }
                pending_refs.push((entity, index, script_ref.clone()));
            }
        }
        pending_refs.sort_by_key(|(entity, index, _)| (*entity, *index));

        for (entity, index, script_ref) in pending_refs {
            let key = (entity, index);
            match self.resolve_instance(&script_ref) {
                Ok(instance) => {
                    self.instances.insert(key, instance);
                }
                Err(error) => {
                    self.failed.insert(
                        key,
                        (script_ref.script_id.clone(), script_ref.source_path.clone()),
                    );
                    if self.errors.push(error) {
                        log::warn!(
                            "script `{}` on entity {:?} did not resolve",
                            script_ref.script_id,
                            entity
                        );
                    }
                }
            }
        }
    }

    /// Resolve one ref: a `.rhai` path reads and compiles once (the cache serves every
    /// later phase), anything else is a registry id.
    fn resolve_instance(&mut self, script_ref: &ScriptRef) -> Result<ScriptInstance, ScriptError> {
        let kind = if script_ref.source_path.ends_with(".rhai") {
            let resolved_path = self.resolve_source_path(&script_ref.source_path);
            let source = common::vfs::read_to_string(std::path::Path::new(&resolved_path))
                .map_err(|error| ScriptError {
                    file: script_ref.source_path.clone(),
                    line: 0,
                    kind: ScriptErrorKind::Runtime,
                    message: format!("could not read script at {resolved_path}: {error}"),
                })?;
            self.rhai.compile(&script_ref.source_path, &source)?;
            InstanceKind::Rhai
        } else if let Some(descriptor) = self.registry.get(&script_ref.script_id) {
            InstanceKind::Native((descriptor.make)())
        } else {
            return Err(ScriptError {
                file: script_ref.script_id.clone(),
                line: 0,
                kind: ScriptErrorKind::UnresolvedScript,
                message: format!("unresolved script id: {}", script_ref.script_id),
            });
        };
        Ok(ScriptInstance {
            script_id: script_ref.script_id.clone(),
            source_path: script_ref.source_path.clone(),
            kind,
        })
    }

    fn build_view(
        &self,
        world: &World,
        input: &InputHandler,
        players: &InputSettings,
        delta_time: f32,
        collisions: Vec<ScriptCollision>,
        physics: Option<&PhysicsSystem>,
    ) -> ScriptView {
        let blackboard = world
            .resource::<Blackboard>()
            .map(|b| b.0.clone())
            .unwrap_or_default();

        // Sorted so a duplicated Name resolves to the same entity every run: the world
        // iterates a HashMap, and the lowest id wins.
        let mut entity_ids = world.entities();
        entity_ids.sort();
        let mut entities = BTreeMap::new();
        for entity in entity_ids {
            if let Some(name) = world.get::<Name>(entity) {
                let transform = world.get::<Transform2D>(entity).copied().unwrap_or_default();
                let velocity = current_velocity(world, physics, entity);
                entities
                    .entry(name.as_str().to_string())
                    .or_insert(EntityState { entity, transform, velocity });
            }
        }

        let player_count = players.player_count();
        let mut player_axes = Vec::with_capacity(player_count);
        let mut player_actions_active = Vec::with_capacity(player_count);
        let mut player_actions_just_activated = Vec::with_capacity(player_count);

        let actions = [
            ("action1", GameAction::Action1),
            ("action2", GameAction::Action2),
            ("action3", GameAction::Action3),
            ("action4", GameAction::Action4),
            ("moveup", GameAction::MoveUp),
            ("movedown", GameAction::MoveDown),
            ("moveleft", GameAction::MoveLeft),
            ("moveright", GameAction::MoveRight),
            ("menu", GameAction::Menu),
            ("cancel", GameAction::Cancel),
            ("select", GameAction::Select),
        ];

        for p in 0..player_count {
            let pid = PlayerId(p as u8);
            player_axes.push(Vec2::new(
                players.move_x(pid, input),
                players.move_y(pid, input),
            ));

            let mut active_set = HashSet::new();
            let mut just_act_set = HashSet::new();
            for (action_name, action_enum) in &actions {
                if players.is_active(pid, *action_enum, input) {
                    active_set.insert(action_name.to_string());
                }
                if players.just_activated(pid, *action_enum, input) {
                    just_act_set.insert(action_name.to_string());
                }
            }
            player_actions_active.push(active_set);
            player_actions_just_activated.push(just_act_set);
        }

        ScriptView {
            delta_time,
            frame: self.frames_run,
            blackboard,
            entities,
            collisions,
            player_axes,
            player_actions_active,
            player_actions_just_activated,
        }
    }

    fn run_phase(
        &mut self,
        ctx: PhaseContext<'_>,
        world: &mut World,
        physics: Option<&mut PhysicsSystem>,
    ) {
        self.prune_and_resolve(world);

        let view = Rc::new(self.build_view(
            world,
            ctx.input,
            ctx.players,
            ctx.delta_time,
            ctx.collisions,
            physics.as_deref(),
        ));

        let mut phase_commands = ScriptCommands::new();
        let instance_keys: Vec<(EntityId, usize)> = self.instances.keys().copied().collect();

        for key in instance_keys {
            let (entity, index) = key;
            let scripts = match world.get::<Scripts>(entity) {
                Some(s) => s,
                None => continue,
            };
            let script_ref = match scripts.0.get(index) {
                Some(r) => r,
                None => continue,
            };
            let instance = match self.instances.get_mut(&key) {
                Some(i) => i,
                None => continue,
            };

            let transform = world
                .get::<Transform2D>(entity)
                .copied()
                .unwrap_or_default();
            let velocity = current_velocity(world, physics.as_deref(), entity);
            let name = world.get::<Name>(entity).map(|n| n.as_str().to_string());

            let me = Rc::new(SelfView {
                entity,
                transform,
                velocity,
                name,
            });

            match &mut instance.kind {
                InstanceKind::Native(behavior) => {
                    let mut params = script_ref.params.clone();
                    if let Some(desc) = self.registry.get(&instance.script_id) {
                        for p in desc.params {
                            params.entry(p.name.to_string()).or_insert_with(|| p.default.clone());
                        }
                    }

                    match ctx.phase {
                        Phase::EarlyUpdate => {
                            behavior.early_update(&me, &view, &params, &mut phase_commands);
                        }
                        Phase::Update => {
                            behavior.update(&me, &view, &params, &mut phase_commands);
                        }
                    }
                }
                InstanceKind::Rhai => {
                    if self.quarantined.contains(&key) {
                        continue;
                    }
                    // Compiled once when the instance resolved; a source edited mid-Play
                    // runs as compiled until the next Play recompiles it.
                    let Some(compiled) = self.rhai.get(&instance.source_path) else {
                        continue;
                    };

                    let mut params_map = rhai::Map::new();
                    for (key_name, value) in &compiled.header_defaults {
                        insert_rhai_value(&mut params_map, key_name, value, world);
                    }
                    for (key_name, value) in &script_ref.params {
                        insert_rhai_value(&mut params_map, key_name, value, world);
                    }

                    let handle = ScriptCommandsHandle::new();
                    let call_args = super::rhai_backend::RhaiCallArgs {
                        me: &me,
                        view: &view,
                        params: &params_map,
                        commands: &handle,
                        delta_time: ctx.delta_time,
                    };
                    let call_result = match ctx.phase {
                        Phase::EarlyUpdate => {
                            self.rhai.call_early_update(&instance.source_path, compiled, &call_args)
                        }
                        Phase::Update => {
                            self.rhai.call_update(&instance.source_path, compiled, &call_args)
                        }
                    };

                    match call_result {
                        Ok(()) => phase_commands.commands.extend(handle.drain()),
                        Err(error) => {
                            if error.kind == ScriptErrorKind::RunawayLoop {
                                self.quarantined.insert(key);
                            }
                            self.errors.push(error);
                        }
                    }
                }
            }
        }

        phase_commands.apply(world, physics, ctx.delta_time, &mut self.errors);
    }

    /// Early update phase: executed before physics simulation step.
    pub fn early_update(
        &mut self,
        world: &mut World,
        input: &InputHandler,
        players: &InputSettings,
        delta_time: f32,
        physics: Option<&mut PhysicsSystem>,
    ) {
        self.run_phase(
            PhaseContext {
                phase: Phase::EarlyUpdate,
                input,
                players,
                delta_time,
                collisions: Vec::new(),
            },
            world,
            physics,
        );
    }

    /// Main update phase: executed after physics step and collision drain.
    pub fn update(
        &mut self,
        world: &mut World,
        input: &InputHandler,
        players: &InputSettings,
        delta_time: f32,
        collisions: &[CollisionData],
        physics: Option<&mut PhysicsSystem>,
    ) {
        let script_collisions: Vec<ScriptCollision> = collisions
            .iter()
            .map(|c| ScriptCollision {
                entity_a: c.event.entity_a,
                entity_b: c.event.entity_b,
                name_a: world.get::<Name>(c.event.entity_a).map(|n| n.as_str().to_string()),
                name_b: world.get::<Name>(c.event.entity_b).map(|n| n.as_str().to_string()),
                started: c.event.started,
                stopped: c.event.stopped,
            })
            .collect();

        self.run_phase(
            PhaseContext {
                phase: Phase::Update,
                input,
                players,
                delta_time,
                collisions: script_collisions,
            },
            world,
            physics,
        );
        self.frames_run += 1;
    }
}

fn insert_rhai_value(
    map: &mut rhai::Map,
    key: &str,
    value: &ScriptValue,
    world: &World,
) {
    let dynamic = match value {
        ScriptValue::F32(v) => rhai::Dynamic::from_float(*v),
        ScriptValue::I32(v) => rhai::Dynamic::from_int(*v as i64),
        ScriptValue::Bool(v) => rhai::Dynamic::from_bool(*v),
        ScriptValue::Str(v) => rhai::Dynamic::from(v.clone()),
        ScriptValue::Vec2(v) => rhai::Dynamic::from(*v),
        ScriptValue::Entity(id) => {
            let target_name = world
                .get::<Name>(*id)
                .map(|n| n.as_str().to_string())
                .unwrap_or_default();
            rhai::Dynamic::from(target_name)
        }
        ScriptValue::Color(c) => {
            let arr = vec![
                rhai::Dynamic::from_float(c[0]),
                rhai::Dynamic::from_float(c[1]),
                rhai::Dynamic::from_float(c[2]),
                rhai::Dynamic::from_float(c[3]),
            ];
            rhai::Dynamic::from_array(arr)
        }
    };
    map.insert(key.into(), dynamic);
}
