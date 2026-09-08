//! Rhai engine construction and API bindings.

use std::rc::Rc;

use glam::Vec2;
use rhai::{Engine, EvalAltResult, ImmutableString};

use common::Transform2D;
use ecs::script::ScriptValue;

use super::commands::{ScriptCommand, ScriptCommandsHandle, Target};
use super::view::{ScriptView, SelfView};

fn register_math_and_conversions(engine: &mut Engine) {
    // Vector type
    engine.register_type_with_name::<Vec2>("Vec2");
    engine.register_fn("vec2", |x: f32, y: f32| Vec2::new(x, y));
    engine.register_fn("vec2", |x: i64, y: i64| Vec2::new(x as f32, y as f32));
    engine.register_fn("vec2", |x: f32, y: i64| Vec2::new(x, y as f32));
    engine.register_fn("vec2", |x: i64, y: f32| Vec2::new(x as f32, y));

    engine.register_get("x", |v: &mut Vec2| v.x);
    engine.register_get("y", |v: &mut Vec2| v.y);
    engine.register_set("x", |v: &mut Vec2, x: f32| v.x = x);
    engine.register_set("x", |v: &mut Vec2, x: i64| v.x = x as f32);
    engine.register_set("y", |v: &mut Vec2, y: f32| v.y = y);
    engine.register_set("y", |v: &mut Vec2, y: i64| v.y = y as f32);

    engine.register_fn("+", |a: Vec2, b: Vec2| a + b);
    engine.register_fn("-", |a: Vec2, b: Vec2| a - b);
    engine.register_fn("-", |a: Vec2| -a);
    engine.register_fn("*", |a: Vec2, scalar: f32| a * scalar);
    engine.register_fn("*", |a: Vec2, scalar: i64| a * (scalar as f32));
    engine.register_fn("*", |scalar: f32, a: Vec2| a * scalar);
    engine.register_fn("*", |scalar: i64, a: Vec2| a * (scalar as f32));
    engine.register_fn("/", |a: Vec2, scalar: f32| a / scalar);
    engine.register_fn("/", |a: Vec2, scalar: i64| a / (scalar as f32));
    engine.register_fn("==", |a: Vec2, b: Vec2| a == b);
    engine.register_fn("!=", |a: Vec2, b: Vec2| a != b);

    engine.register_fn("length", |v: &mut Vec2| v.length());
    engine.register_fn("normalize", |v: &mut Vec2| {
        if v.length_squared() > f32::EPSILON {
            v.normalize()
        } else {
            Vec2::ZERO
        }
    });
    engine.register_fn("dot", |a: &mut Vec2, b: Vec2| a.dot(b));

    // Mixed INT / FLOAT arithmetic
    engine.register_fn("+", |a: i64, b: f32| a as f32 + b);
    engine.register_fn("+", |a: f32, b: i64| a + b as f32);
    engine.register_fn("-", |a: i64, b: f32| a as f32 - b);
    engine.register_fn("-", |a: f32, b: i64| a - b as f32);
    engine.register_fn("*", |a: i64, b: f32| a as f32 * b);
    engine.register_fn("*", |a: f32, b: i64| a * b as f32);
    engine.register_fn("/", |a: i64, b: f32| a as f32 / b);
    engine.register_fn("/", |a: f32, b: i64| a / b as f32);
    engine.register_fn("%", |a: i64, b: f32| (a as f32) % b);
    engine.register_fn("%", |a: f32, b: i64| a % (b as f32));
    engine.register_fn("<", |a: i64, b: f32| (a as f32) < b);
    engine.register_fn("<", |a: f32, b: i64| a < (b as f32));
    engine.register_fn("<=", |a: i64, b: f32| (a as f32) <= b);
    engine.register_fn("<=", |a: f32, b: i64| a <= (b as f32));
    engine.register_fn(">", |a: i64, b: f32| (a as f32) > b);
    engine.register_fn(">", |a: f32, b: i64| a > (b as f32));
    engine.register_fn(">=", |a: i64, b: f32| (a as f32) >= b);
    engine.register_fn(">=", |a: f32, b: i64| a >= (b as f32));
    engine.register_fn("==", |a: i64, b: f32| (a as f32) == b);
    engine.register_fn("==", |a: f32, b: i64| a == (b as f32));
    engine.register_fn("!=", |a: i64, b: f32| (a as f32) != b);
    engine.register_fn("!=", |a: f32, b: i64| a != (b as f32));
}

