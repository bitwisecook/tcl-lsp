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

//! `TclOO` cross-file navigation orchestration: rename, Find All References,
//! and the code lens must all resolve a method through the *same* workspace
//! machinery, so what one of them sees the others see too.
//!
//! * Issue #993 — rename skipped **pure-consumer** documents (a file that only
//!   calls `$obj method` / `Class method` and declares nothing), silently
//!   leaving them bound to a name that no longer exists.
//! * Issue #991 — the code lens's click target and its displayed count both
//!   came from narrower resolvers than Find All References on the same
//!   declaration, so the three disagreed on the same symbol.
//!
//! Dispatch shapes are oracle-checked: `$f make` on an `oo::class` instance
//! and bare `Factory make` on a 9.0 `classmethod` both dispatch to the class
//! member (tclsh8.6 for the instance shape, tclsh9.0 for `classmethod`, which
//! 8.6's `TclOO` has no keyword for), while `Factory make` where `Factory` is
//! an ordinary proc calls that proc with the literal argument `make` — no
//! method edge at all.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The start lines of every rename edit landing in `uri`.
fn edit_lines(edits: &std::collections::BTreeMap<String, Vec<Value>>, uri: &str) -> Vec<i64> {
    edits
        .get(uri)
        .into_iter()
        .flatten()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect()
}

/// The `(uri, start line)` pairs of every location in a references /
/// lens-resolve payload.
fn location_lines(result: &Value, uri: &str) -> std::collections::BTreeSet<i64> {
    locations(result)
        .iter()
        .filter(|l| l.uri == uri)
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect()
}

/// Every `(uri, start line)` pair in a references / lens-resolve payload.
fn all_location_lines(result: &Value) -> std::collections::BTreeSet<(String, i64)> {
    locations(result)
        .iter()
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                .map(|line| (l.uri.clone(), line))
        })
        .collect()
}

/// Resolve the code lens anchored at `line` in `uri`, returning its
/// `command` object (`{title, command, arguments}`).
fn resolve_lens_on_line(lsp: &mut Lsp, uri: &str, line: i64) -> Value {
    let all = match lsp.code_lens(uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    let lens = all
        .iter()
        .find(|l| l["range"]["start"]["line"].as_i64() == Some(line))
        .unwrap_or_else(|| panic!("no lens anchored at line {line} of {uri}: {all:?}"))
        .clone();
    lsp.code_lens_resolve(lens)["command"].clone()
}

/// The lens label the server emits for `count` sites.
fn expected_title(count: usize) -> Value {
    Value::String(match count {
        1 => "1 reference".to_owned(),
        n => format!("{n} references"),
    })
}

/// The `locations` argument a resolved lens hands `tcl-lsp.showReferences`.
fn lens_locations(command: &Value) -> Value {
    command
        .get("arguments")
        .and_then(Value::as_array)
        .and_then(|a| a.get(2))
        .cloned()
        .unwrap_or(Value::Null)
}

// Issue #993 — rename must reach pure-consumer documents.

/// TP: `consumer.tcl` only ever *calls* `Factory`'s `make`; it declares no
/// part of the class.  Renaming from the declaration must rewrite its
/// `$f make` call site, or the consumer keeps calling a method that no
/// longer exists under that name.
#[test]
fn tp_method_rename_reaches_pure_consumer_document() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    method make {} { return 1 }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "set f [Factory new]\n$f make\n");

    // Cursor on the `make` declaration name (line 1, col 11).
    let edits = rename_edits(&lsp.rename(&factory, 1, 11, "produce"));
    assert!(
        edit_lines(&edits, &factory).contains(&1),
        "the declaration itself must rename: {edits:?}"
    );
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![1],
        "the pure-consumer `$f make` call site must rename too: {edits:?}"
    );
    assert!(
        edits
            .get(&consumer)
            .is_some_and(|e| e.iter().all(|e| e["newText"] == "produce")),
        "consumer edits must carry the new name: {edits:?}"
    );
}

