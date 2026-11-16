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

//! The ordered import, forget and definition events on one namespace slot, and the alias live at a call.

use super::CallResolution;
use super::call_resolution::same_namespace;
use super::namespace_imports::{ImportQuery, export_verdict, export_verdict_for_slot_in};
use crate::namespace_import::ExportVerdict;
use crate::source_graph::{RunOrder, RunPoint};
use tcl_compiler::analyser::{AnalysisResult, indirection};

/// One ordered event on an importing namespace's `word` slot, as the
/// same-document tier sees it.
///
/// The four things that can happen to `<importing_ns>::<word>` over the life
/// of a script. Folding them in order answers both questions
/// [`live_import_at`] asks: *what did this namespace already hold when import
/// I ran?* (the non-`-force` conflict) and *what does it hold at the call?*
#[derive(Debug, Clone, Copy)]
enum SlotEvent<'a> {
    /// A `namespace import` covering `word` whose export snapshot passed.
    Import {
        at: u32,
        source_ns: &'a str,
        forced: bool,
    },
    /// A `namespace forget` covering `word`; `source_ns` is `None` for the
    /// simple pattern form, which matches whatever the alias was imported
    /// from.
    Forget { at: u32, source_ns: Option<&'a str> },
    /// A `proc` / class declaration of `<importing_ns>::<word>` — the
    /// namespace's *own* command of that name.
    Declare { at: u32 },
    /// A destroying `rename <source_ns>::<word> {}` — the command object the
    /// alias holds is gone.
    Destroy { at: u32, source_ns: &'a str },
}

impl SlotEvent<'_> {
    pub(super) fn at(self) -> u32 {
        match self {
            SlotEvent::Import { at, .. }
            | SlotEvent::Forget { at, .. }
            | SlotEvent::Declare { at }
            | SlotEvent::Destroy { at, .. } => at,
        }
    }
}

/// Whether a forget event covers an alias taken from `source_ns`.
fn forget_covers(forget_source: Option<&str>, source_ns: &str) -> bool {
    forget_source.is_none_or(|src| same_namespace(src, source_ns))
}

