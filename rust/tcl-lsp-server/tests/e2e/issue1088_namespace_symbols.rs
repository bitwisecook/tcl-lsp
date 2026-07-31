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

//! Issue #1088 — namespace names as navigable symbols.
//!
//! Split out of issue #923 audit idx 75's secondary claim: a namespace *name*
//! written as an argument (`namespace children ::tomato`, `namespace exists`,
//! `namespace delete`, `namespace upvar`, and the `namespace eval` target
//! itself) had no definition / hover / references answer at all, even in a
//! single file — namespaces were modelled as containers, never as symbols.
//!
//! The oracle every positive case here is written against, pinned on tclsh
//! 9.0.4 and 8.6.16 and byte-identical on both:
//!
//! ```text
//! namespace eval ::a { variable x 1 }
//! namespace eval ::a { variable y 2 }
//! namespace exists ::a      -> 1          ;# one namespace, two declaring blocks
//! info vars ::a::*          -> ::a::x ::a::y
//!
//! namespace eval ::p::q::r {}
//! namespace exists ::p      -> 1          ;# parents created implicitly
//!
//! proc ::nope::gone::p {} {} -> can't create procedure "::nope::gone::p": unknown namespace
//! set ::brandnew::v 1        -> can't set "::brandnew::v": parent namespace doesn't exist
//!
//! namespace eval ::outer { namespace eval inner {} ; namespace exists inner } -> 1
//! namespace exists inner    -> 0          ;# …at global scope, a different name
//!
//! namespace delete ::d ; namespace exists ::d -> 0
//! namespace exists ::never  -> 0
//! namespace children ::never -> namespace "::never" not found        (rc 1)
//! namespace delete ::never   -> unknown namespace "::never" in namespace delete command (rc 1)
//! ```

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

// TP — the finding's own shape: `namespace children ::mypkg` jumps to *every*
// `namespace eval mypkg` block, because reopening extends the one namespace.
#[test]
fn tp_namespace_children_argument_reaches_every_declaring_block() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval mypkg {\n\
         \x20   variable version 1.0\n\
         }\n\
         namespace eval mypkg {\n\
         \x20   proc helper {} { return 1 }\n\
         }\n\
         set targets [namespace children ::mypkg]\n",
    );

    let def = lsp.definition(&uri, 6, 33);
    assert_eq!(
        lines_in(&def, &uri),
        vec![0, 3],
        "both `namespace eval mypkg` blocks are declaring sites: {:?}",
        locations(&def),
    );
}

// TP — hover names the namespace rather than showing nothing.
#[test]
fn tp_namespace_name_hover_names_the_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval tomato {}\nputs [namespace exists ::tomato]\n",
    );

    let text = hover_text(&lsp.hover(&uri, 1, 26));
    assert!(
        text.contains("::tomato") && text.contains("Namespace"),
        "hover must describe the namespace, got: {text:?}",
    );
}

// TP — find-references from a declaring block reaches every other spelling,
// and the declarations themselves when asked for.
#[test]
fn tp_references_from_the_declaration_reach_every_spelling() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval mypkg {}\n\
         namespace children ::mypkg\n\
         namespace exists mypkg\n\
         namespace delete ::mypkg\n",
    );

    let refs = lsp.references(&uri, 0, 16, true);
    assert_eq!(
        lines_in(&refs, &uri),
        vec![0, 1, 2, 3],
        "declaration + children + exists + delete: {:?}",
        locations(&refs),
    );
}

// TP — cross-file: the consumer document declares nothing, the declaring
// `namespace eval` lives in a sibling.  This is the tier the qualified
// variable work (#1086) established, applied to namespaces.
#[test]
fn tp_cross_file_namespace_definition() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(
        &decl,
        "namespace eval mypkg {\n    variable version 1.0\n}\n",
    );
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "set targets [namespace children ::mypkg]\n");

    let def = lsp.definition(&user, 0, 33);
    assert_eq!(
        lines_in(&def, &decl),
        vec![0],
        "the sibling document's `namespace eval mypkg` must be the target: {:?}",
        locations(&def),
    );
}