/// TN: a document with its own ordinary `proc make` — no `Factory` instance
/// anywhere — is not a consumer of the class and must be left alone.
#[test]
fn tn_unrelated_same_named_proc_document_untouched_by_method_rename() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    method make {} { return 1 }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "set f [Factory new]\n$f make\n");
    let unrelated = unique_uri("tcl");
    lsp.open_ready(&unrelated, "proc make {} { return 2 }\nmake\n");

    let edits = rename_edits(&lsp.rename(&factory, 1, 11, "produce"));
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![1],
        "the real consumer must still rename: {edits:?}"
    );
    assert!(
        edit_lines(&edits, &unrelated).is_empty(),
        "an unrelated `proc make` document must NOT be rewritten: {edits:?}"
    );
}

/// TN, the class gate under load: the consumer really *does* invoke a class
/// — two of them — and both define `make`.  Only the `Factory` instance's
/// call site may rename; the `Widget` instance's `$g make` calls a different
/// method that keeps its name.  The `proc make` variant above never
/// constructs anything, so it exercises only the "document mentions no class
/// at all" cut, not this one (adversarial review of #1047).
#[test]
fn tn_second_classes_instance_dispatch_untouched_by_method_rename() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    method make {} { return 1 }\n}\n",
    );
    let widget = unique_uri("tcl");
    lsp.open_ready(
        &widget,
        "oo::class create Widget {\n    method make {} { return 2 }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(
        &consumer,
        "set f [Factory new]\nset g [Widget new]\n$f make\n$g make\n",
    );

    let edits = rename_edits(&lsp.rename(&factory, 1, 11, "produce"));
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![2],
        "only `$f make` (the Factory instance) may rename: {edits:?}"
    );
    assert!(
        edit_lines(&edits, &widget).is_empty(),
        "`Widget`'s own declaration must not rename: {edits:?}"
    );
}

/// The classmethod twin of the two-class gate: `Factory make` renames,
/// `Widget make` does not.  Both dispatch shapes are real under tclsh9.0.
#[test]
fn tn_second_classes_bare_dispatch_untouched_by_classmethod_rename() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    classmethod make {} { return 1 }\n}\n",
    );
    let widget = unique_uri("tcl");
    lsp.open_ready(
        &widget,
        "oo::class create Widget {\n    classmethod make {} { return 2 }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "Factory make\nWidget make\n");

    let edits = rename_edits(&lsp.rename(&factory, 1, 16, "produce"));
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![0],
        "only the `Factory make` dispatch may rename: {edits:?}"
    );
    assert!(
        edit_lines(&edits, &widget).is_empty(),
        "`Widget`'s own classmethod declaration must not rename: {edits:?}"
    );
}

/// TP (classmethod variant): a consumer document dispatching on the class's
/// own command (`Factory make`, tclsh9.0-verified) is a consumer just like
/// an instance-dispatch one, and renames with the declaration.
#[test]
fn tp_classmethod_rename_reaches_consumer_bare_class_dispatch() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "set f [Factory new]\nFactory make\n");

    let edits = rename_edits(&lsp.rename(&factory, 1, 16, "produce"));
    assert!(
        edit_lines(&edits, &factory).contains(&1),
        "the classmethod declaration must rename: {edits:?}"
    );
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![1],
        "the consumer's bare `Factory make` dispatch must rename: {edits:?}"
    );
}

/// Rename and Find All References must agree site-for-site on the consumer
/// document — they now share one resolver, and a site one sees but the other
/// misses is exactly the #993 corruption.
#[test]
fn tp_consumer_rename_and_references_agree_on_the_same_sites() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    method make {} { return 1 }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "set f [Factory new]\n$f make\nputs [$f make]\n");

    let refs = lsp.references(&factory, 1, 11, false);
    let ref_lines = location_lines(&refs, &consumer);
    let edits = rename_edits(&lsp.rename(&factory, 1, 11, "produce"));
    let renamed: std::collections::BTreeSet<i64> =
        edit_lines(&edits, &consumer).into_iter().collect();
    assert_eq!(
        renamed, ref_lines,
        "every consumer site references reports must also rename: refs={refs:?} edits={edits:?}"
    );
    assert!(
        renamed.contains(&1) && renamed.contains(&2),
        "both consumer call sites must be covered: {edits:?}"
    );
}

