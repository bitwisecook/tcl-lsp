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

//! Every consumer of `definition::resolve_called_proc` must agree about a call
//! a live `namespace import -force` shadows (issue #1116 item 1).
//!
//! The whole point of that item is that no rule reading only [`MAIN`] can
//! decide it: both directions below hand the providers **byte-identical**
//! source and differ only in what a *sibling* document declares. So every test
//! is a pair — shadowed and not shadowed — over the one fixture, and a
//! threading bug that drops the oracle collapses the pair rather than moving
//! one side, which is why each assertion pins *both* answers.
//!
//! C-Tcl proof — tclsh 8.6.14 and tclsh 9.0.4, both agreeing:
//!
//! ```text
//! # prepended: namespace eval src { namespace export helper }
//! helper 1 2   =>  SRC/1/2     ;# -force replaced ::helper
//! # prepended: namespace eval other { proc q {} { puts Q } }
//! helper 1 2   =>  LOCAL       ;# the import bound nothing
//! ```
//!
//! `::src::helper` takes `{alpha beta}` and `::helper` takes `{args}`, and
//! their bodies print different text, so *which* proc a provider picked is
//! visible in its output — parameter labels, signature text, the item detail,
//! the inlined body — and not only in a span.
//!
//! The call sits at global scope deliberately. A `-force` import rewrites a
//! bare name in the *importing* namespace, and `::` is the one importing
//! namespace whose call sites the inlay-hint segmenter reaches (it walks
//! top-level commands and does not descend into `proc` or `namespace eval`
//! bodies), so this is the one shape that puts every threaded consumer on the
//! same cursor.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::{CallResolution, LspRange, ProgramExports};
use tcl_lsp_core::workspace_index::{NamespaceExportSnapshot, WorkspaceIndex};
use tcl_registry::CommandRegistry;

/// The document every test resolves against — identical in both directions.
const MAIN: &str = "namespace eval src {\n    proc helper {alpha beta} { puts \"SRC/$alpha/$beta\" }\n    proc other {} { puts O }\n    namespace export other\n}\nproc helper {args} { puts LOCAL }\nnamespace import -force ::src::*\nhelper 1 2\n";

/// The sibling that makes the `-force` import bind `helper` — the export the
/// importing document cannot see.
const SIBLING_SHADOWS: &str = "namespace eval src {\n    namespace export helper\n}\n";

/// The sibling that does not: nothing in the program exports `helper`, so the
/// import binds only `other` and the local definition survives. Issue #1116's
/// pinned true negative.
const SIBLING_INERT: &str = "namespace eval other {\n    proc q {} { puts Q }\n}\n";

const MAIN_URI: &str = "file:///main.tcl";

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn reg() -> &'static CommandRegistry {
    tcl_registry::registry_for_dialect("tcl8.6")
}

/// Byte offset of the `helper` call head.
fn call_offset() -> u32 {
    u32::try_from(MAIN.rfind("helper 1 2").expect("call present")).expect("tiny test source")
}

/// Zero-based line of that call (it is the only command on its line).
fn call_line() -> u32 {
    u32::try_from(MAIN[..call_offset() as usize].matches('\n').count()).expect("tiny test source")
}

/// The whole document, as an inlay-hint / code-action range.
const WHOLE_FILE: LspRange = LspRange {
    start_line: 0,
    start_character: 0,
    end_line: 999,
    end_character: 0,
};

/// `MAIN` plus one sibling, indexed, with the export snapshot the server hands
/// its providers.
fn snapshot(sibling: &str) -> (AnalysisResult, std::sync::Arc<NamespaceExportSnapshot>) {
    let main = analyse(MAIN);
    let other = analyse(sibling);
    let mut index = WorkspaceIndex::new();
    index.add_document(MAIN_URI, &main);
    index.add_document("file:///sibling.tcl", &other);
    let exports = index.export_snapshot();
    (main, exports)
}

fn program(exports: &NamespaceExportSnapshot) -> ProgramExports<'_> {
    ProgramExports {
        uri: MAIN_URI,
        oracle: exports,
    }
}

/// The resolution context a workspace-backed host hands every provider.
fn resolution(exports: &NamespaceExportSnapshot) -> CallResolution<'_> {
    CallResolution {
        registry: Some(reg()),
        program: Some(program(exports)),
    }
}

