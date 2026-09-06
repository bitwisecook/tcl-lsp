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

/// Issue #1701: a Tk binding built with `[list [self] METHOD ...]` captures
/// the current object command and later invokes the exported method.  The
/// declaration's references, code lens, and rename must agree on that method
/// word even though it is data inside the prefix-building `list` call.
#[test]
fn tp_list_built_self_bind_callback_is_a_method_reference_everywhere() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require Tk\n\
         oo::class create Test {\n\
             method animTick {x y} { return {} }\n\
             method anim {wl} {\n\
                 bind $wl <ButtonPress-1> [list [self] animTick %x %y]\n\
             }\n\
         }\n",
    );

    let refs = lsp.references(&uri, 2, 11, false);
    assert_eq!(
        location_lines(&refs, &uri),
        std::collections::BTreeSet::from([4]),
        "Find All References must return the callback method word: {refs:?}"
    );

    let command = resolve_lens_on_line(&mut lsp, &uri, 2);
    assert_eq!(command["title"], expected_title(1), "{command:?}");
    assert_eq!(
        location_lines(&lens_locations(&command), &uri),
        std::collections::BTreeSet::from([4]),
        "the lens must open the same callback site: {command:?}"
    );

    let definition = lsp.definition(&uri, 4, 44);
    assert_eq!(
        location_lines(&definition, &uri),
        std::collections::BTreeSet::from([2]),
        "go-to-definition from the callback word must reach the method: {definition:?}"
    );
    let callback_refs = lsp.references(&uri, 4, 44, false);
    assert_eq!(
        location_lines(&callback_refs, &uri),
        std::collections::BTreeSet::from([4]),
        "references started on the callback must agree with the declaration: {callback_refs:?}"
    );

    let prepare = lsp.prepare_rename(&uri, 4, 44);
    assert_eq!(prepare["placeholder"], "animTick", "{prepare:?}");
    assert_eq!(prepare["range"]["start"]["line"], 4, "{prepare:?}");

    let edits = rename_edits(&lsp.rename(&uri, 2, 11, "frameTick"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![2, 4],
        "rename must update the declaration and callback word: {edits:?}"
    );
    let callback_edits = rename_edits(&lsp.rename(&uri, 4, 44, "frameTick"));
    assert_eq!(
        edit_lines(&callback_edits, &uri),
        vec![2, 4],
        "rename from the callback word must update the same family: {callback_edits:?}"
    );
}