// Issue #991 — code-lens click and count must match Find All References.

/// TP: the lens above `Animal`'s `speak` must count — and, on click, open —
/// the sibling document's override declaration and its `$d speak` dispatch,
/// exactly as Find All References on the same declaration does.
/// tclsh8.6/9.0-verified: `$d speak` on a `Dog` (superclass `Animal`) enters
/// `Dog`'s override, which is a member of `Animal::speak`'s override family.
#[test]
fn tp_method_lens_click_and_count_match_find_all_references() {
    let mut lsp = Lsp::tcl();
    let animal = unique_uri("tcl");
    lsp.open_ready(
        &animal,
        "oo::class create Animal {\n    method speak {} { return \"...\" }\n}\n",
    );
    let dog = unique_uri("tcl");
    lsp.open_ready(
        &dog,
        "oo::class create Dog {\n    superclass Animal\n    method speak {} { return \"woof\" }\n}\nset d [Dog new]\n$d speak\n",
    );

    // Cursor on `speak`'s declaration in animal.tcl (line 1, col 11).
    let refs = lsp.references(&animal, 1, 11, false);
    let ref_sites = all_location_lines(&refs);
    assert!(
        !location_lines(&refs, &dog).is_empty(),
        "Find All References must reach the sibling document: {refs:?}"
    );

    let command = resolve_lens_on_line(&mut lsp, &animal, 1);
    let lens_sites = all_location_lines(&lens_locations(&command));
    assert_eq!(
        lens_sites, ref_sites,
        "the lens click must open exactly what Find All References returns: {command:?}"
    );
    assert_eq!(
        command["title"],
        expected_title(ref_sites.len()),
        "the lens count must be the number of sites its click opens: {command:?}"
    );
    assert!(
        ref_sites.contains(&(dog.clone(), 5)),
        "the sibling `$d speak` dispatch is one of them: {refs:?}"
    );
}

/// TN: a method with genuinely no references anywhere still reads
/// "0 references" and opens nothing — the cross-file layer must not invent
/// sites for a member nobody calls.
#[test]
fn tn_method_with_no_cross_file_references_still_reads_zero() {
    let mut lsp = Lsp::tcl();
    let animal = unique_uri("tcl");
    lsp.open_ready(
        &animal,
        "oo::class create Animal {\n    method speak {} { return \"...\" }\n    method quiet {} { return \"\" }\n}\n",
    );
    let dog = unique_uri("tcl");
    lsp.open_ready(
        &dog,
        "oo::class create Dog {\n    superclass Animal\n    method speak {} { return \"woof\" }\n}\nset d [Dog new]\n$d speak\n",
    );

    // `quiet` (line 2) is declared once and never dispatched.
    let command = resolve_lens_on_line(&mut lsp, &animal, 2);
    assert_eq!(
        command["title"],
        Value::String("0 references".to_owned()),
        "an uncalled method must not gain sites from the cross-file layer: {command:?}"
    );
    assert_eq!(
        all_location_lines(&lens_locations(&command)),
        std::collections::BTreeSet::new(),
        "and its click must open nothing: {command:?}"
    );
}