// TP — cross-file hover and references, both directions.
#[test]
fn tp_cross_file_namespace_hover_and_references() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval tomato {\n    variable v 1\n}\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts [namespace children ::tomato]\n");

    let text = hover_text(&lsp.hover(&user, 0, 28));
    assert!(
        text.contains("::tomato"),
        "cross-file hover must name the namespace, got: {text:?}",
    );

    let refs = lsp.references(&decl, 0, 16, true);
    assert!(
        lines_in(&refs, &user).contains(&0),
        "the sibling document's use must be a reference: {:?}",
        locations(&refs),
    );
}

// TP — a **relative** name resolves against the enclosing namespace, so the
// two `inner`s below are different namespaces.  Oracle: inside `namespace
// eval ::outer`, `namespace exists inner` is 1; at global scope it is 0.
#[test]
fn tp_relative_namespace_name_resolves_against_the_enclosing_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::outer {\n\
         \x20   namespace eval inner {}\n\
         \x20   namespace children inner\n\
         }\n\
         namespace children inner\n",
    );

    let inside = lsp.definition(&uri, 2, 24);
    assert_eq!(
        lines_in(&inside, &uri),
        vec![1],
        "a relative name inside `::outer` means `::outer::inner`: {:?}",
        locations(&inside),
    );
    let outside = lsp.definition(&uri, 4, 21);
    assert!(
        lines_in(&outside, &uri).is_empty(),
        "the same word at global scope means `::inner`, which nothing declares: {:?}",
        locations(&outside),
    );
}

// TN — a namespace-name-shaped run of text inside a **comment** substitutes
// nothing in real Tcl, so it must resolve to nothing.
#[test]
fn tn_comment_text_is_not_a_namespace_reference() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval mypkg {}\n# namespace children ::mypkg here\n",
    );

    let def = lsp.definition(&uri, 1, 26);
    assert!(
        lines_in(&def, &uri).is_empty(),
        "a comment is not a reference: {:?}",
        locations(&def),
    );
    let refs = lsp.references(&uri, 0, 16, true);
    assert_eq!(
        lines_in(&refs, &uri),
        vec![0],
        "only the declaration itself: {:?}",
        locations(&refs),
    );
}

// TN — the same inside a **braced data word**: `set d {namespace children
// ::mypkg}` is a literal string on both interpreters, never a command.
#[test]
fn tn_braced_data_word_is_not_a_namespace_reference() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval mypkg {}\nset d {namespace children ::mypkg}\n",
    );

    let def = lsp.definition(&uri, 1, 30);
    assert!(
        lines_in(&def, &uri).is_empty(),
        "a braced data word is not code: {:?}",
        locations(&def),
    );
}

// TN — an **unknown** namespace answers nothing rather than guessing at a
// same-tailed one.  Oracle: `namespace children ::never` fails with
// `namespace "::never" not found`, rc 1, on both interpreters.
#[test]
fn tn_unknown_namespace_resolves_to_nothing() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval ::real::mypkg {}\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "namespace children ::mypkg\n");

    let def = lsp.definition(&user, 0, 21);
    assert!(
        locations(&def).is_empty(),
        "`::mypkg` is not `::real::mypkg`: {:?}",
        locations(&def),
    );
}

// FP guard — `namespace tail` / `qualifiers` take an arbitrary string, not a
// namespace.  Oracle: `namespace tail a::b` answers `b` on both interpreters
// whether or not `::a` exists.
#[test]
fn fp_namespace_tail_argument_is_not_a_namespace_reference() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval mypkg {}\nputs [namespace tail mypkg]\n",
    );

    let def = lsp.definition(&uri, 1, 23);
    assert!(
        lines_in(&def, &uri).is_empty(),
        "a `namespace tail` argument is a string, not a namespace: {:?}",
        locations(&def),
    );
}

// FN guard — a same-named **proc** must not be the answer for a namespace
// word, and vice versa: Tcl keeps the two symbol spaces disjoint.
#[test]
fn fn_a_same_named_proc_does_not_answer_a_namespace_query() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc mypkg {} { return 1 }\n\
         namespace eval mypkg {}\n\
         namespace children mypkg\n",
    );

    let def = lsp.definition(&uri, 2, 20);
    assert_eq!(
        lines_in(&def, &uri),
        vec![1],
        "the namespace, not the same-named proc: {:?}",
        locations(&def),
    );
}

