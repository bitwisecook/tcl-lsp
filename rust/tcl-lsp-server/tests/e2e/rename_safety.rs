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

//! The rename **safety gate**, end-to-end on the wire (PR C3).
//!
//! Three mandates, one mechanism:
//!
//! * issue #923 differential-audit finding **idx 79** — a member dispatched on
//!   a receiver whose class is not tracked;
//! * issue **#981**'s object-command residual — `CLASS create NAME` binds
//!   `NAME` in the *creation site's* namespace, so two same-named object
//!   commands in sibling namespaces must never cross-link;
//! * the workspace **namespace-variable** rename tier, the rename half of the
//!   reference set PR #1086 added.
//!
//! A refusal travels as a JSON-RPC **error** with the gate's own reason, not
//! as a `null` result: `null` means "nothing renameable here" and lets the
//! editor quietly do nothing, which is precisely the wrong signal when the
//! symbol *is* renameable but the rename would break the program.
//!
//! Every behavioural claim below is pinned against tclsh 9.0.4 and 8.6.16,
//! which agree byte-for-byte; each test records the transcript it was written
//! against.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The `newText` values of every edit in the whole workspace edit.
fn all_texts(result: &Value) -> Vec<String> {
    rename_edits(result)
        .values()
        .flatten()
        .map(|e| {
            e.get("newText")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

// -- idx 79: untracked receivers ----------------------------------------

// FP guard (idx 79).  nico-robert/tomato's `Vector3d.tcl` copy-constructor
// shape: `$other` is `[lindex $args 0]`, guarded by a runtime `info object
// isa` test, so it really is a `Vector3d` — but nothing assigns a constructor
// result to it, so the analyser has no binding.
//
// Oracle, tclsh 9.0.4 and 8.6.16 identically:
//   before                      -> `v1 = 7 9`, `v2 = 7 9`
//   after the declaration-only rename (`method X` -> `method GetX`,
//   `export X` -> `export GetX`, `[$other X]` left alone)
//                               -> `unknown method "X": must be Get, GetX, Y
//                                   or destroy` at `"$other X"`, rc=1
//
// That declaration-only edit set is exactly what the server used to return.
// It must now refuse instead.
#[test]
fn fp_rename_refuses_a_member_dispatched_on_an_untracked_receiver() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Vector3d {\n\
         \x20   variable _x _y\n\
         \x20   constructor {args} {\n\
         \x20       if {[llength $args] == 1} {\n\
         \x20           set other [lindex $args 0]\n\
         \x20           set _x [$other X]\n\
         \x20       } else {\n\
         \x20           lassign $args _x _y\n\
         \x20       }\n\
         \x20   }\n\
         \x20   method X {} { return $_x }\n\
         \x20   method Get {} { return \"$_x $_y\" }\n\
         \x20   export X Get\n\
         }\n",
    );
    // Cursor on the `X` of `method X`.
    let err = lsp.rename_error(&uri, 10, 11, "GetX");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("not tracked"),
        "the refusal must say why, got {err}"
    );
}

// FN guard: the same class, the same file, renaming a member the untracked
// receiver never names.  `Get` is dispatched only through tracked receivers,
// so the gate must not block it — over-refusal would make the feature
// unusable on any class with an internal `$var method` helper.
//
// Oracle (9.0.4 / 8.6.16): renaming `Get` -> `Fetch` everywhere it appears
// keeps `v1 = 7 9` / `v2 = 7 9`.
#[test]
fn fn_guard_rename_still_applies_for_a_member_the_untracked_receiver_never_names() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Vector3d {\n\
         \x20   variable _x _y\n\
         \x20   constructor {other} {\n\
         \x20       set _x [$other X]\n\
         \x20   }\n\
         \x20   method X {} { return $_x }\n\
         \x20   method Get {} { return \"$_x $_y\" }\n\
         \x20   export X Get\n\
         }\n",
    );
    // Cursor on the `Get` of `method Get`.
    let result = lsp.rename(&uri, 6, 11, "Fetch");
    let texts = all_texts(&result);
    assert!(
        texts.iter().filter(|t| *t == "Fetch").count() >= 2,
        "declaration + export list must both rename, got {texts:?}"
    );
}

// TP: the `export` list is part of the edit set.
//
// Oracle (9.0.4 / 8.6.16): a `method` whose name starts with an upper-case
// letter is *not* exported by default — `oo::class create A { method Foo {}
// {return 1} }; [A new] Foo` errors `unknown method "Foo": must be destroy`.
// With `export Foo` it prints `foo`.  Rename the method and leave the export
// behind and both interpreters answer `unknown method "Bar": must be
// destroy`, so the export word has to travel with the rename.
#[test]
fn tp_rename_rewrites_the_export_list_with_the_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create A {\n\
         \x20   method Foo {} { return foo }\n\
         \x20   export Foo\n\
         }\n\
         set a [A new]\n\
         puts [$a Foo]\n",
    );
    let result = lsp.rename(&uri, 1, 12, "Bar");
    let texts = all_texts(&result);
    assert!(
        texts.iter().filter(|t| *t == "Bar").count() >= 3,
        "declaration + export + call site must all rename, got {texts:?}"
    );
}