/// TP: the classmethod shape too — a bare `Factory make` dispatch in a
/// consumer document counts towards, and opens from, the declaration's lens
/// (tclsh9.0-verified dispatch).
#[test]
fn tp_classmethod_lens_click_and_count_reach_a_consumer_document() {
    let mut lsp = Lsp::tcl();
    let factory = unique_uri("tcl");
    lsp.open_ready(
        &factory,
        "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "Factory make\n");

    let refs = lsp.references(&factory, 1, 16, false);
    let ref_sites = all_location_lines(&refs);
    assert!(
        location_lines(&refs, &consumer).contains(&0),
        "the consumer's bare dispatch must be a reference: {refs:?}"
    );
    let command = resolve_lens_on_line(&mut lsp, &factory, 1);
    assert_eq!(
        all_location_lines(&lens_locations(&command)),
        ref_sites,
        "the classmethod lens click must match Find All References: {command:?}"
    );
    assert_eq!(
        command["title"],
        expected_title(ref_sites.len()),
        "and its count must be that same number: {command:?}"
    );
}

// Issue #981 — bare class-command dispatch is namespace-scoped.

/// TN, the issue's own repro, across files: two classes named `Factory` in
/// `::a` and `::b`, each with its own `make`, and a consumer document that
/// dispatches `Factory make` inside `namespace eval ::b`.  Real Tcl resolves
/// that to `::b::Factory` (verified on tclsh 8.6.14 and 9.0.4), so a rename
/// of `::a::Factory`'s `make` must not rewrite it — which it did before,
/// corrupting an unrelated class's call site in a different file.
#[test]
fn tn_consumer_bare_dispatch_is_attributed_to_its_own_namespaces_class() {
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    lsp.open_ready(
        &a,
        "namespace eval ::a {\n    oo::class create Factory {\n        classmethod make {} { return 1 }\n    }\n}\n",
    );
    let b = unique_uri("tcl");
    lsp.open_ready(
        &b,
        "namespace eval ::b {\n    oo::class create Factory {\n        classmethod make {} { return 2 }\n    }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "namespace eval ::b {\n    Factory make\n}\n");

    let refs = lsp.references(&a, 2, 20, false);
    assert!(
        location_lines(&refs, &consumer).is_empty(),
        "`::b`'s bare dispatch is not a reference to `::a::Factory`'s make: {refs:?}"
    );

    let edits = rename_edits(&lsp.rename(&a, 2, 20, "produce"));
    assert!(
        edit_lines(&edits, &consumer).is_empty(),
        "and rename must not rewrite it: {edits:?}"
    );
    assert!(
        edit_lines(&edits, &b).is_empty(),
        "nor `::b`'s own class: {edits:?}"
    );
}

/// TP: the same consumer document *is* attributed to `::b::Factory`, so the
/// scoping is a re-attribution, not a loss — and the cross-file consumer
/// rename from #1047 still reaches it.
#[test]
fn tp_consumer_bare_dispatch_belongs_to_the_matching_namespaces_class() {
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    lsp.open_ready(
        &a,
        "namespace eval ::a {\n    oo::class create Factory {\n        classmethod make {} { return 1 }\n    }\n}\n",
    );
    let b = unique_uri("tcl");
    lsp.open_ready(
        &b,
        "namespace eval ::b {\n    oo::class create Factory {\n        classmethod make {} { return 2 }\n    }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "namespace eval ::b {\n    Factory make\n}\n");

    let refs = lsp.references(&b, 2, 20, false);
    assert!(
        location_lines(&refs, &consumer).contains(&1),
        "`::b`'s bare dispatch must be a reference to `::b::Factory`'s make: {refs:?}"
    );
    let edits = rename_edits(&lsp.rename(&b, 2, 20, "produce"));
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![1],
        "and rename must rewrite it: {edits:?}"
    );
}

// Issue #990 — [incr Tcl]'s colon-qualified class-proc dispatch.

/// TP: `Factory::make` is how [incr Tcl] dispatches a class-scoped `proc`
/// (its equivalent of `TclOO`'s `classmethod`), and it is a *single*
/// `::`-qualified command word rather than the two-word `Factory make` shape
/// `TclOO` / snit use.  Definition, references, and rename must all follow it.
///
/// Oracle (tclsh 8.6.14 + Itcl 3.4): `Factory::make` and `::Factory::make`
/// both dispatch the class proc, a bare `make` inside a sibling class body
/// does too, and the two-word `Factory make` instead creates an *object*
/// named `make`.
#[test]
fn tp_itcl_class_proc_colon_dispatch_navigates_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "itcl::class Factory {\n",
            "    proc make {} { return 1 }\n",
            "    proc other {} { return [make] }\n",
            "}\n",
            "Factory::make\n",
            "::Factory::make\n",
        ),
    );

    // Go-to-definition from the qualified call site reaches the `proc`.
    let def = lsp.definition(&uri, 4, 11);
    assert_eq!(
        start_lines(&def),
        [1].into_iter().collect(),
        "`Factory::make` must reach the class proc: {def:?}"
    );

    // References from the declaration cover every dispatch spelling.
    let refs = lsp.references(&uri, 1, 10, true);
    assert_eq!(
        start_lines(&refs),
        [1, 2, 4, 5].into_iter().collect(),
        "decl + bare sibling call + both qualified dispatches: {refs:?}"
    );

    // Rename rewrites the member name and preserves each qualifier.
    let edits = rename_edits(&lsp.rename(&uri, 1, 10, "produce"));
    assert_eq!(edit_lines(&edits, &uri), vec![1, 2, 4, 5], "{edits:?}");
    assert!(
        edits
            .get(&uri)
            .is_some_and(|e| e.iter().all(|e| e["newText"] == "produce")),
        "each edit replaces only the member name: {edits:?}"
    );
}