/// The source namespace of the import alias `importing_ns` holds for `word`
/// **at `call_off`**, or `None` when it holds none — the same-document
/// binding of the shared lifecycle decision
/// [`crate::namespace_import::alias_live_at`].
///
/// An import edge is a link with a lifecycle, not a standing name-visibility
/// fact. Everything that writes the `<importing_ns>::<word>` slot is an event
/// on one ordered log ([`SlotEvent`]); all of it is oracle-confirmed
/// byte-identically on tclsh 9.0.4 and 8.6.14:
///
/// 1. **The import installs it** — gated by the source namespace's export
///    snapshot at the import's own position
///    ([`exported_original_slot_at_import`]).
/// 2. **A conflict makes it install nothing.** Without `-force`, importing
///    onto a name the target namespace already holds raises `can't import
///    command "p": already exists`; the existing command survives and
///    `namespace origin` still answers it. "Already holds" covers a local
///    `proc`/class **and a live alias from a different source**: with `::dst`
///    already importing `::A::*`, a later unforced `namespace import ::B::*`
///    fails with exactly that error and leaves `namespace origin ::dst::p` →
///    `::A::p` (oracle). Re-importing from the *same* source is a silent
///    no-op instead, so it is not a conflict. With `-force` the import wins
///    and replaces whichever of the two was there (oracle: `origin` → `::B::p`),
///    which is what [`ImportQuery::ForcedShadow`] selects for. (Modelling the
///    *error* — that the rest of that script never runs — is deliberately out
///    of scope; only the "installed nothing" half is modelled.)
/// 3. **`namespace forget` removes it** — `namespace forget ::src::p`, or the
///    simple form `namespace forget p`, and a later bare call is `invalid
///    command name`.
/// 4. **Deleting the source command removes it** — the alias holds the
///    command *object*, so `rename ::src::p {}` kills `::dst::p` too, while a
///    plain `rename ::src::p ::src::pp` leaves it alive (only the origin
///    moves) and a redefinition of `::src::p` is seen straight through the
///    link. That asymmetry is `indirection`'s own
///    rename-captures-object-identity rule, which is why only the *deletion*
///    appears here.
/// 5. **Redefining the imported name removes it.** A `proc ::dst::p` written
///    after the import silently recreates `::dst::p` as an ordinary command:
///    oracle (9.0.4 / 8.6.14, `namespace import -force ::src::*` then `proc
///    ::dst::p`) — the redefinition raises no error, `::dst::p` runs the new
///    body, `namespace origin ::dst::p` becomes `::dst::p`, and `::src::p`
///    is untouched. A call written *between* the import and the redefinition
///    still reaches `::src::p`, so this is an ordered event like every other,
///    not a file-wide fact.
///
/// Ordering is [`indirection::in_effect`] — the same "had this statement
/// run?" primitive the export snapshot and the `rename` / `interp alias`
/// timeline use, so none of them can disagree about what "already ran" means.
/// It gates the *installs* as well as the removals: a
/// bare call written before its own `namespace import` reaches nothing
/// (oracle on [`crate::namespace_import::alias_live_at`]), and because the
/// primitive is `in_effect` rather than a raw offset test, a call inside a
/// proc body still observes a top-level import written later in the same file
/// — the whole file loads before any body runs.
///
/// The conflict fold below is deliberately *not* gated that way: which import
/// installs anything is decided at each import's **own** position, wherever
/// the call sits. Only the choice of winning edge, and the edge's own
/// lifecycle, are judged from the call.
pub(super) fn live_import_at(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    importing_ns: &str,
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    use crate::namespace_import::{AliasEvent, AliasEventKind};
    let order = DocumentOrderCell::new();
    let events = slot_events(analysis, ctx, importing_ns, word, query, call_off, &order);
    // Which imports actually install something: an unforced import onto a
    // slot the namespace already holds (a local declaration, or a live alias
    // from a *different* source) raises `already exists` and binds nothing.
    // Decided by folding the log in order, each import judged by what the
    // slot held when *it* ran — the same question the call-site gate below
    // asks at its own point.
    let mut held: Option<&str> = None;
    let mut declared = false;
    let mut installs: Vec<(u32, &str)> = Vec::new();
    let mut latest: Option<(u32, &str)> = None;
    for ev in &events {
        match *ev {
            SlotEvent::Declare { .. } => {
                declared = true;
                held = None;
            }
            SlotEvent::Forget { source_ns, .. } => {
                if held.is_some_and(|src| forget_covers(source_ns, src)) {
                    held = None;
                }
            }
            SlotEvent::Destroy { source_ns, .. } => {
                if held == Some(source_ns) {
                    held = None;
                }
            }
            SlotEvent::Import {
                at,
                source_ns,
                forced,
            } => {
                let conflicts = declared || held.is_some_and(|current| current != source_ns);
                if !forced && conflicts {
                    continue;
                }
                held = Some(source_ns);
                // A `-force` import replaces whatever was there, including a
                // local declaration.
                declared = false;
                installs.push((at, source_ns));
                // The latest install *that has already run at the call* wins
                // the source namespace. An import the
                // call has not reached yet cannot decide which edge the call
                // takes — the same gate `alias_live_at` then applies to the
                // edge's own log, so the two cannot pick different imports.
                if indirection::in_effect(analysis, at, call_off)
                    && latest.is_none_or(|(best, _)| at > best)
                {
                    latest = Some((at, source_ns));
                }
            }
        }
    }
    let (_, source_ns) = latest?;
    // Only the winning source's own installs order against its removals: an
    // import of the same name from a *different* namespace is a different
    // edge, and letting it out-date this edge's forget would resurrect it.
    // The install carries its enclosing body, the removals do not — not an
    // oversight: the fold reads a body span only off the point it is ranking
    // *against* (`ran_after(removal, install)` asks whether the removal had run
    // at the install), and a removal is never that point. Computing one per
    // removal would be a body scan that decides nothing.
    let install_point = |at: u32| in_document_point(analysis, ctx, at);
    let removal_point = |at: u32| RunPoint {
        uri: ctx.uri(),
        at,
        enclosing_body: None,
    };
    let mut log = events
        .iter()
        .filter_map(|ev| match *ev {
            SlotEvent::Import {
                at, source_ns: s, ..
            } if s == source_ns => installs
                .iter()
                .any(|&(i, _)| i == at)
                .then_some(AliasEvent {
                    kind: AliasEventKind::Install,
                    at: install_point(at),
                }),
            SlotEvent::Forget { at, source_ns: s } if forget_covers(s, source_ns) => {
                Some(AliasEvent {
                    kind: AliasEventKind::Remove,
                    at: removal_point(at),
                })
            }
            SlotEvent::Destroy { at, source_ns: s } if s == source_ns => Some(AliasEvent {
                kind: AliasEventKind::Remove,
                at: removal_point(at),
            }),
            SlotEvent::Declare { at } => Some(AliasEvent {
                kind: AliasEventKind::Remove,
                at: removal_point(at),
            }),
            _ => None,
        })
        .collect::<Vec<_>>()
        .into_iter();
    crate::namespace_import::alias_live_at(
        &mut log,
        document_order(&order, analysis, ctx),
        Some(in_document_point(analysis, ctx, call_off)),
    )
    .then(|| source_ns.to_owned())
}