fn register_views(engine: &mut Engine) {
    // Transform2D (getters only)
    engine.register_type_with_name::<Transform2D>("Transform2D");
    engine.register_get("position", |t: &mut Transform2D| t.position);
    engine.register_get("rotation", |t: &mut Transform2D| t.rotation);
    engine.register_get("scale", |t: &mut Transform2D| t.scale);
    engine.register_get("x", |t: &mut Transform2D| t.position.x);
    engine.register_get("y", |t: &mut Transform2D| t.position.y);

    // Rc<SelfView> (getters only)
    engine.register_type_with_name::<Rc<SelfView>>("SelfView");
    engine.register_get("name", |me: &mut Rc<SelfView>| {
        me.name.clone().unwrap_or_default()
    });
    engine.register_get("transform", |me: &mut Rc<SelfView>| me.transform);
    engine.register_get("position", |me: &mut Rc<SelfView>| me.transform.position);
    engine.register_get("rotation", |me: &mut Rc<SelfView>| me.transform.rotation);
    engine.register_get("velocity", |me: &mut Rc<SelfView>| me.velocity);

    // Rc<ScriptView> (getters and query methods only)
    engine.register_type_with_name::<Rc<ScriptView>>("ScriptView");
    engine.register_get("delta_time", |v: &mut Rc<ScriptView>| v.delta_time);
    engine.register_get("frame", |v: &mut Rc<ScriptView>| v.frame as i64);

    engine.register_fn("has_entity", |v: &mut Rc<ScriptView>, name: &str| {
        v.has_entity(name)
    });
    engine.register_fn("has_entity", |v: &mut Rc<ScriptView>, name: ImmutableString| {
        v.has_entity(name.as_str())
    });
    engine.register_fn("transform", |v: &mut Rc<ScriptView>, name: &str| {
        v.transform(name).unwrap_or_default()
    });
    engine.register_fn("transform", |v: &mut Rc<ScriptView>, name: ImmutableString| {
        v.transform(name.as_str()).unwrap_or_default()
    });
    engine.register_fn("position", |v: &mut Rc<ScriptView>, name: &str| {
        v.position(name).unwrap_or(Vec2::ZERO)
    });
    engine.register_fn("position", |v: &mut Rc<ScriptView>, name: ImmutableString| {
        v.position(name.as_str()).unwrap_or(Vec2::ZERO)
    });
    engine.register_fn("velocity", |v: &mut Rc<ScriptView>, name: &str| {
        v.velocity(name).unwrap_or(Vec2::ZERO)
    });
    engine.register_fn("velocity", |v: &mut Rc<ScriptView>, name: ImmutableString| {
        v.velocity(name.as_str()).unwrap_or(Vec2::ZERO)
    });

    engine.register_fn("blackboard_bool", |v: &mut Rc<ScriptView>, key: &str, def: bool| {
        v.blackboard_bool(key, def)
    });
    engine.register_fn("blackboard_bool", |v: &mut Rc<ScriptView>, key: ImmutableString, def: bool| {
        v.blackboard_bool(key.as_str(), def)
    });
    engine.register_fn("blackboard_int", |v: &mut Rc<ScriptView>, key: &str, def: i64| {
        v.blackboard_int(key, def as i32) as i64
    });
    engine.register_fn("blackboard_int", |v: &mut Rc<ScriptView>, key: ImmutableString, def: i64| {
        v.blackboard_int(key.as_str(), def as i32) as i64
    });
    engine.register_fn("blackboard_float", |v: &mut Rc<ScriptView>, key: &str, def: f32| {
        v.blackboard_float(key, def)
    });
    engine.register_fn("blackboard_float", |v: &mut Rc<ScriptView>, key: &str, def: i64| {
        v.blackboard_float(key, def as f32)
    });
    engine.register_fn("blackboard_float", |v: &mut Rc<ScriptView>, key: ImmutableString, def: f32| {
        v.blackboard_float(key.as_str(), def)
    });
    engine.register_fn("blackboard_str", |v: &mut Rc<ScriptView>, key: &str, def: &str| {
        v.blackboard_str(key, def)
    });
    engine.register_fn("blackboard_str", |v: &mut Rc<ScriptView>, key: ImmutableString, def: ImmutableString| {
        v.blackboard_str(key.as_str(), def.as_str())
    });

    engine.register_fn("move_x", |v: &mut Rc<ScriptView>, player: i64| {
        v.move_x(player as usize)
    });
    engine.register_fn("move_y", |v: &mut Rc<ScriptView>, player: i64| {
        v.move_y(player as usize)
    });
    engine.register_fn("is_active", |v: &mut Rc<ScriptView>, player: i64, act: &str| {
        v.is_active(player as usize, act)
    });
    engine.register_fn("is_active", |v: &mut Rc<ScriptView>, player: i64, act: ImmutableString| {
        v.is_active(player as usize, act.as_str())
    });
    engine.register_fn("just_activated", |v: &mut Rc<ScriptView>, player: i64, act: &str| {
        v.just_activated(player as usize, act)
    });
    engine.register_fn("just_activated", |v: &mut Rc<ScriptView>, player: i64, act: ImmutableString| {
        v.just_activated(player as usize, act.as_str())
    });

    engine.register_fn("has_collision", |v: &mut Rc<ScriptView>, a: &str, b: &str| {
        v.has_collision(a, b)
    });
    engine.register_fn("has_collision", |v: &mut Rc<ScriptView>, me: Rc<SelfView>, other: &str| {
        v.has_collision_with(&me, other)
    });
    engine.register_fn("has_collision", |v: &mut Rc<ScriptView>, a: ImmutableString, b: ImmutableString| {
        v.has_collision(a.as_str(), b.as_str())
    });
    engine.register_fn("has_collision", |v: &mut Rc<ScriptView>, me: Rc<SelfView>, other: ImmutableString| {
        v.has_collision_with(&me, other.as_str())
    });
    engine.register_fn("has_collision_started", |v: &mut Rc<ScriptView>, a: &str, b: &str| {
        v.has_collision_started(a, b)
    });
    engine.register_fn("has_collision_started", |v: &mut Rc<ScriptView>, me: Rc<SelfView>, other: &str| {
        v.has_collision_with(&me, other)
    });
    engine.register_fn("has_collision_stopped", |v: &mut Rc<ScriptView>, a: &str, b: &str| {
        v.has_collision_stopped(a, b)
    });
}

