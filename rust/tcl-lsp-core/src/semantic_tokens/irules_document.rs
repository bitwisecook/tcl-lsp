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

//! Document-level iRules facts: declaration heads, event names and object spans.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

/// Return the source offsets whose commands are admitted by the shared
/// iRules top-level declaration boundary.
///
/// Event handlers come from the registry/syntax owner directly.  Procedures
/// use the same top-level segmented command stream and registry-owned
/// declaration shape, because the syntax owner intentionally only exposes the
/// `when`-specific boundary record.  Recursive walks must consult these facts
/// rather than infer declaration placement from their own recursion depth:
/// command identity mutations and the iRules lexer grammar are document facts,
/// not properties of this token walk.
pub(super) fn irules_top_level_declaration_heads(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    identities: &tcl_compiler::realm::CommandBindingRealm,
) -> FxHashSet<u32> {
    let mut heads: FxHashSet<u32> =
        tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
            source, registry, identities,
        )
        .into_iter()
        .map(|handler| handler.span.start())
        .collect();

    // Read under the document's own registry and lexer configuration only.
    // The heads are consulted solely for a command the document's registry
    // marks `IS_EVENT_HANDLER` (see `original.rs`), so a process-wide iRules
    // store could only ever colour a document whose registry already has its
    // own event handler — where it would donate another registry's schema.
    // A plain Tcl buffer has no such command, and no `when` declaration role.
    for seg in segment_commands_with_offset_and_config(source, 0, config) {
        let Some(head_token) = seg.argv.first() else {
            continue;
        };
        let Some(head) = seg.texts.first() else {
            continue;
        };
        let resolved = tcl_registry::events::CommandHeadResolver::resolve(
            identities,
            head,
            head_token.span.start(),
        );
        let Some(closed) = tcl_registry::events::closed_braced_argument_words(
            source,
            seg.arg_tokens(),
            seg.arg_single_token(),
        ) else {
            continue;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let Some(arguments) = tcl_registry::events::IrulesDeclarationArguments::new(
            &args,
            seg.arg_tokens(),
            seg.arg_single_token(),
            &closed,
        ) else {
            continue;
        };
        if matches!(
            registry.irules_top_level_declaration_shape(resolved.as_ref(), arguments),
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
        ) {
            heads.insert(head_token.span.start());
        }
    }
    heads
}

/// True when `s` looks like an iRules event name (`^[A-Z][A-Z0-9_]+$`).
pub(super) fn is_event_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

/// The BIG-IP object-reference spans of an iRules document, read under the
/// document's own analysis: the source context is retained from that
/// analysis, its input and its unit — the caller's, or the one the analysis'
/// retained source request rebuilds — so no default iRules input is
/// recreated for it. An analysis that yields no current iRules source context
/// yields no spans (the standalone source API is not substituted).
pub(super) fn irules_document_object_spans(
    source: &str,
    analysis: &AnalysisResult,
    cu: Option<&CompilationUnit>,
    registry: &CommandRegistry,
) -> Vec<Span> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let rebuilt;
    let module = if let Some(cu) = cu {
        &cu.ir_module
    } else {
        let Some(unit) = analysis.original_optimiser_source_unit(source, registry, None) else {
            return Vec::new();
        };
        rebuilt = unit;
        &rebuilt.ir_module
    };
    let Some(original) =
        tcl_irules::OriginalIrulesSourceContext::from_analysis(source, analysis, module, &context)
    else {
        return Vec::new();
    };
    let mut spans: Vec<Span> =
        tcl_irules::extract_original_irules_source_object_references(source, &original, None)
            .into_iter()
            .map(|reference| reference.range)
            .collect();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}
