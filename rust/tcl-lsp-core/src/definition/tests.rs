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

use super::*;

mod call_resolution;
mod command_heads;
mod import_slot_events;
mod method_receivers;
mod namespace_imports;
mod object_dispatch;
mod variables;

use std::fmt::Write as _;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn logical_procedure_navigation_analysis(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn definition_uses_the_complete_original_command_name() {
    for (name, call, cursor) in [(";", "{;}", 1), ("a b", "a\\ b", 2), ("𐐀;x", "{𐐀;x}", 3)] {
        let source = format!("proc {{{name}}} {{}} {{return ok}}\n{call}\nputs {call}\n");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let ranges = definition(&source, 1, cursor, &analysis);
        assert_eq!(ranges.len(), 1, "{source:?} {ranges:?}");
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(
            definition(&source, 2, 5 + cursor, &analysis),
            [] as [crate::definition::LspRange; 0]
        );
        assert_eq!(
            definition(&source.replace("proc", "puts"), 1, cursor, &analysis),
            [] as [crate::definition::LspRange; 0]
        );
    }
}

#[test]
fn logical_definition_calls_retain_original_alias_move_and_redefinition_headers() {
    let source = "proc join {first second} {}\ninterp alias {} joined {} join captured\njoined value\nrename join archived\nproc join {new} {}\narchived alpha beta\njoin one\n";
    let mut analysis = logical_procedure_navigation_analysis(source);
    assert!(analysis.allows_retained_logical_declaration_advice());
    analysis.all_procs.clear();
    analysis.proc_declaration_sites.clear();
    for (needle, expected_line) in [("joined value", 0), ("archived alpha", 0), ("join one", 4)] {
        let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        let call =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                source,
                &analysis,
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
            .expect("genuine original call survives reporting-map removal");
        assert_eq!(
            call.declaration(&analysis)
                .unwrap()
                .metadata()
                .name_span
                .start(),
            if expected_line == 0 {
                5
            } else {
                u32::try_from(source.find("proc join {new}").unwrap() + 5).unwrap()
            }
        );
        let ranges = definition(source, position.line, position.character.get(), &analysis);
        assert_eq!(
            ranges
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![expected_line]
        );
    }
    let offset = u32::try_from(source.find("proc join").unwrap() + 5).unwrap();
    assert_eq!(
        procedure_name_at_cursor(&analysis, offset).unwrap().start(),
        offset
    );
}

#[test]
fn logical_definition_call_refusals_do_not_resurrect_report_procedure_names() {
    let source = "proc helper {value} {}\nrename helper {}\nhelper ignored\ninterp alias {} builtin {} puts\nbuiltin text\nset data helper\n$head data\n";
    let analysis = logical_procedure_navigation_analysis(source);
    for needle in ["helper ignored", "builtin text", "$head data"] {
        let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
        assert!(
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                source, &analysis, offset,
            )
            .is_none()
        );
        let position = LineIndex::new(source).position_at_utf16(offset + 1, source);
        assert_eq!(
            definition(source, position.line, position.character.get(), &analysis),
            [] as [crate::definition::LspRange; 0]
        );
    }
    let offset = u32::try_from(source.find("data helper").unwrap() + 5).unwrap();
    assert!(
        indirect_definition_target(
            &analysis,
            CallResolution::document_only(),
            source,
            offset,
            "helper"
        )
        .is_none()
    );
    let changed = source.replace("helper ignored", "helper altered");
    assert_eq!(
        definition(&changed, 2, 1, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn logical_definition_call_requires_current_complete_analysis_input() {
    let source = "proc helper {value} {}\nhelper data\n";
    let analysis = logical_procedure_navigation_analysis(source);
    assert_eq!(definition(source, 1, 1, &analysis).len(), 1);
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        definition(source, 1, 1, &missing),
        [] as [crate::definition::LspRange; 0]
    );
    let mut foreign = analysis.clone();
    foreign.resolved_input = Analyser::new().analyse(source, "tcl8.6").resolved_input;
    assert_eq!(
        definition(source, 1, 1, &foreign),
        [] as [crate::definition::LspRange; 0]
    );
}

fn external_mask_at(
    source: &str,
    analysis: &AnalysisResult,
    selector: &str,
) -> ExternalObjectDispatchMask {
    let cursor = u32::try_from(source.rfind(selector).unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(cursor, source);
    object_dispatch_mask(analysis, source, position.line, position.character.get())
}

fn assert_original_class_publications() {
    let source = r"oo::class create c\uD800 {}
oo::class create c\uD801 {}
c\uD800";
    let call = u32::try_from(source.rfind(r"c\uD800").unwrap()).unwrap();
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        let span = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .name_span;
        let mut counterfeit = analysis.all_classes.values().next().unwrap().clone();
        counterfeit.name_span = tcl_lexer::Span::new(0, 1);
        analysis.all_classes.clear();
        analysis
            .all_classes
            .insert("::counterfactual".to_owned(), counterfeit);
        assert_eq!(
            resolve_class_target_at(
                &analysis,
                source,
                CallResolution::document_only(),
                call,
                "counterfactual",
            )
            .unwrap()
            .1
            .name_span,
            span,
            "{dialect}"
        );
        assert!(
            resolve_class_target_at(
                &analysis,
                &format!("{source}\n# changed"),
                CallResolution::document_only(),
                call,
                "counterfactual",
            )
            .is_none()
        );
    }
}

#[test]
fn constructed_colon_namespace_context_survives_editor_projection() {
    let source = "namespace eval : {namespace eval : {list HERE}}";
    let offset = u32::try_from(source.find("list HERE").unwrap()).unwrap();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        assert_eq!(
            namespace_context_at(&analysis.global_scope, offset, &[]),
            "::::::",
            "{dialect}"
        );
    }
    for namespace in ["::", ":::", "::a:::b", "::::::"] {
        let overrides = [(tcl_lexer::Span::new(0, 100), namespace.to_owned())];
        assert_eq!(
            namespace_context_at(&tcl_compiler::analyser::Scope::global(), 1, &overrides),
            namespace
        );
    }
}

/// The document-only resolution context: no registry, no whole-program
/// export oracle — what a single-document test resolves against.
const DOC: CallResolution<'static> = CallResolution::document_only();

#[test]
fn jump_to_proc_definition() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the second `greet` (line 1, char 2).
    let locs = definition(src, 1, 2, &analysis);
    assert_eq!(locs.len(), 1);
    // The proc name span is on line 0 starting at column 5.
    assert_eq!(locs[0].start_line, 0);
    assert_eq!(locs[0].start_character, 5);
}

