//! Integration test for the `ComponentMeta` derive macro.

use ecs_macros::ComponentMeta;

// The ecs crate depends on ecs_macros, so the trait the macro targets is
// restated here in the shape the macro emits for.
pub trait ComponentMeta {
    fn type_name() -> &'static str
    where
        Self: Sized;
    fn field_names() -> &'static [&'static str]
    where
        Self: Sized;
}

#[derive(Debug, Clone, ComponentMeta)]
pub struct TestComponent {
    pub health: f32,
    pub name: String,
    pub active: bool,
}

#[test]
fn test_derive_emits_type_name_and_field_names_in_declaration_order() {
    // The registry keys on type_name and the inspector renders fields in
    // this order, so both are the contract.
    assert_eq!(TestComponent::type_name(), "TestComponent");
    assert_eq!(TestComponent::field_names(), &["health", "name", "active"]);
}
