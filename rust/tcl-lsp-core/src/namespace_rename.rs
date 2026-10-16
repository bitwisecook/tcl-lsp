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

//! The **namespace rename tier**.
//!
//! Renaming a namespace is not one symbol's rename: `::old` is written in
//! every `namespace eval` block that opens it, in every qualified name
//! beneath it (`::old::p`, `$::old::v`, `::old::Cls`), in every
//! `NamespaceName`-role argument that mentions it, and in the import /
//! forget patterns that name it.  Miss one and the program stops running;
//! guess at one and it runs differently.  So this tier either produces the
//! complete edit set or **refuses with a reason**.
//!
//! # What one edit looks like
//!
//! Only the namespace's **final component** changes. Original native geometry
//! selects it before editing. A lexical variable root edits its verified name
//! extent; an escaped word or list child edits its authentic complete static
//! container and reparses it to preserve every other native value:
//!
//! ```text
//! namespace eval ::app::old { … }   ->  namespace eval ::app::new { … }
//! set ::app::old::v 1               ->  set ::app::new::v 1
//! namespace eval ::app { namespace eval old { … } }
//!                                   ->  … { namespace eval new { … } }
//! namespace eval ::app { proc use {} { old::p } }
//!                                   ->  … { proc use {} { new::p } }
//! ```
//!
//! A spelling that does **not** write the segment needs no edit at all and
//! gets none: inside `namespace eval ::app::old`, a bare `p` still resolves
//! to the same proc after the block's own name changes.
//! The shared native namespace projector selects byte components; the common
//! original-input edit planner preserves containers. Shared written component
//! extents supply the explicitly selected Logical advice domain separately.
//!
//! # The refusal gate
//!
//! Coverage equals edits, literally: the gate is asked about every word the
//! edit collector did **not** consider.  A word naming something beneath the
//! namespace that no analyser table attributes — a callback script
//! (`after 0 {::old::tick}`), a `namespace code` body, a dispatch-table
//! literal — is exactly the occurrence an edit set would leave behind, so it
//! refuses.  So do:
//!
//! * a **computed** namespace word (`namespace eval $ns { … }`) that could
//!   spell this namespace — there is no word to rewrite, and the value may be
//!   this very name;
//! * a `namespace path` word built from a **computed** value (`namespace path
//!   $paths`) — its entries are unknowable, and any of them may be this
//!   namespace.  A *literal* path list is rewritten, not refused: each of its
//!   elements is recorded as a `NamespaceRef` at its own span, so the entry
//!   moves through the ordinary edit path;
//! * a **collision** with a namespace that already exists — the server's
//!   half, since it is a workspace question.
//!
//! # Bounds
//!
//! The unattributed-word scan recognises `::`-rooted spellings.  A *relative*
//! name embedded in a string (`after 0 {old::tick}` written inside `::app`)
//! resolves through the frame the script eventually runs in, which is not a
//! lexical fact of the word — abstaining there would mean refusing on any
//! word containing any of the namespace's segments, which would refuse every
//! rename.  Rooted spellings are what a callback string can portably use, and
//! they are what this scan covers.

use tcl_syntax::naming::{
    AuthoredNamespaceComponent as WrittenCellOccurrence,
    AuthoredNamespaceOccurrencePurpose as WrittenNamePurpose,
    authored_namespace_component_extent as written_cell_occurrence,
};

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Span};

use crate::definition::span_to_range;
use crate::rename::TextEdit;
use crate::rename_safety::RenameRefusal;

mod original;
pub use original::original_namespace_rename_edits;

/// Every edit renaming the namespace `cell` — a `::`-rooted qualified name —
/// so that its final segment becomes `new_tail`, within one document.
///
/// `Err` is a refusal: this document holds an occurrence the rename can
/// neither rewrite nor rule out, so the whole rename must be refused rather
/// than half-applied.  See the [module docs](self) for the gate.
///
/// The result is deduplicated by range; the caller merges each document's
/// share into the workspace edit.
///
/// # Errors
///
/// Returns the [`RenameRefusal`] for the first unprovable occurrence found.
pub fn namespace_rename_edits(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    cell: &str,
    new_tail: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    let index = LineIndex::new(source);
    if let Some((scope, policy)) =
        crate::namespace_symbol::retained_namespace_for_report(analysis, cell)
    {
        return original_namespace_rename_edits(
            source,
            dialect,
            analysis,
            &crate::namespace_symbol::OriginalNamespaceSymbol::from_retained(scope, policy),
            new_tail,
        );
    }
    if !analysis.allows_lexical_declaration_advice() {
        return Err(RenameRefusal::at(
            "the displayed namespace has no unique original byte selection".to_owned(),
            source,
            &index,
            None,
        ));
    }
    lexical_namespace_rename_edits(source, dialect, analysis, cell, new_tail)
}

