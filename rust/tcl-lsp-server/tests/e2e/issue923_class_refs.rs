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

//! Issue #923 — a fully-qualified class name used as a `superclass` / `mixin` /
//! `inherit` argument in another file is a first-class reference to the class.
//!
//! Before the fix, `superclass ::ns::Base` in one file was invisible to Find
//! All References on `::ns::Base` and was left dangling by rename (silent
//! corruption of the inheritance graph).  These TP/FP tests drive the packaged
//! server over real JSON-RPC across two or three open documents.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The reference/definition-target start lines that land in document `uri`.
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

// TP: `superclass ::ns::Base` in a sibling file is a reference, and rename
// rewrites it — the reported #923 shape (a class referenced by its
// fully-qualified name from another file).
#[test]
fn tp_superclass_cross_file_reference_and_rename() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "namespace eval ::ns {\n    oo::class create Base {}\n}\n",
    );
    let sub = unique_uri("tcl");
    lsp.open_ready(
        &sub,
        "namespace eval ::app {\n    oo::class create Sub {\n        superclass ::ns::Base\n    }\n}\n",
    );

    // Cursor on the `Base` declaration → references include the cross-file
    // superclass usage (line 2 of `sub`).
    let refs = lsp.references(&base, 1, 21, true);
    assert!(
        lines_in(&refs, &sub).contains(&2),
        "cross-file superclass must be a reference: {:?}",
        locations(&refs)
    );

    // Rename rewrites the declaration *and* the cross-file superclass site.
    let edits = rename_edits(&lsp.rename(&base, 1, 21, "Base2"));
    assert!(
        edits.contains_key(&base) && edits.get(&sub).is_some_and(|e| !e.is_empty()),
        "rename must rewrite the cross-file superclass site: {edits:?}"
    );
}

// TP: go-to-definition on the cross-file `superclass ::ns::Base` jumps to the
// real `oo::class create` site.
#[test]
fn tp_superclass_cross_file_go_to_definition() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "namespace eval ::ns {\n    oo::class create Base {}\n}\n",
    );
    let sub = unique_uri("tcl");
    lsp.open_ready(
        &sub,
        "namespace eval ::app {\n    oo::class create Sub {\n        superclass ::ns::Base\n    }\n}\n",
    );

    // Cursor on `::ns::Base` in the superclass line.
    let defs = lsp.definition(&sub, 2, 22);
    assert!(
        lines_in(&defs, &base).contains(&1),
        "definition must jump to the base's creation site: {:?}",
        locations(&defs)
    );
}

// FP guard: a same-named class in an unrelated namespace must not be
// cross-linked.  References on `::ns::Base` must not include the
// `superclass ::other::Base` site of a different class.
#[test]
fn fp_same_named_class_in_another_namespace_not_cross_linked() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "namespace eval ::ns {\n    oo::class create Base {}\n}\n",
    );
    let other = unique_uri("tcl");
    lsp.open_ready(
        &other,
        "namespace eval ::other {\n    oo::class create Base {}\n    oo::class create OtherSub {\n        superclass ::other::Base\n    }\n}\n",
    );
    let sub = unique_uri("tcl");
    lsp.open_ready(
        &sub,
        "namespace eval ::app {\n    oo::class create Sub {\n        superclass ::ns::Base\n    }\n}\n",
    );

    let refs = lsp.references(&base, 1, 21, true);
    assert!(
        lines_in(&refs, &sub).contains(&2),
        "the true cross-file superclass must be found: {:?}",
        locations(&refs)
    );
    assert!(
        lines_in(&refs, &other).is_empty(),
        "the same-named ::other::Base superclass must NOT be cross-linked: {:?}",
        locations(&refs)
    );
}

// TP: the inline `oo::define ::app::Sub superclass ::ns::Base` form (no
// `{body}` block) is a reference and renames cross-file — the shared recorder
// must cover the inline path too, not only the braced body walk.
#[test]
fn tp_inline_oo_define_superclass_cross_file_reference_and_rename() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "namespace eval ::ns {\n    oo::class create Base {}\n}\n",
    );
    let sub = unique_uri("tcl");
    lsp.open_ready(
        &sub,
        "namespace eval ::app {\n    oo::class create Sub {}\n}\noo::define ::app::Sub superclass ::ns::Base\n",
    );

    // The inline `superclass ::ns::Base` is on line 3 of `sub`.
    let refs = lsp.references(&base, 1, 21, true);
    assert!(
        lines_in(&refs, &sub).contains(&3),
        "inline oo::define superclass must be a reference: {:?}",
        locations(&refs)
    );

    let edits = rename_edits(&lsp.rename(&base, 1, 21, "Base2"));
    assert!(
        edits.get(&sub).is_some_and(|e| !e.is_empty()),
        "rename must rewrite the inline oo::define superclass site: {edits:?}"
    );
}

// TP: `mixin ::ns::Role` in a sibling file is a reference to the class.
#[test]
fn tp_mixin_cross_file_reference() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "namespace eval ::ns {\n    oo::class create Role {}\n}\n",
    );
    let sub = unique_uri("tcl");
    lsp.open_ready(
        &sub,
        "namespace eval ::app {\n    oo::class create C {\n        mixin ::ns::Role\n    }\n}\n",
    );

    let refs = lsp.references(&base, 1, 21, true);
    assert!(
        lines_in(&refs, &sub).contains(&2),
        "cross-file mixin must be a reference: {:?}",
        locations(&refs)
    );
}

// TP (itcl): `inherit ::ns::Base` in a sibling file is a reference, the same as
// TclOO's `superclass`.
#[test]
fn tp_itcl_inherit_cross_file_reference() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    lsp.open_ready(&base, "itcl::class ::ns::Base {}\n");
    let sub = unique_uri("tcl");
    lsp.open_ready(&sub, "itcl::class ::ns::Sub {\n    inherit ::ns::Base\n}\n");

    // Cursor on `Base` in the itcl class declaration name `::ns::Base`.
    let refs = lsp.references(&base, 0, 18, true);
    assert!(
        lines_in(&refs, &sub).contains(&1),
        "cross-file itcl inherit must be a reference: {:?}",
        locations(&refs)
    );
}
