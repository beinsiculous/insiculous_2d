//! Generic component inspector using serde for field extraction.
//!
//! This module provides a way to inspect any component that implements
//! `Serialize` without hardcoding field display logic per component type.

use glam::Vec2;
use serde::Serialize;
use serde_json::Value;
use ui::{Color, UIContext};

/// Configuration for the component inspector display.
#[derive(Debug, Clone)]
pub struct InspectorStyle {
    /// Left padding for content
    pub padding: f32,
    /// Height of each line
    pub line_height: f32,
    /// Indentation for nested fields
    pub indent: f32,
    /// Color for field names
    pub label_color: Color,
    /// Color for field values
    pub value_color: Color,
    /// Color for section headers
    pub header_color: Color,
}

impl Default for InspectorStyle {
    fn default() -> Self {
        Self {
            padding: 8.0,
            line_height: 20.0,
            indent: 12.0,
            label_color: Color::new(0.7, 0.7, 0.7, 1.0),
            value_color: Color::new(1.0, 1.0, 1.0, 1.0),
            header_color: Color::new(0.9, 0.9, 0.5, 1.0),
        }
    }
}

/// Read a component's state as a serde value — the read half of the
/// inspector, split from rendering (audit §9.2): the command API's
/// `describe` query and the pixel inspector consume the same data.
pub fn component_value<T: Serialize>(component: &T) -> Result<Value, serde_json::Error> {
    serde_json::to_value(component)
}

/// Renders a component's fields using serde serialization.
///
/// Returns the Y position after rendering (for layout chaining).
///
/// # Example
/// ```
/// use editor::{inspect_component, InspectorStyle};
/// use ecs::{Transform2D, World};
/// use glam::Vec2;
///
/// # let mut world = World::new();
/// # let entity = world.create_entity();
/// # world.add_component(&entity, Transform2D::new(Vec2::new(10.0, 20.0))).ok();
/// # let mut ui = ui::UIContext::new();
/// # let style = InspectorStyle::default();
/// # let (x, start_y) = (8.0_f32, 30.0_f32);
/// let mut y = start_y;
/// if let Some(transform) = world.get::<Transform2D>(entity) {
///     y = inspect_component(&mut ui, "Transform2D", transform, x, y, &style);
/// }
/// // The returned Y advances past the header and rendered fields.
/// assert!(y > start_y);
/// ```
pub fn inspect_component<T: Serialize>(
    ui: &mut UIContext,
    type_name: &str,
    component: &T,
    x: f32,
    y: f32,
    style: &InspectorStyle,
) -> f32 {
    let mut current_y = y;

    // Render component type header
    ui.label(type_name, Vec2::new(x, current_y));
    current_y += style.line_height;

    // Serialize to JSON value for field extraction
    let value = match component_value(component) {
        Ok(v) => v,
        Err(e) => {
            ui.label(
                &format!("  (serialization error: {})", e),
                Vec2::new(x, current_y),
            );
            return current_y + style.line_height;
        }
    };

    // Render fields
    current_y = render_value(ui, &value, x + style.indent, current_y, style, 0);

    current_y
}

/// Renders a JSON value recursively with proper formatting.
fn render_value(
    ui: &mut UIContext,
    value: &Value,
    x: f32,
    y: f32,
    style: &InspectorStyle,
    depth: usize,
) -> f32 {
    let mut current_y = y;

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                current_y = render_field(ui, key, val, x, current_y, style, depth);
            }
        }
        Value::Array(arr) => {
            // For arrays, show count and elements
            ui.label(&format!("[{} items]", arr.len()), Vec2::new(x, current_y));
            current_y += style.line_height;

            // Show first few elements if small array
            if arr.len() <= 4 {
                for (i, elem) in arr.iter().enumerate() {
                    let formatted = format_simple_value(elem);
                    ui.label(&format!("  [{}]: {}", i, formatted), Vec2::new(x, current_y));
                    current_y += style.line_height;
                }
            }
        }
        _ => {
            // Simple value at top level (shouldn't happen for components)
            ui.label(&format_simple_value(value), Vec2::new(x, current_y));
            current_y += style.line_height;
        }
    }

    current_y
}

