//! Utility macros to reduce boilerplate.

/// Generate builder-style setter methods for struct fields.
///
/// # Example
/// ```
/// use common::with_fields;
/// use glam::Vec2;
///
/// struct Sprite {
///     offset: Vec2,
///     rotation: f32,
///     scale: Vec2,
/// }
///
/// impl Sprite {
///     with_fields! {
///         offset: Vec2,
///         rotation: f32,
///         scale: Vec2,
///     }
/// }
///
/// let sprite = Sprite { offset: Vec2::ZERO, rotation: 0.0, scale: Vec2::ONE }
///     .offset(Vec2::new(10.0, 20.0))
///     .rotation(1.5);
/// assert_eq!(sprite.offset, Vec2::new(10.0, 20.0));
/// assert_eq!(sprite.rotation, 1.5);
/// ```
#[macro_export]
macro_rules! with_fields {
    ($($field:ident : $type:ty),* $(,)?) => {
        $(
            #[doc = concat!("Set the `", stringify!($field), "` field (builder pattern).")]
            #[inline]
            pub fn $field(mut self, value: $type) -> Self {
                self.$field = value;
                self
            }
        )*
    };
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    struct Sprite {
        position: Vec2,
        scale: f32,
    }

    impl Sprite {
        fn new() -> Self {
            Self {
                position: Vec2::ZERO,
                scale: 1.0,
            }
        }

        with_fields! {
            position: Vec2,
            scale: f32,
        }
    }

    #[test]
    fn test_with_fields_generates_chainable_builders_that_set_each_field() {
        let sprite = Sprite::new();

        let built = sprite.position(Vec2::new(10.0, 20.0)).scale(2.0);

        assert_eq!(built.position, Vec2::new(10.0, 20.0));
        assert_eq!(built.scale, 2.0);
    }
}
