//! Flow-sensitive per-SSA-version var-escape analysis (C33e).
//!
//! Mirrors `core/compiler/var_escape/_cfg_propagation.py`.
//!
//! Strips:
//!
//! * **C33e1** — [`state`]: `CfgEscapeResult` + `CfgState`.
//! * **C33e2** — `collect_known_names_from_cfg`.
//! * **C33e3** — per-call handlers (cfg variants).
//! * **C33e4** — barrier handlers + `escape_every_name_touched_tree`.
//! * **C33e5** — `handle_call` dispatcher + value/expr scans.
//! * **C33e6** — `handle_statement` + `walk_block` + `block_order` +
//!   `analyse_cfg_function` entry point.

pub mod handlers;
pub mod known_names;
pub mod state;
pub mod walker;

pub use known_names::collect_known_names_from_cfg;
pub use state::{CfgEscapeResult, CfgState};
pub use walker::analyse_cfg_function;