// FN guard — `namespace inscope` references, it does not declare.  Oracle:
// `namespace inscope ::never {}` on an unknown namespace fails, while
// `namespace eval ::never {}` creates it.
#[test]
fn fn_inscope_target_is_a_reference_not_a_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::a {}\nnamespace inscope ::a {set x 1}\n",
    );

    let def = lsp.definition(&uri, 1, 20);
    assert_eq!(
        lines_in(&def, &uri),
        vec![0],
        "`inscope`'s target reaches the declaring `eval`, not itself: {:?}",
        locations(&def),
    );
}

// ---------------------------------------------------------------------------
// Review of PR #1112 — three P2 tier-completeness findings.
// ---------------------------------------------------------------------------

// TN (finding 1) — a namespace-name position is **definitive**: it must never
// fall through to command resolution.  `namespace exists string` with
// `::string` declared in a sibling document produced the built-in `string`
// command's documentation, and because that is an answer the server returned
// it and never consulted the cross-document namespace tier at all.
#[test]
fn tn_command_hover_never_claims_a_namespace_word() {
    let mut lsp = Lsp::tcl();
    let decl = unique_uri("tcl");
    lsp.open_ready(&decl, "namespace eval ::string {}\n");
    let user = unique_uri("tcl");
    lsp.open_ready(&user, "puts [namespace exists string]\n");

    let text = hover_text(&lsp.hover(&user, 0, 24));
    assert!(
        !text.contains("built-in command"),
        "command hover claimed a namespace word: {text:?}",
    );
    assert!(
        text.contains("::string") && text.contains("Namespace"),
        "the sibling declaration must answer instead: {text:?}",
    );
}

// TN (finding 1) — the same rule for find-references.  Asking *without*
// declarations from a namespace's only declaring block leaves the local set
// empty, which is exactly what used to route the query to the proc/class
// workspace tier.
#[test]
fn tn_references_never_fall_through_to_the_proc_tier() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(&lib, "proc widget {} { return 1 }\nwidget\n");
    let here = unique_uri("tcl");
    lsp.open_ready(&here, "namespace eval widget {}\n");

    let refs = lsp.references(&here, 0, 16, false);
    assert!(
        lines_in(&refs, &lib).is_empty(),
        "proc sites claimed a namespace word: {:?}",
        locations(&refs),
    );
}

// TN (finding 1) — and for rename.  Prepare must offer no UI, and a client
// that skips prepare must meet a *refusal*, not an empty edit set: an empty
// set falls through to the server's workspace-resolved rename branch, which
// resolved the word as a command and rewrote the same-spelled proc — pointing
// the editor's highlight at a declaration on a completely different line.
#[test]
fn tn_rename_refuses_a_namespace_word_rather_than_renaming_a_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc widget {} { return 1 }\nnamespace eval widget {}\nnamespace children widget\n",
    );

    assert!(
        lsp.prepare_rename(&uri, 2, 20).is_null(),
        "prepare-rename must not offer to rename a namespace word",
    );
    let err = lsp.rename_error(&uri, 2, 20, "gadget");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("is a namespace name"),
        "rename must refuse with a reason naming the namespace, got {err}",
    );
}

// TP (finding 2) — a namespace reopened here **and** next door reports both
// blocks.  Returning as soon as the in-document provider answered (and
// excluding the current URI from the index lookup) reported only the local
// half, contradicting the feature's own contract that every declaring block
// is a target.
//
// Oracle — tclsh 9.0.4 and 8.6.16, byte-identical: sourcing both files,
// `namespace eval mypkg` twice leaves one namespace holding both blocks'
// contents (`info vars ::mypkg::*` -> `::mypkg::v`, `::mypkg::p` callable).
#[test]
fn tp_definition_unions_local_and_sibling_declaring_blocks() {
    let mut lsp = Lsp::tcl();
    let sib = unique_uri("tcl");
    lsp.open_ready(&sib, "namespace eval mypkg {\n    variable v 1\n}\n");
    let here = unique_uri("tcl");
    lsp.open_ready(
        &here,
        "namespace eval mypkg {\n    proc p {} {}\n}\nset t [namespace children ::mypkg]\n",
    );

    let def = lsp.definition(&here, 3, 28);
    assert_eq!(
        lines_in(&def, &here),
        vec![0],
        "the local block is a target"
    );
    assert_eq!(
        lines_in(&def, &sib),
        vec![0],
        "so is the sibling document's block: {:?}",
        locations(&def),
    );
}