/// #1704: a one-hop local constant carries a registry-declared callback
/// prefix.  Every navigation provider must use the builder method span, not
/// the later `$cb` registration as a guessed command reference.
#[test]
fn tp_stored_callback_prefix_agrees_across_navigation_providers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Test {\n\
             method tick {} { return {} }\n\
             method wire {} {\n\
                 set cb [list my tick]\n\
                 after 0 $cb\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(3).unwrap().find("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    let refs = lsp.references(&uri, 1, 11, false);
    assert_eq!(
        location_lines(&refs, &uri),
        std::collections::BTreeSet::from([3]),
        "stored callback must be a method reference: {refs:?}"
    );
    let command = resolve_lens_on_line(&mut lsp, &uri, 1);
    assert_eq!(command["title"], expected_title(1), "{command:?}");
    assert_eq!(
        location_lines(&lens_locations(&command), &uri),
        std::collections::BTreeSet::from([3]),
        "the lens must use the stored callback site: {command:?}"
    );

    let definition = lsp.definition(&uri, 3, callback_char);
    assert_eq!(
        location_lines(&definition, &uri),
        std::collections::BTreeSet::from([1])
    );
    let prepare = lsp.prepare_rename(&uri, 3, callback_char);
    assert_eq!(prepare["placeholder"], "tick", "{prepare:?}");
    let edits = rename_edits(&lsp.rename(&uri, 1, 11, "tock"));
    assert_eq!(edit_lines(&edits, &uri), vec![1, 3], "{edits:?}");
}

/// #1705: a captured `[self]` object command in an inheriting class reaches
/// the provider's exported implementation — tclsh 8.6.16 / 9.0.4 both run
/// `Base`'s body for `[list [Child new] tick]` — so every provider must place
/// the occurrence in `Base::tick`'s family, from either direction.
#[test]
fn tp_inherited_list_built_self_callback_joins_the_provider_family() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Base {\n\
             method tick {} { return 1 }\n\
         }\n\
         oo::class create Child {\n\
             superclass Base\n\
             method wire {} {\n\
                 after idle [list [self] tick]\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(6).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    let from_declaration = lsp.references(&uri, 1, 11, false);
    assert!(
        location_lines(&from_declaration, &uri).contains(&6),
        "the inherited callback is a call site of the provider: {from_declaration:?}"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 6, callback_char), &uri),
        std::collections::BTreeSet::from([1]),
        "go-to-definition from the callback word must reach the provider"
    );
    let from_callback = lsp.references(&uri, 6, callback_char, false);
    assert_eq!(
        location_lines(&from_callback, &uri),
        location_lines(&from_declaration, &uri),
        "references started on the callback must agree with the declaration"
    );
    let command = resolve_lens_on_line(&mut lsp, &uri, 1);
    assert!(
        location_lines(&lens_locations(&command), &uri).contains(&6),
        "the lens must count and open the inherited callback: {command:?}"
    );
    assert_eq!(
        lsp.prepare_rename(&uri, 6, callback_char)["placeholder"],
        "tick"
    );
    let edits = rename_edits(&lsp.rename(&uri, 6, callback_char, "tock"));
    assert!(
        edit_lines(&edits, &uri).contains(&1) && edit_lines(&edits, &uri).contains(&6),
        "rename must edit the declaration and the callback together: {edits:?}"
    );
}

/// #1705: the receiver's own `unexport` decides, not the provider's
/// declaration.  tclsh 8.6.16 / 9.0.4 answer `[Child new] tick` with `unknown
/// method "tick"` even though `Base` exports it, so the capture is not a call
/// site of `Base::tick` and no provider may offer a partial edit for it.
#[test]
fn tp_receiver_unexport_keeps_the_inherited_callback_out_of_the_family() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Base {\n\
             method tick {} { return 1 }\n\
         }\n\
         oo::class create Child {\n\
             superclass Base\n\
             unexport tick\n\
             method wire {} {\n\
                 after idle [list [self] tick]\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(7).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    let from_declaration = lsp.references(&uri, 1, 11, false);
    assert!(
        !location_lines(&from_declaration, &uri).contains(&7),
        "an unexported receiver name is not a callback call site: {from_declaration:?}"
    );
    let definition = lsp.definition(&uri, 7, callback_char);
    assert!(
        definition.is_null() || definition.as_array().is_some_and(Vec::is_empty),
        "definition from a suppressed callback cursor must be empty: {definition:?}"
    );
    assert!(
        lsp.prepare_rename(&uri, 7, callback_char).is_null(),
        "prepareRename must reject a callback the receiver cannot dispatch"
    );
    assert!(
        lsp.rename(&uri, 7, callback_char, "tock").is_null(),
        "rename must not offer a partial ancestor-only edit"
    );
}

/// #1705: an override answers the capture, so the base's family must not
/// claim it — renaming `Base::tick` would otherwise rewrite a word that never
/// called it.
#[test]
fn tp_overriding_receiver_keeps_its_callback_in_its_own_family() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Base {\n\
             method tick {} { return 1 }\n\
         }\n\
         oo::class create Child {\n\
             superclass Base\n\
             method tick {} { return 2 }\n\
             method wire {} {\n\
                 after idle [list [self] tick]\n\
             }\n\
         }\n";
    let callback_char = u32::try_from(src.lines().nth(7).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&uri, src);

    assert!(
        !location_lines(&lsp.references(&uri, 1, 11, false), &uri).contains(&7),
        "the base declaration must not collect the override's capture"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 7, callback_char), &uri),
        std::collections::BTreeSet::from([5]),
        "the capture resolves to the override"
    );
    assert!(
        location_lines(&lsp.references(&uri, 5, 11, false), &uri).contains(&7),
        "the override's own family collects it"
    );
}