/// Run `probe` against both siblings, returning `(shadowed, unshadowed)`.
fn both_directions<T>(probe: impl Fn(&AnalysisResult, &NamespaceExportSnapshot) -> T) -> (T, T) {
    let (main_s, exports_s) = snapshot(SIBLING_SHADOWS);
    let shadowed = probe(&main_s, &exports_s);
    let (main_i, exports_i) = snapshot(SIBLING_INERT);
    let unshadowed = probe(&main_i, &exports_i);
    (shadowed, unshadowed)
}

// ---------------------------------------------------------------------------
// Go-to-definition — the surface the fix started from, asserted here against
// the same fixture so a regression shows up beside the providers threaded to
// match it.
// ---------------------------------------------------------------------------

#[test]
fn definition_follows_the_shadow() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::definition::definition_with(
            MAIN,
            call_line(),
            0,
            analysis,
            Some(program(exports)),
        )
        .iter()
        .map(|r| r.start_line)
        .collect::<Vec<_>>()
    });
    let src_line = u32::try_from(
        MAIN[..MAIN
            .find("proc helper {alpha beta}")
            .expect("src proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");
    let local_line = u32::try_from(
        MAIN[..MAIN.find("proc helper {args}").expect("local proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");
    assert_eq!(shadowed, vec![src_line], "the import replaced ::helper");
    assert_eq!(
        unshadowed,
        vec![local_line],
        "the import bound nothing, so ::helper survives"
    );
}

// ---------------------------------------------------------------------------
// Inlay hints — the parameter labels name the reached proc's parameters.
// ---------------------------------------------------------------------------

#[test]
fn inlay_hints_label_the_reached_proc_not_the_shadowed_local() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::inlay_hints::inlay_hints_in_program(
            MAIN,
            "tcl8.6",
            WHOLE_FILE,
            Some(analysis),
            resolution(exports),
            false,
            true,
        )
        .into_iter()
        .map(|h| h.label)
        .collect::<Vec<_>>()
    });

    // TP — the call runs `::src::helper {alpha beta}`, so those are the
    // parameter names that belong on the arguments `1` and `2`.
    assert!(
        shadowed.iter().any(|l| l.contains("alpha")) && shadowed.iter().any(|l| l.contains("beta")),
        "a live -force shadow must label the imported proc's parameters: {shadowed:?}"
    );
    // TN — the import bound nothing, so the variadic local `{args}` is reached
    // and contributes no positional labels.
    assert!(
        !unshadowed
            .iter()
            .any(|l| l.contains("alpha") || l.contains("beta")),
        "with no covering export the local proc is reached: {unshadowed:?}"
    );
}

// ---------------------------------------------------------------------------
// Signature help — the rendered parameter list is the reached proc's.
// ---------------------------------------------------------------------------

#[test]
fn signature_help_describes_the_reached_proc_not_the_shadowed_local() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::signature_help::signature_help_in_program(
            MAIN,
            call_line(),
            8,
            analysis,
            resolution(exports),
        )
        .map(|h| h.signatures[0].label.clone())
        .expect("signature help on a resolvable call")
    });

    assert!(
        shadowed.contains("alpha") && shadowed.contains("beta"),
        "a live -force shadow must render `::src::helper`'s signature: {shadowed:?}"
    );
    assert!(
        unshadowed.contains("args") && !unshadowed.contains("alpha"),
        "with no covering export the local `helper {{args}}` is correct: {unshadowed:?}"
    );
}

// ---------------------------------------------------------------------------
// Call hierarchy — `prepare` roots the graph at a call resolution.
// ---------------------------------------------------------------------------

#[test]
fn call_hierarchy_prepare_roots_at_the_reached_proc() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::call_hierarchy::prepare_in_program(
            MAIN,
            call_line(),
            0,
            analysis,
            resolution(exports),
        )
        .into_iter()
        .map(|i| i.detail.unwrap_or_default())
        .collect::<Vec<_>>()
    });

    // The two procs differ in arity, which the item detail carries — so this
    // pins *which* definition the hierarchy is rooted at, not merely that one
    // was found.
    assert_eq!(
        shadowed,
        vec!["(2 params)".to_owned()],
        "a live -force shadow roots the hierarchy at `::src::helper {{alpha beta}}`"
    );
    assert_eq!(
        unshadowed,
        vec!["(1 params)".to_owned()],
        "with no covering export it roots at the local `helper {{args}}`"
    );
}