// The import edge's own lifecycle.
//
// The import edge is a *snapshot* fact; these pin it as a **link
// with a lifetime**. Every case is oracle-pinned against tclsh 9.0.4 and
// 8.6.14, byte-identical on both, and drives
// `proc_visible_via_wildcard_import` / `proc_visible_via_forced_import`
// directly for the same reason the export-snapshot block above does: the lenient
// `fallback_proc_by_simple_name` in `resolve_called_proc` would answer
// either way and prove nothing about this gate.

/// Byte offset just past the last occurrence of `needle`.
fn after_last(src: &str, needle: &str) -> u32 {
    u32::try_from(src.rfind(needle).expect("needle present") + needle.len())
        .expect("tiny test source")
}

/// Byte offset of the first occurrence of `needle`.
fn at_first(src: &str, needle: &str) -> u32 {
    u32::try_from(src.find(needle).expect("needle present")).expect("tiny test source")
}

/// Byte offset of the head word of the last call written on its own
/// indented line as `    word`. A call-site query sits on the call's own
/// head word, which is what the original declaration selection reads
/// (test-input correction, user decision 2026-10-18T16:00Z
/// `forced_import_sibling_offsets`; fixtures and expected values are
/// unchanged).
fn last_call_head(src: &str, word: &str) -> u32 {
    let line = format!("    {word}\n");
    u32::try_from(src.rfind(&line).expect("call line present") + 4).expect("tiny test source")
}

