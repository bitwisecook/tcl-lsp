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

//! Command resolution order: lookup bases, the candidate qualified names and `resolve_command_with`.

use super::constructed_keys::canonical_written_command;
use super::qualification::normalise_qualified_name;
use super::written_names::qualifier_segments_owned;

/// A command lookup base supplied by a consumer's context owner.
/// Coordinates carry no namespace liveness or binding evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLookupBase<T> {
    /// The current namespace or an ordered namespace-path entry.
    Context(T),
    /// The interpreter's actual global namespace, resolved when reached.
    Root,
}

/// Visit current/path/root bases for a relative command, or root alone for
/// an absolute command. Consumers project their own coordinate representation.
pub fn command_lookup_bases<T>(
    absolute: bool,
    current: T,
    path: impl IntoIterator<Item = T>,
) -> impl Iterator<Item = CommandLookupBase<T>> {
    let mut contexts = std::iter::once(current).chain(path).fuse();
    let mut root_pending = true;
    std::iter::from_fn(move || {
        if !absolute && let Some(context) = contexts.next() {
            return Some(CommandLookupBase::Context(context));
        }
        std::mem::take(&mut root_pending).then_some(CommandLookupBase::Root)
    })
}

/// Candidate qualified names for Tcl's real bareword command/procedure
/// resolution, in priority order: the current namespace first, then global.
///
/// The `namespace path`-free specialisation of
/// [`command_resolution_candidates`] — see there for the full rule.  Kept
/// as the common entry point for consumers that do not model
/// `namespace path` (the static analyser, the optimiser's interprocedural
/// proc-identity resolution).
///
/// ```
/// use tcl_syntax::naming::bareword_resolution_candidates;
/// assert_eq!(bareword_resolution_candidates("::ns", "::foo"), vec!["::foo"]);
/// assert_eq!(
///     bareword_resolution_candidates("::ns", "inner::p"),
///     vec!["::ns::inner::p", "::inner::p"],
/// );
/// assert_eq!(bareword_resolution_candidates("::ns", "foo"), vec!["::ns::foo", "::foo"]);
/// assert_eq!(bareword_resolution_candidates("::", "foo"), vec!["::foo"]);
/// ```
#[must_use]
pub fn bareword_resolution_candidates(namespace: &str, cmd_name: &str) -> Vec<String> {
    command_resolution_candidates::<&str>(namespace, &[], cmd_name)
}

