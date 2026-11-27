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

//! Whether a call reaches a procedure through `namespace import`: wildcard imports, forced imports and export verdicts.

use super::command_heads::invocation_head_at;
use super::import_slot_events::{
    DocumentOrderCell, exported_original_slot_at_import_in, in_document_point, live_import_at,
};
use super::{CallResolution, ProgramExports, namespace_context_at};
use crate::namespace_import::ExportVerdict;
use tcl_compiler::analyser::{AnalysisResult, indirection};

/// Resolve a bareword `word` (`namespace` context, honouring `namespace
/// path` exactly like [`proc_visible_from_namespace`]) through an
/// in-document `namespace import NS::*` — a glob (or exact) pattern
/// recorded in `analysis.namespace_imports`, as the call at `call_off` sees
/// it.
///
/// For each candidate qualified name Tcl's own lookup would try (the
/// caller's namespace, then each `namespace path` entry, then global — the
/// same order [`proc_visible_from_namespace`] walks), this checks whether
/// that candidate's namespace **still holds a live imported alias** for
/// `word` at `call_off` — see [`live_import_at`] for the lifecycle that
/// question covers, and
/// [`crate::namespace_import::exported_at_import_site`] for the export
/// snapshot each import site is judged by.
///
/// Only resolves a source namespace defined in **this** document
/// (`analysis.all_procs`); a source namespace defined in another file is
/// the cross-document oracle's job
/// (`tcl_lsp_core::workspace_index::WorkspaceIndex::resolve_wildcard_import`).
pub(super) fn proc_visible_via_wildcard_import<'a>(
    analysis: &'a AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let source_ns = wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)?;
    analysis
        .all_procs
        .get(&tcl_syntax::naming::qualify(&source_ns, word))
}

/// Whether a live `namespace import -force` covering `word` has taken the
/// name away from `namespace`'s own command table by the time the call at
/// `call_off` runs — the only import that outranks a same-named command the
/// importing namespace already holds.
///
/// Oracle (tclsh 8.6.14 / 9.0.4, byte-identical): with `proc ::dst::p`
/// returning `LOCAL` and `::src::p` returning `SRC`, `namespace eval ::dst
/// {namespace import -force ::src::*}` makes `::dst::p` return `SRC` and
/// `namespace origin ::dst::p` answer `::src::p`. Without `-force` the same
/// import raises `can't import command "p": already exists` and installs
/// nothing at all — which [`live_import_at`] models by declining the import
/// outright, so the two cases agree by construction.
///
/// Consulted *before* [`proc_visible_from_namespace`] in
/// [`resolve_called_proc`], because that is the whole point of `-force`:
/// from the import's own position onward, the local definition is gone. It
/// answers a plain boolean rather than a `ProcDef` because the shadow holds
/// whether or not this document can see the source — a `-force` import of a
/// namespace defined in *another* file still deletes the local command, and
/// answering with that local definition would be wrong; the cross-document
/// resolver takes it from there.
pub(crate) fn forced_import_shadows(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> bool {
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
        return false;
    }
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    import_hop(
        analysis,
        ctx,
        namespace,
        path,
        word,
        call_off,
        ImportQuery::ForcedShadow,
    )
    .is_some()
}

/// [`forced_import_shadows`] over a call's already-computed candidate list —
/// the shape the cross-document resolver has in hand.
///
/// `resolve_workspace_symbols` (`tcl-lsp-server`) is a separate resolver, not
/// a caller of [`resolve_called_proc`], so it needs its own application of the
/// same rule: a candidate naming this document's own `proc bar` must not
/// settle the call while a live `namespace import -force ::Lib::*` has
/// replaced it — the call reaches `::Lib::bar`, which only the workspace tier
/// can find. Without this the two resolvers disagree, and the one that answers
/// first wins with the definition the program no longer has.
#[must_use]
pub fn forced_import_shadows_call(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    word: &str,
    resolution_candidates: &[String],
    call_off: u32,
) -> bool {
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
        return false;
    }
    live_import_over_candidates(
        analysis,
        ctx,
        resolution_candidates.iter().map(String::as_str),
        word,
        call_off,
        ImportQuery::ForcedShadow,
    )
    .is_some()
}