/// The document key every in-document import event carries **when no
/// workspace view is attached**.
///
/// This tier holds exactly one document, so the *identity* of the key is
/// irrelevant — what matters is that every point shares it, which is what makes
/// [`crate::source_graph::RunOrder`] fall through to the single-document rule
/// ([`tcl_compiler::analyser::indirection::in_effect_within`]) without
/// consulting any graph. The cross-document tier passes real URIs to the same
/// functions, so the two cannot drift.
///
/// The order passed with those points is **empty**: one document needs no
/// `source` graph — every point shares this key, so
/// [`crate::source_graph::RunOrder`] answers from the single-document rule
/// alone and never looks at an edge. Building it allocates nothing.
///
/// With a [`ProgramExports`] view attached the points carry the document's
/// **real** URI instead, so the workspace's own order can rank a foreign
/// export against them — and, just as importantly, can rank *this* document's
/// exports against them by offset rather than shrugging.
pub(super) const IN_DOCUMENT_URI: &str = "";

/// An offset in the document being analysed, as a
/// [`crate::source_graph::RunPoint`] — with the enclosing definition body the
/// load-order rule needs, which is the only thing
/// [`indirection::in_effect`] reads out of the analysis.
pub(super) fn in_document_point<'a>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'a>,
    at: u32,
) -> RunPoint<'a> {
    RunPoint {
        uri: ctx.uri(),
        at,
        enclosing_body: analysis.original_definition_body_span(at).or_else(|| {
            analysis
                .allows_lexical_declaration_advice()
                .then(|| analysis.innermost_definition_body_span(at))
                .flatten()
        }),
    }
}

