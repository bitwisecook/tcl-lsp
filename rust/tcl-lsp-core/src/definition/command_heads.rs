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

//! The command head at the cursor: indirection, ensemble subcommands, recorded invocations and autoload requests.

use super::call_resolution::resolve_called_proc;
use super::{CallResolution, namespace_context_at};
use crate::original_context::CurrentSourcePosition;
use tcl_compiler::analyser::AnalysisResult;

/// The command name a word written at `cursor_off` actually reaches once the
/// document's `rename` / `interp alias` statements are taken into account, or
/// `None` when the command table leaves the word alone there.
///
/// The single navigation entry to the shared hop walk
/// ([`tcl_compiler::analyser::indirection::walk`]) that the analyser's
/// constructor typing already uses — order-gated, hop-capped, and declining an
/// argument-prepending alias, all decided in one place so a class that W307
/// resolves through a rename and a class go-to-definition resolves through the
/// same rename can never disagree.
///
/// Navigation asks only "which name does this reach"; whether that name has a
/// user `proc` / class to jump to is the caller's own question, so no liveness
/// predicate is applied here.  Canonicalisation is the plain
/// qualified-name normalisation the alias/rename tables are keyed by — a
/// bare `mycmd` matches an alias stored as `::mycmd`, exactly like the
/// `lookup_alias` it replaces.
///
/// `O(1)` hash lookups bounded by the hop cap: no invocation scan, no source
/// rescan, so it is safe to call on every request.
#[must_use]
pub(crate) fn command_indirection(
    analysis: &AnalysisResult,
    word: &str,
    cursor_off: u32,
) -> Option<tcl_compiler::analyser::indirection::Indirection> {
    tcl_compiler::analyser::indirection::walk(
        analysis,
        word,
        cursor_off,
        &tcl_syntax::naming::normalise_qualified_name,
    )
}

/// [`command_indirection`] for the callers that only need the terminal name.
#[must_use]
pub(crate) fn command_indirection_target(
    analysis: &AnalysisResult,
    word: &str,
    cursor_off: u32,
) -> Option<String> {
    let hop = command_indirection(analysis, word, cursor_off)?;
    hop.lookup_spelling()?;
    Some(hop.target)
}

/// Whether `qualified` is a name **this document** only gains through a
/// `rename` / `interp alias` that has not run yet at `cursor_off`.
///
/// The workspace index's command links are position-free by design — a
/// mutation in *another* file is unconditionally in effect, since that file
/// loads wholesale — but a link this same document established later in its
/// own text is not, and following it would resolve a call tclsh answers with
/// `invalid command name`. The cross-document resolver
/// (`tcl-lsp-server::resolve_workspace_symbols`) asks this before chasing a
/// link so its ordering matches the in-document provider's.
///
/// A name can carry both a `rename` and an `interp alias` record; the document
/// gains the name as soon as the *first* of them runs, so the question is
/// asked of the earliest — reading one map ahead of the other would call a
/// name pending on the strength of a mutation that is not the one that
/// introduced it.
#[must_use]
pub fn indirection_pending_at(analysis: &AnalysisResult, qualified: &str, cursor_off: u32) -> bool {
    let key = tcl_syntax::naming::normalise_qualified_name(qualified);
    let earliest = analysis
        .rename_offsets
        .get(&key)
        .into_iter()
        .chain(analysis.alias_offsets.get(&key))
        .min();
    earliest.is_some_and(|&at| {
        !tcl_compiler::analyser::indirection::in_effect(analysis, at, cursor_off)
    })
}

/// Resolve `head sub` (as returned by [`original_instance_method_at_cursor`], with
/// `is_dollar == false`) to the target command it dispatches to, when
/// `head` names a known `namespace ensemble create -map`/`-subcommands`
/// ensemble and `sub` resolves against its literal, statically-recorded
/// subcommands — `namespace` is the call site's own
/// command-resolution namespace, so a relative `head` (the overwhelmingly
/// common case: an ensemble dispatches through its own short name) resolves
/// the same way an ordinary bare command call would.
///
/// `sub` need not be written in full: a real ensemble abbreviates, so
/// `e fo` dispatching to `-map {foo ::e::Foo}` is navigable here too
/// The rule is
/// [`AnalysisResult::resolve_ensemble_subcommand`]'s — exact first, then a
/// unique prefix unless the ensemble declared `-prefixes 0` — so navigation
/// lands on whatever the ensemble itself would dispatch, and abstains
/// wherever it would error.
pub(crate) fn ensemble_subcommand_target<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    head: &str,
    sub: &str,
) -> Option<&'a str> {
    tcl_syntax::naming::bareword_resolution_candidates(namespace, head)
        .into_iter()
        .find_map(|cand| analysis.resolve_ensemble_subcommand(&cand, sub))
        .map(|entry| entry.target.as_str())
}