/// #1705: `Base` and `Child` in separate documents.  The workspace tier must
/// answer with the same effective provider the single-document tier does, in
/// references, lens, rename and call hierarchy alike.
#[test]
fn tp_cross_file_inherited_self_callback_agrees_everywhere() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method tick {} { return 1 }\n}\n",
    );
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let callback_char =
        u32::try_from(child_src.lines().nth(3).unwrap().rfind("tick").unwrap()).unwrap();
    lsp.open_ready(&child, child_src);

    assert_eq!(
        location_lines(&lsp.definition(&child, 3, callback_char), &base),
        std::collections::BTreeSet::from([1]),
        "the cross-file provider answers go-to-definition"
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&3),
        "cross-file callback missing from references: {refs:?}"
    );
    let lens = resolve_lens_on_line(&mut lsp, &base, 1);
    assert!(
        location_lines(&lens_locations(&lens), &child).contains(&3),
        "cross-file callback missing from lens: {lens:?}"
    );
    let edits = rename_edits(&lsp.rename(&child, 3, callback_char, "tock"));
    assert!(edit_lines(&edits, &base).contains(&1), "{edits:?}");
    assert!(edit_lines(&edits, &child).contains(&3), "{edits:?}");

    let item = lsp.prepare_call_hierarchy(&base, 1, 11)[0].clone();
    let incoming = lsp.incoming_calls(item);
    assert!(
        incoming.to_string().contains("wire"),
        "cross-file callback missing from hierarchy: {incoming:?}"
    );
}

/// #1705: the mirror of the suppression case, and the one a provider-only
/// reading gets wrong in the other direction.  `Base` unexports its own
/// `tock`, the sibling document's `Child` exports the inherited name, and
/// tclsh 8.6.16 / 9.0.4 run `Base`'s body for `[Child new] tock`.
#[test]
fn tp_cross_file_receiver_reexport_revives_the_inherited_callback() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method tock {} { return 1 }\n    unexport tock\n}\n",
    );
    let child_src = "oo::class create Child {\n    superclass Base\n    export tock\n    method wire {} {\n        after idle [list [self] tock]\n    }\n}\n";
    let callback_char =
        u32::try_from(child_src.lines().nth(4).unwrap().rfind("tock").unwrap()).unwrap();
    lsp.open_ready(&child, child_src);

    assert_eq!(
        location_lines(&lsp.definition(&child, 4, callback_char), &base),
        std::collections::BTreeSet::from([1]),
        "a re-exporting receiver revives the inherited callback target"
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&4),
        "the revived cross-file callback is a reference: {refs:?}"
    );
}

/// #1705: a class of the same simple name in an unrelated document shares no
/// family, so its identically-spelled capture must stay out of the answer.
#[test]
fn tp_unrelated_same_named_class_keeps_its_callback_isolated() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let other = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method tick {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &other,
        "namespace eval other {\n    oo::class create Base {\n        method tick {} { return 2 }\n        method wire {} {\n            after idle [list [self] tick]\n        }\n    }\n}\n",
    );

    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        !location_lines(&refs, &other).contains(&4),
        "an unrelated namespace's class must not share the family: {refs:?}"
    );
}

/// #1703: a tcllib-shaped namespace capture may wrap a command-prefix builder.
/// `my` retains current-object/private dispatch, so all navigation providers
/// must agree even when the target was explicitly unexported.
#[test]
fn tp_namespace_wrapped_my_callback_agrees_across_navigation_providers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\n\
         oo::class create C {\n\
             method read {} { return {} }\n\
             unexport read\n\
             method wire {chan} {\n\
                 fileevent $chan readable [namespace code [list my read]]\n\
             }\n\
         }\n",
    );

    let refs = lsp.references(&uri, 2, 11, false);
    assert_eq!(
        location_lines(&refs, &uri),
        std::collections::BTreeSet::from([3, 5])
    );
    let command = resolve_lens_on_line(&mut lsp, &uri, 2);
    assert_eq!(command["title"], expected_title(2), "{command:?}");
    assert_eq!(
        location_lines(&lens_locations(&command), &uri),
        std::collections::BTreeSet::from([3, 5]),
        "{command:?}"
    );
    assert_eq!(
        location_lines(&lsp.definition(&uri, 5, 51), &uri),
        std::collections::BTreeSet::from([2])
    );
    assert_eq!(lsp.prepare_rename(&uri, 5, 51)["placeholder"], "read");
    assert_eq!(
        edit_lines(&rename_edits(&lsp.rename(&uri, 5, 51, "consume")), &uri),
        vec![2, 5, 3]
    );
}

