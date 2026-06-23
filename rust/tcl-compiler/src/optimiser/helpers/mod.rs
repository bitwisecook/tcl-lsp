//! Standalone helper functions for the optimiser.
//!
//! Broken into focused sub-modules so each pass pulls in exactly
//! the helpers it needs:
//!
//! - [`naming`] — namespace / proc-name resolution.
//! - [`literals`] — literal parsing + Tcl-source rendering.
//! - [`select`] — overlap-aware optimisation selection (the
//!   `manager`'s final output filter).

pub mod expr_simplify;
pub mod literals;
pub mod naming;
pub mod select;
pub mod spans;
pub mod tokens;
