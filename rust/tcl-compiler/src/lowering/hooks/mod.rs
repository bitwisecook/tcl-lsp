//! Per-command lowering specialisations, one file per command.
//!
//! Each submodule exposes a `try_lower_<name>` entry point that takes
//! a [`LoweringCommand`](crate::lowering_hooks::LoweringCommand) and
//! returns a [`Statement`](crate::ir::Statement). The shared
//! dispatcher in [`crate::lowering_hooks::try_lower_hook`] routes
//! command names to the matching submodule.
//!
//! Mirrors the per-hook layout of `core/compiler/lowering_hooks/`,
//! split out from the original monolithic
//! `crate::lowering_hooks` module so each command's logic lives in
//! its own file (chunk **C43**).

pub mod incr;