// -- issue #981: object commands are namespace-scoped --------------------

// TN + TP.  Oracle, tclsh 9.0.4 and 8.6.16 identically:
//
//   in ::a -> a-made
//   in ::b -> b-made
//   global a: a-made
//   global b: b-made
//
// Renaming `::b::Widget::make` -> `produce` *and* rewriting `::a`'s own `rex
// make` (what the server did before this fix) makes both interpreters fail
// with `unknown method "produce": must be destroy or make` at `"rex produce"`
// inside `::a`.  Renaming only `::b`'s half keeps the transcript identical.
#[test]
fn tn_object_command_rename_does_not_cross_namespaces() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::a {\n\
         \x20   oo::class create Factory {\n\
         \x20       method make {} { return \"a-made\" }\n\
         \x20       export make\n\
         \x20   }\n\
         \x20   Factory create rex\n\
         \x20   puts [rex make]\n\
         }\n\
         namespace eval ::b {\n\
         \x20   oo::class create Widget {\n\
         \x20       method make {} { return \"b-made\" }\n\
         \x20       export make\n\
         \x20   }\n\
         \x20   Widget create rex\n\
         \x20   puts [rex make]\n\
         }\n",
    );
    // Cursor on `make` in `::b::Widget`'s declaration (line 10).
    let result = lsp.rename(&uri, 10, 15, "produce");
    let edits = rename_edits(&result);
    let lines: Vec<i64> = edits
        .get(&uri)
        .into_iter()
        .flatten()
        .map(|e| {
            e.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                .unwrap_or(-1)
        })
        .collect();
    assert!(
        !lines.contains(&6),
        "::a's own `rex make` (line 6) must never be rewritten (issue #981): {lines:?}"
    );
    assert!(
        lines.contains(&14),
        "::b's own `rex make` (line 14) must rename — the finding's lost-site \
         half: {lines:?}"
    );
    assert!(
        lines.contains(&10),
        "::b's declaration must rename: {lines:?}"
    );
}

// -- workspace namespace-variable tier ----------------------------------

// TP: renaming `$::mypkg::version` from a *consumer* document rewrites the
// declaring sibling's `variable version` too.
//
// Oracle (9.0.4 / 8.6.16, sourcing both files): `version : 1.0` before and
// after the complete rename.
#[test]
fn tp_namespace_variable_rename_spans_documents() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n    variable version 1.0\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let result = lsp.rename(&user, 0, 20, "release");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&decl),
        "the declaring document must be edited too: {edits:?}"
    );
    assert!(
        edits.contains_key(&user),
        "the consumer document must be edited: {edits:?}"
    );
    let texts = all_texts(&result);
    assert!(
        texts.iter().any(|t| t == "release"),
        "the declaration renames to a bare name: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "$::mypkg::release"),
        "the qualified read keeps its qualifier: {texts:?}"
    );
}

// FN guard: a proc-local `variable v` alias in the declaring document — and
// every unqualified `$v` it enables — renames with the cell.  Rewriting only
// the declaration and the qualified read would leave `p` reading a cell that
// no longer exists.
//
// Oracle (9.0.4 / 8.6.16): the script prints `1` / `1` before and after the
// complete rename.
#[test]
fn fn_guard_namespace_variable_rename_rewrites_a_proc_local_alias() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n\
         \x20   variable version 1\n\
         \x20   proc show {} {\n\
         \x20       variable version\n\
         \x20       return $version\n\
         \x20   }\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let result = lsp.rename(&user, 0, 20, "release");
    let edits = rename_edits(&result);
    let decl_lines: Vec<i64> = edits
        .get(&decl)
        .into_iter()
        .flatten()
        .map(|e| {
            e.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                .unwrap_or(-1)
        })
        .collect();
    assert!(
        decl_lines.contains(&1),
        "the namespace declaration must rename: {decl_lines:?}"
    );
    assert!(
        decl_lines.contains(&3),
        "the proc-local `variable version` alias must rename: {decl_lines:?}"
    );
    assert!(
        decl_lines.contains(&4),
        "the alias's unqualified `$version` read must rename: {decl_lines:?}"
    );
}