/// TN: itcl's two-word `Factory make` is object *creation*
/// (`ClassName instanceName`) — never a class-proc dispatch — so the class
/// proc keeps only its declaration.
#[test]
fn tn_itcl_two_word_object_creation_is_not_a_class_proc_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "itcl::class Factory {\n    proc make {} { return 1 }\n}\nFactory make\n",
    );
    let refs = lsp.references(&uri, 1, 10, true);
    assert_eq!(
        start_lines(&refs),
        [1].into_iter().collect(),
        "declaration only: {refs:?}"
    );
}

// Issue #1019 idx 16 — method names that are not identifiers.

/// TP: a hyphenated method (`with-dash`) and a TIP 558 property accessor
/// (`<ReadProp-x>`) are ordinary dispatchable method names — oracle-checked
/// on tclsh 8.6.14 and 9.0.4 — so definition, references, hover, and rename
/// must all resolve them from their call sites.  Before the fix the cursor
/// word stopped at `-` / `<` / `>`, truncating the name to something no
/// class declares.
#[test]
fn tp_non_identifier_method_names_navigate_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Widget {\n",
            "    method with-dash {} { return 1 }\n",
            "    method <ReadProp-x> {} { return 2 }\n",
            "    method probe {} { my <ReadProp-x> }\n",
            "}\n",
            "Widget create rex\n",
            "rex with-dash\n",
        ),
    );

    // Go-to-definition from the hyphenated call site (line 6).
    let def = lsp.definition(&uri, 6, 7);
    assert_eq!(
        start_lines(&def),
        [1].into_iter().collect(),
        "`rex with-dash` must reach the declaration: {def:?}"
    );

    // Go-to-definition from the `my <ReadProp-x>` internal dispatch.
    let prop_def = lsp.definition(&uri, 3, 28);
    assert_eq!(
        start_lines(&prop_def),
        [2].into_iter().collect(),
        "`my <ReadProp-x>` must reach the accessor declaration: {prop_def:?}"
    );

    // References from the declaration cover the call site.
    let refs = lsp.references(&uri, 1, 13, true);
    assert_eq!(
        start_lines(&refs),
        [1, 6].into_iter().collect(),
        "declaration + `rex with-dash`: {refs:?}"
    );

    // Hover on the call site names the method.
    let hover = hover_text(&lsp.hover(&uri, 6, 7));
    assert!(
        hover.contains("with-dash"),
        "hover must describe the hyphenated method: {hover:?}"
    );

    // Rename rewrites both sites.  The *new* name still has to pass the
    // existing safe-symbol gate (`is_safe_symbol_name`), which is about what
    // an editor may write, not about what Tcl can dispatch — so renaming
    // *from* a hyphenated name works, renaming *to* one does not.
    let edits = rename_edits(&lsp.rename(&uri, 1, 13, "renamed"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![1, 6],
        "both the declaration and the call site must rename: {edits:?}"
    );
}