/// #1705: the receiver class is local but its inherited private provider is
/// in a sibling file.  The typed `my` callback must seed the workspace MRO,
/// so definition, references, lens, rename and hierarchy agree on `Base::read`.
#[test]
fn tp_cross_file_inherited_wrapped_my_callback_agrees_everywhere() {
    let mut lsp = Lsp::tcl();
    let base = unique_uri("tcl");
    let child = unique_uri("tcl");
    lsp.open_ready(
        &base,
        "oo::class create Base {\n    method read {} { return {} }\n    unexport read\n}\n",
    );
    lsp.open_ready(
        &child,
        "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n",
    );

    assert_eq!(
        location_lines(&lsp.definition(&child, 3, 60), &base),
        std::collections::BTreeSet::from([1])
    );
    let refs = lsp.references(&base, 1, 11, true);
    assert!(
        location_lines(&refs, &child).contains(&3),
        "cross-file callback missing from references: {refs:?}"
    );
    let lens = resolve_lens_on_line(&mut lsp, &base, 1);
    assert!(
        location_lines(&lens_locations(&lens), &child).contains(&3),
        "cross-file callback missing from lens: {lens:?}"
    );
    let edits = rename_edits(&lsp.rename(&child, 3, 60, "consume"));
    assert!(edit_lines(&edits, &base).contains(&1), "{edits:?}");
    assert!(edit_lines(&edits, &child).contains(&3), "{edits:?}");

    let item = lsp.prepare_call_hierarchy(&base, 1, 11)[0].clone();
    let incoming = lsp.incoming_calls(item);
    assert!(
        incoming.to_string().contains("wire"),
        "cross-file callback missing from hierarchy: {incoming:?}"
    );
}

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

/// TP (issue #994 C5b / #1143): a receiver typed only by the compiler's
/// object-type lattice — `set b [$a make]`, the method-return edge — must be
/// seen identically by Find All References, the code lens, and rename.
/// Before the unification these keyed off the weaker `instance_classes` map
/// and missed the site that hover / semantic tokens already resolved.
/// tclsh9.0-verified: `[$a make]` returns the `B` instance, so `$b greet`
/// dispatches `::B::greet`.
#[test]
fn tp_lattice_typed_receiver_agrees_across_references_lens_and_rename() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create A { method make {} { ::return [::B new] } }\n\
         oo::class create B {\n    method greet {} { ::return \"hi\" }\n}\n\
         set a [A new]\n\
         set b [$a make]\n\
         $b greet\n",
    );

    // Cursor on `greet`'s declaration (line 2, col 11).
    let refs = lsp.references(&uri, 2, 11, false);
    let ref_sites = all_location_lines(&refs);
    assert!(
        ref_sites.contains(&(uri.clone(), 6)),
        "Find All References must reach the lattice-typed `$b greet`: {refs:?}"
    );

    let command = resolve_lens_on_line(&mut lsp, &uri, 2);
    let lens_sites = all_location_lines(&lens_locations(&command));
    assert_eq!(
        lens_sites, ref_sites,
        "the lens click must open exactly what Find All References returns: {command:?}"
    );
    assert_eq!(
        command["title"],
        expected_title(ref_sites.len()),
        "the lens count must match: {command:?}"
    );

    let edits = rename_edits(&lsp.rename(&uri, 2, 11, "salute"));
    assert!(
        edit_lines(&edits, &uri).contains(&6),
        "rename must rewrite the lattice-typed call site too: {edits:?}"
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

// Issue #1168 — suppression must reach the declaring document too.

/// TP: the suppression direction of the class-side channel, queried from the
/// **declaring document itself**.  `decl.tcl` declares and dispatches a
/// class-side `cm`; `flip.tcl` `self unexport`s it.  The in-document provider
/// resolves `C cm` from its local tables, so before the fix the cross-file
/// flip suppressed the member for every document *except* the one declaring
/// the class — exactly where the author is navigating.
///
/// Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create C { self { method cm {} { return 1 } } }
/// oo::define C { self unexport cm }
/// C cm    ;# -> unknown method "cm": must be create, destroy or new
/// ```
#[test]
fn tp_cross_file_self_unexport_suppresses_in_the_declaring_document() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    let src = concat!(
        "oo::class create C {\n",
        "    self { method cm {} { return 1 } }\n",
        "}\n",
        "C cm\n",
    );
    let flip = unique_uri("tcl");
    lsp.open_ready(&flip, "oo::define C { self unexport cm }\n");
    lsp.open_ready(&decl, src);
    assert!(
        locations(&lsp.definition(&decl, 3, 3)).is_empty(),
        "a cross-file `self unexport` must suppress the declaring document's own dispatch",
    );
    // Hover shares the tier order, so it must decline at the same site.
    assert!(
        hover_text(&lsp.hover(&decl, 3, 3)).is_empty(),
        "hover must not describe a member the visibility union suppresses",
    );
}

/// TP: a cross-file `self deletemethod` tombstone suppresses the declaring
/// document's dispatch the same way (the member is gone, not merely
/// unexported).
#[test]
fn tp_cross_file_self_deletemethod_suppresses_in_the_declaring_document() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    let src = concat!(
        "oo::class create C {\n",
        "    self { method cm {} { return 1 } }\n",
        "}\n",
        "C cm\n",
    );
    let flip = unique_uri("tcl");
    lsp.open_ready(&flip, "oo::define C { self deletemethod cm }\n");
    lsp.open_ready(&decl, src);
    assert!(
        locations(&lsp.definition(&decl, 3, 3)).is_empty(),
        "a cross-file `self deletemethod` must suppress the declaring document's own dispatch",
    );
}