// TN: a same-tailed cell in a sibling namespace is a different variable.
// `$other::v` never searches enclosing namespaces (9.0.4 / 8.6.16 both keep
// `::a::v` and `::b::v` independent), so it must be untouched.
#[test]
fn tn_namespace_variable_rename_leaves_a_sibling_namespaces_cell_alone() {
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    lsp.open_ready(&a, "namespace eval alpha {\n    variable v 1\n}\n");
    let b = unique_uri("tcl");
    lsp.open_ready(&b, "namespace eval beta {\n    variable v 2\n}\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::alpha::v\nputs $::beta::v\n");

    let result = lsp.rename(&user, 0, 15, "total");
    let edits = rename_edits(&result);
    assert!(
        !edits.contains_key(&b),
        "`::beta::v` must not be touched by a `::alpha::v` rename: {edits:?}"
    );
    assert!(
        edits.contains_key(&a),
        "`::alpha::v` must rename: {edits:?}"
    );
}

// FP guard: a document in the cell's namespace that computes a variable name
// (`set $n …` — a registry `ArgRole::VarWrite` word that
// `names_a_dynamic_variable`) may be naming this very cell, with no word to
// rewrite.  Refuse rather than emit an edit set that silently misses it.
//
// Oracle (9.0.4 / 8.6.16): `namespace eval ns { variable v 1; proc bump {n}
// {variable $n; set $n 2} }; ns::bump v; puts $::ns::v` prints `2` — the
// dynamic name really does reach the cell.
#[test]
fn fp_namespace_variable_rename_refuses_beside_a_computed_variable_name() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n\
         \x20   variable version 1\n\
         \x20   proc bump {n} {\n\
         \x20       variable $n\n\
         \x20       set $n 2\n\
         \x20   }\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let err = lsp.rename_error(&user, 0, 20, "release");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("computed at run time"),
        "the refusal must say why, got {err}"
    );
}

// TN (issue #1093): the refusal is **per site**.  A dynamic variable name
// written under a *different*, statically-spelled namespace cannot name a
// cell in `::mypkg`, so it must not block the rename — the previous gate
// refused on any dynamic variable word anywhere in a touched document.
//
// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1}; namespace eval
// ::other {}; set n {::ns::v}; set ::other::$n 99` fails with `can't set
// "::other::::ns::v": parent namespace doesn't exist` — the substituted value
// lands *under* the written prefix, so no value of `$n` can reach `::ns::v`.
#[test]
fn tn_namespace_variable_rename_ignores_a_dynamic_name_under_another_namespace() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n\
         \x20   variable version 1\n\
         }\n\
         namespace eval other {\n\
         \x20   proc bump {n} { set ::other::$n 2 }\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let result = lsp.rename(&user, 0, 20, "release");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&decl) && edits.contains_key(&user),
        "an out-of-namespace dynamic name must not refuse the rename: {edits:?}",
    );
}

// -- Codex review of PR #1091: fan-out coverage -------------------------

// FP guard (finding 1, issue #1092): the hazard lives in a **pure-consumer**
// document — one that neither defines nor extends any family class.  The
// consumer leg of the edit collector visits it; the gate must too, or the
// gate's guarantee is hollow: the declaration moves while `$who speak` keeps
// naming a member that no longer exists.
//
// Oracle, tclsh 9.0.4 and 8.6.16 identically:
//   before                        -> `direct   -> woof`, `indirect -> woof`,
//                                    rc 0
//   after renaming `speak` -> `bark` in the definer and at the tracked call
//   site, with the consumer's `[$who speak]` left alone
//                                 -> `direct   -> woof` then `unknown method
//                                    "speak": must be bark or destroy` at
//                                    `"$who speak"`, rc 1
#[test]
fn fp_rename_refuses_a_hazard_that_lives_only_in_a_consumer_document() {
    let mut lsp = Lsp::tcl();
    let definer = unique_uri("tcl");
    lsp.open_ready(
        &definer,
        "oo::class create Dog {\n\
         \x20   method speak {} { return \"woof\" }\n\
         \x20   export speak\n\
         }\n",
    );
    // Pure consumer: constructs a `Dog` (so the index lists it as a consumer
    // document) and *also* dispatches on a receiver nothing binds to a class.
    let consumer = unique_uri("tcl");
    lsp.open_ready(
        &consumer,
        "proc bark {who} {\n\
         \x20   return [$who speak]\n\
         }\n\
         set d [Dog new]\n\
         puts [$d speak]\n\
         puts [bark $d]\n",
    );

    // Cursor on the `speak` of `method speak`, in the definer.
    let err = lsp.rename_error(&definer, 1, 11, "bark");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("not tracked"),
        "a hazard in a consumer document must refuse the whole rename, got {err}"
    );
}

