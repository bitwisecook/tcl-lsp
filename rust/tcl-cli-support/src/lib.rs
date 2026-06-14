//! Shared CLI plumbing for the native `tcl` / `f5` Rust CLIs.
//!
//! This is the Rust home of `tooling/cli/_utils.py`: input-document
//! resolution (files / directories / inline `--source` / stdin), the
//! source-combining rule the verbs share, output writers (with Python-faithful
//! tab expansion), and the per-dialect [`CommandRegistry`] cache.
//!
//! Behaviour here is part of the parity contract with the Python CLI, so the
//! discovery order, supported-extension set, skip-directory set, and the
//! `"\n\n".join(rstrip)` combine rule all mirror `_utils.py` exactly.

#![forbid(unsafe_code)]

mod highlight;
mod input;
mod output;
mod registry;

pub use highlight::{highlight_ansi, highlight_html};
pub use input::{combine_sources, read_input_documents, CliError, InputDocument};
pub use output::{
    expand_tabs, resolve_use_colour, write_highlighted_output, write_text_output, OutputTarget,
};
pub use registry::registry_for_dialect;

/// Result alias for fallible CLI-support operations.
pub type Result<T> = std::result::Result<T, CliError>;