/// TN (CRITICAL FP guard): with no cross-file flip in view, the declaring
/// document's own `C cm` must keep resolving — the gate abstains without
/// positive suppression evidence.
#[test]
fn tn_declaring_document_dispatch_survives_without_suppressing_evidence() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        concat!(
            "oo::class create C {\n",
            "    self { method cm {} { return 1 } }\n",
            "}\n",
            "C cm\n",
        ),
    );
    assert!(
        !locations(&lsp.definition(&decl, 3, 3)).is_empty(),
        "the declaring document's class-side dispatch must keep resolving",
    );
}

/// TN (CRITICAL FP guard): the gate is class-side only — an identically-named
/// *instance* method dispatched from the declaring document must survive the
/// class-side flip (the two sides never share a visibility record).
#[test]
fn tn_declaring_document_instance_dispatch_survives_a_class_side_flip() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    let src = concat!(
        "oo::class create C {\n",
        "    method cm {} { return 1 }\n",
        "    self { method cm {} { return 2 } }\n",
        "}\n",
        "set o [C new]\n",
        "$o cm\n",
    );
    let flip = unique_uri("tcl");
    lsp.open_ready(&flip, "oo::define C { self unexport cm }\n");
    lsp.open_ready(&decl, src);
    assert!(
        !locations(&lsp.definition(&decl, 5, 4)).is_empty(),
        "an instance `$o cm` in the declaring document must survive a class-side unexport",
    );
}

// Issue #1170 — per-object member state reaches dispatch resolution.

/// TP: `oo::objdefine $o { unexport m }` masks a class-provided member for
/// this object's external dispatch — oracle (tclsh 9.0.4 / 8.6.14):
///
/// ```tcl
/// oo::class create C { method m {} { return 1 } } ; set o [C new]
/// oo::objdefine $o { unexport m } ; $o m
/// ;# -> unknown method "m": must be destroy or n
/// ```
///
/// so `$o m` must resolve to nothing (and hover must decline), while a
/// sibling instance keeps the class dispatch.
#[test]
fn tp_per_object_unexport_masks_the_objects_own_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method m {} { return 1 }\n",
            "}\n",
            "set o [C new]\n",
            "set p [C new]\n",
            "oo::objdefine $o { unexport m }\n",
            "$o m\n",
            "$p m\n",
        ),
    );
    assert!(
        locations(&lsp.definition(&uri, 6, 3)).is_empty(),
        "a per-object unexport must mask `$o m`",
    );
    assert!(
        hover_text(&lsp.hover(&uri, 6, 3)).is_empty(),
        "hover must not describe a per-object-masked member",
    );
    // TN (CRITICAL FP guard): the sibling object is untouched.
    assert!(
        !locations(&lsp.definition(&uri, 7, 3)).is_empty(),
        "a sibling instance keeps the class dispatch",
    );
}

/// TP: `oo::objdefine $o { export M }` revives a member the `TclOO` name rule
/// left unexported — `$o M` dispatches (tclsh 9.0.4 / 8.6.14) and must
/// resolve to the class's declaration.
#[test]
fn tp_per_object_export_revives_an_unexported_member() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method M {} { return 1 }\n",
            "}\n",
            "set o [C new]\n",
            "oo::objdefine $o { export M }\n",
            "$o M\n",
        ),
    );
    let def = lsp.definition(&uri, 5, 3);
    assert_eq!(
        start_lines(&def),
        [1].into_iter().collect(),
        "a per-object export must revive the member's dispatch: {def:?}"
    );
}