// TP (finding 2) — find-references unions both documents' sites too.
#[test]
fn tp_references_union_local_and_sibling_sites() {
    let mut lsp = Lsp::tcl();
    let sib = unique_uri("tcl");
    lsp.open_ready(&sib, "namespace eval mypkg {}\nnamespace exists ::mypkg\n");
    let here = unique_uri("tcl");
    lsp.open_ready(
        &here,
        "namespace eval mypkg {}\nnamespace children ::mypkg\n",
    );

    let refs = lsp.references(&here, 1, 22, true);
    assert_eq!(lines_in(&refs, &here), vec![0, 1], "local sites");
    assert_eq!(
        lines_in(&refs, &sib),
        vec![0, 1],
        "sibling sites: {:?}",
        locations(&refs),
    );
}

// TP (finding 2) — hover counts the whole workspace, not just this file.  A
// hover saying "1 block" while go-to-definition offers three contradicts
// itself.
#[test]
fn tp_hover_counts_declaring_blocks_across_the_workspace() {
    let mut lsp = Lsp::tcl();
    let sib = unique_uri("tcl");
    lsp.open_ready(&sib, "namespace eval mypkg {}\nnamespace eval mypkg {}\n");
    let here = unique_uri("tcl");
    lsp.open_ready(
        &here,
        "namespace eval mypkg {}\nnamespace children ::mypkg\n",
    );

    let text = hover_text(&lsp.hover(&here, 1, 22));
    assert!(
        text.contains("3 `namespace eval` blocks") && text.contains("across 2 documents"),
        "hover must count every declaring block: {text:?}",
    );
}

// TP (finding 3) — the **empty literal** names the global namespace at global
// scope, so `namespace eval {} { … }` is a declaration of `::` and
// `namespace children {}` is a reference to it.
//
// Oracle — tclsh 9.0.4 and 8.6.16, byte-identical:
//   namespace exists {}                       -> 1
//   namespace children {}                     == namespace children ::
//   namespace inscope {} {namespace current}  -> ::
//   namespace eval {} {namespace current}     -> ::
//   namespace eval :: {variable a 1}; namespace eval {} {variable b 1}
//                                             -> both ::a and ::b exist
#[test]
fn tp_empty_literal_is_the_global_namespace_at_global_scope() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval :: {}\nnamespace eval {} {}\nnamespace children {}\n",
    );

    let def = lsp.definition(&uri, 2, 20);
    assert_eq!(
        lines_in(&def, &uri),
        vec![0, 1],
        "both spellings declare the same global namespace: {:?}",
        locations(&def),
    );
    let refs = lsp.references(&uri, 0, 15, true);
    assert_eq!(
        lines_in(&refs, &uri),
        vec![0, 1, 2],
        "every spelling is one symbol: {:?}",
        locations(&refs),
    );
}

// TN (finding 3) — inside another namespace the very same word names a
// namespace that *cannot exist*, so it must resolve to nothing rather than to
// the enclosing namespace's declarations.
//
// Oracle — tclsh 9.0.4 and 8.6.16, byte-identical, inside `namespace eval
// ::outer`:
//   namespace exists {}     -> 0
//   namespace children {}   -> namespace "" not found in "::outer"        (rc 1)
//   namespace eval {} {…}   -> can't create namespace "": only global
//                              namespace can have empty name              (rc 1)
#[test]
fn tn_empty_literal_inside_a_namespace_names_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::outer {\n    namespace exists {}\n}\n",
    );

    let def = lsp.definition(&uri, 1, 21);
    assert!(
        locations(&def).is_empty(),
        "`{{}}` inside `::outer` is not `::outer`: {:?}",
        locations(&def),
    );
}

// TP (finding 3, adjacent) — a **braced** namespace name is an ordinary name,
// so the data-brace inertness test must not veto it.
//
// Oracle — tclsh 9.0.4 and 8.6.16, byte-identical: `namespace eval {my ns} {
// variable v 7; proc p {} {return P} }` gives `namespace exists {my ns}` -> 1,
// `namespace children ::` listing `{::my ns}`, `set {::my ns::v}` -> 7, and
// `{::my ns::p}` -> P.
#[test]
fn tp_a_braced_namespace_name_is_navigable() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval {my ns} { variable v 7 }\nnamespace children {my ns}\n",
    );

    let def = lsp.definition(&uri, 1, 22);
    assert_eq!(
        lines_in(&def, &uri),
        vec![0],
        "a braced namespace name is a real name: {:?}",
        locations(&def),
    );
}
