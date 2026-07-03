//! BIG-IP object model.
//!
//! The long-tail kinds share [`minimal::BigipMinimalObject`] /
//! [`minimal::BigipGenericObject`]; rich typed kinds (pool, virtual,
//! node, monitor, profile, rule, …) live in their per-module submodules.

pub mod enums;
pub mod r#gen;
pub mod minimal;
pub mod port_names;

pub use enums::{DataGroupType, ProfileType};
pub use r#gen::*;
pub use minimal::{BigipGenericObject, BigipMinimalObject};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_structs_carry_defaults() {
        // Non-empty string defaults survive into the Rust Default.
        let pool = BigipPool::default();
        assert_eq!(pool.module, "ltm");
        assert!(pool.full_path.is_empty());
        // Enum field defaults resolve.
        assert_eq!(BigipProfile::default().profile_type, ProfileType::Other);
        assert_eq!(BigipDataGroup::default().kind, DataGroupType::Internal);
    }
}
