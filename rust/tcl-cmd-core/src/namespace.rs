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

//! `namespace` text-op cores — the pure `::`-qualified-name manipulations
//! (`tail`, `qualifiers`), shared verbatim by both runtimes.
//!
//! The `tail`/`qualifiers` ops are pure byte→byte operations on the *literal*
//! name (no namespace resolution, no runtime state), so each runtime hands in
//! the name bytes and builds its own result value from the returned slice. The
//! `current`/`which` cores *do* read namespace state, so they are generic over
//! the [`Namespaces`] role trait + [`ValueOps`].

use tcl_runtime_api::Namespaces;
use tcl_syntax::glob::string_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// The byte range `(start, end)` of the last `::` separator **run** (two or more
/// consecutive colons) in `s`: `s[..start]` is the qualifier, `s[end..]` the
/// tail. `None` when there is no `::`. Mirrors C Tcl's `TclGetNamespaceForQualName`
/// colon-run handling — a run of 3+ colons is one separator (so `foo:::` has
/// qualifier `foo` and an empty tail), where a naive `rsplit("::")` diverges.
fn last_sep_run(s: &[u8]) -> Option<(usize, usize)> {
    // Scan back for the last "::" pair, then extend over every adjacent colon.
    let mut i = s.len();
    while i >= 2 {
        if s[i - 1] == b':' && s[i - 2] == b':' {
            let mut start = i - 2;
            while start > 0 && s[start - 1] == b':' {
                start -= 1;
            }
            let mut end = i;
            while end < s.len() && s[end] == b':' {
                end += 1;
            }
            return Some((start, end));
        }
        i -= 1;
    }
    None
}

/// The namespace portion of a written qualified name, retaining whether the
/// spelling is rooted.  `namespace qualifiers` intentionally erases this
/// distinction for its public string result, but name resolution cannot: a
/// leading `::` means the global namespace, even when there is no separator
/// before the command tail (`::lassign`).  This is the three-way contract used
/// by `TclGetNamespaceForQualName` in `tclNamesp.c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier<'a> {
    /// A rooted name; the slice is the written namespace prefix (`b"::"` for
    /// a global command, `b"::a"` for a command in namespace `a`).
    Absolute(&'a [u8]),
    /// A namespace-relative qualified name (`a::b`).
    Relative(&'a [u8]),
    /// An unqualified name (`b`).
    Unqualified,
}

/// Split a written command/name into its resolution qualifier while retaining
/// the absolute marker.  The public [`qualifiers`] operation below continues
/// to return the historical empty string for `::lassign`.
#[must_use]
pub fn qualifier(name: &[u8]) -> Qualifier<'_> {
    let rooted = name.starts_with(b"::");
    match last_sep_run(name) {
        Some((start, _)) => {
            // The leading root is itself the qualifier when the only `::`
            // run is the rooted marker (`::name`, `::`, or `:::`).
            let prefix = if rooted && start == 0 {
                &name[..2]
            } else {
                &name[..start]
            };
            if rooted {
                Qualifier::Absolute(prefix)
            } else {
                Qualifier::Relative(prefix)
            }
        }
        None if rooted => Qualifier::Absolute(b"::"),
        None => Qualifier::Unqualified,
    }
}

/// `namespace qualifiers string` — everything before the last `::` run (the
/// empty string when `string` is unqualified).
#[must_use]
pub fn qualifiers(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((start, _)) => &name[..start],
        None => b"",
    }
}

/// `namespace tail string` — the simple name after the last `::` run (the whole
/// run is skipped, so `foo:::` yields the empty string).
#[must_use]
pub fn tail(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((_, end)) => &name[end..],
        None => name,
    }
}

/// Expand a static `namespace import` pattern for a bare command name.
///
/// This is the shared glob decision seam for analyser and LSP consumers:
/// Tcl uses the full `*`/`?`/bracket-class grammar, while the source
/// namespace follows the colon-run qualifier rules above.
#[must_use]
pub fn imported_command_candidate(pattern: &str, name: &str) -> Option<String> {
    let pattern_tail = std::str::from_utf8(tail(pattern.as_bytes())).ok()?;
    if !string_match(pattern_tail, name) {
        return None;
    }
    match qualifier(pattern.as_bytes()) {
        Qualifier::Absolute(ns) | Qualifier::Relative(ns) => {
            let ns = std::str::from_utf8(ns).ok()?;
            Some(tcl_syntax::naming::qualify(ns, name))
        }
        Qualifier::Unqualified => (pattern_tail == name).then(|| name.to_owned()),
    }
}

/// `namespace current` — the fully-qualified name of the current namespace
/// (`"::"` at the global level).
pub fn current<O: ValueOps + Namespaces>(ops: &mut O) -> O::Value {
    let ns = Namespaces::current(ops);
    let name = ops.name(ns);
    ops.new_string(name)
}

