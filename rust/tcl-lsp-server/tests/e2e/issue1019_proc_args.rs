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

//! Issue #1019 differential audit, `proc_args` group — the end-to-end tier.
//!
//! Findings idx 11 / 28 / 37 / 62 / 67 / 104.  Each of these was already
//! fixed in the providers and covered by unit tests, but had no test at the
//! wire tier — or, for idx 28, was fixed only for the single-file shape while
//! the corpus shape is cross-file.  Every claim below is pinned against
//! tclsh 9.0.4 and 8.6.16, which agree byte-for-byte; each test records the
//! transcript it was written against.

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

/// Every diagnostic code in `diags`, in wire order.
fn codes(diags: &[Value]) -> Vec<String> {
    diags
        .iter()
        .map(|d| {
            d.get("code")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

// -- idx 11: a proc named after a package-gated registry command ---------

// FP guard.  georgtree/argparse's own implementation file defines `proc
// ::argparse {args}` — the entry point of the `argparse` *package*, which the
// registry also models as a command.  Neither W113 ("shadows built-in
// command") nor W120 ("requires `package require argparse`") describes
// anything real here: the proc *is* the package's definition, and nothing
// needs loading to make the name exist.
//
// Oracle, tclsh 9.0.4 and 8.6.16 identically: `info commands argparse` is
// empty in a bare interpreter and a bare `argparse` errors `invalid command
// name "argparse"` — so it is not a built-in — while sourcing the file and
// calling `argparse` with no arguments prints `case0`.
#[test]
fn fp_a_proc_named_after_a_package_gated_command_is_not_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc ::argparse {args} {\n\
         \x20   if {[llength $args] == 0} { return case0 }\n\
         \x20   return [lindex $args 0]\n\
         }\n\
         puts [argparse]\n",
    );
    let seen = codes(&diags);
    assert!(
        !seen.iter().any(|c| c == "W113"),
        "the proc defines the package's own command; it shadows no built-in: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c == "W120"),
        "the file defines `argparse` itself, so no `package require` is missing: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c == "E002" || c == "E003"),
        "the 0-argument call runs (`case0`); no arity error may fire: {seen:?}"
    );
}

// FP guard, the side discovery the verification pass found: the *consumer*
// half.  A file that `source`s the definition above and calls `argparse` with
// no arguments got a hard `E002 Too few arguments for 'argparse': expected at
// least 1, got 0` — resolved against the *package's* spec, which this
// workspace never loads.
//
// Oracle (9.0.4 / 8.6.16, running the pair): `case0`.  The call is correct.
//
// The analyser is single-file by construction, so it cannot see the sourced
// definition; what it *can* see is that the package is not required here, and
// a count assertion against a spec whose command is not proven to be the one
// being called is exactly the false positive.  W120 still reports (as a
// warning, and the workspace tier may retract it) — the arity error must not.
#[test]
fn fp_no_arity_error_for_a_package_command_the_document_never_loads() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "source helper.tcl\nputs [argparse]\n");
    let seen = codes(&diags);
    assert!(
        !seen.iter().any(|c| c == "E002" || c == "E003"),
        "no arity verdict may be asserted against an unloaded package's spec: {seen:?}"
    );
}

// TP control for both guards above: with the package actually required, the
// identity *is* proven, and a genuinely bad call is still an error.
//
// Oracle (9.0.4 / 8.6.16, with tcllib's argparse installed): `argparse`
// with no arguments raises `wrong # args`.
#[test]
fn tp_arity_still_fires_for_a_package_command_the_document_requires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "package require argparse\nputs [argparse]\n");
    let seen = codes(&diags);
    assert!(
        seen.iter().any(|c| c == "E002"),
        "a required package's spec is the command being called — arity must \
         still be checked: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c == "W120"),
        "the package is required, so W120 must not fire: {seen:?}"
    );
}

// TP control for the W113 half: redefining a real core built-in still warns.
// It is the *package-gated* exclusion that idx 11 added, not a blanket one.
#[test]
fn tp_redefining_a_core_builtin_still_fires_w113() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc lindex {a b} { return $a }\n");
    assert!(
        codes(&diags).iter().any(|c| c == "W113"),
        "`lindex` is a core built-in: {:?}",
        codes(&diags)
    );
}

