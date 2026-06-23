//! Per-command lowering specialisations, one file per command.
//!
//! Each submodule exposes a `try_lower_<name>` entry point that takes
//! a [`LoweringCommand`](crate::lowering_hooks::LoweringCommand) and
//! returns a [`Statement`](crate::ir::Statement). The shared
//! dispatcher in [`crate::lowering_hooks::try_lower_hook`] routes
//! command names to the matching submodule.
//!
//! Split out from the `crate::lowering_hooks` module so each
//! command's logic lives in its own file.

pub mod control;
pub mod incr;