fn lexical_namespace_rename_edits(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    cell: &str,
    new_tail: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    let line_index = LineIndex::new(source);
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)
        .ok_or_else(|| RenameRefusal::at(
            format!("cannot rename `{cell}`: the complete original source and metadata are unavailable"),
            source, &line_index, None,
        ))?;
    let variable_config = current.config();
    let mut considered: Vec<Span> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut record = |word: Span,
                      resolved: &str,
                      purpose: WrittenNamePurpose,
                      considered: &mut Vec<Span>|
     -> Result<(), RenameRefusal> {
        match written_cell_occurrence(source, word, resolved, cell, purpose) {
            Some(WrittenCellOccurrence::Written(segment)) => spans.push(segment),
            Some(WrittenCellOccurrence::Inherited) => {}
            None => {
                return Err(RenameRefusal::at(
                    format!(
                        "cannot rename `{cell}`: the selected namespace component cannot be mapped to its complete written name"
                    ),
                    source,
                    &line_index,
                    Some(word),
                ));
            }
        }
        // An inherited omission and an exact mapping are independently
        // established. Failed mappings never count as covered occurrences.
        considered.push(word);
        Ok(())
    };

    // 1. Every `NamespaceName`-role word naming the namespace or one beneath
    //    it — the `namespace eval` / `children` / `exists` / `delete` /
    //    `upvar` / `inscope` arguments the analyser recorded.  Descendants
    //    count: `namespace eval ::old::sub {}` writes `old` too, and it is
    //    also how an implicitly-created namespace is renamed
    //    at all — it has no declaring word of its own.
    for nref in &analysis.namespace_refs {
        if names_at_or_under(cell, &nref.qualified_name) {
            record(
                nref.span,
                &nref.qualified_name,
                WrittenNamePurpose::Namespace,
                &mut considered,
            )?;
        }
    }
    // 2. Qualified variable occurrences beneath it (`$::old::v`, `set
    //    app::old::v 1`).
    for vref in &analysis.qualified_var_refs {
        if names_under(cell, &vref.qualified_name) {
            record(
                vref.span,
                &vref.qualified_name,
                WrittenNamePurpose::Variable(variable_config),
                &mut considered,
            )?;
        }
    }
    // 3. Command call sites resolved beneath it.  An indirect site's span is
    //    not the written name, so it is never a rewrite target — and
    //    never a silent miss either: the gate below sees it as an
    //    unattributed word if it writes the namespace.
    for inv in &analysis.command_invocations {
        let Some(resolved) = inv.resolved_qualified_name.as_deref() else {
            continue;
        };
        if inv.indirect || !names_under(cell, resolved) {
            continue;
        }
        record(
            inv.range,
            resolved,
            WrittenNamePurpose::Entity,
            &mut considered,
        )?;
    }
    // 4. Declaration name words beneath it — `proc ::old::p`, `oo::class
    //    create ::old::C`.
    for proc_def in analysis.all_procs.values() {
        if names_under(cell, &proc_def.qualified_name) {
            record(
                proc_def.name_span,
                &proc_def.qualified_name,
                WrittenNamePurpose::Entity,
                &mut considered,
            )?;
        }
    }
    for class_def in analysis.all_classes.values() {
        if names_under(cell, &class_def.qualified_name) {
            record(
                class_def.name_span,
                &class_def.qualified_name,
                WrittenNamePurpose::Entity,
                &mut considered,
            )?;
        }
    }
    // 5. Import / forget patterns naming it.  `namespace export` patterns are
    //    never qualified — a `::`-qualified export is a hard error in real
    //    Tcl (`invalid export pattern "::foo": pattern can't specify a
    //    namespace`) — so they name no other namespace and need no edit.
    for imp in &analysis.namespace_imports {
        if let Some(source) = &imp.source {
            let pattern = source.constructed_pattern();
            if names_under(cell, &pattern) {
                record(
                    imp.range,
                    &pattern,
                    WrittenNamePurpose::Entity,
                    &mut considered,
                )?;
            }
        }
    }
    for forget in &analysis.namespace_forgets {
        let Some(source_ns) = forget.source_ns.as_deref() else {
            continue;
        };
        if !names_at_or_under(cell, source_ns) {
            continue;
        }
        // The recorded span covers the whole pattern word (`::old::p`), and
        // the pattern's own namespace is what names the renamed one.
        // The source namespace is constructed metadata; retain literal colon
        // segments while the shared owner qualifies the written pattern tail.
        let resolved = tcl_syntax::naming::qualify(source_ns, &forget.pattern);
        record(
            forget.range,
            &resolved,
            WrittenNamePurpose::Entity,
            &mut considered,
        )?;
    }

    if let Some(refusal) =
        namespace_rename_hazard(source, dialect, analysis, cell, &considered, &line_index)
    {
        return Err(refusal);
    }

    let mut edits: Vec<TextEdit> = spans
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|span| TextEdit {
            range: span_to_range(source, &line_index, span),
            new_text: new_tail.to_owned(),
        })
        .collect();
    edits.sort_by_key(|e| {
        (
            e.range.start_line,
            e.range.start_character,
            e.range.end_line,
            e.range.end_character,
        )
    });
    edits.dedup_by(|a, b| a.range == b.range);
    Ok(edits)
}

/// The byte span of `cell`'s final segment inside the namespace-name word the
/// cursor at `cursor` sits in — the range `prepare_rename` offers and the
/// rename rewrites.
///
/// `None` when the word does not spell that segment: inside `namespace eval
/// ::app::old`, the word of a nested `namespace eval sub` writes no `old`, so
/// there is nothing there for a rename of `::app::old` to anchor on.
#[must_use]
pub fn namespace_segment_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    cell: &str,
) -> Option<Span> {
    if let Some(symbol) =
        crate::namespace_symbol::original_namespace_at_offset(source, analysis, cursor)
    {
        if symbol.scope().display().as_deref() != Some(cell) {
            return None;
        }
        return original_namespace_rename_span_at_offset(source, analysis, cursor);
    }
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let hit = analysis
        .namespace_refs
        .iter()
        .find(|r| r.span.start() <= cursor && cursor < r.span.end())?;
    match written_cell_occurrence(
        source,
        hit.span,
        &hit.qualified_name,
        cell,
        WrittenNamePurpose::Namespace,
    )? {
        WrittenCellOccurrence::Written(span) => Some(span),
        WrittenCellOccurrence::Inherited => None,
    }
}

