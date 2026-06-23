//! var-escape analysis.
//!
//! Per-proc static analysis tagging each Tcl variable as
//! [`types::EscapeTag::Local`] (stays in a WASM local) or
//! [`types::EscapeTag::Frame`] (must live in the runtime frame
//! so the interpreter or an `upvar` alias can see it by name).
//!
//! Components:
//!
//! * [`types`]: vocabulary + summary types.
//! * [`cfg_propagation`]: intra-procedural rule audit and
//!   flow-sensitive SSA-version propagation.
//! * [`info_subcommands`]: which `info` subcommands
//!   force pessimism.
//! * [`interprocedural`]: thread escapes across call
//!   edges.
//! * [`slot_resolution`]: compile-time
//!   slot indices for proc-locals.

pub mod api;
pub mod cfg_propagation;
pub mod handlers;
pub mod helpers;
pub mod info_subcommands;
pub mod interprocedural;
pub mod known_names;
pub mod slot_resolution;
pub mod state;
pub mod types;
pub mod walker;

pub use api::{TOP_LEVEL_QNAME, analyse_var_escape, analyse_var_escape_cu, cfg_result_to_summary};
pub use cfg_propagation::analyse_cfg_function;
pub use interprocedural::solve_interprocedural_escape;
pub use slot_resolution::{LOCALS_ARRAY_CAP, assign_local_slots, populate_local_slots};
pub use walker::analyse_script;

pub use info_subcommands::{
    FRAME_INSPECTING_SUBCOMMANDS, INTERPRETER_GLOBAL_SUBCOMMANDS,
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};
pub use types::{
    Barrier, BarrierKind, EscapeReason, EscapeReasonKind, EscapeTag, ProcEscapeSummary, join,
};