/// The proc whose **own declaration name token** covers `cursor_off`.
///
/// The `greet` of `proc greet {…} {…}` is an *argument* of `proc`, never a
/// command head, so a provider gated on head position
/// ([`offset_is_command_head`]) must answer it here instead. Declaration
/// cursors in the retained Logical domain select their own genuine original
/// header, including a declaration displaced by a later same-named procedure.
/// The separate legacy branch retains its reporting-map behaviour.
#[must_use]
pub(crate) fn proc_declaration_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let retained_source = std::str::from_utf8(image.bytes()).ok()?;
    crate::original_context::CurrentSourceContext::capture(retained_source, analysis)?;
    if analysis.allows_retained_logical_declaration_advice() {
        return logical_procedure_header_at(analysis, cursor_off)
            .map(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::metadata);
    }
    let covers = |span: tcl_lexer::Span| span.start() <= cursor_off && cursor_off < span.end();
    if let Some(proc_def) = analysis.all_procs.values().find(|p| covers(p.name_span)) {
        return Some(proc_def);
    }
    let (qualified, _) = analysis
        .proc_declaration_sites
        .iter()
        .find(|(_, span)| covers(*span))?;
    analysis.all_procs.get(qualified)
}

/// Select the unique genuine original Logical header under this source cursor.
/// The header owner checks the complete retained image, input and realm.
pub(super) fn logical_procedure_header_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
    let mut selected = analysis
        .logical_source_procedure_declarations()
        .filter(|header| {
            header.original_name_word().is_some_and(|word| {
                word.span().start() <= cursor_off && cursor_off < word.span().end()
            })
        });
    let header = selected.next()?;
    selected.next().is_none().then_some(header)
}

/// Select readonly metadata through this actual original Logical source call.
pub(super) fn logical_called_procedure_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
) -> Option<&'a tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let call =
        tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
            source,
            analysis,
            cursor_off,
        )?;
    call.declaration(analysis)
}

/// The procedure `head sub` at `cursor_off` dispatches to: the dispatch's own
/// target receipt at the ensemble invocation, else the static ensemble table's
/// target resolved like a call.
pub(crate) fn ensemble_called_procedure<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    (head, sub): (&str, &str),
    cursor_off: u32,
    view: CallResolution<'_>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let receipt = analysis
        .command_invocations
        .iter()
        .find(|inv| {
            inv.ensemble_dispatch.is_some()
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .and_then(|inv| inv.resolved_command_reference.as_ref()?.definition())
        .and_then(|definition| analysis.proc_for_definition(definition, source));
    if receipt.is_some() {
        return receipt;
    }
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let target = ensemble_subcommand_target(analysis, &namespace, head, sub)?;
    resolve_called_proc(analysis, source, "::", target, cursor_off, view)
}

/// The command invocation the analyser recorded whose **head token** covers
/// `cursor_off`, if any.
///
/// The one place definition / hover ask "is this position a command head at
/// all, and what did the resolution machinery decide it names?".  Recorded
/// invocations are the analyser's own answer: an entry exists exactly because
/// the analyser walked that word *as a command name*, in a region it proved
/// was live script — so it is a far stronger statement than the pure text
/// word-boundary scan ([`crate::hover::find_word_span_at_position`]) the
/// bareword fallbacks otherwise run on.
#[must_use]
pub(crate) fn invocation_head_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.lookup.is_execution_site()
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// A positioned name reference, including names consumed by another command.
/// Its lookup point is retained independently of the editable name span.
#[must_use]
pub(crate) fn invocation_reference_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| inv.range.start() <= cursor_off && cursor_off < inv.range.end())
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// Whether the word at `cursor_off` occupies its enclosing command's **head**
/// position — i.e. whether resolving it as a command name can be right at all.
///
/// Handing whatever word the text scan returned straight to
/// [`resolve_called_proc`] would let an ordinary *argument* that happens to
/// share a proc's name resolve onto that proc — a wrong answer, not an
/// abstention (`return [$types(callback:$tag) jsondump $huddle_object]`
/// resolving the ensemble subcommand `jsondump` onto the very proc containing
/// the call, which `anotherproc dump` reproduces with no indirection at all).
///
/// The gate is the analyser's recorded invocation set, not a second text
/// scan, so it agrees with find-references / rename / call-hierarchy by
/// construction — those already key off the same records.
///
/// **Deliberate limit.** A word inside a *braced argument of an unregistered
/// command* (`mycustom { helper }`) is not recorded, because nothing proves
/// that brace holds script rather than data, so the gate reads it as "not a
/// command head" and the provider abstains.  That is the honest answer: the
/// call may never happen.  Registry-known script arguments (`if`, `foreach`,
/// `switch` arms, `try`, `catch`, `eval`, `uplevel`, `namespace eval`,
/// `apply` bodies, `oo::define` member bodies, command-prefix callbacks such
/// as `lsort -command cb`, and `{*}`-expanded constant heads) *are* recorded
/// and keep resolving.
#[must_use]
pub(crate) fn offset_is_command_head(analysis: &AnalysisResult, cursor_off: u32) -> bool {
    invocation_head_at(analysis, cursor_off).is_some()
}