/// Prepare an original namespace edit at the cursor. Literal source returns
/// its independently mapped component extent. Escaped words and list children
/// return their authentic static container; computed readonly values decline.
/// The range grants no execution, namespace lifetime or command existence.
#[must_use]
pub fn original_namespace_rename_span_at_offset(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Span> {
    let symbol = crate::namespace_symbol::original_namespace_at_offset(source, analysis, cursor)?;
    let row = analysis
        .namespace_refs
        .iter()
        .find(|row| row.span.start() <= cursor && cursor < row.span.end())?;
    let input = row.original_name_input.as_ref()?;
    let container = input.original_static_list_container()?;
    if !container.ordinals().is_empty() {
        return Some(container.parent_word().span());
    }
    let key = input.original_word_key()?;
    let current = row.source_context.as_ref()?.context()?;
    let wanted = symbol.scope().context()?;
    let member = tcl_syntax::naming::native_written_namespace_member_extent(
        symbol.policy().recipe(),
        current,
        input.bytes(),
        wanted,
    )?;
    let name = key.original_word().content_span().ok()?;
    let raw = key.source_image().bytes().get(name.as_range())?;
    let literal = tcl_syntax::backslash::native_source_literal_bytes(
        raw,
        key.source_image().channel(),
        key.policy().string_protocol(),
    )
    .ok()?;
    if literal.as_ref() != input.bytes() {
        return Some(key.span());
    }
    let extent = tcl_syntax::backslash::native_source_literal_extent(
        raw,
        key.source_image().channel(),
        key.policy().string_protocol(),
        member,
    )?;
    Some(Span::new(
        name.start()
            .checked_add(u32::try_from(extent.start).ok()?)?,
        name.start().checked_add(u32::try_from(extent.end).ok()?)?,
    ))
}

/// Whether `candidate` is the namespace `cell` itself or one beneath it.
fn names_at_or_under(cell: &str, candidate: &str) -> bool {
    tcl_syntax::naming::reported_namespace_contains(cell, candidate)
}

/// Whether the qualified *name* `candidate` (a proc / variable / class, not a
/// namespace) lives beneath `cell`.
fn names_under(cell: &str, candidate: &str) -> bool {
    crate::namespace_symbol::namespace_strictly_contains(cell, candidate)
}

// Entity occurrence geometry used by the finite source mapper controls.
#[cfg(test)]
fn written_cell_segment(source: &str, word: Span, resolved: &str, cell: &str) -> Option<Span> {
    match written_cell_occurrence(source, word, resolved, cell, WrittenNamePurpose::Entity)? {
        WrittenCellOccurrence::Written(span) => Some(span),
        WrittenCellOccurrence::Inherited => None,
    }
}

/// Why this document makes the namespace rename unprovable, if it does.
///
/// `considered` is the exact set of word spans the edit collector looked at,
/// so "everything else" is what the gate examines — the coverage-equals-edits
/// discipline the member tier applies, applied here.
fn namespace_rename_hazard(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    cell: &str,
    considered: &[Span],
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    // A `namespace path` word the analyser could not read as a literal list —
    // `namespace path $paths`, `namespace path "$ns ::a"`.  Its entries are
    // whatever the value turns out to be, which may include this namespace,
    // and there is no element word to rewrite.  A *literal* list needs no
    // arm here: each of its elements is recorded as a `NamespaceRef` at its
    // own span, so the edit collector rewrites the ones naming the renamed
    // cell like any other spelling.
    if let Some(&span) = analysis.namespace_path_computed.first() {
        let written = source
            .get(span.start() as usize..span.end() as usize)
            .unwrap_or("");
        return Some(RenameRefusal::at(
            format!(
                "cannot rename `{cell}`: a `namespace path` is built from a value computed \
                 at run time (`{written}`), whose entries may name this very namespace — \
                 there is no element word to rewrite, so the path could keep naming a \
                 namespace that no longer exists."
            ),
            source,
            line_index,
            Some(span),
        ));
    }
    let unavailable = || {
        RenameRefusal::at(
            format!(
                "cannot rename `{cell}`: the complete original source and metadata are unavailable"
            ),
            source,
            line_index,
            None,
        )
    };
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Some(unavailable());
    };
    let config = current.config();
    let metadata = analysis.resolved_input.as_ref().and_then(|input| {
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
            current.registry(),
            input,
            config,
            Some(current.profile()),
        )
    });
    let mut hazard: Option<(Span, HazardKind)> = None;
    let mut visit = |cmd: &tcl_compiler::segmenter::SegmentedCommand| {
        if hazard.is_some() {
            return;
        }
        // A word carrying a script is walked as code in its own right, so its
        // text must not be read as a name here — a body word contains every
        // name written inside it.
        let nested = crate::references::dispatch_scan_regions(source, analysis, dialect, cmd);
        let words = tcl_compiler::registry_invocation::source_structure::source_registry_words(
            source, analysis, cmd,
        );
        let roles = words
            .as_ref()
            .filter(|words| words.roles().is_some())
            .map(|words| words.written_argument_roles());
        for (idx, word) in cmd.texts.iter().skip(1).enumerate() {
            if !word.contains(['$', '[']) {
                continue;
            }
            let kind = match &roles {
                Some(roles) if roles.contains(&(idx, tcl_registry::ArgRole::NamespaceName)) => {
                    HazardKind::Computed
                }
                Some(_) => continue,
                None => HazardKind::UnknownRole,
            };
            if matches!(kind, HazardKind::Computed)
                && tcl_compiler::dynamic_names::source_name_pattern_can_spell(
                    word,
                    cell,
                    tcl_compiler::dynamic_names::SourceNamePatternPurpose::NamespaceName,
                    metadata,
                ) == Some(false)
            {
                continue;
            }
            if let Some(tok) = cmd.argv.get(idx + 1) {
                hazard = Some((tok.span, kind));
                return;
            }
        }
        for tok in &cmd.argv {
            let span = tok.span;
            if considered.iter().any(|c| spans_overlap(*c, span)) {
                continue;
            }
            if nested
                .iter()
                .any(|(s, e)| (span.start() as usize) < *e && *s < span.end() as usize)
            {
                continue;
            }
            let Some(text) = source.get(span.start() as usize..span.end() as usize) else {
                continue;
            };
            if embedded_rooted_name_under(text, cell) {
                hazard = Some((span, HazardKind::Unattributed));
                return;
            }
        }
    };
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            visit(command);
            false
        },
    );
    let (span, kind) = hazard?;
    let written = source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or("");
    let reason = match kind {
        HazardKind::UnknownRole => format!(
            "cannot rename `{cell}`: the original command's argument roles are unavailable for `{written}`, which may name this namespace"
        ),
        HazardKind::Computed => format!(
            "cannot rename `{cell}`: this document names a namespace through a value \
             computed at run time (`{written}`), which may be this very namespace — there \
             is no word to rewrite, so the rename cannot be completed from source edits."
        ),
        HazardKind::Unattributed => format!(
            "cannot rename `{cell}`: `{written}` names something beneath it in a position \
             this rename cannot attribute — a callback script, a dispatch-table literal, or \
             another embedded name. Rewriting the declarations would leave it naming a \
             namespace that no longer exists."
        ),
    };
    Some(RenameRefusal::at(reason, source, line_index, Some(span)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HazardKind {
    UnknownRole,
    Computed,
    Unattributed,
}

fn spans_overlap(a: Span, b: Span) -> bool {
    a.start() < b.end() && b.start() < a.end()
}

/// Whether `text` embeds a `::`-rooted name beneath `cell` — the shape a
/// callback string or dispatch-table literal carries.
///
/// The word is split on the characters that separate words inside a script or
/// list literal, so `{::old::tick $arg}` is examined token by token rather
/// than as one string.  Rooted spellings only; see the module docs' bounds.
fn embedded_rooted_name_under(text: &str, cell: &str) -> bool {
    tcl_syntax::naming::unattributed_rooted_name_candidates(text)
        .any(|candidate| names_at_or_under(cell, candidate) || names_under(cell, candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn namespace_hazard_keeps_original_roles_and_unavailable_owner_refusal() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        // Source edit refusal inventory, not a native namespace or runtime call.
        use std::sync::Arc;
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "source_namespace_reference",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::NamespaceName)],
            surface: registry.get("dict").unwrap().surface,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = Arc::new(registry);
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::clone(&store)),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let source = "source_namespace_reference $candidate";
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let index = LineIndex::new(source);
        let hazard = namespace_rename_hazard(source, profile, &analysis, "::kept", &[], &index)
            .expect("actual custom NamespaceName role keeps the computed-name blocker");
        assert!(hazard.reason.contains("computed at run time"));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(store),
        );
        let older_input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, older, config);
        let older_analysis = Analyser::new()
            .with_resolved_input(older_input)
            .analyse(source, profile.name);
        assert!(
            namespace_rename_hazard(source, profile, &older_analysis, "::kept", &[], &index)
                .unwrap()
                .reason
                .contains("argument roles are unavailable")
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
        assert!(
            namespace_rename_hazard(source, profile, &analysis, "::kept", &[], &index)
                .unwrap()
                .reason
                .contains("metadata are unavailable")
        );
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = None;
        assert!(
            namespace_rename_hazard(source, profile, &analysis, "::kept", &[], &index)
                .unwrap()
                .reason
                .contains("metadata are unavailable")
        );
    }

    #[test]
    fn supplied_namespace_hazard_patterns_keep_parentheses_and_unknown_roles() {
        // naming.core.original-dynamic-name-value-purpose
        // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
        use std::sync::Arc;
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "source_namespace_reference",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::NamespaceName)],
            surface: registry.get("dict").unwrap().surface,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = Arc::new(registry);
        for (source, cell, expected) in [
            ("source_namespace_reference ::other::$n", "::kept", false),
            (
                "source_namespace_reference ::kept($n)",
                "::kept(TAIL)",
                true,
            ),
        ] {
            let context = Arc::new(
                tcl_registry::model::ingress::static_context_for("tcl8.6")
                    .with_command_store(store.clone()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            let current = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            assert_eq!(
                namespace_rename_hazard(
                    source,
                    profile,
                    &current,
                    cell,
                    &[],
                    &LineIndex::new(source),
                )
                .is_some(),
                expected,
                "{source}"
            );
        }
        let source = "source_namespace_reference ::other::$n";
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(store),
        );
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let older = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(
            namespace_rename_hazard(
                source,
                profile,
                &older,
                "::kept",
                &[],
                &LineIndex::new(source)
            )
            .unwrap()
            .reason
            .contains("argument roles are unavailable")
        );
        let source = "namespace eval ::other::$n {}";
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            namespace_rename_hazard(
                source,
                native.resolved_profile().unwrap(),
                &native,
                "::kept",
                &[],
                &LineIndex::new(source)
            )
            .is_some()
        );
    }

    #[test]
    fn constructed_colon_namespace_membership_keeps_the_root_separate() {
        assert!(names_at_or_under(":::", ":::"));
        assert!(!names_at_or_under(":::", "::"));
        assert!(names_at_or_under(":::", "::::::"));
        assert!(!names_at_or_under(":::", "::other"));
    }

    /// The document rewritten by applying the tier's edits, or the refusal
    /// reason.
    fn renamed(source: &str, cell: &str, new_tail: &str) -> Result<String, String> {
        let analysis = analyse(source);
        let edits = namespace_rename_edits(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            cell,
            new_tail,
        )
        .map_err(|r| r.reason)?;
        // Apply back-to-front so earlier spans keep their offsets.
        let line_index = LineIndex::new(source);
        let mut offsets: Vec<(usize, usize, String)> = edits
            .iter()
            .map(|e| {
                let start = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.start_line,
                    e.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.end_line,
                    e.range.end_character,
                ) as usize;
                (start, end, e.new_text.clone())
            })
            .collect();
        offsets.sort_by_key(|(start, _, _)| *start);
        let mut out = source.to_owned();
        for (start, end, text) in offsets.into_iter().rev() {
            out.replace_range(start..end, &text);
        }
        Ok(out)
    }

    /// TP — the tier's core claim: every written spelling of the namespace
    /// moves, and nothing else does.
    ///
    /// tclsh-proof (8.6.14) that the before/after programs agree: the source
    /// below prints `1 P`, and the same script with `old` spelled `new`
    /// throughout prints `1 P` too.
    #[test]
    fn tp_rewrites_every_written_spelling_of_the_namespace() {
        let src = "namespace eval ::old {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { return P }\n\
                   }\n\
                   namespace eval ::old { proc q {} { return Q } }\n\
                   puts $::old::v\n\
                   puts [::old::p]\n\
                   namespace children ::old\n";
        let out = renamed(src, "::old", "new").expect("a clean document must rename");
        assert!(!out.contains("::old"), "no spelling may be left: {out}");
        assert_eq!(out.matches("::new").count(), 5, "{out}");
        // The unqualified names inside the block are untouched — they still
        // resolve to the same cells after the block's own name changes.
        assert!(
            out.contains("variable v 1") && out.contains("proc p {}"),
            "{out}"
        );
    }

    /// TP — a **relative** spelling written inside an enclosing block moves
    /// too, and only its own segment does.
    #[test]
    fn tp_rewrites_a_relatively_written_segment_only() {
        let src = "namespace eval ::app {\n\
                   \x20   namespace eval old { proc p {} { return P } }\n\
                   \x20   proc use {} { return [old::p] }\n\
                   }\n";
        let out = renamed(src, "::app::old", "new").expect("rename");
        assert!(out.contains("namespace eval new {"), "{out}");
        assert!(out.contains("[new::p]"), "{out}");
        assert!(
            out.contains("namespace eval ::app {"),
            "the parent is untouched: {out}"
        );
    }

    /// TP — a deeper block is how an **implicitly created** namespace is
    /// renamed at all: it has no declaring word of its own, so the edit is the
    /// covering segment of the deeper name.
    ///
    /// tclsh-proof (8.6.14): `namespace eval ::p::q::r {}` leaves `namespace
    /// exists ::p::q` -> 1, so `::p::q` is a real namespace with no block.
    #[test]
    fn tp_renames_an_implicitly_created_namespace_through_its_deeper_block() {
        let src = "namespace eval ::p::q::r { proc f {} {} }\nputs [::p::q::r::f]\n";
        let out = renamed(src, "::p::q", "z").expect("rename");
        assert!(out.contains("namespace eval ::p::z::r"), "{out}");
        assert!(out.contains("[::p::z::r::f]"), "{out}");
    }

    /// TN — a name that merely *shares the tail* is a different namespace and
    /// must not move.
    #[test]
    fn tn_a_same_tailed_sibling_namespace_is_untouched() {
        let src = "namespace eval ::a::old { proc p {} {} }\n\
                   namespace eval ::b::old { proc p {} {} }\n\
                   puts [::b::old::p]\n";
        let out = renamed(src, "::a::old", "new").expect("rename");
        assert!(out.contains("namespace eval ::a::new"), "{out}");
        assert!(out.contains("namespace eval ::b::old"), "{out}");
        assert!(out.contains("[::b::old::p]"), "{out}");
    }

    /// TP (refusal) — a computed namespace word may be this very namespace and
    /// has no word to rewrite.
    #[test]
    fn tp_refuses_a_computed_namespace_target() {
        let src = "namespace eval ::old { variable v 1 }\n\
                   proc mk {ns} { namespace eval $ns { variable w 2 } }\n";
        let reason = renamed(src, "::old", "new").expect_err("a computed target must refuse");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    /// TN — a computed namespace word that provably cannot spell this
    /// namespace does not refuse (the per-site provenance rule, applied to
    /// namespace words).
    #[test]
    fn tn_a_computed_namespace_word_elsewhere_does_not_refuse() {
        let src = "namespace eval ::old { variable v 1 }\n\
                   proc mk {ns} { namespace eval ::other::$ns { variable w 2 } }\n";
        assert!(
            renamed(src, "::old", "new").is_ok(),
            "`::other::$ns` cannot name `::old`",
        );
    }

    /// TP — a callback script the registry marks as a *body* is walked as
    /// code, so the name inside it is an attributed call site and moves with
    /// the namespace rather than refusing.
    #[test]
    fn tp_rewrites_a_name_inside_a_walked_callback_body() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   after 0 {::old::tick}\n";
        let out = renamed(src, "::old", "new").expect("a walked body is attributed");
        assert!(out.contains("after 0 {::new::tick}"), "{out}");
    }

    /// TP (refusal) — a rooted name beneath the namespace sitting in a plain
    /// data word is exactly what an edit set would leave behind: nothing
    /// attributes it, and it may well be `eval`'d or dispatched later.
    #[test]
    fn tp_refuses_an_unattributed_embedded_name() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   set handlers [list ::old::tick]\n";
        let reason = renamed(src, "::old", "new").expect_err("an embedded name must refuse");
        assert!(reason.contains("cannot attribute"), "{reason}");
    }

    /// TN control for the pair above: a data word naming something in a
    /// *different* namespace is not this rename's problem.
    #[test]
    fn tn_an_unattributed_name_elsewhere_does_not_refuse() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   set handlers [list ::other::tick]\n";
        assert!(renamed(src, "::old", "new").is_ok());
    }

    /// TP — a literal `namespace path` entry naming the
    /// namespace is *rewritten*, not refused: each element carries its own
    /// span, so the tier edits the element's segment like any other spelling.
    ///
    /// tclsh-proof (8.6.14) that the before/after programs agree: the source
    /// below prints `P`, and the same script with `old` spelled `new`
    /// throughout prints `P` too — while leaving the entry behind fails with
    /// `namespace "::old" not found in "::"`.
    #[test]
    fn tp_rewrites_a_namespace_path_entry() {
        let src = "namespace eval ::old { proc p {} { return P } ; namespace export p }\n\
                   namespace eval ::user { namespace path {::old}\n\
                   \x20   proc use {} { return [p] } }\n\
                   puts [::user::use]\n";
        let out = renamed(src, "::old", "new").expect("a literal path entry is rewritable");
        assert!(out.contains("namespace path {::new}"), "{out}");
        assert!(out.contains("namespace eval ::new {"), "{out}");
    }

    /// TP — only the entries naming the renamed namespace move; siblings in
    /// the same list word are left exactly as written.
    #[test]
    fn tp_rewrites_only_the_matching_namespace_path_entries() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::keep { proc q {} {} }\n\
                   namespace eval ::user { namespace path {::keep ::old::deep ::other} }\n";
        let out = renamed(src, "::old", "new").expect("rename");
        assert!(
            out.contains("namespace path {::keep ::new::deep ::other}"),
            "{out}"
        );
    }

    /// TP — a **relative** entry roots against the namespace the `namespace
    /// path` runs in, and only the segment that spells the renamed namespace
    /// moves.
    ///
    /// tclsh-proof (8.6.14): inside `namespace eval ::app`, `namespace path
    /// {old}` echoes `::app::old` — the entry roots against the namespace the
    /// command runs in, current-namespace-only (the same block written one
    /// level deeper, in `::app::user`, fails `namespace "old" not found in
    /// "::app::user"`).  So a rename of `::app::old` must rewrite the bare
    /// `old` word.
    #[test]
    fn tp_rewrites_a_relatively_written_namespace_path_entry() {
        let src = "namespace eval ::app {\n\
                   \x20   namespace eval old { proc p {} {} }\n\
                   \x20   namespace path {old}\n\
                   }\n";
        let out = renamed(src, "::app::old", "new").expect("rename");
        assert!(out.contains("namespace path {new}"), "{out}");
    }

    /// TN — a path list that names nothing beneath the renamed namespace is
    /// untouched and does not refuse.
    #[test]
    fn tn_an_unrelated_namespace_path_neither_moves_nor_refuses() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::user { namespace path {::a ::b} }\n";
        let out = renamed(src, "::old", "new").expect("an unrelated path must not refuse");
        assert!(out.contains("namespace path {::a ::b}"), "{out}");
    }

    /// TP (refusal) — a `namespace path` built from a value
    /// computed at run time still refuses: its entries are unknowable, any of
    /// them may be this namespace, and there is no element word to rewrite.
    #[test]
    fn tp_refuses_a_computed_namespace_path() {
        for src in [
            "namespace eval ::old { proc p {} {} }\n\
             namespace eval ::user { namespace path $paths }\n",
            "namespace eval ::old { proc p {} {} }\n\
             namespace eval ::user { namespace path \"$ns ::a\" }\n",
        ] {
            let reason = renamed(src, "::old", "new").expect_err("a computed path must refuse");
            assert!(reason.contains("namespace path"), "{reason}");
            assert!(reason.contains("computed at run time"), "{reason}");
        }
    }

    /// TN — a **braced** `$`-spelled entry is not computed: braces suppress
    /// substitution, so the word is a literal path list and the tier neither
    /// refuses nor rewrites it.
    ///
    /// tclsh-proof (8.6.14): `namespace eval n { namespace path {::$ns ::a} }`
    /// (with a namespace literally named `::$ns`) echoes `{::$ns} ::a` — the
    /// entry is a name, never a variable read.
    #[test]
    fn tn_a_braced_dollar_entry_is_a_literal_path() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::user { namespace path {::a ::b} }\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_path_computed.is_empty(),
            "a braced literal list records no abstention",
        );
        assert!(renamed(src, "::old", "new").is_ok());

        // Preserve the original control and add the discriminating literal
        // spelling: the braced `$` belongs to the namespace name, not a read.
        let dollar_src = "namespace eval ::old { proc p {} {} }\n\
                          namespace eval ::user { namespace path {::$ns ::a} }\n";
        let dollar_analysis = analyse(dollar_src);
        assert!(
            dollar_analysis.namespace_path_computed.is_empty(),
            "a braced dollar-spelled literal list records no abstention",
        );
        assert!(renamed(dollar_src, "::old", "new").is_ok());
    }

    /// TP — import patterns follow the namespace.
    #[test]
    fn tp_rewrites_an_import_pattern() {
        let src = "namespace eval ::old { proc p {} {} ; namespace export p }\n\
                   namespace eval ::user { namespace import ::old::p }\n";
        let out = renamed(src, "::old", "new").expect("rename");
        assert!(out.contains("namespace import ::new::p"), "{out}");
        // The export pattern is unqualified and names no namespace, so it is
        // untouched — a `::`-qualified export is a hard error in real Tcl.
        assert!(out.contains("namespace export p"), "{out}");
    }

    /// The segment finder is the one place that decides which byte range a
    /// written name gives to the rename.
    #[test]
    fn written_segment_is_the_namespace_segment_only() {
        let src = "set ::app::old::v 1\n";
        let at = u32::try_from(src.find("::app").expect("word")).expect("offset fits");
        let len = u32::try_from("::app::old::v".len()).expect("length fits");
        let span =
            written_cell_segment(src, Span::new(at, at + len), "::app::old::v", "::app::old")
                .expect("segment");
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "old",
            "only the namespace's own segment is rewritten",
        );
        // A word that does not spell the segment yields nothing to rewrite —
        // a bare `v` inside the namespace still resolves after the rename.
        assert!(
            written_cell_segment("v\n", Span::new(0, 1), "::app::old::v", "::app::old").is_none()
        );
    }

    #[test]
    fn logical_written_namespace_geometry_retains_complete_separator_runs_and_purpose() {
        // naming.editor.logical-written-component-geometry
        // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
        for written in [
            "::app:::old::v",
            "::app::::old::v",
            "::::app::old::v",
            "app:::::old::v",
            "${::::app:::old::v}",
            "$::::app:::old::v(index)",
        ] {
            let span = Span::new(0, u32::try_from(written.len()).unwrap());
            let Some(WrittenCellOccurrence::Written(selected)) = written_cell_occurrence(
                written,
                span,
                "::app::old::v",
                "::app::old",
                if written.starts_with('$') {
                    WrittenNamePurpose::Variable(tcl_lexer::LexerConfig::for_profile(Some(
                        tcl_dialect::DialectProfile::plain_tcl(),
                    )))
                } else {
                    WrittenNamePurpose::Entity
                },
            ) else {
                panic!("{written}")
            };
            assert_eq!(written.get(selected.as_range()), Some("old"), "{written}");
        }
        for (written, resolved, purpose) in [
            (
                "::::app:::old::::",
                "::app::old",
                WrittenNamePurpose::Namespace,
            ),
            (
                "::::app:::old::::",
                "::app::old::",
                WrittenNamePurpose::Entity,
            ),
            ("::::é:::旧::v", "::é::旧::v", WrittenNamePurpose::Entity),
        ] {
            let span = Span::new(0, u32::try_from(written.len()).unwrap());
            let cell = if written.contains('é') {
                "::é::旧"
            } else {
                "::app::old"
            };
            let Some(WrittenCellOccurrence::Written(selected)) =
                written_cell_occurrence(written, span, resolved, cell, purpose)
            else {
                panic!("{written}")
            };
            assert_eq!(
                written.get(selected.as_range()),
                Some(tcl_syntax::naming::key_tail(cell))
            );
        }
        for (resolved, purpose) in [
            ("::app::old", WrittenNamePurpose::Entity),
            ("::app::old::", WrittenNamePurpose::Namespace),
        ] {
            let written = "::::app:::old::::";
            assert!(
                written_cell_occurrence(
                    written,
                    Span::new(0, u32::try_from(written.len()).unwrap()),
                    resolved,
                    "::app::old",
                    purpose
                )
                .is_none()
            );
        }
        let written = "::::app:::old(x)::v";
        let Some(WrittenCellOccurrence::Written(selected)) = written_cell_occurrence(
            written,
            Span::new(0, u32::try_from(written.len()).unwrap()),
            "::app::old(x)::v",
            "::app::old(x)",
            WrittenNamePurpose::Entity,
        ) else {
            panic!("{written}")
        };
        assert_eq!(written.get(selected.as_range()), Some("old(x)"));
        assert_eq!(
            written_cell_occurrence(
                "v",
                Span::new(0, 1),
                "::app::old::v",
                "::app::old",
                WrittenNamePurpose::Entity
            ),
            Some(WrittenCellOccurrence::Inherited)
        );
        assert!(
            written_cell_occurrence(
                "::app::other::v",
                Span::new(0, 15),
                "::app::old::v",
                "::app::old",
                WrittenNamePurpose::Entity
            )
            .is_none()
        );
        assert!(
            written_cell_occurrence(
                "::app::\\u006fold::v",
                Span::new(0, 19),
                "::app::old::v",
                "::app::old",
                WrittenNamePurpose::Entity
            )
            .is_none()
        );
    }

    #[test]
    fn logical_braced_variable_geometry_uses_the_actual_header_span_and_close_rule() {
        // naming.editor.logical-written-component-geometry
        // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
        // Actual produced source span; no Native cell or edit admission.
        let source = "namespace eval {::app::old(x)} {}\nset {::app::old(x)::v} 1\nputs ${::::app:::old(x)::v}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let row = analysis
            .qualified_var_refs
            .iter()
            .find(|row| {
                source
                    .get(row.span.as_range())
                    .is_some_and(|raw| raw.starts_with("${"))
            })
            .expect("actual braced variable occurrence");
        let config = analysis.body_lexer_config.unwrap();
        assert_eq!(source.as_bytes().get(row.span.end() as usize), Some(&b'}'));
        let Some(WrittenCellOccurrence::Written(selected)) = written_cell_occurrence(
            source,
            row.span,
            &row.qualified_name,
            "::app::old(x)",
            WrittenNamePurpose::Variable(config),
        ) else {
            panic!("{row:?}")
        };
        assert_eq!(source.get(selected.as_range()), Some("old(x)"));
        let cursor = source.find("${").unwrap();
        for style in [
            tcl_dialect::BracedVarStyle::FirstClose,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ] {
            let mut actual = config;
            actual.braced_var = style;
            let extent = Span::new(u32::try_from(cursor).unwrap(), row.span.end());
            let Some(WrittenCellOccurrence::Written(selected)) = written_cell_occurrence(
                source,
                extent,
                "::app::old(x)::v",
                "::app::old(x)",
                WrittenNamePurpose::Variable(actual),
            ) else {
                panic!("{style:?}")
            };
            assert_eq!(source.get(selected.as_range()), Some("old(x)"));
        }
        let original = "${::app::old{a}b::v}";
        let mut nesting = config;
        nesting.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
        let span = Span::new(0, u32::try_from(original.len() - 1).unwrap());
        let Some(WrittenCellOccurrence::Written(selected)) = written_cell_occurrence(
            original,
            span,
            "::app::old{a}b::v",
            "::app::old{a}b",
            WrittenNamePurpose::Variable(nesting),
        ) else {
            panic!("nesting original header")
        };
        assert_eq!(original.get(selected.as_range()), Some("old{a}b"));
        let mut first_close = config;
        first_close.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
        assert!(
            written_cell_occurrence(
                original,
                span,
                "::app::old{a}b::v",
                "::app::old{a}b",
                WrittenNamePurpose::Variable(first_close)
            )
            .is_none()
        );
        let malformed = "${::app::old(x)::v";
        assert!(
            written_cell_occurrence(
                malformed,
                Span::new(0, u32::try_from(malformed.len()).unwrap()),
                "::app::old(x)::v",
                "::app::old(x)",
                WrittenNamePurpose::Variable(config)
            )
            .is_none()
        );
    }

    #[test]
    fn logical_namespace_rename_maps_source_and_refuses_failed_occurrence_coverage() {
        // naming.editor.logical-written-component-geometry
        // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
        let source = "namespace eval ::::app:::old {}\nset ::::app::::old:::v 1\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let profile = analysis.resolved_profile().unwrap();
        let edits =
            namespace_rename_edits(source, profile, &analysis, "::app::old", "new").unwrap();
        assert_eq!(edits.len(), 2);
        let index = LineIndex::new(source);
        let mut changed = source.to_owned();
        let mut ranges = edits
            .iter()
            .map(|edit| {
                let start = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.start_line,
                    edit.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.end_line,
                    edit.range.end_character,
                ) as usize;
                assert_eq!(source.get(start..end), Some("old"));
                (start, end, edit.new_text.as_str())
            })
            .collect::<Vec<_>>();
        ranges.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        for (start, end, replacement) in ranges {
            changed.replace_range(start..end, replacement);
        }
        assert_eq!(
            changed,
            "namespace eval ::::app:::new {}\nset ::::app::::new:::v 1\n"
        );
        let mut mismatched = analysis.clone();
        let row = mismatched
            .namespace_refs
            .iter_mut()
            .find(|row| row.qualified_name == "::app::old")
            .unwrap();
        row.qualified_name = "::app::old::foreign".into();
        assert!(namespace_rename_edits(source, profile, &mismatched, "::app::old", "new").is_err());
        assert!(
            namespace_rename_edits(
                &format!("#{source}"),
                profile,
                &analysis,
                "::app::old",
                "new"
            )
            .is_err()
        );
        let cursor = u32::try_from(source.find("old").unwrap()).unwrap();
        assert_eq!(
            source.get(
                namespace_segment_at(source, &analysis, cursor, "::app::old")
                    .unwrap()
                    .as_range()
            ),
            Some("old")
        );
        assert!(
            namespace_segment_at(&format!("#{source}"), &analysis, cursor, "::app::old").is_none()
        );
        let mut foreign_config = analysis.clone();
        foreign_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(namespace_segment_at(source, &foreign_config, cursor, "::app::old").is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(namespace_rename_edits(source, profile, &missing, "::app::old", "new").is_err());
        assert!(namespace_segment_at(source, &missing, cursor, "::app::old").is_none());
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let native = Analyser::new().analyse(source, dialect);
            assert!(!native.allows_lexical_declaration_advice(), "{dialect}");
        }
    }

    #[test]
    fn logical_namespace_forget_rename_retains_complete_source_and_hazard_guards() {
        // naming.editor.logical-written-component-geometry
        // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
        let source = "namespace eval 茶 {}\nnamespace forget ::茶::*\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        assert!(analysis.allows_lexical_declaration_advice());
        assert_eq!(analysis.namespace_forgets.len(), 1);
        let edits = namespace_rename_edits(source, profile, &analysis, "::茶", "box").unwrap();
        assert_eq!(edits.len(), 2);
        let index = LineIndex::new(source);
        for edit in edits {
            let start = crate::definition::byte_offset_at(
                &index,
                source,
                edit.range.start_line,
                edit.range.start_character,
            );
            let end = crate::definition::byte_offset_at(
                &index,
                source,
                edit.range.end_line,
                edit.range.end_character,
            );
            assert_eq!(&source[start as usize..end as usize], "茶");
            assert_eq!(edit.new_text, "box");
        }
        assert!(
            namespace_rename_edits(
                "namespace eval 茶 {}\nnamespace forget ::茶::other\n",
                profile,
                &analysis,
                "::茶",
                "box"
            )
            .is_err()
        );
        let hazardous = "namespace eval 茶 {}\nafter 0 {::茶::callback}";
        let unknown = Analyser::new().analyse(hazardous, "tcl");
        assert!(namespace_rename_edits(hazardous, profile, &unknown, "::茶", "box").is_err());
        assert_eq!(tcl_syntax::naming::qualify("::茶:::", "*"), "::茶:::::*");
        assert_eq!(tcl_syntax::naming::qualify("::", "*"), "::*");
    }
}