/// Whether the original-name providers abstain at the call whose head covers
/// `offset` (user decision 2026-10-22T07:18Z, `force_import_shadow_direction`).
///
/// A `namespace import -force` that the document alone shows binding nothing
/// leaves the fresh-interpreter answer (the local command) in place. When
/// only the caller's whole-program export view says a sibling exports the
/// name, the shadow depends on cross-document execution order, which stays
/// conditional (decision 2026-10-17T20:30Z `export_order`): neither the
/// local definition nor the sibling's export is proven, so no provider
/// answers for that call.
#[must_use]
pub fn program_forced_import_abstains(
    analysis: &AnalysisResult,
    program: Option<ProgramExports<'_>>,
    offset: u32,
) -> bool {
    let Some(program) = program else {
        return false;
    };
    let Some(invocation) = invocation_head_at(analysis, offset) else {
        return false;
    };
    let word = invocation.name.as_str();
    let call_off = invocation.range.start();
    let namespace = namespace_context_at(
        &analysis.global_scope,
        call_off,
        &analysis.namespace_overrides,
    );
    forced_import_shadows(
        analysis,
        CallResolution::document_only().in_program(program),
        &namespace,
        word,
        call_off,
    ) && !forced_import_shadows(
        analysis,
        CallResolution::document_only(),
        &namespace,
        word,
        call_off,
    )
}

/// Which question [`import_hop`] / [`live_import_at`] are being asked, and
/// therefore what an *absent* `namespace export` record means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImportQuery {
    /// "Which namespace does this call really reach?" Every import counts,
    /// and the export gate is strict: only a proven export
    /// ([`ExportVerdict::Exported`]) installs, because an unproven one gives
    /// nothing to resolve *to*.
    Resolve,
    /// "Has a `namespace import -force` taken this name away from the
    /// importing namespace's own command table?" ([`forced_import_shadows`])
    /// Only forced imports count, and the export gate abstains the other way:
    /// everything but a proven [`ExportVerdict::NotExported`] counts as having
    /// installed. `namespace import -force ::Lib::*` deletes the local command
    /// whether or not anything here can see `::Lib`'s export declaration, and
    /// answering with the deleted definition would be worse than abstaining.
    /// The same "absence of an export is evidence only where the exports are
    /// visible too" rule `WorkspaceIndex::live_command_links` applies
    /// cross-document.
    ForcedShadow,
}

impl ImportQuery {
    /// Whether an import whose export gate returned `verdict` counts as having
    /// installed, for this question.
    ///
    /// The two arms are the two abstention directions in one place, so neither
    /// can drift: [`Resolve`](Self::Resolve) needs proof to answer,
    /// [`ForcedShadow`](Self::ForcedShadow) needs proof to *stop* answering.
    pub(super) fn installs(self, verdict: ExportVerdict) -> bool {
        match self {
            ImportQuery::Resolve => verdict == ExportVerdict::Exported,
            ImportQuery::ForcedShadow => verdict != ExportVerdict::NotExported,
        }
    }
}

