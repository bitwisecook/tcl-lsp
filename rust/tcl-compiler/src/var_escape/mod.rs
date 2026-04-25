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

pub mod handlers;
pub mod helpers;
pub mod info_subcommands;
pub mod known_names;
pub mod state;
pub mod types;

pub use info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand, FRAME_INSPECTING_SUBCOMMANDS,
    INTERPRETER_GLOBAL_SUBCOMMANDS,
};
pub use types::{join, EscapeTag, ProcEscapeSummary};