// Behaviour 2c — the whole-program export oracle.
//
// The single document [`PARTLY_OBSERVABLE`] is the *same bytes* in every
// test below. Only the program around it changes, and with it the correct
// answer — which is the whole reason this tier needed an oracle. Oracle
// transcript (tclsh 8.6.14 and 9.0.4, byte-identical), the file loaded
// from a two-line loader that does or does not also source an
// `exports.tcl` holding `namespace eval ::src {namespace export helper}`:
//
//   with exports.tcl:      call -> SRC     origin -> ::src::helper
//   without exports.tcl:   call -> LOCAL   origin -> ::app::helper

/// The pinned two-file shape's single document, byte-for-byte.
///
/// `::src` is *partly* observable here: its procs and one of its exports
/// (`other`) are in this file. Whether it also exports `helper` is decided
/// somewhere this document cannot see.
const PARTLY_OBSERVABLE: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

/// A stand-in [`NamespaceExportOracle`] over an explicit table: a
/// namespace absent from it is [`ExportVerdict::Unknown`], one present
/// answers from its listed names.
///
/// Deliberately position-blind: what this fixture isolates is the
/// three-way verdict itself. Ordering a real workspace's export events
/// against an import site is `WorkspaceIndex`'s job, pinned against the
/// real snapshot in
/// [`the_real_oracle_does_not_resurrect_an_export_written_after_the_import`]
/// and in `workspace_index`'s own both-tiers tests.
struct FakeExports(&'static [(&'static str, &'static [&'static str])]);

impl NamespaceExportOracle for FakeExports {
    fn exported_at(
        &self,
        source_ns: &str,
        name: &str,
        _import_site: RunPoint<'_>,
    ) -> ExportVerdict {
        let bare = tcl_syntax::naming::unroot_rooted_key(source_ns).unwrap_or(source_ns);
        match self.0.iter().find(|(ns, _)| *ns == bare) {
            None => ExportVerdict::Unknown,
            Some((_, names)) if names.contains(&name) => ExportVerdict::Exported,
            Some(_) => ExportVerdict::NotExported,
        }
    }
}

/// [`PARTLY_OBSERVABLE`] resolved against `oracle`.
fn partly_observable_target(oracle: &dyn NamespaceExportOracle) -> Option<String> {
    let analysis = analyse(PARTLY_OBSERVABLE);
    let call = last_call_head(PARTLY_OBSERVABLE, "helper");
    let ctx = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle,
    });
    let shadow = forced_import_shadows(&analysis, ctx, "::app", "helper", call);
    let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, ctx);
    // The two halves must not contradict: a live shadow means the local
    // command is gone, so the resolver may never answer with it.
    assert!(
        !(shadow && hit.is_some_and(|p| p.qualified_name == "::app::helper")),
        "shadow fired yet the resolver answered the deleted local command",
    );
    hit.map(|p| p.qualified_name.clone())
}