/// Whether `source_ns` had exported `word` when the import at `import_at` ran,
/// as everything available to this resolution can tell — the single decision
/// both [`ImportQuery`] arms read, differing only in which way they abstain.
///
/// Three sources of evidence, in order of authority:
///
/// 1. **This document's own export records.** A covering export that had run
///    at the import is proof, and it is proof even when the workspace index is
///    stale relative to the buffer being edited — which is why it is consulted
///    first rather than left to the oracle.
/// 2. **The whole-program oracle**, when one is attached. It sees every
///    indexed file's exports, ranks them against this import with the `source`
///    graph's run order, and may itself answer
///    [`ExportVerdict::Unknown`].
/// 3. **The document-only fallback**, when no oracle is attached: absence of
///    an export is evidence exactly where this document holds *some* export
///    record for `source_ns`, and no information otherwise.
///
/// # Why an oracle at all
///
/// The document-only fallback is the whole problem. It is the in-document twin of
/// `WorkspaceIndex::observable_namespaces`, and cross-document that framing is
/// right — the index really does hold every file. In *one* document it is not:
/// a namespace can be partly visible here (its procs, and some of its exports)
/// while the one `namespace export` that covers this name sits in another
/// file. The false-positive and the true-negative then have **byte-identical**
/// single-document inputs and opposite correct answers (transcript on
/// [`NamespaceExportOracle`]), so no rule reading only this document can
/// separate them — hence the oracle, and hence its optionality: a caller with
/// no workspace keeps the document-only fallback.
pub(super) fn export_verdict(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source_ns: &str,
    word: &str,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let Some(name) = original_export_command(analysis, source_ns, word) else {
        return ExportVerdict::Unknown;
    };
    export_verdict_for_slot_in(analysis, ctx, name.slot(), name.policy(), import_at, order)
}

#[cfg(test)]
pub(super) fn export_verdict_for_slot(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
) -> ExportVerdict {
    export_verdict_for_slot_in(
        analysis,
        ctx,
        slot,
        policy,
        import_at,
        &DocumentOrderCell::new(),
    )
}

pub(super) fn export_verdict_for_slot_in(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let local = exported_original_slot_at_import_in(analysis, ctx, slot, policy, import_at, order);
    if local == ExportVerdict::Exported {
        return local;
    }
    if let Some(program) = ctx.program {
        return program.oracle.exported_at_original(
            slot,
            policy,
            in_document_point(analysis, ctx, import_at),
        );
    }
    let scope = tcl_names::namespace::SignatureNamespaceScope::C(slot.namespace.clone());
    let observable = namespace_exports_observable(analysis, &scope, policy);
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_EXPORT_SOURCE_QUERY at={import_at} scope={scope:?} tail={:?} local={local:?} observable={observable} exports={} unknown={}",
            slot.simple.as_bytes(),
            analysis.original_namespace_exports().count(),
            analysis.original_namespace_export_unknowns().len()
        );
    }
    if observable {
        local
    } else {
        ExportVerdict::Unknown
    }
}

/// Whether this document holds *any* `namespace export` record for
/// the exact namespace scope and policy — the document-only fallback
/// [`export_verdict`] uses when no
/// whole-program oracle is attached.
///
/// The proposition being asserted is about the export list, so the evidence
/// has to be the export list: a document that declares no export at all for
/// that namespace knows nothing about what it exports. Its **residual** — a
/// document holding some of the namespace's exports but not the covering one
/// still reads the gap as a fact — is precisely what the oracle removes when
/// one is available, and what remains, deliberately, when one is not.
pub(super) fn namespace_exports_observable(
    analysis: &AnalysisResult,
    scope: &tcl_names::namespace::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    analysis
        .original_namespace_exports()
        .any(|event| event.context() == scope && event.policy() == policy)
}

/// Associate an advisory qualified label with already retained original source
/// publication records. A label cannot create bytes; disagreeing publications
/// and missing original records are terminal.
fn original_export_command<'a>(
    analysis: &'a AnalysisResult,
    source_ns: &str,
    word: &str,
) -> Option<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    let label = tcl_syntax::naming::qualify(source_ns, word);
    let names = analysis
        .original_procedure_declarations()
        .filter(|record| record.metadata().qualified_name == label)
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .chain(
            analysis
                .original_class_declarations()
                .filter(|record| record.metadata().qualified_name == label)
                .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name),
        );
    let mut found = None;
    for name in names {
        if found.is_some_and(
            |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                previous.slot() != name.slot() || previous.policy() != name.policy()
            },
        ) {
            return None;
        }
        found = Some(name);
    }
    found
}

