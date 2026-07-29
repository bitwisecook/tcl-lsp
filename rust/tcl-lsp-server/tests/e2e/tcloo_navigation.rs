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
