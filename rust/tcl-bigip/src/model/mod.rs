//! BIG-IP object model — Rust port of `dialects/f5/bigip/model/`.
//!
//! The long-tail kinds share [`minimal::BigipMinimalObject`] /
//! [`minimal::BigipGenericObject`]; rich typed kinds (pool, virtual,
//! node, monitor, profile, rule, …) land in their per-module submodules
//! as they are ported.

pub mod minimal;

pub use minimal::{BigipGenericObject, BigipMinimalObject};