/// Every [`SlotEvent`] bearing on `<importing_ns>::<word>` in this document,
/// in **execution** order.
///
/// Ordering the four record kinds against each other once, here, is what lets
/// [`live_import_at`] answer the conflict question and the call-site question
/// with the same log instead of two ad-hoc scans.
///
/// # Why not source order
///
/// The whole file loads — running every top-level statement — before any
/// proc/class body runs, so a top-level statement written *below* a
/// body-local one still happens first. That is the same load-order rule
/// [`indirection::in_effect`] states pointwise; sorted here it is the total
/// order the conflict fold needs. Oracle, byte-identical on tclsh 8.6.14 and
/// 9.0.4:
///
/// ```tcl
/// namespace eval ::A { proc x {} {return A}; namespace export x }
/// namespace eval ::B { proc x {} {return B}; namespace export x }
/// namespace eval ::dst { proc p {} { namespace import ::B::x } }
/// namespace eval ::dst { namespace import ::A::* }
/// namespace origin ::dst::x   ;# → ::A::x
/// ::dst::p                    ;# → can't import command "x": already exists
/// namespace origin ::dst::x   ;# → ::A::x   (unchanged)
/// ```
///
/// Folding that log in raw offset order gets *both* halves wrong: it lets the
/// body-local `::B` import install, and then conflicts the top-level `::A`
/// one away. Within one tier — all top level, or all inside one body — the
/// key degenerates to a plain offset comparison, so genuinely sequenced
/// statements are unaffected. Two events in *different* bodies have no static
/// order either way; they keep their offset order.
///
/// The workspace tier reaches the same answer through
/// `WildcardImportIndex::conflicting_alias_at`, which asks
/// [`indirection::in_effect_within`] per candidate rather than sorting a log —
/// same rule, stated over the facts each tier holds.
fn slot_events<'a>(
    analysis: &'a AnalysisResult,
    ctx: CallResolution<'_>,
    importing_ns: &str,
    word: &str,
    query: ImportQuery,
    call_off: u32,
    order: &DocumentOrderCell,
) -> Vec<SlotEvent<'a>> {
    let mut events: Vec<SlotEvent<'a>> = Vec::new();
    #[cfg(debug_assertions)]
    trace_slot_query(analysis, importing_ns, word, call_off);
    for imp in analysis
        .namespace_imports
        .iter()
        .filter(|i| i.ns == importing_ns)
    {
        if query == ImportQuery::ForcedShadow && !imp.forced {
            continue;
        }
        let Some(source) = &imp.source else {
            continue;
        };
        let source_ns = source.namespace.as_str();
        let export_tail = source.tail_pattern.as_str();
        // A pattern rooted at the global namespace (`namespace import ::p`,
        // `namespace import ::*`) splits to an *empty* source namespace, which
        // is easy to read as "no source" and skip — the one import shape that
        // would then bypass the export gate entirely. Real Tcl gives it no
        // special treatment: `proc p {}` with no
        // `namespace export p` at global level makes `namespace import ::p` a
        // silent no-op like any other unexported import, and `info commands
        // ::dst::*` stays empty (oracle 8.6.14 / 9.0.4). `::` *is* the source
        // namespace — the spelling every export record for global scope
        // already uses.
        let source_ns = if source_ns.is_empty() {
            "::"
        } else {
            source_ns
        };
        let at = imp.range.start();
        let original = analysis.original_namespace_patterns().find(|pattern| {
            pattern.span() == imp.range
                && pattern.purpose()
                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
        });
        let input = slot_call_inputs(analysis, call_off);
        let verdict = if let (Some(original), Some(first)) = (original, input.first()) {
            if input.iter().any(|other| *other != *first) {
                continue;
            }
            let Some(slot) = original.source_slot_for_call_input(first) else {
                continue;
            };
            if original.matches_imported_command(&slot, first.policy()) != Some(true) {
                continue;
            }
            export_verdict_for_slot_in(analysis, ctx, &slot, first.policy(), at, order)
        } else if let Some(original) = original {
            // Explicit authored query labels associate existing original
            // publication tails; they cannot produce native name bytes.
            let names = slot_named_publications(analysis, word);
            let Some(first) = names.first() else {
                continue;
            };
            if names.iter().any(|other| {
                other.slot().simple != first.slot().simple || other.policy() != first.policy()
            }) {
                continue;
            }
            let Some(slot) = original.source_slot_for_command_slot(first.slot(), first.policy())
            else {
                continue;
            };
            if original.matches_imported_command(&slot, first.policy()) != Some(true) {
                continue;
            }
            export_verdict_for_slot_in(analysis, ctx, &slot, first.policy(), at, order)
        } else {
            if !analysis.allows_lexical_declaration_advice()
                || !tcl_syntax::glob::string_match(export_tail, word)
            {
                continue;
            }
            export_verdict(analysis, ctx, source_ns, word, at, order)
        };
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_IMPORT_SOURCE_PATTERN ns={importing_ns} source={source_ns} at={at} original={} call_inputs={} verdict={verdict:?}",
                original.is_some(),
                input.len()
            );
        }
        if !query.installs(verdict) {
            continue;
        }
        events.push(SlotEvent::Import {
            at,
            source_ns,
            forced: imp.forced,
        });
        // The source command's own destruction, ordered against this edge.
        if let Some(&at) = analysis
            .destroyed_commands
            .get(&tcl_syntax::naming::qualify(source_ns, word))
        {
            events.push(SlotEvent::Destroy { at, source_ns });
        }
    }
    append_slot_lifecycle_events(analysis, importing_ns, word, &mut events);
    events
}

#[cfg(debug_assertions)]
fn trace_slot_query(analysis: &AnalysisResult, importing_ns: &str, word: &str, call_off: u32) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_IMPORT_SOURCE_QUERY ns={importing_ns} word={word} at={call_off} imports={} patterns={} exports={} unknown={}",
            analysis
                .namespace_imports
                .iter()
                .filter(|imp| imp.ns == importing_ns)
                .count(),
            analysis.original_namespace_patterns().count(),
            analysis.original_namespace_exports().count(),
            analysis.original_namespace_export_unknowns().len()
        );
    }
}

