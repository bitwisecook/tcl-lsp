//! var-escape analysis (C33).
//!
//! Per-proc static analysis tagging each Tcl variable as
//! [`types::EscapeTag::Local`] (stays in a WASM local) or
//! [`types::EscapeTag::Frame`] (must live in the runtime frame
//! so the interpreter or an `upvar` alias can see it by name).
//!
//! Mirrors `core/compiler/var_escape/` (main commits `69aa16eb` +
//! follow-ups).
//!
//! Strips:
//!
//! * **C33a** — [`types`]: vocabulary + summary types.
//! * **C33b** — [`propagation`]: intra-procedural rule audit.
//! * **C33c** — [`info_subcommands`]: which `info` subcommands
//!   force pessimism.
//! * **C33d** — [`interprocedural`]: thread escapes across call
//!   edges.
//! * **C33e** — [`cfg_propagation`]: flow-sensitive SSA-version
//!   propagation.

pub mod types;

pub use types::{join, EscapeTag, ProcEscapeSummary};