/// Shared candidate walk behind [`proc_visible_via_wildcard_import`],
/// [`proc_visible_via_forced_import`] and [`resolve_class_target_at`]'s
/// import fallback: find the namespace whose command `word` really names
/// when the call at `call_off` reaches it through one or more import edges,
/// or `None`.
///
/// The caller joins the result with `word` ([`tcl_syntax::naming::qualify`])
/// and looks the qualified name up in whichever map (`all_procs` /
/// `all_classes`) applies — which is also the map this walk consults to
/// decide whether a chain has terminated.
///
/// Restricted to a genuine bareword `word` (no embedded `::`) — `namespace
/// import` / `namespace export` patterns only ever name simple command
/// tails; a qualified call
/// (`inner::p`) never goes through an imported alias in real Tcl, so this
/// abstains rather than risk mismatching a namespace-path segment against
/// an unrelated import.
///
/// # Import chains
///
/// An import edge may land on a name that is *itself* imported: with `::C`
/// exporting `p`, `::B` importing `::C::*` and re-exporting, and `::A`
/// importing `::B::*`, `::A::p` runs `::C`'s body and `namespace origin
/// ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4).
/// Statically the middle hop is in no `all_procs`, so a single-hop walk
/// finds nothing. This one follows the chain while every hop is provable,
/// bounded by [`indirection::MAX_COMMAND_NAME_HOPS`] — the same cap the
/// `rename` / `interp alias` walk applies to the same kind of chain, so a
/// mutually-importing pair cannot spin — and abstains otherwise.
///
/// The whole chain is judged at the *call's* offset, not at each import's:
/// a forget anywhere along it kills the call (oracle: forgetting `::C::p`
/// inside `::B` makes `::A::p` an `invalid command name` too, because
/// deleting an imported command deletes the commands imported *from* it).
pub(super) fn wildcard_import_source_namespace(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<String> {
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
        return None;
    }
    let mut current = namespace.to_owned();
    let mut first_hop = true;
    for _ in 0..indirection::MAX_COMMAND_NAME_HOPS {
        let path = if first_hop {
            analysis
                .namespace_paths
                .get(current.as_str())
                .map_or(&[][..], Vec::as_slice)
        } else {
            // A chain's inner hop starts from the previous hop's source
            // namespace, where the alias is an ordinary command of that
            // namespace — `namespace path` is a *caller*-side search rule and
            // has no bearing on it.
            &[][..]
        };
        let hop = import_hop(
            analysis,
            ctx,
            &current,
            path,
            word,
            call_off,
            ImportQuery::Resolve,
        )?;
        // The chain terminates where a real definition lives: everything the
        // caller can look the name up in.
        let qualified = tcl_syntax::naming::qualify(&hop, word);
        if analysis.all_procs.contains_key(&qualified)
            || analysis.all_classes.contains_key(&qualified)
        {
            return Some(hop);
        }
        current = hop;
        first_hop = false;
    }
    None
}

/// One hop of [`wildcard_import_source_namespace`]: the source namespace of
/// the live import that makes `word` callable from `namespace` (or one of its
/// `path` entries) at `call_off`.
fn import_hop<S: AsRef<str>>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    path: &[S],
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    let candidates = tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
        namespace, path, word,
    );
    live_import_over_candidates(
        analysis,
        ctx,
        candidates.iter().map(String::as_str),
        word,
        call_off,
        query,
    )
}

/// [`import_hop`] over an already-computed candidate list: the first candidate
/// namespace holding a live import edge for `word` wins, in Tcl's own
/// resolution order.
fn live_import_over_candidates<'c>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    candidates: impl Iterator<Item = &'c str>,
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    for candidate in candidates {
        let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(candidate);
        if tail != word {
            continue;
        }
        let candidate_ns = if prefix.is_empty() { "::" } else { prefix };
        if let Some(hop) = live_import_at(analysis, ctx, candidate_ns, word, call_off, query) {
            return Some(hop);
        }
    }
    None
}

/// Whether the call at `call_off` in `namespace` reaches `def_qualified`
/// through a live import edge — the entry point `references.rs` shares with
/// go-to-definition so the two cannot disagree about an import's lifecycle.
///
/// Returns the terminal source namespace the chain lands on, which the caller
/// compares against the definition it is gathering references for.
pub(crate) fn import_chain_target(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<String> {
    wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)
}
