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

//! Private, undocumented `::tcl::` implementation namespaces (issue #988).
//!
//! Real Tcl backs a handful of built-in ensemble commands with a private
//! sub-namespace under `::tcl::` — `dict create` is implemented by
//! `::tcl::dict::create`, `string totitle` by `::tcl::string::totitle`, and
//! so on. These namespaces are undocumented internals: calling into them
//! directly works (confirmed against tclsh 8.6/9.0 — see the `analyser`
//! diagnostic that consumes this table), but it is never a documented or
//! supported way to write Tcl, and membership churns release to release
//! (confirmed against the 8.4.20/8.6.16/9.0.4 C sources and live tclsh — for
//! example `::tcl::zlib` does not exist on every 8.6 build).
//!
//! Because of that churn, and because none of this is a documented
//! contract, this table is deliberately **namespace-prefix-level only** —
//! it does not model individual subcommands or per-version availability the
//! way an ordinary [`crate::spec::CommandSpec`] does. It exists purely to
//! drive a single generic "this is a private implementation namespace, use
//! the public command instead" diagnostic.

/// The 11 private `::tcl::` ensemble-backing sub-namespaces, confirmed
/// against real Tcl (8.4.20 through 9.1b0 C source, and live tclsh 8.6/9.0)
/// while investigating issue #988. Each entry's bare tail segment under
/// `::tcl::` doubles as the public ensemble command it backs (e.g. `dict`
/// backs both `::tcl::dict::*` and the public `dict` command).
pub const PRIVATE_TCL_NAMESPACES: &[&str] = &[
    "dict",
    "string",
    "array",
    "file",
    "info",
    "clock",
    "binary",
    "namespace",
    "encoding",
    "zlib",
    "chan",
];

/// A direct call into one of [`PRIVATE_TCL_NAMESPACES`], classified from a
/// command head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateTclNamespaceCall {
    /// The public ensemble command to use instead (e.g. `"dict"`).
    pub public_command: &'static str,
    /// The concrete suggested replacement call — the public command
    /// followed by the same subcommand tail (e.g. `"dict create"` for
    /// `::tcl::dict::create`).
    pub suggestion: String,
}

/// Classify `cmd_name` — the literal command-head text as written (e.g.
/// `::tcl::dict::create` or `tcl::dict::create`) — as a direct call into one
/// of the 11 private `::tcl::` namespaces.
///
/// Accepts both the fully qualified (`::tcl::…`) and bare (`tcl::…`)
/// spellings. Matches the namespace segment **exactly**, not as a
/// substring, so a near-miss (`::tcl::dictionary::foo`) or a user's own
/// namespace nested under `tcl::` (`::tcl::mycustom::foo`) is never
/// flagged. Public, documented namespaces that also live directly under
/// `tcl::` (`tcl::mathop::+`, `tcl::mathfunc::sin`, `tcl::prefix`) are
/// likewise unaffected, since none of them appear in
/// [`PRIVATE_TCL_NAMESPACES`].
///
/// Returns `None` when the namespace segment has no subcommand tail at all
/// (`::tcl::dict` alone) — there is nothing to suggest in its place.
#[must_use]
pub fn classify_private_tcl_namespace_call(cmd_name: &str) -> Option<PrivateTclNamespaceCall> {
    let rest = cmd_name.strip_prefix("::").unwrap_or(cmd_name);
    let rest = rest.strip_prefix("tcl::")?;
    let (namespace, tail) = rest.split_once("::")?;
    if tail.is_empty() {
        return None;
    }
    let public_command = *PRIVATE_TCL_NAMESPACES.iter().find(|ns| **ns == namespace)?;
    Some(PrivateTclNamespaceCall {
        public_command,
        suggestion: format!("{public_command} {tail}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_and_bare_spellings_both_classify() {
        for cmd in ["::tcl::dict::create", "tcl::dict::create"] {
            let call = classify_private_tcl_namespace_call(cmd).unwrap();
            assert_eq!(call.public_command, "dict");
            assert_eq!(call.suggestion, "dict create");
        }
    }

    #[test]
    fn every_private_namespace_classifies() {
        for ns in PRIVATE_TCL_NAMESPACES {
            let cmd = format!("::tcl::{ns}::sub");
            let call = classify_private_tcl_namespace_call(&cmd)
                .unwrap_or_else(|| panic!("expected {cmd} to classify"));
            assert_eq!(call.public_command, *ns);
            assert_eq!(call.suggestion, format!("{ns} sub"));
        }
    }

    #[test]
    fn near_miss_namespace_not_flagged() {
        assert_eq!(
            classify_private_tcl_namespace_call("::tcl::dictionary::foo"),
            None
        );
    }

    #[test]
    fn users_own_tcl_scoped_namespace_not_flagged() {
        assert_eq!(
            classify_private_tcl_namespace_call("::tcl::mycustom::foo"),
            None
        );
    }

    #[test]
    fn public_documented_tcl_namespaces_not_flagged() {
        for cmd in ["::tcl::mathop::+", "tcl::mathop::+"] {
            assert_eq!(classify_private_tcl_namespace_call(cmd), None);
        }
        for cmd in ["::tcl::mathfunc::sin", "tcl::mathfunc::sin"] {
            assert_eq!(classify_private_tcl_namespace_call(cmd), None);
        }
        // `tcl::prefix` is a real, public, documented command (Tcl 8.6+),
        // invoked as `tcl::prefix match ...` — a single command word with an
        // ordinary subcommand argument, not a `::`-qualified private
        // namespace member, so it has no `::` tail to classify at all.
        for cmd in ["tcl::prefix", "::tcl::prefix"] {
            assert_eq!(classify_private_tcl_namespace_call(cmd), None);
        }
    }

    #[test]
    fn ordinary_public_command_not_flagged() {
        assert_eq!(classify_private_tcl_namespace_call("dict"), None);
        assert_eq!(classify_private_tcl_namespace_call("dict create"), None);
    }

    #[test]
    fn bare_namespace_with_no_tail_not_flagged() {
        assert_eq!(classify_private_tcl_namespace_call("::tcl::dict"), None);
        assert_eq!(classify_private_tcl_namespace_call("::tcl::dict::"), None);
    }
}