/// Renders a single field (key: value pair).
fn render_field(
    ui: &mut UIContext,
    key: &str,
    value: &Value,
    x: f32,
    y: f32,
    style: &InspectorStyle,
    depth: usize,
) -> f32 {
    let mut current_y = y;

    match value {
        // For nested objects, show header and recurse
        Value::Object(map) if !map.is_empty() => {
            // Check if it's a simple "vec-like" object (x, y or x, y, z, w)
            if is_vec_like(map) {
                let formatted = format_vec_like(map);
                ui.label(&format!("{}: {}", key, formatted), Vec2::new(x, current_y));
                current_y += style.line_height;
            } else {
                // Complex nested object
                ui.label(&format!("{}:", key), Vec2::new(x, current_y));
                current_y += style.line_height;
                current_y = render_value(ui, value, x + style.indent, current_y, style, depth + 1);
            }
        }
        // For arrays, show inline if small
        Value::Array(arr) => {
            if arr.len() <= 4 && arr.iter().all(is_simple_value) {
                let formatted: Vec<String> = arr.iter().map(format_simple_value).collect();
                ui.label(
                    &format!("{}: [{}]", key, formatted.join(", ")),
                    Vec2::new(x, current_y),
                );
                current_y += style.line_height;
            } else {
                ui.label(&format!("{}: [{} items]", key, arr.len()), Vec2::new(x, current_y));
                current_y += style.line_height;
            }
        }
        // Simple values
        _ => {
            let formatted = format_simple_value(value);
            ui.label(&format!("{}: {}", key, formatted), Vec2::new(x, current_y));
            current_y += style.line_height;
        }
    }

    current_y
}

/// Check if a JSON map has exactly the given keys (no more, no less).
fn has_exact_keys(map: &serde_json::Map<String, Value>, keys: &[&str]) -> bool {
    map.len() == keys.len() && keys.iter().all(|k| map.contains_key(*k))
}

/// Check if a JSON object looks like a vector (has x, y or x, y, z, w keys).
fn is_vec_like(map: &serde_json::Map<String, Value>) -> bool {
    has_exact_keys(map, &["x", "y"])
        || has_exact_keys(map, &["x", "y", "z"])
        || has_exact_keys(map, &["x", "y", "z", "w"])
}

/// Format a vec-like object as (x, y) or (x, y, z) or (x, y, z, w).
fn format_vec_like(map: &serde_json::Map<String, Value>) -> String {
    let x = format_simple_value(map.get("x").unwrap_or(&Value::Null));
    let y = format_simple_value(map.get("y").unwrap_or(&Value::Null));

    if let Some(z) = map.get("z") {
        if let Some(w) = map.get("w") {
            format!(
                "({}, {}, {}, {})",
                x,
                y,
                format_simple_value(z),
                format_simple_value(w)
            )
        } else {
            format!("({}, {}, {})", x, y, format_simple_value(z))
        }
    } else {
        format!("({}, {})", x, y)
    }
}

/// Check if a value is "simple" (not object or array).
fn is_simple_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

/// Format a simple JSON value for display.
fn format_simple_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            // Format floats nicely
            if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    format!("{:.3}", f)
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => format!("\"{}\"", s),
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(map) => {
            if is_vec_like(map) {
                format_vec_like(map)
            } else {
                "{...}".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_inspector_formats_values_the_way_the_panel_shows_them() {
        // Floats keep one decimal or round to three, strings are quoted,
        // x/y(/z(/w)) objects read as tuples, anything else collapses.
        let cases = [
            (serde_json::json!(42.0), "42.0"),
            (serde_json::json!(1.23456), "1.235"),
            (serde_json::json!(true), "true"),
            (serde_json::json!("hello"), "\"hello\""),
            (serde_json::json!({"x": 1.0, "y": 2.0}), "(1.0, 2.0)"),
            (serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}), "(1.0, 2.0, 3.0)"),
            (serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0, "w": 4.0}), "(1.0, 2.0, 3.0, 4.0)"),
            (serde_json::json!({"a": 1.0, "b": 2.0}), "{...}"),
            (serde_json::json!([1, 2, 3]), "[3 items]"),
        ];
        for (value, expected) in cases {
            assert_eq!(format_simple_value(&value), expected, "{value}");
        }
    }
}