fn slot_call_inputs(
    analysis: &AnalysisResult,
    call_off: u32,
) -> Vec<&tcl_compiler::signature_scan::scope::SignatureSourceNameInput> {
    analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            invocation.range.start() == call_off
                || invocation
                    .original_lookup
                    .as_ref()
                    .is_some_and(|lookup| lookup.site().offset == call_off)
        })
        .filter_map(|invocation| {
            invocation
                .original_lookup
                .as_ref()
                .map(tcl_compiler::command_binding::OriginalCommandLookup::name_input)
        })
        .collect::<Vec<_>>()
}

fn slot_named_publications<'a>(
    analysis: &'a AnalysisResult,
    word: &str,
) -> Vec<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    analysis
        .original_procedure_declarations()
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .chain(
            analysis
                .original_class_declarations()
                .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name),
        )
        .filter(|name| name.slot().simple.try_utf8().ok() == Some(word))
        .collect::<Vec<_>>()
}

fn append_slot_lifecycle_events<'a>(
    analysis: &'a AnalysisResult,
    importing_ns: &str,
    word: &str,
    events: &mut Vec<SlotEvent<'a>>,
) {
    for f in analysis
        .namespace_forgets
        .iter()
        .filter(|f| f.ns == importing_ns && tcl_syntax::glob::string_match(&f.pattern, word))
    {
        events.push(SlotEvent::Forget {
            at: f.range.start(),
            source_ns: f.source_ns.as_deref(),
        });
    }
    let qualified = tcl_syntax::naming::qualify(importing_ns, word);
    for at in analysis
        .proc_declarations(&qualified)
        .map(|p| p.name_span.start())
        .chain(
            analysis
                .all_classes
                .get(&qualified)
                .map(|c| c.name_span.start()),
        )
    {
        events.push(SlotEvent::Declare { at });
    }
    // Load level first, then bodies — see the doc comment. The key is the one
    // the cross-document tier compares its two import sites with, so neither
    // can drift. `sort_by_cached_key` because the body test is a scan of the
    // recorded definition bodies, so it must run once per event, not once per
    // comparison. Equal offsets imply equal body-ness, so the dedup below
    // still sees its duplicates adjacent.
    events.sort_by_cached_key(|ev| {
        crate::namespace_import::load_order(
            ev.at(),
            analysis.offset_is_inside_any_definition_body(ev.at()),
        )
    });
    events.dedup_by_key(|ev| (ev.at(), std::mem::discriminant(ev)));
}

/// One lazily captured in-document source order per resolution, shared by
/// every import site's export snapshot and the alias lifecycle fold.
pub(super) type DocumentOrderCell = std::cell::OnceCell<RunOrder>;

fn document_order<'o>(
    order: &'o DocumentOrderCell,
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
) -> &'o RunOrder {
    order.get_or_init(|| in_document_source_order(analysis, ctx))
}

/// Retain the actual analysis owner even without an attached workspace view.
/// Missing source transport stays conditional/unavailable, never constructed.
/// Straight-line positions of this document are ordered by its own sequence;
/// deferred bodies, conditional branches and possible early exits stay
/// Conditional.
pub(super) fn in_document_source_order(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
) -> RunOrder {
    RunOrder::for_source_navigation(
        &[],
        crate::source_graph::SourceOrderContext::capture(analysis)
            .map(|owner| (ctx.uri().to_owned(), owner))
            .into_iter(),
    )
}

#[cfg(test)]
pub(super) fn exported_original_slot_at_import(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
) -> ExportVerdict {
    exported_original_slot_at_import_in(
        analysis,
        ctx,
        slot,
        policy,
        import_at,
        &DocumentOrderCell::new(),
    )
}

pub(super) fn exported_original_slot_at_import_in(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let site = in_document_point(analysis, ctx, import_at);
    let order = document_order(order, analysis, ctx);
    if analysis
        .original_namespace_export_unknowns()
        .iter()
        .any(|span| {
            order
                .navigation_precedence(in_document_point(analysis, ctx, span.start()), site)
                .and_then(crate::source_graph::SourceNavigationOrder::modeled_source_exclusion)
                .unwrap_or(true)
        })
    {
        return ExportVerdict::Unknown;
    }
    crate::namespace_import::exported_original_at_import_site(
        analysis.original_namespace_exports().map(|event| {
            (
                event,
                in_document_point(analysis, ctx, event.span().start()),
            )
        }),
        slot,
        policy,
        order,
        site,
    )
}
