//! Standalone helper functions for the optimiser (C30x2).
//!
//! Ported from `core/compiler/optimiser/_helpers.py`. The Python
//! module is one 653-line file; this port breaks it into focused
//! sub-modules so each pass pulls in exactly the helpers it
//! needs:
//!
//! - [`naming`] — namespace / proc-name resolution.
//! - [`literals`] — literal parsing + Tcl-source rendering.
//! - [`select`] — overlap-aware optimisation selection (the
//!   `manager`'s final output filter).
//!
//! Token-parsing helpers (`_parse_single_command_from_range`,
//! `_full_command_range`, `_extract_body_text`, `_tokens_for_statement`)
//! and command-specific folders (`_try_fold_list_command`,
//! `_try_fold_lindex_command`, `_parse_string_length_arg`,
//! `_try_incr_idiom`) are deliberately not ported here — each
//! consumer pulls them in as they land (`pattern_recognition`,
//! `elimination`, `propagation`).

pub mod expr_simplify;
pub mod literals;
pub mod naming;
pub mod select;
pub mod spans;
pub mod tokens;
