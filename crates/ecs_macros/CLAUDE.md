# ECS Macros Crate — Agent Context

Procedural macros for the ECS crate.

## Provides
- `#[derive(DeriveComponentMeta)]` — auto-implements `ComponentMeta` trait (type_name, field_names)
- `define_component!` macro — component struct definition with Default impl

## Testing
- `cargo test -p ecs_macros`