/// TN: an unexported per-object member (`method M` under the name rule)
/// masks the name for external dispatch even though the same class exports
/// an `m`-style sibling — `$o M` is `unknown method` in real Tcl.
#[test]
fn tn_an_unexported_per_object_member_does_not_navigate_externally() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "}\n",
            "set o [C new]\n",
            "oo::objdefine $o { method Hidden {} { return 1 } }\n",
            "$o Hidden\n",
        ),
    );
    assert!(
        locations(&lsp.definition(&uri, 4, 4)).is_empty(),
        "an unexported per-object member must not resolve for `$o Hidden`",
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

/// TP, issue #1119 review (Codex P2): a **pure consumer** file — one that
/// contains only `C cm` and declares no part of `C` — must resolve the
/// class-side member of a class declared in another document.
///
/// The receiver *classification* used to run on the local analysis alone:
/// `classmethod_dispatch_class` needs the class's own `class_methods` table,
/// and the server's workspace-oracle reanalysis supplies class **names** only.
/// So the request fell through both receiver branches and definition /
/// references / rename from an ordinary consumer answered nothing, even though
/// the workspace class-side dispatch chain held the fact.
#[test]
fn tp_pure_consumer_resolves_a_cross_file_class_side_member() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "oo::class create C {\n    self { method cm {} { return 1 } }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "C cm\n");

    // Go-to-definition lands on the declaration in the other document.
    let defs = locations(&lsp.definition(&consumer, 0, 3));
    assert_eq!(
        defs.iter().map(|l| l.uri.as_str()).collect::<Vec<_>>(),
        [decl.as_str()],
        "a pure consumer must reach the class-side declaration",
    );

    // References from the consumer see both the declaration and this call.
    let refs = lsp.references(&consumer, 0, 3, true);
    let uris: std::collections::BTreeSet<String> =
        locations(&refs).iter().map(|l| l.uri.clone()).collect();
    assert!(
        uris.contains(&decl) && uris.contains(&consumer),
        "references from the consumer must span both documents: {uris:?}"
    );

    // Rename from the consumer rewrites the declaration and the call site.
    let edits = rename_edits(&lsp.rename(&consumer, 0, 3, "renamed"));
    assert_eq!(
        edit_lines(&edits, &consumer),
        vec![0],
        "the consumer's own call must rename: {edits:?}"
    );
    assert_eq!(
        edit_lines(&edits, &decl),
        vec![1],
        "the cross-file declaration must rename too: {edits:?}"
    );
}

// Issue #1121 review — renaming a moved member must not produce a class that
// cannot run.  The moved member's declaration site IS the `renamemethod`'s
// destination word, so the edit rewrites that word; for some new names the
// result is a body real Tcl refuses (the shapes W315 diagnoses).

/// The refusal reason for a rename request, or `None` when it produced edits.
fn rename_refusal_reason(
    lsp: &mut Lsp,
    uri: &str,
    line: u32,
    ch: u32,
    new: &str,
) -> Option<String> {
    lsp.rename_error(uri, line, ch, new)
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// TP: renaming the moved member back to its own retracted source produces
/// `renamemethod old old`, which fails `cannot rename method to itself` and
/// creates no class (tclsh 9.0.4 / 8.6.14).
#[test]
fn tp_renaming_a_moved_member_to_its_source_is_refused() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create C {\n    method old {} { return 1 }\n    renamemethod old new\n}\n",
    );
    let reason = rename_refusal_reason(&mut lsp, &uri, 2, 21, "old")
        .expect("renaming `new` back to `old` must be refused");
    assert!(
        reason.contains("cannot rename method to itself"),
        "the refusal must name the interpreter error: {reason}"
    );
}

/// TP: renaming the moved member onto a **live same-side sibling** produces
/// `renamemethod old sib`, which fails `method called sib already exists`.
#[test]
fn tp_renaming_a_moved_member_onto_a_live_sibling_is_refused() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method old {} { return 1 }\n",
            "    method sib {} { return 2 }\n",
            "    renamemethod old new\n",
            "}\n",
        ),
    );
    let reason = rename_refusal_reason(&mut lsp, &uri, 3, 21, "sib")
        .expect("renaming `new` onto the live `sib` must be refused");
    assert!(
        reason.contains("method called sib already exists"),
        "the refusal must name the interpreter error: {reason}"
    );
}