// -- idx 28: cross-file mixin / inherited method hover -------------------

// TP.  SpiceGenTcl's shape: `Utility` (the mixin) and `Model` live in the
// library file, `RModel` in the device file, and `RModel`'s constructor calls
// `my ArgsPreprocess` — a two-hop `mixin` + `superclass` MRO dispatch that
// crosses documents.
//
// Oracle (9.0.4 / 8.6.16, sourcing both files and instantiating `RModel`):
// `ArgsPreprocess dispatched via class: ::Demo::Utility` — the call really
// does reach `Utility`'s method.
//
// Go-to-definition already crossed the boundary; hover answered nothing at
// all, because the in-document hover provider only knows the classes this
// document declares.
#[test]
fn tp_cross_file_mixin_method_hover() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Demo {\n\
         \x20   oo::class create Utility {\n\
         \x20       method ArgsPreprocess {switches params supp args} { return ok }\n\
         \x20   }\n\
         \x20   oo::class create Model {\n\
         \x20       mixin Utility\n\
         \x20   }\n\
         }\n",
    );
    let dev = unique_uri("tcl");
    lsp.open_ready(
        &dev,
        "namespace eval ::Demo::Xyce {\n\
         \x20   oo::class create RModel {\n\
         \x20       superclass ::Demo::Model\n\
         \x20       constructor {args} {\n\
         \x20           my ArgsPreprocess {defw r} {name type} type {*}$args\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );

    // Cursor on `ArgsPreprocess` in the `my` dispatch.
    let text = hover_text(&lsp.hover(&dev, 4, 20));
    assert!(
        text.contains("ArgsPreprocess"),
        "hover must name the method it dispatches to, got {text:?}"
    );
    assert!(
        text.contains("::Demo::Utility"),
        "hover must name the *providing* class from the sibling document, got {text:?}"
    );
    assert!(
        text.contains("inherited from"),
        "the MRO note is the fact that makes the answer useful, got {text:?}"
    );
    // The hover and the definition jump must agree about the provider.
    let def = lsp.definition(&dev, 4, 20);
    assert_eq!(
        lines_in(&def, &lib),
        vec![2],
        "definition must land on `Utility`'s declaration: {:?}",
        locations(&def)
    );
}

// TN control: a `my` dispatch of a method no class in the chain provides has
// no cross-file answer to give.  Without this, "hover something, anything"
// would pass the test above.
#[test]
fn tn_cross_file_hover_abstains_for_a_method_no_class_provides() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::Demo {\n\
         \x20   oo::class create Utility {\n\
         \x20       method ArgsPreprocess {a} { return ok }\n\
         \x20   }\n\
         \x20   oo::class create Model {\n\
         \x20       mixin Utility\n\
         \x20   }\n\
         }\n",
    );
    let dev = unique_uri("tcl");
    lsp.open_ready(
        &dev,
        "namespace eval ::Demo::Xyce {\n\
         \x20   oo::class create RModel {\n\
         \x20       superclass ::Demo::Model\n\
         \x20       constructor {args} {\n\
         \x20           my NoSuchMember\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    let text = hover_text(&lsp.hover(&dev, 4, 20));
    assert!(
        !text.contains("ArgsPreprocess"),
        "an unprovided member must not borrow a sibling's hover, got {text:?}"
    );
}

// -- idx 37: `next` inside a constructor ---------------------------------

