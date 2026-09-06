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

//! Issue #923 differential audit — cross-document resolution tiers
//! (findings idx 19 / 65 / 72 / 73 / 75 / 78 / 80).
//!
//! Two gaps, both "the answer is in another file":
//!
//! * a **namespace variable** (`$::ns::v`) whose declaring `namespace eval ns {
//!   variable v }` lives in a sibling document — definition, hover, and
//!   find-references all silently answered nothing (idx 65 / 75 / 78);
//! * a **class-reference argument** (`superclass ::ns::Base`) naming a class
//!   defined in a sibling document, through the split / nested `namespace eval`
//!   shape `SpiceGenTcl` uses (idx 19).
//!
//! Every positive case here is one tclsh 9.0.4 and 8.6.16 both run; each test
//! records the oracle output it was written against.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The definition/reference target start lines that land in document `uri`.
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

// TP (idx 75 / 78): `$::mypkg::version` resolves to the `variable version`
// declaration in the sibling document that opens the namespace.
//
// Oracle — tclsh 9.0.4 and 8.6.16, sourcing both files: `version : 1.0`.
#[test]
fn tp_cross_file_namespace_variable_definition() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n    variable version 1.0\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let def = lsp.definition(&user, 0, 20);
    assert_eq!(
        lines_in(&def, &decl),
        vec![1],
        "the declaring document's `variable version` must be the target: {:?}",
        locations(&def),
    );
}

// TP (idx 65): the same, with a *relative* qualifier on both sides —
// `namespace eval app::colors` declaring it, `$app::colors::palette` reading
// it — and find-references reaching the consumer from the declaration.
//
// Oracle — tclsh 9.0.4 and 8.6.16: `palette-> red green blue`, and `namespace
// which -variable` confirms both files address the one real
// `::app::colors::palette`.
#[test]
fn tp_cross_file_namespace_variable_references_from_declaration() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval app::colors {\n    variable palette {red green blue}\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $app::colors::palette\n");

    let refs = lsp.references(&decl, 1, 13, true);
    assert!(
        lines_in(&refs, &decl).contains(&1),
        "the declaration itself must be included: {:?}",
        locations(&refs),
    );
    assert!(
        lines_in(&refs, &user).contains(&0),
        "the sibling document's read must be a reference: {:?}",
        locations(&refs),
    );
}

// TP (idx 78): hover on a cross-file namespace variable names the cell it
// found, rather than showing nothing at all.
#[test]
fn tp_cross_file_namespace_variable_hover() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval tomato::helper {\n    variable TolEquals 1e-8\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "set t $::tomato::helper::TolEquals\n");

    let text = hover_text(&lsp.hover(&user, 0, 30));
    assert!(
        text.contains("::tomato::helper::TolEquals"),
        "hover must name the resolved cell, got: {text:?}",
    );
}

// TN (idx 65 / 75 / 78): an **unqualified** variable read must not be widened
// to a same-named namespace variable in another file — a bare `$palette` is
// whatever the local scope chain supplies, which no sibling document can know.
#[test]
fn tn_unqualified_variable_does_not_resolve_cross_file() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval app::colors {\n    variable palette {red green blue}\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "proc show {} {\n    puts $palette\n}\n");

    let def = lsp.definition(&user, 1, 12);
    assert!(
        lines_in(&def, &decl).is_empty(),
        "a bare `$palette` must not link to another file's namespace variable: {:?}",
        locations(&def),
    );
}

// TN: a `$ns::var`-shaped substring inside a **comment** substitutes nothing in
// real Tcl, so it must not resolve cross-file either.
#[test]
fn tn_commented_qualified_variable_does_not_resolve_cross_file() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n    variable version 1.0\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "# reads $::mypkg::version here\nputs ok\n");

    let def = lsp.definition(&user, 0, 20);
    assert!(
        lines_in(&def, &decl).is_empty(),
        "a comment is not a reference: {:?}",
        locations(&def),
    );
}

// TP (idx 19): a fully-qualified `superclass` argument naming a class created
// inside a *separately reopened, nested* `namespace eval` in another document
// resolves — definition, references from the declaration, and hover.
//
// Oracle — tclsh 9.0.4 and 8.6.16 on this exact pair: prints `42`.
#[test]
fn tp_cross_file_superclass_through_split_nested_namespace() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Outer {\n\
         \x20   namespace eval Inner::Sub {\n\
         \x20       namespace export Foo\n\
         \x20   }\n\
         }\n\
         namespace eval ::Outer::Inner::Sub {\n\
         \x20   oo::class create Foo {\n\
         \x20       method show {} { return 42 }\n\
         \x20   }\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(
        &user,
        "namespace eval ::Derived {\n\
         \x20   oo::class create Baz {\n\
         \x20       superclass ::Outer::Inner::Sub::Foo\n\
         \x20   }\n\
         }\n",
    );

    let def = lsp.definition(&user, 2, 40);
    assert_eq!(
        lines_in(&def, &lib),
        vec![6],
        "the superclass argument must reach the class creation site: {:?}",
        locations(&def),
    );
    let refs = lsp.references(&lib, 6, 21, true);
    assert!(
        lines_in(&refs, &user).contains(&2),
        "the cross-file superclass argument must be a reference: {:?}",
        locations(&refs),
    );
    assert!(
        hover_text(&lsp.hover(&user, 2, 40)).contains("::Outer::Inner::Sub::Foo"),
        "hover on the superclass argument must describe the class",
    );
}