/// TP, the mirror: renaming an ordinary sibling *into* the name a later
/// `renamemethod` needs free produces the same collision at that site
/// (`method new {} …; renamemethod old new` -> `method called new already
/// exists`, oracle-pinned on both interpreters).
#[test]
fn tp_renaming_a_sibling_onto_a_later_rename_destination_is_refused() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method old {} { return 1 }\n",
            "    method sib {} { return 2 }\n",
            "    renamemethod old new\n",
            "}\n",
        ),
    );
    let reason = rename_refusal_reason(&mut lsp, &uri, 2, 11, "new")
        .expect("renaming `sib` onto the later rename destination must be refused");
    assert!(
        reason.contains("method called new already exists"),
        "the refusal must name the interpreter error: {reason}"
    );
}

/// TN: a fresh destination is a perfectly good rename and must still produce
/// edits — the gate is about the two aborting shapes, nothing wider.
#[test]
fn tn_renaming_a_moved_member_to_a_fresh_name_still_works() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create C {\n    method old {} { return 1 }\n    renamemethod old new\n}\n",
    );
    let edits = rename_edits(&lsp.rename(&uri, 2, 21, "fresh"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![2],
        "a fresh destination must rewrite the renamemethod's destination word: {edits:?}"
    );
}

/// TN (CRITICAL): a name live only on the **other** side is not a collision —
/// `method old {} …; self { method sib {} … }; renamemethod old sib` runs fine
/// and leaves the instance side with `sib` (9.0.4 / 8.6.14 identical).
#[test]
fn tn_renaming_a_moved_member_onto_a_cross_side_name_is_allowed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method old {} { return 1 }\n",
            "    self { method sib {} { return 2 } }\n",
            "    renamemethod old new\n",
            "}\n",
        ),
    );
    let edits = rename_edits(&lsp.rename(&uri, 3, 21, "sib"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![3],
        "a class-side `sib` must not block an instance-side rename: {edits:?}"
    );
}

/// TN: a destination **deleted before** the rename is free — the check reads
/// the side's table at the move's own point in the body, as Tcl does.
#[test]
fn tn_renaming_a_moved_member_onto_an_earlier_deleted_name_is_allowed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create C {\n",
            "    method old {} { return 1 }\n",
            "    method sib {} { return 2 }\n",
            "    deletemethod sib\n",
            "    renamemethod old new\n",
            "}\n",
        ),
    );
    let edits = rename_edits(&lsp.rename(&uri, 4, 21, "sib"));
    assert_eq!(
        edit_lines(&edits, &uri),
        vec![4],
        "`sib` was deleted before the rename, so it is free: {edits:?}"
    );
}

// Issue #1178 review — a bare receiver resolves like any other command word:
// against the namespace in effect where it is written, then the global one,
// then through `namespace import`.  A literal `C`/`::C` match missed the
// relative spelling, which is how a namespaced class is normally called.

/// TP: `namespace eval ::a { C cm }` in a pure consumer must reach `::a::C`
/// declared in another document.  Oracle (9.0.4 / 8.6.14): the call answers
/// `a-C-cm`.
#[test]
fn tp_pure_consumer_resolves_a_relatively_named_namespaced_class() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval ::a {\n    oo::class create C {\n        self { method cm {} { return 1 } }\n    }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "namespace eval ::a {\n    C cm\n}\n");
    assert_eq!(
        locations(&lsp.definition(&consumer, 1, 7))
            .iter()
            .map(|l| l.uri.as_str())
            .collect::<Vec<_>>(),
        [decl.as_str()],
        "a relatively-named namespaced class must resolve cross-file",
    );
}

