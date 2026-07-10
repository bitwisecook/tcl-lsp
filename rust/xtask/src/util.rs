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

/// Placeholder registry entry (`irules_disabled.rs`) marking a command as
/// unavailable in iRules — never a real command name to project into a
/// generated editor grammar/query. Shared by every generator that walks
/// [`tcl_registry::CommandRegistry::command_names`] (`gen_zed_queries`,
/// `gen_tmlanguage_keywords`).
pub const DISABLED_SENTINEL: &str = "disabled_in_irules";

/// `true` when `name` is safe to embed literally in a generated regex
/// alternation / tree-sitter `#any-of?` list: ASCII alphanumerics, `_`, or
/// `:` only. Shared by every generator that projects
/// [`tcl_registry::CommandRegistry::command_names`] into a static editor
/// grammar/query (`gen_zed_queries`, `gen_tmlanguage_keywords`).
#[must_use]
pub fn is_safe_command_name(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// `true` when `name` is registry noise that is real (has a `CommandSpec`)
/// but not a "common built-in" or "common command" by any reasonable
/// definition, so generated editor grammars/queries should exclude it even
/// though it passes the ambient-dialect / no-`required_package` filter:
///
/// * `testXxx` — Tcl core's own, stable naming convention for commands that
///   exist only in the special `tcltest`-instrumented debug build
///   (`tclUnixTest.c` et al), never present in a normal user interpreter.
/// * `::tcl::…` / `tcl::…` — Tcl's own reserved internal-implementation
///   namespace (documented as such), technically callable but not something
///   a user would recognise as a built-in command. Checked as a *prefix*
///   (`name.starts_with("tcl::")`, after stripping a leading `::`), not a
///   substring match — a substring check would also match unrelated
///   namespaces that merely end in `tcl` before their own `::`, e.g.
///   `itcl::class` (the `itcl` package's own namespace, not `::tcl::`).
///
/// Shared by every generator that projects
/// [`tcl_registry::CommandRegistry::command_names`] into a static editor
/// grammar/query (`gen_zed_queries`, `gen_tmlanguage_keywords`).
#[must_use]
pub fn is_internal_or_test_harness_noise(name: &str) -> bool {
    let is_test_harness = name.starts_with("test") && name.len() > "test".len();
    let is_tcl_internal_namespace = name.strip_prefix("::").unwrap_or(name).starts_with("tcl::");
    is_test_harness || is_tcl_internal_namespace
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_namespace_check_is_a_prefix_not_a_substring() {
        // The actual reserved namespace — must be excluded, bare and `::`-qualified.
        assert!(is_internal_or_test_harness_noise("tcl::mathop::eq"));
        assert!(is_internal_or_test_harness_noise("::tcl::mathop::eq"));
        assert!(is_internal_or_test_harness_noise("::tcl::process"));
        // A DIFFERENT package's namespace that merely ends in "tcl" before its
        // own "::" must NOT be excluded (issue caught in PR #866 review: a
        // `.contains("tcl::")` substring check wrongly matched this).
        assert!(!is_internal_or_test_harness_noise("itcl::class"));
        assert!(!is_internal_or_test_harness_noise("itcl::configure"));
    }

    #[test]
    fn test_prefix_check_requires_more_than_the_bare_word() {
        assert!(is_internal_or_test_harness_noise("testalarm"));
        assert!(is_internal_or_test_harness_noise("testchannel"));
        // Real commands that merely start with "test" as a substring of a
        // longer unrelated word, or are exactly "test", must survive.
        assert!(!is_internal_or_test_harness_noise("test"));
    }

    #[test]
    fn safe_command_name_rejects_regex_metacharacters() {
        assert!(is_safe_command_name("lappend"));
        assert!(is_safe_command_name("tcl::mathop::eq"));
        assert!(!is_safe_command_name("+"));
        assert!(!is_safe_command_name("a.b"));
    }
}