// ---------------------------------------------------------------------------
// Inline-proc refactor — substituting the wrong body is a behaviour change.
// ---------------------------------------------------------------------------

#[test]
fn inline_proc_inlines_the_reached_body_not_the_shadowed_local() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::refactor::inline_proc_in_program(
            MAIN,
            call_offset(),
            analysis,
            resolution(exports),
        )
        .map(|r| {
            r.edits
                .iter()
                .map(|e| e.new_text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .expect("inline-proc offered on a resolvable call")
    });

    // TP — inlining the local body over a call that runs `::src::helper`
    // would silently change what the program does.
    assert!(
        shadowed.contains("SRC/") && !shadowed.contains("LOCAL"),
        "a live -force shadow must inline the imported body: {shadowed:?}"
    );
    // TN — the local proc really is what runs, so its body is the right one.
    assert!(
        unshadowed.contains("LOCAL") && !unshadowed.contains("SRC/"),
        "with no covering export the local body is the right substitution: {unshadowed:?}"
    );
}

// ---------------------------------------------------------------------------
// Code actions — the same refactor reached through the provider entry point,
// which is the path the server actually takes.
// ---------------------------------------------------------------------------

#[test]
fn code_actions_offer_the_reached_body_for_inlining() {
    let cursor = LspRange {
        start_line: call_line(),
        start_character: 0,
        end_line: call_line(),
        end_character: 0,
    };
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::code_actions::code_actions_in_program(
            MAIN,
            cursor,
            Some(analysis),
            Some(program(exports)),
        )
        .into_iter()
        .filter(|a| a.title.starts_with("Inline proc"))
        .flat_map(|a| a.edits.into_iter().map(|e| e.new_text))
        .collect::<Vec<_>>()
        .join("\n")
    });

    assert!(
        shadowed.contains("SRC/") && !shadowed.contains("LOCAL"),
        "a live -force shadow must offer the imported body: {shadowed:?}"
    );
    assert!(
        unshadowed.contains("LOCAL") && !unshadowed.contains("SRC/"),
        "with no covering export the local body is offered: {unshadowed:?}"
    );
}

// ---------------------------------------------------------------------------
// The document-only tier must be untouched: a host with no workspace index
// (the `tcl` CLI, a single-document test) keeps exactly its old behaviour.
// ---------------------------------------------------------------------------