/// TN (CRITICAL): with **both** `::a::C` and a global `::C` indexed, a call
/// inside `::a` resolves the inner one — Tcl's current-namespace-first rule.
/// Oracle: inside `::a` the call answers `a-C-cm`, at global scope `global-C-cm`
/// (identical on 9.0.4 and 8.6.14).
#[test]
fn tn_an_inner_namespaced_class_shadows_the_global_one() {
    let mut lsp = Lsp::tcl();
    let inner = unique_uri("tcl");
    lsp.open_ready(
        &inner,
        "namespace eval ::a {\n    oo::class create C {\n        self { method cm {} { return 1 } }\n    }\n}\n",
    );
    let global = unique_uri("tcl");
    lsp.open_ready(
        &global,
        "oo::class create ::C {\n    self { method cm {} { return 2 } }\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(&consumer, "namespace eval ::a {\n    C cm\n}\nC cm\n");
    assert_eq!(
        locations(&lsp.definition(&consumer, 1, 7))
            .iter()
            .map(|l| l.uri.as_str())
            .collect::<Vec<_>>(),
        [inner.as_str()],
        "inside ::a the inner class must win",
    );
    assert_eq!(
        locations(&lsp.definition(&consumer, 3, 3))
            .iter()
            .map(|l| l.uri.as_str())
            .collect::<Vec<_>>(),
        [global.as_str()],
        "at global scope the global class must win",
    );
}

/// TP: a class made reachable by `namespace import` resolves under the
/// imported name.  Oracle: `namespace eval ::user { namespace import ::lib::* ;
/// W cm }` answers `lib-W-cm` on both interpreters.
#[test]
fn tp_pure_consumer_resolves_an_imported_class() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval ::lib {\n    oo::class create W {\n        self { method cm {} { return 1 } }\n    }\n    namespace export W\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(
        &consumer,
        "namespace eval ::user {\n    namespace import ::lib::*\n    W cm\n}\n",
    );
    assert_eq!(
        locations(&lsp.definition(&consumer, 2, 7))
            .iter()
            .map(|l| l.uri.as_str())
            .collect::<Vec<_>>(),
        [decl.as_str()],
        "an imported class must resolve through the import",
    );
}

/// TN (the C7 invariant): an import that has **not run yet** at the call's own
/// site binds nothing, so the call must not resolve.  Oracle, identical on
/// 9.0.4 and 8.6.14:
///
/// ```tcl
/// namespace eval ::u1 { W cm ; namespace import ::lib::* }
/// ;# -> invalid command name "W"
/// ```
#[test]
fn tn_a_not_yet_run_import_does_not_resolve_the_receiver() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval ::lib {\n    oo::class create W {\n        self { method cm {} { return 1 } }\n    }\n    namespace export W\n}\n",
    );
    let consumer = unique_uri("tcl");
    lsp.open_ready(
        &consumer,
        "namespace eval ::u1 {\n    W cm\n    namespace import ::lib::*\n}\n",
    );
    assert!(
        locations(&lsp.definition(&consumer, 1, 7)).is_empty(),
        "a call written before its import must not resolve",
    );
}

// Issue #1019 idx 16 — a class member hovers at its own declaration.

/// TP (end-to-end): hovering a method's own name token describes that method,
/// in the `oo::class create` block **and** in a later `oo::define` block that
/// reopens the same class.
///
/// The reopening block is the `SpiceGenTcl` `generalClasses.tcl` shape
/// (`oo::configurable create Parameter { … }` followed by `oo::define
/// Parameter { method <WriteProp-value> … }`).  Oracle, tclsh 9.0.4: the
/// method the second block declares really is dispatchable on the class the
/// first block created, so both declarations describe members of one class
/// and must hover identically.
///
/// Before this, neither block hovered at a declaration at all — only call
/// sites did — which made the reopening half look like a block-specific bug.
#[test]
fn tp_member_declaration_hovers_in_both_class_blocks() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Widget {\n",
            "    method show {} { return shown }\n",
            "}\n",
            "oo::define Widget {\n",
            "    method reopened {val} { return $val }\n",
            "}\n",
            "Widget create w\n",
            "w reopened hi\n",
        ),
    );

    let creation = hover_text(&lsp.hover(&uri, 1, 13));
    assert!(
        creation.contains("::Widget::show"),
        "creation-block declaration must hover: {creation:?}"
    );

    let reopened = hover_text(&lsp.hover(&uri, 4, 13));
    assert!(
        reopened.contains("::Widget::reopened"),
        "reopening-block declaration must hover: {reopened:?}"
    );
    assert!(
        reopened.contains("1 param"),
        "…with the member's real signature: {reopened:?}"
    );

    // The declaration and its call site describe the same member the same
    // way — the asymmetry idx 16 reported is gone in both directions.
    let call_site = hover_text(&lsp.hover(&uri, 7, 4));
    assert_eq!(
        reopened, call_site,
        "declaration and call site must render identically",
    );
}