/// The declaration span an **already-resolved indirect** command head at
/// `cursor_off` reaches, or `None` when no such head covers the cursor.
///
/// The analyser resolves `set ns ::tc; ${ns}::setdef` to `::tc::setdef` and
/// records it as a `command_invocations` entry with `indirect: true`.  Falling
/// through to the bareword lookup instead would answer an unrelated same-named
/// `::other::setdef`: the span does not carry the written command name, so
/// there is nothing for a text scan to resolve — the recorded resolution is
/// the only correct answer, and it must win.  The same record settles a
/// constant `$cmd` head (`set cmd greet; $cmd`), which the text scan can
/// likewise never resolve.
///
/// Abstains — rather than blocking the ordinary fallback — whenever the
/// indirect entry is *unresolved* (`resolved_qualified_name` is `None`, or it
/// names nothing this document declares): an unknown indirection is no reason
/// to withhold an answer the bareword path can still give.
pub(super) fn resolved_indirect_head_target(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<tcl_lexer::Span> {
    if let Some(proc_def) = resolved_indirect_head_proc(analysis, cursor_off) {
        return Some(proc_def.name_span);
    }
    let invocation = indirect_invocation_head_at(analysis, cursor_off)?;
    let definition = if let Some(reference) = &invocation.resolved_command_reference {
        Some(
            reference
                .linked_definition()
                .or_else(|| reference.definition())?,
        )
    } else {
        invocation.resolved_definition.as_ref()
    };
    if let Some(definition) = definition {
        let tcl_names::origin::SourceOriginKind::Authored(source) =
            definition.allocation().site.source.kind()
        else {
            return None;
        };
        return analysis
            .class_for_definition(definition, source.try_text().ok()?)
            .map(|declaration| declaration.name_span);
    }
    let resolved = resolved_indirect_head_name(analysis, cursor_off)?;
    analysis
        .all_classes
        .get(resolved)
        .map(|class_def| class_def.name_span)
}

/// The qualified name an already-resolved indirect head at `cursor_off`
/// settled on — the shared first half of [`resolved_indirect_head_target`]
/// and [`resolved_indirect_head_proc`].
fn resolved_indirect_head_name(analysis: &AnalysisResult, cursor_off: u32) -> Option<&str> {
    indirect_invocation_head_at(analysis, cursor_off)?
        .resolved_qualified_name
        .as_deref()
}

fn indirect_invocation_head_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.lookup.is_execution_site()
                && inv.indirect
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// The `ProcDef` an already-resolved indirect head at `cursor_off` reaches —
/// the hover-side twin of [`resolved_indirect_head_target`], so
/// hover and go-to-definition answer the same indirection from one record.
#[must_use]
pub(crate) fn resolved_indirect_head_proc(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let invocation = indirect_invocation_head_at(analysis, cursor_off)?;
    let definition = if let Some(reference) = &invocation.resolved_command_reference {
        Some(
            reference
                .linked_definition()
                .or_else(|| reference.definition())?,
        )
    } else {
        invocation.resolved_definition.as_ref()
    };
    if let Some(definition) = definition {
        let tcl_names::origin::SourceOriginKind::Authored(source) =
            definition.allocation().site.source.kind()
        else {
            return None;
        };
        return analysis.proc_for_definition(definition, source.try_text().ok()?);
    }
    let resolved = resolved_indirect_head_name(analysis, cursor_off)?;
    let proc_def = analysis.all_procs.get(resolved)?;
    Some(
        analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, cursor_off)
            .unwrap_or(proc_def),
    )
}