#[test]
fn without_a_workspace_view_every_provider_keeps_the_document_only_answer() {
    let analysis = analyse(MAIN);
    let doc_only = CallResolution::document_only().with_registry(reg());

    // `MAIN` holds an export record for `::src` (of `other`) but none for
    // `helper`, which the document-only rule reads as "not exported" — so the
    // import binds nothing and the local definition stands.
    let sig = tcl_lsp_core::signature_help::signature_help_in_program(
        MAIN,
        call_line(),
        8,
        &analysis,
        doc_only,
    )
    .expect("document-only signature help");
    assert!(
        sig.signatures[0].label.contains("args"),
        "document-only resolution keeps the local proc: {:?}",
        sig.signatures[0].label
    );

    let hints: Vec<String> = tcl_lsp_core::inlay_hints::inlay_hints_in_program(
        MAIN,
        "tcl8.6",
        WHOLE_FILE,
        Some(&analysis),
        doc_only,
        false,
        true,
    )
    .into_iter()
    .map(|h| h.label)
    .collect();
    assert!(
        !hints.iter().any(|l| l.contains("alpha")),
        "document-only resolution labels no imported parameters: {hints:?}"
    );

    assert_eq!(
        tcl_lsp_core::call_hierarchy::prepare_in_program(MAIN, call_line(), 0, &analysis, doc_only)
            .into_iter()
            .map(|i| i.detail.unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["(1 params)".to_owned()],
        "document-only resolution roots the hierarchy at the local proc"
    );
}

// ---------------------------------------------------------------------------
// The legacy entry points must be exactly their `_in_program` counterparts
// with no view attached — otherwise the delegation quietly changed behaviour
// for every existing caller (the `tcl` CLI and the single-document tests).
// ---------------------------------------------------------------------------

#[test]
fn the_legacy_entry_points_still_equal_the_document_only_program_view() {
    let analysis = analyse(MAIN);
    let doc_only = CallResolution::document_only().with_registry(reg());

    assert_eq!(
        tcl_lsp_core::signature_help::signature_help(MAIN, call_line(), 8, &analysis, Some(reg())),
        tcl_lsp_core::signature_help::signature_help_in_program(
            MAIN,
            call_line(),
            8,
            &analysis,
            doc_only
        ),
        "signature_help delegation changed nothing"
    );

    assert_eq!(
        tcl_lsp_core::inlay_hints::inlay_hints(
            MAIN,
            "tcl8.6",
            WHOLE_FILE,
            Some(&analysis),
            Some(reg()),
            true,
            true,
        ),
        tcl_lsp_core::inlay_hints::inlay_hints_in_program(
            MAIN,
            "tcl8.6",
            WHOLE_FILE,
            Some(&analysis),
            doc_only,
            true,
            true,
        ),
        "inlay_hints delegation changed nothing"
    );

    assert_eq!(
        tcl_lsp_core::call_hierarchy::prepare(MAIN, call_line(), 0, &analysis),
        tcl_lsp_core::call_hierarchy::prepare_in_program(
            MAIN,
            call_line(),
            0,
            &analysis,
            CallResolution::document_only()
        ),
        "call_hierarchy delegation changed nothing"
    );

    let cursor = LspRange {
        start_line: call_line(),
        start_character: 0,
        end_line: call_line(),
        end_character: 0,
    };
    assert_eq!(
        tcl_lsp_core::code_actions::code_actions(MAIN, cursor, Some(&analysis))
            .into_iter()
            .map(|a| a.title)
            .collect::<Vec<_>>(),
        tcl_lsp_core::code_actions::code_actions_in_program(MAIN, cursor, Some(&analysis), None)
            .into_iter()
            .map(|a| a.title)
            .collect::<Vec<_>>(),
        "code_actions delegation changed nothing"
    );
}

// ---------------------------------------------------------------------------
// Rename — retargeting matters most here, because the edit is destructive.
// ---------------------------------------------------------------------------

#[test]
fn rename_retargets_with_the_shadow() {
    // Renaming from a `-force`-shadowed call must rewrite the definition that
    // call *reaches*. Rewriting the local one the import deleted would rename
    // an unrelated proc while leaving the call pointing where it always did.
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::rename::rename_in_program(
            MAIN,
            "tcl8.6",
            call_line(),
            0,
            "renamed",
            analysis,
            resolution(exports),
        )
        .expect("no refusal on a resolvable call")
        .into_iter()
        .map(|e| e.range.start_line)
        .collect::<Vec<_>>()
    });

    let src_line = u32::try_from(
        MAIN[..MAIN
            .find("proc helper {alpha beta}")
            .expect("src proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");
    let local_line = u32::try_from(
        MAIN[..MAIN.find("proc helper {args}").expect("local proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");

    assert!(
        shadowed.contains(&src_line) && !shadowed.contains(&local_line),
        "a live -force shadow retargets the rename to `::src::helper`: {shadowed:?}"
    );
    assert!(
        unshadowed.contains(&local_line) && !unshadowed.contains(&src_line),
        "with no covering export the local definition is renamed: {unshadowed:?}"
    );
}

#[test]
fn prepare_rename_offers_the_reached_definition() {
    let (shadowed, unshadowed) = both_directions(|analysis, exports| {
        tcl_lsp_core::rename::prepare_rename_in_program(
            MAIN,
            call_line(),
            0,
            analysis,
            resolution(exports),
        )
        .map(|p| p.range.start_line)
    });
    let src_line = u32::try_from(
        MAIN[..MAIN
            .find("proc helper {alpha beta}")
            .expect("src proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");
    let local_line = u32::try_from(
        MAIN[..MAIN.find("proc helper {args}").expect("local proc present")]
            .matches('\n')
            .count(),
    )
    .expect("tiny");
    assert_eq!(shadowed, Some(src_line), "prepare follows the shadow");
    assert_eq!(
        unshadowed,
        Some(local_line),
        "prepare keeps the local when nothing is imported"
    );
}