// TN (idx 19): a word that merely *looks* like the class name but sits in an
// ordinary data argument is not a class reference.
#[test]
fn tn_plain_argument_word_does_not_resolve_to_a_sibling_class() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Outer::Inner::Sub {\n\
         \x20   oo::class create Foo {}\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts ::Outer::Inner::Sub::Foo\n");

    let def = lsp.definition(&user, 0, 25);
    assert!(
        lines_in(&def, &lib).is_empty(),
        "a `puts` argument is data, not a class reference: {:?}",
        locations(&def),
    );
}

// TP (idx 44): an absolute command head inside a TclOO constructor reaches a
// proc declared in a **sibling document**. Absolute spelling is essential:
// Tcl 9 resolves relative method-body heads in the receiver namespace.
//
// The same mechanic is already pinned single-file (issue #1132), but idx 44's
// real corpus (`ticklecharts`) splits it across files — the utility proc in
// `utils.tcl`, the class in `dataset.tcl` — which is the tier a
// workspace-index regression would break without touching the single-file
// tests at all.
//
// Oracle (tclsh 9.0.4 and 8.6.16, sourcing both files): prints
// `resolved options dict = id {-default hello}`; this projection pins the
// resulting absolute `::ticklecharts::setdef` identity across documents.
#[test]
fn tp_cross_file_rooted_namespace_head_reaches_its_proc_923_idx44() {
    let mut lsp = Lsp::tcl();
    let utils = unique_uri("tcl");
    lsp.open_ready(
        &utils,
        "namespace eval ticklecharts {}\n\
         proc ticklecharts::setdef {d key args} {\n\
         \x20   upvar 1 $d dict\n\
         \x20   dict set dict $key $args\n\
         \x20   return [dict get $dict]\n\
         }\n",
    );
    let dataset = unique_uri("tcl");
    lsp.open_ready(
        &dataset,
        "oo::class create ticklecharts::dataset {\n\
         \x20   variable options\n\
         \x20   constructor {value} {\n\
         \x20       ::ticklecharts::setdef options id -default $value\n\
         \x20   }\n\
         }\n",
    );

    // Go-to-definition from the constructor's rooted `setdef` head (line 3).
    let def = lsp.definition(&dataset, 3, 17);
    assert_eq!(
        lines_in(&def, &utils),
        vec![1],
        "the rooted `setdef` head must reach the sibling document's proc: {:?}",
        locations(&def),
    );

    // And the reverse direction: references from that declaration reach the
    // constructor's call site.
    let refs = lsp.references(&utils, 1, 20, true);
    assert!(
        !lines_in(&refs, &dataset).is_empty(),
        "find-references from the declaration must reach the cross-file \
         constructor call site: {:?}",
        locations(&refs),
    );
}

// TP (idx 19), *unit-level* companion to
// `tp_cross_file_superclass_through_split_nested_namespace`: the split /
// nested `namespace eval ::Outer { namespace eval Inner::Sub { … } }` shape
// must give the class its full `::Outer::Inner::Sub::Foo` qualified name in a
// single document too — the fact the cross-document tier consumes. A
// regression that mis-homed the class would break every cross-file query
// built on it, and this pins the cause rather than one symptom.
//
// Oracle (tclsh 9.0.4 / 8.6.16): the two-file program prints `42`, so
// `::Outer::Inner::Sub::Foo` is the real command name.
#[test]
fn tp_split_nested_namespace_eval_homes_the_class_fully_923_idx19() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::Outer {\n\
         \x20   namespace eval Inner::Sub {\n\
         \x20       namespace export Foo\n\
         \x20   }\n\
         }\n\
         namespace eval ::Outer::Inner::Sub {\n\
         \x20   oo::class create Foo {\n\
         \x20       method show {} { return 1 }\n\
         \x20   }\n\
         }\n\
         oo::class create Baz {\n\
         \x20   superclass ::Outer::Inner::Sub::Foo\n\
         }\n",
    );
    // `superclass ::Outer::Inner::Sub::Foo` (line 11): the argument starts at
    // column 15, so its `Foo` tail sits at column 36.
    let def = lsp.definition(&uri, 11, 37);
    assert_eq!(
        lines_in(&def, &uri),
        vec![6],
        "the superclass argument must reach the class the split/nested \
         `namespace eval` declares: {:?}",
        locations(&def),
    );
    // Hover names the fully-qualified command, which is what proves the
    // homing rather than a same-tail-name coincidence.
    let hover = crate::common::helpers::hover_text(&lsp.hover(&uri, 6, 22));
    assert!(
        hover.contains("::Outer::Inner::Sub::Foo"),
        "hover must name the fully-qualified class: {hover:?}",
    );
}
