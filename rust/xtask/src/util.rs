// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Small shared helpers for the xtask subcommands.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The AGPL-3.0 copyright banner, every line prefixed with `line_comment`
/// (e.g. `"//"` for Rust/TS/Kotlin/JS, `"#"` for Python/shell).
///
/// Generated source files must carry the same licence header as hand-written
/// ones; the generator inserts it at the top of the file it emits. The banner
/// ends with a trailing newline but no blank line, so callers append their own
/// `\n` separator before the generated-marker / body.
#[must_use]
pub fn license_banner(line_comment: &str) -> String {
    const LINES: &[&str] = &[
        "tcl-lsp — a language server and toolchain for Tcl",
        "Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>",
        "",
        "This program is free software: you can redistribute it and/or modify",
        "it under the terms of the GNU Affero General Public License as published by",
        "the Free Software Foundation, either version 3 of the License, or",
        "(at your option) any later version.",
        "",
        "This program is distributed in the hope that it will be useful,",
        "but WITHOUT ANY WARRANTY; without even the implied warranty of",
        "MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the",
        "GNU Affero General Public License for more details.",
        "",
        "You should have received a copy of the GNU Affero General Public License",
        "along with this program.  If not, see <https://www.gnu.org/licenses/>.",
        "",
        "SPDX-License-Identifier: AGPL-3.0-or-later",
    ];
    let mut out = String::new();
    for line in LINES {
        if line.is_empty() {
            let _ = writeln!(out, "{line_comment}");
        } else {
            let _ = writeln!(out, "{line_comment} {line}");
        }
    }
    out
}

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