fn register_commands(engine: &mut Engine) {
    engine.register_type_with_name::<ScriptCommandsHandle>("ScriptCommandsHandle");

    // set_position
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, pos: Vec2| {
        out.push(ScriptCommand::SetPosition { target: Target::Entity(me.entity), position: pos });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, name: &str, pos: Vec2| {
        out.push(ScriptCommand::SetPosition { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, name: ImmutableString, pos: Vec2| {
        out.push(ScriptCommand::SetPosition { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: f32, y: f32| {
        out.push(ScriptCommand::SetPosition { target: Target::Entity(me.entity), position: Vec2::new(x, y) });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: i64, y: i64| {
        out.push(ScriptCommand::SetPosition { target: Target::Entity(me.entity), position: Vec2::new(x as f32, y as f32) });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, name: &str, x: f32, y: f32| {
        out.push(ScriptCommand::SetPosition { target: Target::Named(name.to_string()), position: Vec2::new(x, y) });
    });
    engine.register_fn("set_position", |out: &mut ScriptCommandsHandle, name: &str, x: i64, y: i64| {
        out.push(ScriptCommand::SetPosition { target: Target::Named(name.to_string()), position: Vec2::new(x as f32, y as f32) });
    });

    // set_rotation
    engine.register_fn("set_rotation", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, radians: f32| {
        out.push(ScriptCommand::SetRotation { target: Target::Entity(me.entity), radians });
    });
    engine.register_fn("set_rotation", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, radians: i64| {
        out.push(ScriptCommand::SetRotation { target: Target::Entity(me.entity), radians: radians as f32 });
    });
    engine.register_fn("set_rotation", |out: &mut ScriptCommandsHandle, name: &str, radians: f32| {
        out.push(ScriptCommand::SetRotation { target: Target::Named(name.to_string()), radians });
    });
    engine.register_fn("set_rotation", |out: &mut ScriptCommandsHandle, name: &str, radians: i64| {
        out.push(ScriptCommand::SetRotation { target: Target::Named(name.to_string()), radians: radians as f32 });
    });

    // set_velocity
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, vel: Vec2| {
        out.push(ScriptCommand::SetVelocity { target: Target::Entity(me.entity), velocity: vel });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, name: &str, vel: Vec2| {
        out.push(ScriptCommand::SetVelocity { target: Target::Named(name.to_string()), velocity: vel });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, name: ImmutableString, vel: Vec2| {
        out.push(ScriptCommand::SetVelocity { target: Target::Named(name.to_string()), velocity: vel });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, vx: f32, vy: f32| {
        out.push(ScriptCommand::SetVelocity { target: Target::Entity(me.entity), velocity: Vec2::new(vx, vy) });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, vx: i64, vy: i64| {
        out.push(ScriptCommand::SetVelocity { target: Target::Entity(me.entity), velocity: Vec2::new(vx as f32, vy as f32) });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, name: &str, vx: f32, vy: f32| {
        out.push(ScriptCommand::SetVelocity { target: Target::Named(name.to_string()), velocity: Vec2::new(vx, vy) });
    });
    engine.register_fn("set_velocity", |out: &mut ScriptCommandsHandle, name: &str, vx: i64, vy: i64| {
        out.push(ScriptCommand::SetVelocity { target: Target::Named(name.to_string()), velocity: Vec2::new(vx as f32, vy as f32) });
    });

    // set_velocity_x and set_velocity_y: one axis; the other keeps its value at apply time
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: f32| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Entity(me.entity), x });
    });
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: i64| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Entity(me.entity), x: x as f32 });
    });
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, name: &str, x: f32| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Named(name.to_string()), x });
    });
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, name: &str, x: i64| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Named(name.to_string()), x: x as f32 });
    });
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, name: ImmutableString, x: f32| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Named(name.to_string()), x });
    });
    engine.register_fn("set_velocity_x", |out: &mut ScriptCommandsHandle, name: ImmutableString, x: i64| {
        out.push(ScriptCommand::SetVelocityX { target: Target::Named(name.to_string()), x: x as f32 });
    });

    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, y: f32| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Entity(me.entity), y });
    });
    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, y: i64| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Entity(me.entity), y: y as f32 });
    });
    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, name: &str, y: f32| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Named(name.to_string()), y });
    });
    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, name: &str, y: i64| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Named(name.to_string()), y: y as f32 });
    });
    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, name: ImmutableString, y: f32| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Named(name.to_string()), y });
    });
    engine.register_fn("set_velocity_y", |out: &mut ScriptCommandsHandle, name: ImmutableString, y: i64| {
        out.push(ScriptCommand::SetVelocityY { target: Target::Named(name.to_string()), y: y as f32 });
    });

    // set_kinematic_target
    engine.register_fn("set_kinematic_target", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, pos: Vec2| {
        out.push(ScriptCommand::SetKinematicTarget { target: Target::Entity(me.entity), position: pos });
    });
    engine.register_fn("set_kinematic_target", |out: &mut ScriptCommandsHandle, name: &str, pos: Vec2| {
        out.push(ScriptCommand::SetKinematicTarget { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("set_kinematic_target", |out: &mut ScriptCommandsHandle, name: ImmutableString, pos: Vec2| {
        out.push(ScriptCommand::SetKinematicTarget { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("set_kinematic_target", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: f32, y: f32| {
        out.push(ScriptCommand::SetKinematicTarget { target: Target::Entity(me.entity), position: Vec2::new(x, y) });
    });
    engine.register_fn("set_kinematic_target", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: i64, y: i64| {
        out.push(ScriptCommand::SetKinematicTarget { target: Target::Entity(me.entity), position: Vec2::new(x as f32, y as f32) });
    });

    // reset_body
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, pos: Vec2| {
        out.push(ScriptCommand::ResetBody { target: Target::Entity(me.entity), position: pos });
    });
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, name: &str, pos: Vec2| {
        out.push(ScriptCommand::ResetBody { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, name: ImmutableString, pos: Vec2| {
        out.push(ScriptCommand::ResetBody { target: Target::Named(name.to_string()), position: pos });
    });
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, x: f32, y: f32| {
        out.push(ScriptCommand::ResetBody { target: Target::Entity(me.entity), position: Vec2::new(x, y) });
    });
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, name: &str, x: f32, y: f32| {
        out.push(ScriptCommand::ResetBody { target: Target::Named(name.to_string()), position: Vec2::new(x, y) });
    });
    engine.register_fn("reset_body", |out: &mut ScriptCommandsHandle, name: &str, x: i64, y: i64| {
        out.push(ScriptCommand::ResetBody { target: Target::Named(name.to_string()), position: Vec2::new(x as f32, y as f32) });
    });

    // set_sprite_color: [r, g, b, a]
    engine.register_fn("set_sprite_color", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, color: rhai::Array| -> Result<(), Box<EvalAltResult>> {
        out.push(ScriptCommand::SetSpriteColor { target: Target::Entity(me.entity), color: color_channels(color)? });
        Ok(())
    });
    engine.register_fn("set_sprite_color", |out: &mut ScriptCommandsHandle, name: &str, color: rhai::Array| -> Result<(), Box<EvalAltResult>> {
        out.push(ScriptCommand::SetSpriteColor { target: Target::Named(name.to_string()), color: color_channels(color)? });
        Ok(())
    });
    engine.register_fn("set_sprite_color", |out: &mut ScriptCommandsHandle, name: ImmutableString, color: rhai::Array| -> Result<(), Box<EvalAltResult>> {
        out.push(ScriptCommand::SetSpriteColor { target: Target::Named(name.to_string()), color: color_channels(color)? });
        Ok(())
    });

    // set_sprite_visible
    engine.register_fn("set_sprite_visible", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, visible: bool| {
        out.push(ScriptCommand::SetSpriteVisible { target: Target::Entity(me.entity), visible });
    });
    engine.register_fn("set_sprite_visible", |out: &mut ScriptCommandsHandle, name: &str, visible: bool| {
        out.push(ScriptCommand::SetSpriteVisible { target: Target::Named(name.to_string()), visible });
    });

    // set_label_text
    engine.register_fn("set_label_text", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, text: &str| {
        out.push(ScriptCommand::SetLabelText { target: Target::Entity(me.entity), text: text.to_string() });
    });
    engine.register_fn("set_label_text", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>, text: ImmutableString| {
        out.push(ScriptCommand::SetLabelText { target: Target::Entity(me.entity), text: text.to_string() });
    });
    engine.register_fn("set_label_text", |out: &mut ScriptCommandsHandle, name: &str, text: &str| {
        out.push(ScriptCommand::SetLabelText { target: Target::Named(name.to_string()), text: text.to_string() });
    });
    engine.register_fn("set_label_text", |out: &mut ScriptCommandsHandle, name: ImmutableString, text: ImmutableString| {
        out.push(ScriptCommand::SetLabelText { target: Target::Named(name.to_string()), text: text.to_string() });
    });

    // blackboard writes
    engine.register_fn("set_blackboard_bool", |out: &mut ScriptCommandsHandle, key: &str, value: bool| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::Bool(value) });
    });
    engine.register_fn("set_blackboard_bool", |out: &mut ScriptCommandsHandle, key: ImmutableString, value: bool| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::Bool(value) });
    });
    engine.register_fn("set_blackboard_int", |out: &mut ScriptCommandsHandle, key: &str, value: i64| -> Result<(), Box<EvalAltResult>> {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::I32(blackboard_int(value)?) });
        Ok(())
    });
    engine.register_fn("set_blackboard_int", |out: &mut ScriptCommandsHandle, key: ImmutableString, value: i64| -> Result<(), Box<EvalAltResult>> {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::I32(blackboard_int(value)?) });
        Ok(())
    });
    engine.register_fn("set_blackboard_float", |out: &mut ScriptCommandsHandle, key: &str, value: f32| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::F32(value) });
    });
    engine.register_fn("set_blackboard_float", |out: &mut ScriptCommandsHandle, key: &str, value: i64| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::F32(value as f32) });
    });
    engine.register_fn("set_blackboard_float", |out: &mut ScriptCommandsHandle, key: ImmutableString, value: f32| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::F32(value) });
    });
    engine.register_fn("set_blackboard_str", |out: &mut ScriptCommandsHandle, key: &str, value: &str| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::Str(value.to_string()) });
    });
    engine.register_fn("set_blackboard_str", |out: &mut ScriptCommandsHandle, key: ImmutableString, value: ImmutableString| {
        out.push(ScriptCommand::SetBlackboard { key: key.to_string(), value: ScriptValue::Str(value.to_string()) });
    });

    // despawn
    engine.register_fn("despawn", |out: &mut ScriptCommandsHandle, me: Rc<SelfView>| {
        out.push(ScriptCommand::Despawn { target: Target::Entity(me.entity) });
    });
    engine.register_fn("despawn", |out: &mut ScriptCommandsHandle, name: &str| {
        out.push(ScriptCommand::Despawn { target: Target::Named(name.to_string()) });
    });
    engine.register_fn("despawn", |out: &mut ScriptCommandsHandle, name: ImmutableString| {
        out.push(ScriptCommand::Despawn { target: Target::Named(name.to_string()) });
    });
}

/// Rhai integers are `i64` and the blackboard stores `i32`: a value that does not fit is
/// the script's error, never a silent zero.
fn blackboard_int(value: i64) -> Result<i32, Box<EvalAltResult>> {
    i32::try_from(value)
        .map_err(|_| format!("blackboard integer {value} is outside the i32 range").into())
}

/// A colour reaches Rhai as `[r, g, b, a]`; anything but four numbers is the script's error.
fn color_channels(array: rhai::Array) -> Result<[f32; 4], Box<EvalAltResult>> {
    if array.len() != 4 {
        return Err(format!("a color is [r, g, b, a]; got {} elements", array.len()).into());
    }
    let mut channels = [0.0f32; 4];
    for (channel, value) in channels.iter_mut().zip(array) {
        *channel = if let Some(float) = value.clone().try_cast::<f32>() {
            float
        } else if let Some(integer) = value.try_cast::<i64>() {
            integer as f32
        } else {
            return Err("a color channel must be a number".into());
        };
    }
    Ok(channels)
}

/// Create a configured Rhai execution engine.
pub fn create_rhai_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(200_000);
    register_math_and_conversions(&mut engine);
    register_views(&mut engine);
    register_commands(&mut engine);
    engine
}