#[test]
fn no_definition_for_unknown_word() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        definition(src, 0, 6, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn jump_to_class_definition() {
    let src = "oo::class create Greeter {}\nGreeter\n";
    let analysis = analyse(src);
    // Cursor on `Greeter` on line 1.
    let locs = definition(src, 1, 2, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
}

#[test]
fn byte_offset_at_handles_newlines() {
    let src = "abc\ndef\nghi\n";
    let line_index = LineIndex::new(src);
    assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
    assert_eq!(byte_offset_at(&line_index, src, 0, 3), 3);
    assert_eq!(byte_offset_at(&line_index, src, 1, 0), 4);
    assert_eq!(byte_offset_at(&line_index, src, 2, 2), 10);
}

#[test]
fn byte_offset_at_uses_lsp_utf16_columns() {
    let src = "a😀b";
    let line_index = LineIndex::new(src);
    assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
    assert_eq!(byte_offset_at(&line_index, src, 0, 1), 1);
    assert_eq!(byte_offset_at(&line_index, src, 0, 3), 5);
    assert_eq!(byte_offset_at(&line_index, src, 0, 4), 6);
}

#[test]
fn span_to_range_translates_offsets() {
    let src = "abc\ndef\n";
    let line_index = LineIndex::new(src);
    // Span covering `def` (offsets 4..7).
    let span = tcl_lexer::Span::new(4, 7);
    let range = span_to_range(src, &line_index, span);
    assert_eq!(range.start_line, 1);
    assert_eq!(range.start_character, 0);
    assert_eq!(range.end_line, 1);
    assert_eq!(range.end_character, 3);
}

#[test]
fn span_to_range_uses_lsp_utf16_columns() {
    let src = "é😀x\n";
    let line_index = LineIndex::new(src);
    let range = span_to_range(src, &line_index, tcl_lexer::Span::new(6, 7));
    assert_eq!(range.start_line, 0);
    assert_eq!(range.start_character, 3);
    assert_eq!(range.end_line, 0);
    assert_eq!(range.end_character, 4);
}

/// `[self] m` / `[self object] m` is `TclOO`'s own
/// same-object dispatch idiom — `self`, called with no argument (or the
/// explicit `object` word), returns the current object's own command
/// name. It reaches the enclosing class exactly like `my` does, but
/// through a bracketed command substitution rather than a bareword
/// dispatch keyword. Shared by both spellings' tests below.
fn assert_self_bracket_dispatch_resolves_like_my(head: &str) {
    let src = format!(
        "oo::class create C {{\n    method animTick {{}} {{ return 1 }}\n    method anim {{}} {{ {head} animTick }}\n}}\n"
    );
    let call_col = u32::try_from(src.lines().nth(2).unwrap().find("animTick").unwrap()).unwrap();
    let locs = definition(&src, 2, call_col, &analyse(&src));
    assert_eq!(locs.len(), 1, "{locs:?} (head={head:?})");
    assert_eq!(locs[0].start_line, 1);
}

/// The line each version resolves the `nth` occurrence of `word` to.
fn method_body_definition_lines(src: &str, word: &str, nth: usize) -> Vec<Vec<u32>> {
    let offset = src.match_indices(word).nth(nth).unwrap().0;
    let line = u32::try_from(src[..offset].matches('\n').count()).unwrap();
    let column = u32::try_from(offset - src[..offset].rfind('\n').map_or(0, |at| at + 1)).unwrap();
    ["tcl8.6", "tcl9.0", "tcl9.1"]
        .into_iter()
        .map(|version| {
            let analysis = Analyser::new().analyse(src, version).clone();
            definition(src, line, column, &analysis)
                .iter()
                .map(|range| range.start_line)
                .collect()
        })
        .collect()
}

#[test]
fn tn_a_proc_declaration_name_still_resolves_to_itself() {
    // TN — a declaration's own name token is an *argument* of `proc`,
    // so the head gate would refuse it; the declaration-site check runs
    // first and must keep answering.
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let locs = definition(src, 0, 6, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}

#[test]
fn tp_definition_on_an_implicit_parent_namespace_answers_the_covering_prefix() {
    // `namespace eval ::p::q::r {}` really creates
    // `::p::q` (tclsh 8.6.16 / 9.0.4: `namespace exists ::p::q` is 1),
    // but the name is written nowhere on its own.  The answer is the
    // prefix of the deeper word that spells exactly it.
    let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
    let analysis = analyse(src);
    // Line 1, col 20 — inside the `::p::q` reference.
    let locs = definition(src, 1, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
    assert_eq!(locs[0].start_character, 15, "{locs:?}");
    assert_eq!(
        locs[0].end_character, 21,
        "the answer must stop at `::p::q`, not cover `::p::q::r`: {locs:?}",
    );
}

#[test]
fn tn_a_declared_namespace_still_answers_its_own_block() {
    // TN — the implicit fallback only runs when nothing declares the
    // namespace outright.
    let src = "namespace eval ::p::q {}\nnamespace eval ::p::q::r {}\nnamespace children ::p::q\n";
    let analysis = analyse(src);
    let locs = definition(src, 2, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{locs:?}");
}

#[test]
fn constructed_itcl_member_projection_preserves_colon_tail_and_actual_family_guard() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Explicit descriptive rows; no native class or reached dispatch issuer.
    use tcl_compiler::analyser::types::{ClassDef, MethodDef};
    let mut analysis = Analyser::new().analyse("", "tcl");
    let member = |name: &str| MethodDef {
        name: name.to_owned(),
        params: Vec::new(),
        formal_count: tcl_compiler::signature_scan::formal_count::SourceFormalCount::Unknown,
        params_computed: true,
        name_span: tcl_lexer::Span::new(0, 0),
        body_span: tcl_lexer::Span::new(0, 0),
        kind: "classmethod".to_owned(),
        is_self_method: false,
        visibility: "public".to_owned(),
        doc: String::new(),
        forward_target: None,
    };
    let mut class = ClassDef {
        qualified_name: "::茶".to_owned(),
        metaclass: "itcl::class".to_owned(),
        ..ClassDef::default()
    };
    class.class_methods.insert(":".to_owned(), member(":"));
    class
        .class_methods
        .insert("méthode".to_owned(), member("méthode"));
    class
        .methods
        .insert("instance".to_owned(), member("instance"));
    analysis
        .all_classes
        .insert(class.qualified_name.clone(), class);
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let registry = crate::registry_for_dialect_profile(profile);
    assert_eq!(
        itcl_class_proc_at_in_registry(&analysis, registry, "::茶:::"),
        Some((&"::茶".to_owned(), ":".to_owned()))
    );
    assert_eq!(
        itcl_class_proc_at_in_registry(&analysis, registry, "::茶::méthode"),
        Some((&"::茶".to_owned(), "méthode".to_owned()))
    );
    for key in [
        "méthode",
        "::茶::",
        "::茶::missing",
        "::茶::instance",
        "::other::méthode",
    ] {
        assert!(
            itcl_class_proc_at_in_registry(&analysis, registry, key).is_none(),
            "{key}"
        );
    }
    analysis.all_classes.get_mut("::茶").unwrap().metaclass = "oo::class".to_owned();
    assert!(itcl_class_proc_at_in_registry(&analysis, registry, "::茶:::").is_none());
}

/// Definition answers from the analysis' own retained grammar, and an
/// analysis whose metadata context is unavailable (an uninstalled pack
/// overlay, the real route) refuses. tclsh 8.4.20: `p` returns FIRST.
#[test]
fn definition_follows_the_retained_grammar_and_refuses_an_unavailable_context() {
    let source = "proc p {} {return FIRST}; p";
    let call = u32::try_from(source.rfind('p').unwrap()).unwrap();
    let analysis = Analyser::new().analyse(source, "tcl8.4").clone();
    assert_eq!(
        definition(source, 0, call, &analysis),
        [LspRange {
            start_line: 0,
            start_character: 5,
            end_line: 0,
            end_character: 6,
        }]
    );
    let unavailable = crate::diagnostic_report::standalone_findings(
        &crate::diagnostic_report::StandaloneDocument {
            source,
            file_path: None,
            dialect: crate::profile_for_dialect("tcl8.4"),
            registry: crate::registry_for_dialect("tcl8.4"),
            pack_overlay: u64::MAX,
            external_call_sites: None,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        },
        &std::collections::BTreeSet::new(),
    )
    .analysis;
    assert!(unavailable.analysis_context_unavailable.is_some());
    assert_eq!(
        definition(source, 0, call, &unavailable),
        [] as [LspRange; 0]
    );
}
