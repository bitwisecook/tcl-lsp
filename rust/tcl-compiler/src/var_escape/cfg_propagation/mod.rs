//! Flow-sensitive per-SSA-version var-escape analysis.
//!
//! Components:
//!
//! * [`state`]: `CfgEscapeResult` + `CfgState`.
//! * `collect_known_names_from_cfg`.
//! * per-call handlers (cfg variants).
//! * barrier handlers + `escape_every_name_touched_tree`.
//! * `handle_call` dispatcher + value/expr scans.
//! * `handle_statement` + `walk_block` + `block_order` +
//!   `analyse_cfg_function` entry point.

pub mod handlers;
pub mod known_names;
pub mod state;
pub mod walker;

pub use known_names::collect_known_names_from_cfg;
pub use state::{CfgEscapeResult, CfgState};
pub use walker::analyse_cfg_function;