/// TN: `$x-1` is arithmetic.  The widened word rule must not read it as a
/// `-1` method dispatch on an object-holding variable, so the class's own
/// method keeps exactly one reference (its declaration).
#[test]
fn tn_subtraction_is_not_a_method_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Widget {\n",
            "    method with-dash {} { return 1 }\n",
            "}\n",
            "set x [Widget new]\n",
            "set y [expr {$x-1}]\n",
        ),
    );
    let refs = lsp.references(&uri, 1, 13, true);
    assert_eq!(
        start_lines(&refs),
        [1].into_iter().collect(),
        "only the declaration: {refs:?}"
    );
}

/// Whether any diagnostic in `diags` carries `code`.
fn carries_code(diags: &[Value], code: &str) -> bool {
    diags
        .iter()
        .any(|d| d.get("code").and_then(Value::as_str) == Some(code))
}

// Issue #1119 — the class-side visibility channel must cross files.

/// TP: `decl.tcl` declares a class-side `Cm` and immediately `self unexport`s
/// it, so its own document cannot dispatch `C Cm` — the in-document provider
/// correctly declines.  `revive.tcl` then `self export`s it, and the call must
/// resolve again.
///
/// This is the class-side visibility channel end to end: the export written in
/// one file has to reach the other file's **class-command** dispatch.  Before
/// the channel existed a `self export` / `self unexport` was recorded nowhere
/// the workspace index could see, so it never travelled at all — the
/// instance-side pair is the instance-side record by contract and a class-side
/// flip must not enter it (issue #1098/#1119).
///
/// Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create C { self { method Cm {} {…} ; unexport Cm } }
/// C Cm                             ;# -> unknown method "Cm"
/// oo::define C { self export Cm }
/// C Cm                             ;# -> 1
/// ```
#[test]
fn tp_cross_file_self_export_revives_class_command_definition() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    let src = concat!(
        "oo::class create C {\n",
        "    self { method Cm {} { return 1 }\n",
        "           unexport Cm }\n",
        "}\n",
        "C Cm\n",
    );
    let revive = unique_uri("tcl");
    lsp.open_ready(&revive, "oo::define C { self export Cm }\n");
    lsp.open_ready(&decl, src);
    assert!(
        !locations(&lsp.definition(&decl, 4, 3)).is_empty(),
        "a cross-file `self export` must revive the class-side member's dispatch",
    );
}

/// TN (CRITICAL FP guard): the same cross-file `self export` must not revive an
/// identically-named *instance* method the class also unexports — the two
/// sides never share a visibility record, so a class-side export says nothing
/// about `$obj Cm` (oracle: `[C new] Cm` -> `unknown method "Cm"`).
#[test]
fn tn_cross_file_self_export_does_not_revive_the_instance_method() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    let src = concat!(
        "oo::class create C {\n",
        "    method Cm {} { return 1 }\n",
        "    unexport Cm\n",
        "    self { method Cm {} { return 2 }\n",
        "           unexport Cm }\n",
        "}\n",
        "set o [C new]\n",
        "$o Cm\n",
    );
    lsp.open_ready(&decl, src);
    let revive = unique_uri("tcl");
    lsp.open_ready(&revive, "oo::define C { self export Cm }\n");
    lsp.open_ready(&decl, src);
    assert!(
        locations(&lsp.definition(&decl, 7, 4)).is_empty(),
        "a class-side `self export` must not make an unexported instance method dispatch",
    );
}