// TP, the definition half (the arity half already has an e2e test).
// `next` in a constructor dispatches to the superclass's *constructor*, which
// is not in the class's method table at all — so it used to resolve to
// nothing while the identical `next` in an ordinary method resolved fine.
//
// Oracle (9.0.4 / 8.6.16): `Base ctor n=5` then `Derived describe` — the
// constructor `next` really does run `Base`'s constructor.
#[test]
fn tp_next_in_a_constructor_resolves_to_the_superclass_constructor() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Base {\n\
         \x20   constructor {n} { puts \"Base ctor n=$n\" }\n\
         \x20   method describe {} { return Base }\n\
         }\n\
         oo::class create Derived {\n\
         \x20   superclass Base\n\
         \x20   constructor {n} { next $n }\n\
         \x20   method describe {} { return [next] }\n\
         }\n",
    );
    // Cursor on the `next` inside `Derived`'s constructor.
    let ctor = lsp.definition(&uri, 6, 23);
    assert_eq!(
        lines_in(&ctor, &uri),
        vec![1],
        "`next` in a constructor must resolve to `Base`'s constructor: {:?}",
        locations(&ctor)
    );
    // Control: the ordinary-method `next` on the same class still resolves to
    // the method, so the two forms are symmetric.
    let method = lsp.definition(&uri, 7, 34);
    assert_eq!(
        lines_in(&method, &uri),
        vec![2],
        "`next` in a method must resolve to `Base`'s method: {:?}",
        locations(&method)
    );
}

// -- idx 62: a bare `[list procName ...]` command-prefix callback --------

// TP.  ticklecharts' shape: a `trace add variable` write callback built with
// a bare `[list procName baked args]` — no `namespace code` wrapper, and with
// further baked arguments after the head, one of them a nested command
// substitution.
//
// Oracle (9.0.4 / 8.6.16): the trace fires, printing
// `onVersionWrite called: minversion=1.0.0 baseversion=0.9.0` — the callback
// head really is `demo::onVersionWrite`.
#[test]
fn tp_references_reach_a_bare_list_built_trace_callback() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval demo {\n\
         \x20   variable echarts_version 0.9.0\n\
         \x20   proc MINVER {} { return 1.0.0 }\n\
         \x20   proc onVersionWrite {minv basev args} { return $minv }\n\
         \x20   trace add variable echarts_version write \
         [list demo::onVersionWrite [MINVER] $echarts_version]\n\
         }\n",
    );
    // From the declaration: the callback head must be a reference.
    let from_decl = lsp.references(&uri, 3, 10, true);
    let decl_lines = lines_in(&from_decl, &uri);
    assert!(
        decl_lines.contains(&3),
        "the declaration itself must be included: {decl_lines:?}"
    );
    assert!(
        decl_lines.contains(&4),
        "the `[list demo::onVersionWrite ...]` callback head must be a \
         reference: {decl_lines:?}"
    );
    // And from the callback head itself, go-to-definition must land on the
    // proc — the same relationship read the other way.
    let def = lsp.definition(&uri, 4, 51);
    assert_eq!(
        lines_in(&def, &uri),
        vec![3],
        "the callback head must resolve to the proc: {:?}",
        locations(&def)
    );
}

// TN control: a `[list ...]` whose head names nothing resolves to nothing —
// the mechanism reads the written head, it does not guess.
#[test]
fn tn_a_list_built_callback_head_that_names_nothing_resolves_to_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval demo {\n\
         \x20   variable v 1\n\
         \x20   proc onWrite {args} { return 1 }\n\
         \x20   trace add variable v write [list demo::notDefined $v]\n\
         }\n",
    );
    let def = lsp.definition(&uri, 3, 40);
    assert!(
        locations(&def).is_empty(),
        "an undefined callback head must resolve to nothing: {:?}",
        locations(&def)
    );
}

// -- idx 67: a qualified proc written outside its namespace block --------

// TP.  pix's shape: `namespace eval ::pix { namespace eval svg {} }` opens the
// namespaces, and `proc pix::svg::parse {...}` is written afterwards at the
// top level.  The proc belongs to `::pix::svg` and must be shown there.
//
// Oracle (9.0.4 / 8.6.16): `namespace which -command pix::svg::parse` answers
// `::pix::svg::parse`.
#[test]
fn tp_a_qualified_proc_nests_under_the_namespace_it_names() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::pix {\n\
         \x20   namespace eval svg {}\n\
         }\n\
         proc pix::svg::parse {svg} { return $svg }\n",
    );
    let symbols = lsp.document_symbols(&uri);
    let flat = symbols.to_string();
    assert!(
        flat.contains("parse"),
        "the proc must appear in the outline: {flat}"
    );
    // `parse` must be a *descendant* of the `::pix` namespace symbol, not a
    // sibling of it at the top level.
    let top = symbols.as_array().cloned().unwrap_or_default();
    let top_names: Vec<String> = top
        .iter()
        .map(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert!(
        !top_names.iter().any(|n| n == "parse"),
        "`parse` must not float at the top level beside the namespace tree: {top_names:?}"
    );
    let pix = top
        .iter()
        .find(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.contains("pix"))
        })
        .expect("the `::pix` namespace must be in the outline");
    assert!(
        pix.to_string().contains("parse"),
        "`parse` must nest beneath `::pix`: {pix}"
    );
}

