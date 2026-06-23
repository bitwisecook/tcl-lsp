//! Small shared helpers for the xtask subcommands.

use std::path::{Path, PathBuf};

/// The repository root, derived from this crate's location
/// (`<root>/rust/xtask`) so tasks work regardless of the working
/// directory `cargo xtask` is invoked from.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask crate lives at <root>/rust/xtask")
        .to_path_buf()
}