/// TN (CRITICAL FP guard): the same `self unexport cm` must leave an
/// identically-named *instance* method alone — the two sides never share a
/// visibility record.  Oracle: `[C new] cm` still answers after the flip.
#[test]
fn tn_cross_file_self_unexport_leaves_the_instance_method_dispatchable() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        concat!(
            "oo::class create C {\n",
            "    method cm {} { return 1 }\n",
            "    self { method cm {} { return 2 } }\n",
            "}\n",
        ),
    );
    let flip = unique_uri("tcl");
    lsp.open_ready(&flip, "oo::define C { self unexport cm }\n");
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "set o [C new]\n$o cm\n");
    assert!(
        !locations(&lsp.definition(&consumer, 1, 4)).is_empty(),
        "an instance `$o cm` must survive a class-side unexport",
    );
}

// Issue #1121 — the renamed destination is a navigable member.

/// TP: `renamemethod old new` makes `new` a real member carrying `old`'s body
/// (`[C new] new` -> the old body, `info class definition ::C new` -> the old
/// parameter list and body, on 9.0.4 and 8.6.14).  Outline, hover and
/// go-to-definition must all reach it.
#[test]
fn tp_renamed_member_is_navigable_under_its_new_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method old {} { return OLDBODY }\n",
            "    renamemethod old new\n",
            "}\n",
            "set o [C new]\n",
            "$o new\n",
        ),
    );
    // Outline lists the destination name, not the source.
    let outline = lsp.document_symbols(&uri);
    let listed = format!("{outline}");
    assert!(
        listed.contains("\"new\""),
        "outline must list `new`: {outline}"
    );
    assert!(
        !listed.contains("\"old\""),
        "outline must not keep the retracted `old`: {outline}"
    );
    // Go-to-definition from the call site lands on a real span.
    assert!(
        !locations(&lsp.definition(&uri, 5, 4)).is_empty(),
        "`$o new` must resolve",
    );
    // Hover on the renamed member answers rather than coming back empty.
    assert!(
        !hover_text(&lsp.hover(&uri, 5, 4)).is_empty(),
        "hover on the renamed member must answer",
    );
}

// Issue #1120 — W315 on a definition that cannot run.

/// TP: retract-first aborts the whole class definition in real Tcl, so the
/// file declares a class that never exists.  The server reports it — and still
/// serves the partial class's outline, the same degradation a parse error gets.
#[test]
fn tp_retract_before_declare_reports_w315_and_keeps_the_outline() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    deletemethod ghost\n",
            "    method ghost {} { return 1 }\n",
            "    method kept {} { return 2 }\n",
            "}\n",
        ),
    );
    assert!(carries_code(&diags, "W315"), "expected W315: {diags:?}");
    let outline = format!("{}", lsp.document_symbols(&uri));
    assert!(
        outline.contains("\"kept\""),
        "the partial class must still serve an outline: {outline}"
    );
}

/// TN (CRITICAL): a cross-side `export` / `unexport` is a **silent no-op** in
/// real Tcl, not the hard error `deletemethod` raises — so it must draw no
/// W315.  Oracle: `oo::class create E { method onlyinst {} {} }` then
/// `oo::define E { self unexport onlyinst }` succeeds and changes nothing.
#[test]
fn tn_cross_side_visibility_word_draws_no_w315() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        concat!(
            "oo::class create E {\n",
            "    method onlyinst {} { return 1 }\n",
            "}\n",
            "oo::define E { self unexport onlyinst }\n",
        ),
    );
    assert!(!carries_code(&diags, "W315"), "unexpected W315: {diags:?}");
}

/// TN: a cross-file `oo::define` stub retracting a member declared elsewhere is
/// the normal shape, not an error — this record has no tables to judge against.
#[test]
fn tn_cross_file_stub_retraction_draws_no_w315() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "oo::define C { deletemethod m }\n");
    assert!(!carries_code(&diags, "W315"), "unexpected W315: {diags:?}");
}