/// `namespace which -command name` — the fully-qualified name `name` resolves to
/// as a command from the current namespace, or the empty string if it resolves
/// to nothing. (Option parsing and the `-variable` form stay in each adapter.)
pub fn which_command<O: ValueOps + Namespaces>(ops: &mut O, name: &str) -> O::Value {
    let cur = Namespaces::current(ops);
    match ops
        .find_command(cur, name)
        .and_then(|id| ops.command_name(id))
    {
        Some(fqn) => ops.new_string(fqn),
        None => ops.empty(),
    }
}

/// `namespace exists name` — whether `name` (resolved from the current
/// namespace) names an existing namespace.
pub fn exists<O: ValueOps + Namespaces>(ops: &mut O, name: &str) -> O::Value {
    let cur = Namespaces::current(ops);
    let present = ops.find_namespace(cur, name).is_some();
    ops.new_bool(present)
}

/// `namespace parent ?name?` — the FQN of the (named, or current) namespace's
/// parent (the global root's parent is the empty string).
///
/// # Errors
/// `namespace "<name>" not found` if `name` is given and does not resolve.
pub fn parent<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&str>,
) -> Result<O::Value, CmdError> {
    let ns = resolve_target(ops, name)?;
    let fqn = match ops.parent(ns) {
        Some(p) => ops.name(p),
        None => String::new(), // the global root has no parent
    };
    Ok(ops.new_string(fqn))
}

/// `namespace children ?name? ?pattern?` — the FQNs of the (named, or current)
/// namespace's child namespaces, glob-filtered and sorted. A pattern without a
/// leading `::` is qualified with the target namespace's FQN first (C's
/// `NamespaceChildrenCmd`).
///
/// # Errors
/// `namespace "<name>" not found` if `name` is given and does not resolve.
pub fn children<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&str>,
    pattern: Option<&str>,
) -> Result<O::Value, CmdError> {
    let ns = resolve_target(ops, name)?;
    let target_fqn = ops.name(ns);
    let qualified = pattern.map(|p| {
        if p.starts_with("::") {
            p.to_string()
        } else if target_fqn == "::" {
            format!("::{p}")
        } else {
            format!("{target_fqn}::{p}")
        }
    });
    // Collect child FQNs (the ids are owned, so the `name` lookups don't alias).
    let mut names: Vec<String> = Vec::new();
    for child in ops.children(ns) {
        names.push(ops.name(child));
    }
    names.retain(|n| qualified.as_deref().is_none_or(|p| string_match(p, n)));
    names.sort();
    let items: Vec<O::Value> = names.iter().map(|n| ops.new_str(n)).collect();
    Ok(ops.new_list(items))
}

/// Resolve the optional `name` argument to a namespace handle: the current
/// namespace when absent, else `name` resolved from it (erroring if it doesn't
/// exist) — the shared first step of `namespace parent`/`children`.
fn resolve_target<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&str>,
) -> Result<tcl_runtime_api::NsId, CmdError> {
    let cur = Namespaces::current(ops);
    match name {
        None => Ok(cur),
        Some(n) => ops
            .find_namespace(cur, n)
            .ok_or_else(|| CmdError::new(format!("namespace \"{n}\" not found"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifiers_and_tail_match_c() {
        assert_eq!(qualifiers(b"::foo::bar"), b"::foo");
        assert_eq!(tail(b"::foo::bar"), b"bar");
        // Unqualified: no qualifier, the whole name is the tail.
        assert_eq!(qualifiers(b"bar"), b"");
        assert_eq!(tail(b"bar"), b"bar");
        // Colon runs (3+) are a single separator (C semantics; the naive
        // `rsplit("::")` the VM used yielded `:` for the tail here).
        assert_eq!(qualifiers(b"foo:::"), b"foo");
        assert_eq!(tail(b"foo:::"), b"");
        assert_eq!(qualifiers(b"a::::b"), b"a");
        assert_eq!(tail(b"a::::b"), b"b");
        // A trailing simple separator.
        assert_eq!(qualifiers(b"::foo::"), b"::foo");
        assert_eq!(tail(b"::foo::"), b"");
        assert_eq!(qualifier(b"::lassign"), Qualifier::Absolute(b"::"));
        assert_eq!(qualifier(b"a:::b"), Qualifier::Relative(b"a"));
        assert_eq!(qualifier(b"::a:::b"), Qualifier::Absolute(b"::a"));
        assert_eq!(qualifier(b"foo:::"), Qualifier::Relative(b"foo"));
        assert_eq!(qualifier(b"b"), Qualifier::Unqualified);
    }

    #[test]
    fn imported_command_candidate_uses_tcl_globs_and_colon_runs() {
        assert_eq!(
            imported_command_candidate("::src:::im*", "image"),
            Some("::src::image".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::he*r", "helper"),
            Some("::src::helper".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::helpe?", "helper"),
            Some("::src::helper".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::[lp]*", "lookup"),
            Some("::src::lookup".to_owned())
        );
        assert_eq!(
            imported_command_candidate("foo:::", ""),
            Some("::foo::".to_owned())
        );
        assert_eq!(imported_command_candidate("::src::im*", "other"), None);
    }
}