// FN guard (same finding): consumer documents whose dispatches are all
// tracked and literal must still rename.  The fan-out widened to reach them;
// it must not start refusing on their account.
//
// Oracle (9.0.4 / 8.6.16): renaming `speak` -> `bark` throughout keeps
// `direct -> woof` / `via   -> woof`, rc 0.
#[test]
fn fn_guard_rename_still_applies_across_tracked_consumer_documents() {
    let mut lsp = Lsp::tcl();
    let definer = unique_uri("tcl");
    lsp.open_ready(
        &definer,
        "oo::class create Dog {\n\
         \x20   method speak {} { return \"woof\" }\n\
         \x20   export speak\n\
         }\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(
        &consumer,
        "set d [Dog new]\n\
         puts [$d speak]\n",
    );

    let result = lsp.rename(&definer, 1, 11, "bark");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&definer),
        "the definer must be edited: {edits:?}"
    );
    assert!(
        edits.contains_key(&consumer),
        "the tracked consumer call site must be edited too: {edits:?}"
    );
    let texts = all_texts(&result);
    assert!(
        texts.iter().filter(|t| *t == "bark").count() >= 3,
        "declaration, export, and the consumer call all rename: {texts:?}"
    );
}

// TP (finding 2): a document whose only stake in the cell is an alias written
// from **another namespace** is visited and rewritten.  A global `proc p {} {
// namespace upvar ::ns v local; … }` declares nothing in `::ns` and writes no
// qualified occurrence, so it is reached only through the index's alias
// table.
//
// Oracle, tclsh 9.0.4 and 8.6.16 identically:
//   before                        -> `p    -> 1`, `cell -> 1`, rc 0
//   declaration renamed, alias left as `namespace upvar ::ns v local`
//                                 -> `can't read "local": no such variable`
//                                    at `"return $local"`, rc 1
//   declaration renamed and the alias's target word rewritten to
//   `namespace upvar ::ns total local`
//                                 -> `p    -> 1`, `cell -> 1`, rc 0
#[test]
fn tp_namespace_variable_rename_reaches_an_alias_in_another_namespace() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval mypkg {\n    variable version 1\n}\n");
    let aliaser = unique_uri("tcl");
    lsp.open_ready(
        &aliaser,
        "proc show {} {\n\
         \x20   namespace upvar ::mypkg version local\n\
         \x20   return $local\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let result = lsp.rename(&user, 0, 20, "release");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&aliaser),
        "the out-of-namespace aliasing document must be edited: {edits:?}"
    );
    let aliaser_texts: Vec<String> = edits
        .get(&aliaser)
        .into_iter()
        .flatten()
        .map(|e| {
            e.get("newText")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    assert_eq!(
        aliaser_texts,
        vec!["release".to_string()],
        "exactly one edit there — the word naming the cell, not the local \
         alias `local` nor its read: {aliaser_texts:?}"
    );
    let line: i64 = edits
        .get(&aliaser)
        .and_then(|e| e.first())
        .and_then(|e| e.get("range"))
        .and_then(|r| r.get("start"))
        .and_then(|s| s.get("line"))
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    assert_eq!(line, 1, "the edit is on the `namespace upvar` line");
}

// FP guard (finding 2, completeness half): an alias whose *cell* is computed
// (`namespace upvar $ns version local`) names no fixed variable, so no
// candidate scan can find it and no edit can keep it consistent.  The rename
// must refuse rather than move the declaration out from under it.
//
// Oracle, tclsh 9.0.4 and 8.6.16 identically:
//   before                        -> `show -> 1`, `cell -> 1`, rc 0
//   `variable version` renamed to `variable release`, the computed alias
//   necessarily left as written
//                                 -> `can't read "local": no such variable`
//                                    at `"return $local"`, rc 1
#[test]
fn fp_namespace_variable_rename_refuses_beside_a_computed_alias_cell() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval mypkg {\n    variable version 1\n}\n");
    let aliaser = unique_uri("tcl");
    lsp.open_ready(
        &aliaser,
        "proc show {ns} {\n\
         \x20   namespace upvar $ns version local\n\
         \x20   return $local\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let err = lsp.rename_error(&user, 0, 20, "release");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("computed at run time"),
        "the refusal must say why, got {err}"
    );
}