/// Candidate qualified names for Tcl's command resolution, in priority
/// order, including the current namespace's `namespace path`.
///
/// This is the **canonical encoding of C Tcl's command-lookup order**
/// (`Tcl_FindCommand`, `generic/tclNamesp.c`, Tcl 9.0.4) — the algorithm
/// every backend must agree with (behaviour pinned against tclsh 8.6.16
/// and 9.0.4 by `tests/command_resolution_conformance.rs`):
///
/// * An absolute name (`::foo`, `::ns::foo`) is taken as-is — one
///   candidate; the path is **not** consulted.
/// * Any relative name — bare (`foo`) *or* with embedded qualifiers
///   (`inner::p`) — is resolved against the current namespace first, then
///   each `namespace path` entry in order, then the global namespace.
///   The dispatch target is the first candidate that **exists**; mere
///   existence of an intermediate *namespace* does not commit resolution
///   (calling `inner::p` from `::outer` reaches `::inner::p` even when
///   the namespace `::outer::inner` exists but holds no `p`).
/// * No implicit ancestor walk: a bare `helper` inside `::a::b` never
///   reaches `::a::helper` unless `::a` is on the `namespace path`.
///
/// `path` entries may be given rooted (`::pathns`) or as written
/// (`pathns`): a *relative* entry is **current-namespace-relative only**
/// — `namespace path inner` inside `::outer` means `::outer::inner`,
/// never `::inner` (tclsh 8.6.16 / 9.0.4: the set errors `namespace
/// "inner" not found in "::outer"` when `::outer::inner` does not exist,
/// even with `::inner` present — namespace names have no global
/// fallback, unlike command names).  Consumers that do not model
/// `namespace path` pass `&[]` (equivalently, use
/// [`bareword_resolution_candidates`]).
///
/// ```
/// use tcl_syntax::naming::command_resolution_candidates;
/// // path slots between the current namespace and global:
/// assert_eq!(
///     command_resolution_candidates("::c", &["::pathns"], "helper"),
///     vec!["::c::helper", "::pathns::helper", "::helper"],
/// );
/// // a relative path entry is current-namespace-relative (tclsh-pinned):
/// assert_eq!(
///     command_resolution_candidates("::d", &["pathq"], "sub::q"),
///     vec!["::d::sub::q", "::d::pathq::sub::q", "::sub::q"],
/// );
/// // absolute names ignore the path entirely:
/// assert_eq!(
///     command_resolution_candidates("::d", &["::pathq"], "::sub::q"),
///     vec!["::sub::q"],
/// );
/// // duplicates collapse, keeping the highest-priority position:
/// assert_eq!(
///     command_resolution_candidates("::", &["::"], "foo"),
///     vec!["::foo"],
/// );
/// ```
#[must_use]
pub fn command_resolution_candidates<S: AsRef<str>>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
) -> Vec<String> {
    let absolute = cmd_name.starts_with("::");
    let canonical_cmd = canonical_written_command(cmd_name);
    let selected_path = path.iter().map(|entry| {
        let entry = entry.as_ref();
        if entry.starts_with("::") {
            normalise_qualified_name(entry)
        } else {
            let canonical_entry = qualifier_segments_owned(entry).join("::");
            if namespace.is_empty() || namespace == "::" {
                format!("::{canonical_entry}")
            } else if namespace.starts_with("::") {
                format!("{namespace}::{canonical_entry}")
            } else {
                format!("::{namespace}::{canonical_entry}")
            }
        }
    });
    let mut out = Vec::with_capacity(path.len() + 2);
    for base in command_lookup_bases(absolute, namespace.to_owned(), selected_path) {
        let candidate = match base {
            CommandLookupBase::Root if absolute => canonical_cmd.clone(),
            CommandLookupBase::Root => format!("::{canonical_cmd}"),
            CommandLookupBase::Context(base) if base.is_empty() || base == "::" => {
                format!("::{canonical_cmd}")
            }
            CommandLookupBase::Context(base) if base.starts_with("::") => {
                format!("{base}::{canonical_cmd}")
            }
            CommandLookupBase::Context(base) => format!("::{base}::{canonical_cmd}"),
        };
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

/// Ordered command candidates using retained constructed namespace keys.
///
/// The command operand is written syntax; current and path namespaces are
/// already selected contexts. Literal-colon components in those keys remain
/// data. A candidate key does not certify a globally callable source spelling.
#[must_use]
pub fn command_resolution_candidates_from_namespace_keys<S: AsRef<str>>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
) -> Vec<String> {
    let absolute = cmd_name.starts_with("::");
    let command = canonical_written_command(cmd_name);
    let mut candidates = Vec::with_capacity(path.len() + 2);
    for base in command_lookup_bases(absolute, namespace, path.iter().map(AsRef::as_ref)) {
        let key = match base {
            CommandLookupBase::Root if absolute => command.clone(),
            CommandLookupBase::Root => format!("::{command}"),
            CommandLookupBase::Context(context) if context.is_empty() || context == "::" => {
                format!("::{command}")
            }
            CommandLookupBase::Context(context) if context.starts_with("::") => {
                format!("{context}::{command}")
            }
            CommandLookupBase::Context(context) => format!("::{context}::{command}"),
        };
        if !candidates.contains(&key) {
            candidates.push(key);
        }
    }
    candidates
}

/// Resolve a command name the way C Tcl's `Tcl_FindCommand` does: walk
/// [`command_resolution_candidates`] in priority order and return the
/// first candidate for which `exists` is true.
///
/// `exists` is the caller's command table — the analyser's collected
/// definitions, a compilation unit's proc map, or a live interpreter's
/// registry.  Returns `None` when no candidate exists (Tcl would raise
/// `invalid command name` / fall through to `unknown`).
///
/// ```
/// use tcl_syntax::naming::resolve_command_with;
/// let defined = ["::inner::p"];
/// let exists = |q: &str| defined.contains(&q);
/// // local candidate absent -> falls back to the global command:
/// assert_eq!(
///     resolve_command_with::<&str, _>("::outer", &[], "inner::p", exists),
///     Some("::inner::p".to_string()),
/// );
/// // nothing defined -> unresolved:
/// assert_eq!(resolve_command_with::<&str, _>("::outer", &[], "nope", exists), None);
/// ```
#[must_use]
pub fn resolve_command_with<S: AsRef<str>, F: FnMut(&str) -> bool>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
    mut exists: F,
) -> Option<String> {
    command_resolution_candidates(namespace, path, cmd_name)
        .into_iter()
        .find(|candidate| exists(candidate))
}
