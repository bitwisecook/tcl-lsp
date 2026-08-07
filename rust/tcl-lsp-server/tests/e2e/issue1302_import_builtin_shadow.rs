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

//! Issue #1302, end-to-end: a `namespace import` cannot bind a name the
//! **registry** says a fresh interpreter already holds, so a bare call to
//! such a name is not a reference to the exported proc.
//!
//! The unit-level matrix lives in `tcl_lsp_core::workspace_index`; this is the
//! same question asked through the real server on the requests a user
//! actually issues — `textDocument/references` and `textDocument/definition`
//! — because the defect's blast radius is find-references and the reference
//! count code lenses, not the index in the abstract.
//!
//! Every expectation is oracle-verified on tclsh 9.0.4 (built from
//! `core-9-0-4`) and 8.6.14, byte-identical. Each test records the transcript
//! it was written against.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The reference start lines that land in document `uri`.
fn lines_in(result: &Value, uri: &str) -> Vec<i64> {
    locations(result)
        .iter()
        .filter(|l| l.uri == uri)
        .map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                .unwrap_or(-1)
        })
        .collect()
}

/// TP — the defect itself. `::Foo` exports `set`; the unforced import is
/// refused, so `set x 1` reaches the builtin and is **not** a reference to
/// `::Foo::set`.
///
/// Oracle:
/// ```text
/// namespace eval ::Foo { proc set {a b} {return SHADOW}; namespace export set }
/// namespace import ::Foo::*   -> can't import command "set": already exists
/// namespace origin ::set      -> ::set
/// ```
#[test]
fn tp_a_bare_builtin_call_is_not_a_reference_to_an_exported_shadow() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace import ::Foo::*\nset x 1\n");

    // Cursor on the `set` of `proc set` in the library document.
    let refs = lsp.references(&lib, 1, 10, false);
    assert!(
        lines_in(&refs, &user).is_empty(),
        "the import raised `already exists` and installed nothing, so the bare \
         `set x 1` must not be filed as a reference: {:?}",
        locations(&refs),
    );
}

/// TN — `-force` is the one import that *does* replace a command the target
/// namespace already holds, so there the call really is a reference.
///
/// Oracle: `namespace import -force ::Foo::*` then
/// `namespace origin ::set` -> `::Foo::set`.
#[test]
fn tn_a_forced_import_does_shadow_the_builtin_and_the_call_is_a_reference() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace import -force ::Foo::*\nset x 1\n");

    let refs = lsp.references(&lib, 1, 10, false);
    assert_eq!(
        lines_in(&refs, &user),
        vec![1],
        "`-force` replaces the builtin, so the bare call is a real reference: {:?}",
        locations(&refs),
    );
}

/// FP guard — the conflict is about the **target** namespace's command table,
/// and the builtins live in `::`. Importing into `::Bar` binds `::Bar::set`
/// with no conflict at all, so the call from `::Bar` must still resolve.
///
/// Oracle: `namespace eval ::Bar { namespace import ::Foo::* }` then
/// `namespace origin ::Bar::set` -> `::Foo::set`.
#[test]
fn fp_guard_importing_a_builtin_name_into_a_sub_namespace_still_resolves() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(
        &user,
        "namespace eval ::Bar {\n    namespace import ::Foo::*\n    proc go {} { set x 1 }\n}\n",
    );

    let refs = lsp.references(&lib, 1, 10, false);
    assert_eq!(
        lines_in(&refs, &user),
        vec![2],
        "`::Bar::set` is not a builtin, so the import installs and the call \
         from ::Bar is a reference: {:?}",
        locations(&refs),
    );
}

/// FN guard for the operator spellings — the case a naive
/// `registry.get(name).is_some()` gate would break. `+` is **not** a command
/// in `::` at all, so exporting `+` and importing it into `::` genuinely
/// binds.
///
/// Oracle:
/// ```text
/// info commands ::+   -> {}          (and `+ 1 2` -> invalid command name "+")
/// namespace eval ::Ops { proc + {a b} {return OPS}; namespace export + }
/// namespace import ::Ops::*
/// namespace origin ::+  -> ::Ops::+
/// ```
#[test]
fn fn_guard_an_operator_name_still_binds_into_the_global_namespace() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Ops {\n    proc + {a b} { return OPS }\n    namespace export +\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace import ::Ops::*\n+ 1 2\n");

    // Column 9 is the `+` itself — the name is one character wide, unlike the
    // `set` / `mything` cases above where column 10 still lands inside it.
    let refs = lsp.references(&lib, 1, 9, false);
    assert_eq!(
        lines_in(&refs, &user),
        vec![1],
        "a bare operator spelling is the post-`namespace import ::tcl::mathop::*` \
         form, not a command a fresh interpreter holds: {:?}",
        locations(&refs),
    );
}

/// TN — an ordinary, non-builtin exported name is unaffected. Guards against
/// the gate having simply made the whole wildcard tier answer "no".
#[test]
fn tn_a_non_builtin_exported_name_still_resolves_through_the_import() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Foo {\n    proc mything {} {}\n    namespace export mything\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace import ::Foo::*\nmything\n");

    let refs = lsp.references(&lib, 1, 10, false);
    assert_eq!(
        lines_in(&refs, &user),
        vec![1],
        "an ordinary exported name must still resolve: {:?}",
        locations(&refs),
    );
}

/// The exact-pattern twin: `namespace import ::Foo::set` is refused for the
/// identical reason, so go-to-definition from the bare call must not land on
/// the shadow either.
///
/// Oracle: `namespace import ::Foo::set` -> `can't import command "set":
/// already exists`.
#[test]
fn tp_an_exact_import_of_a_builtin_name_installs_no_link() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace import ::Foo::set\nset x 1\n");

    let refs = lsp.references(&lib, 1, 10, false);
    assert!(
        lines_in(&refs, &user).is_empty(),
        "an exact import of an already-existing name installs nothing either: {:?}",
        locations(&refs),
    );
}