// TN control (the finding's own auto-vivification refutation): a qualified
// proc whose namespace was *never* opened must stay where it was written —
// `proc` does not create the namespace, so there is nothing to nest under.
#[test]
fn tn_a_qualified_proc_whose_namespace_is_never_opened_stays_put() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc never::opened::parse {svg} { return $svg }\n");
    let symbols = lsp.document_symbols(&uri);
    let top = symbols.as_array().cloned().unwrap_or_default();
    assert!(
        !top.iter().any(|s| s
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| n.contains("never"))),
        "no namespace symbol may be invented for an unopened namespace: {symbols}"
    );
}

// -- idx 104: a parameter whose name shadows a command -------------------

// TP.  Tk's `library/tk.tcl` shape: `proc ::tk::RestoreFocusGrab {grab focus
// {destroy destroy}}` — the third parameter is named `destroy` and defaults
// to the *literal* `destroy`, while a `destroy` command also exists.
//
// Oracle (9.0.4 / 8.6.16): `$destroy` always reads the local parameter and a
// bare `destroy $grab` always calls the command — the two never mix.
//
// The hover half is covered by `parameter_default_literal_does_not_hover_923_idx104`;
// this pins the definition and references half at the wire tier.
#[test]
fn tp_a_parameter_shadowing_a_proc_resolves_to_itself() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc destroy {w} { return \"destroying $w\" }\n\
         proc RestoreFocusGrab {grab focus {destroy destroy}} {\n\
         \x20   if {$destroy eq \"destroy\"} { return [destroy $grab] }\n\
         \x20   return $destroy\n\
         }\n",
    );
    // Cursor on the parameter's own declaration name.
    let def = lsp.definition(&uri, 1, 36);
    assert_eq!(
        lines_in(&def, &uri),
        vec![1],
        "the parameter's own name must resolve to itself, not the same-named \
         proc on line 0: {:?}",
        locations(&def)
    );
    // References from the parameter reach both `$destroy` reads and never the
    // proc's declaration.
    let refs = lsp.references(&uri, 1, 36, true);
    let ref_lines = lines_in(&refs, &uri);
    assert!(
        ref_lines.contains(&2) && ref_lines.contains(&3),
        "both `$destroy` reads must be references: {ref_lines:?}"
    );
    assert!(
        !ref_lines.contains(&0),
        "the unrelated `proc destroy` must never be a reference to the \
         parameter: {ref_lines:?}"
    );
}

// TN control: the parameter's *default value* word is pure data — it names
// nothing, so it must resolve to nothing rather than to the same-spelled
// command.
#[test]
fn tn_a_parameter_default_literal_names_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc destroy {w} { return \"destroying $w\" }\n\
         proc RestoreFocusGrab {grab focus {destroy destroy}} {\n\
         \x20   return [destroy $grab]\n\
         }\n",
    );
    // Cursor on the default-value word.
    let def = lsp.definition(&uri, 1, 44);
    assert!(
        locations(&def).is_empty(),
        "a parameter default literal is data, not a reference: {:?}",
        locations(&def)
    );
    // FP guard: the *real* call in the body still resolves to the proc.
    let call = lsp.definition(&uri, 2, 12);
    assert_eq!(
        lines_in(&call, &uri),
        vec![0],
        "the bare `destroy $grab` call must still reach the proc: {:?}",
        locations(&call)
    );
}