/// Original C loader advice at one complete current command-head producer.
/// This retains caller geometry without proving loading or a callable target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAutoloadRequest {
    pub(super) name: tcl_names::word::SignatureSourceNameKey,
    invocation: tcl_compiler::command_binding::SourceInvocationBinding,
    unresolved_lookup: Option<tcl_compiler::command_binding::OriginalCommandLookup>,
}

impl OriginalAutoloadRequest {
    /// Byte offset of the original invocation head.
    #[must_use]
    pub fn head_offset(&self) -> u32 {
        self.name.span().start()
    }

    /// Files selected by genuine `auto_index` entries and the retained caller scope.
    #[must_use]
    pub fn files(
        &self,
        resolver: &crate::package_resolver::PackageResolver,
    ) -> Vec<std::path::PathBuf> {
        resolver.resolve_original_auto_command(&self.name, &self.invocation)
    }

    /// Select readonly source candidates from the implementations of the
    /// caller's required packages. The authentic positioned lookup supplies
    /// name policy and search order, independently of loading or dispatch.
    #[must_use]
    pub const fn unresolved_source_lookup(
        &self,
    ) -> Option<&tcl_compiler::command_binding::OriginalCommandLookup> {
        self.unresolved_lookup.as_ref()
    }
}

/// Select original autoload advice only at the retained complete invocation head.
#[must_use]
pub fn original_autoload_request_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<OriginalAutoloadRequest> {
    use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    let realm = analysis.retained_command_realm()?;
    if !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let mut sites = analysis.command_invocations.iter().filter(|site| {
        site.lookup == tcl_compiler::signature_scan::types::SignatureCommandLookup::InvocationHead
            && site.range.start() <= offset
            && offset < site.range.end()
    });
    let site = sites.next()?;
    if sites.next().is_some() {
        return None;
    }
    let invocation = realm.invocation_at_source("", site.range.start());
    let name_input = invocation.original_recorded_head_name_input()?;
    if site.original_name_input.as_ref() != Some(&name_input)
        || !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            &name_input,
            site.range,
        )
    {
        return None;
    }
    let SignatureSourceNameInput::OriginalWord(name) = name_input else {
        return None;
    };
    invocation.original_autoload_command_candidates(&name)?;
    let unresolved_lookup =
        if site.resolved_command_reference.is_none() && site.resolved_definition.is_none() {
            invocation
                .original_static_command_lookup(&name)
                .filter(|lookup| site.original_lookup.as_ref() == Some(lookup))
        } else {
            None
        };
    Some(OriginalAutoloadRequest {
        name,
        invocation,
        unresolved_lookup,
    })
}

/// The bare command word at `(line, character)` together with the
/// `::`-prefixed namespace in effect there.
///
/// For the autoload-definition fallback: when a command head resolves to
/// nothing in the current document *or* the workspace index, the caller feeds
/// this `(word, namespace)` to the package / auto-load database
/// ([`crate::package_resolver::PackageResolver::resolve_auto_command`]) to find
/// the library file that defines it.  Returns `None` off any word.  The word
/// is returned verbatim (the resolver auto-qualifies it against the
/// namespace); `namespace` is `"::"` at global scope.
#[must_use]
pub fn command_head_and_namespace_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<(String, String)> {
    let config = analysis.body_lexer_config?;
    if analysis.allows_retained_logical_declaration_advice() {
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
        let mut candidates =
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?
            .into_iter()
            .filter(|candidate| {
                candidate
                    .original_head()
                    .is_some_and(|head| head.span().start() <= offset && offset < head.span().end())
            });
        let candidate = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        let head = candidate.original_head()?;
        let word = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            head,
        )?)
        .ok()?;
        let namespace = candidate
            .declared_source_context()
            .map_or_else(tcl_core_types::ByteNamespacePath::root, |context| {
                context.namespace_recipe().clone()
            });
        return Some((
            word,
            tcl_syntax::naming::checked_namespace_path_key(&namespace)?,
        ));
    }
    None
}
