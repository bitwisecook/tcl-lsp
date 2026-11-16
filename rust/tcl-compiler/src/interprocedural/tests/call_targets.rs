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

//! Resolving a command name to a procedure of the module across namespaces.

use super::*;

fn known_set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn resolve_absolute_names() {
    let known = known_set(&["::foo::bar"]);
    assert_eq!(
        resolve_internal_call("::foo::bar", "::top", &known),
        Some("::foo::bar".into())
    );
    // Absolute name not in the known set returns None.
    assert_eq!(
        resolve_internal_call("::foo::missing", "::top", &known),
        None
    );
}

#[test]
fn resolve_relative_with_segments() {
    // `foo::bar` from a global-level caller → `::foo::bar` (the only
    // candidate, since the caller's own namespace is already `::`).
    let known = known_set(&["::foo::bar"]);
    assert_eq!(
        resolve_internal_call("foo::bar", "::top", &known),
        Some("::foo::bar".into())
    );
}

#[test]
fn resolve_relative_dotted_name_prefers_caller_namespace() {
    // A relative dotted word (`ns2::inner`) resolves against the
    // caller's own namespace first, not straight at global (confirmed
    // against tclsh 9.0.4 — see `bareword_resolution_candidates`).
    let known = known_set(&["::ns::ns2::inner", "::ns2::inner"]);
    assert_eq!(
        resolve_internal_call("ns2::inner", "::ns::caller", &known),
        Some("::ns::ns2::inner".into()),
        "must prefer the caller-namespace proc over the root one",
    );
    // No caller-namespace candidate → falls back to the root proc.
    assert_eq!(
        resolve_internal_call("ns2::inner", "::top", &known),
        Some("::ns2::inner".into()),
    );
}

#[test]
fn resolve_bare_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — the
    // caller's own namespace, then global — absent an explicit
    // `namespace path`. A proc defined in a *grandparent* namespace
    // must not resolve for a bare call from `::ns::a::caller`.
    let known = known_set(&["::ns::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::a::caller", &known),
        None,
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: the *direct* enclosing namespace still resolves.
    let known2 = known_set(&["::ns::a::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::a::caller", &known2),
        Some("::ns::a::helper".into()),
    );
}

#[test]
fn resolve_bare_falls_through_to_global() {
    let known = known_set(&["::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::caller", &known),
        Some("::helper".into())
    );
}

#[test]
fn resolve_bare_returns_none_when_not_found() {
    let known = known_set(&["::other"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::caller", &known),
        None
    );
}

#[test]
fn resolve_empty_command_is_none() {
    let known = known_set(&["::helper"]);
    assert_eq!(resolve_internal_call("", "::top", &known), None);
}

#[test]
fn namespace_parts_from_proc_extracts_segments() {
    assert_eq!(
        namespace_parts_from_proc("::foo::bar::baz"),
        vec!["foo", "bar"]
    );
    assert_eq!(namespace_parts_from_proc("::simple"), Vec::<String>::new());
    assert_eq!(namespace_parts_from_proc("::"), Vec::<String>::new());
    // Colon runs are one separator (C's `TclGetNamespaceForQualName`;
    // tclsh8.6: `foo:::bar` names `::foo::bar`) — the shared
    // `qualifier_segments` split, where a naive `split("::")` would keep
    // a stray `:` segment.
    assert_eq!(
        namespace_parts_from_proc("::foo:::bar::baz"),
        vec!["foo", "bar"]
    );
    assert_eq!(namespace_parts_from_proc("a:::b"), vec!["a"]);
}

#[test]
fn resolve_call_target_delegates() {
    let known = known_set(&["::helper"]);
    assert_eq!(
        resolve_call_target("helper", &[], "::top", &known),
        Some("::helper".into())
    );
}