// FN guard (same): a computed alias cell that provably is *not* this cell —
// its literal tail names a different variable — must not block the rename.
// Refusing on the mere presence of a `namespace upvar $ns … …` anywhere in
// the workspace would make the tier unusable.
#[test]
fn fn_guard_namespace_variable_rename_ignores_a_computed_alias_of_another_cell() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval mypkg {\n    variable version 1\n}\n");
    let aliaser = unique_uri("tcl");
    lsp.open_ready(
        &aliaser,
        "proc show {ns} {\n\
         \x20   namespace upvar $ns counter local\n\
         \x20   return $local\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts $::mypkg::version\n");

    let result = lsp.rename(&user, 0, 20, "release");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&decl),
        "a computed alias of `counter` says nothing about `version`: {edits:?}"
    );
    assert!(
        !edits.contains_key(&aliaser),
        "and it must not be edited: {edits:?}"
    );
}

// -- Issue #1114: the namespace rename tier -----------------------------
//
// Renaming a namespace rewrites every *written* spelling of it — the
// `namespace eval` blocks that open it, the qualified names beneath it, the
// `NamespaceName`-role arguments, the import patterns — across every
// document, or refuses with the reason.  Only the namespace's final segment
// moves, which is what makes rooted and relative spellings work out of one
// rule.
//
// tclsh-proof (8.6.14, the interpreter available in this container): the
// before and after programs behave identically.
//   namespace eval ::old {variable v 1; proc p {} {return P}}
//   puts $::old::v ; puts [::old::p]                       -> 1 / P
//   …the same script with `old` spelled `new` throughout   -> 1 / P

// TP — the rename spans documents: the declaring file and the consumer file
// both move.
#[test]
fn tp_namespace_rename_spans_documents() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval ::old {\n\
         \x20   variable v 1\n\
         \x20   proc p {} { return P }\n\
         }\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(
        &user,
        "namespace children ::old\nputs $::old::v\nputs [::old::p]\n",
    );

    // The cursor is on the `NamespaceName`-role word, which is what makes the
    // request a *namespace* rename rather than a proc one.
    let result = lsp.rename(&user, 0, 21, "new");
    let edits = rename_edits(&result);
    assert!(
        edits.contains_key(&decl) && edits.contains_key(&user),
        "both documents must move: {edits:?}",
    );
    assert_eq!(
        edits.get(&user).map(Vec::len),
        Some(3),
        "the namespace word, the qualified variable, and the call all move: {edits:?}",
    );
    for (uri, es) in &edits {
        for e in es {
            assert_eq!(
                e.get("newText").and_then(Value::as_str),
                Some("new"),
                "only the namespace's own segment is rewritten ({uri}): {e:?}",
            );
        }
    }
}

// TN — a sibling namespace that merely shares the tail is untouched.
#[test]
fn tn_namespace_rename_leaves_a_same_tailed_sibling_alone() {
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    lsp.open_ready(&a, "namespace eval ::a::old { proc p {} { return A } }\n");
    let b = unique_uri("tcl");
    lsp.open_ready(&b, "namespace eval ::b::old { proc p {} { return B } }\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts [::a::old::p]\n");

    let result = lsp.rename(&user, 0, 12, "new");
    let edits = rename_edits(&result);
    assert!(edits.contains_key(&a), "`::a::old` must move: {edits:?}");
    assert!(
        !edits.contains_key(&b),
        "`::b::old` is a different namespace: {edits:?}",
    );
}

// FP guard — a document holding a rooted name beneath the namespace in a data
// word attributes nothing, so the whole rename refuses rather than leaving
// that word naming a namespace that no longer exists.
#[test]
fn fp_namespace_rename_refuses_an_unattributed_embedded_name() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval ::old { proc tick {} {} }\n");
    let holder = unique_uri("tcl");
    lsp.open_ready(&holder, "set handlers [list ::old::tick]\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace children ::old\n");

    let err = lsp.rename_error(&user, 0, 21, "new");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("cannot attribute"),
        "the refusal must say why, got {err}",
    );
}

// FP guard — renaming onto a namespace that already exists would merge two
// namespaces into one.
#[test]
fn fp_namespace_rename_refuses_a_collision_with_an_existing_namespace() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval ::old { proc p {} {} }\n");
    let other = unique_uri("tcl");
    lsp.open_ready(&other, "namespace eval ::taken { proc q {} {} }\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace children ::old\n");

    let err = lsp.rename_error(&user, 0, 21, "taken");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("already exists"),
        "the refusal must name the collision, got {err}",
    );
}
