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

//! Source call-hierarchy advice.
//!
//! [`prepare`], [`incoming_calls`] and [`outgoing_calls`] route Native source
//! queries through the shared sealed declaration identity and
//! `original_call_hierarchy` kernel. Genuine original command or member inputs
//! retain their selected allocation, role, side and complete source/configuration.
//! Independently owned URI/source inventories supply cross-document candidates;
//! ambiguous providers, stale receipts and unavailable original targets decline
//! advice. Item labels and wire ranges cannot reconstruct a declaration.
//!
//! These edges describe readonly source relationships. They do not establish
//! live native dispatch, an entered object/frame or source edit permission.
//! Alias routes may retain the canonical target allocation while direct editable
//! spelling remains a separate question.
//!
//! Production Logical CALL publication uses the sealed [`LogicalCallIdentity`]
//! and typed [`LogicalCallAdvice`] inventory adapters. They retain original
//! source requests, caller ancestry and independent source/input currentness;
//! missing representations remain applicability debt through the host lease.
//!
//! Explicit standalone lexical compatibility scanners below retain their
//! original callers and assertions. Their reporting fields cannot construct a
//! production Logical CALL identity or cast to Native source authority.

use tcl_compiler::analyser::{AnalysisResult, ClassDef, MemberSide, MethodDef, ProcDef};
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

mod logical_source;
pub(crate) use logical_source::declared_logical_source_items;
pub use logical_source::{
    LogicalCallAdvice, LogicalCallHierarchyEdge, LogicalCallHierarchyItem, LogicalCallIdentity,
    logical_incoming_from_inventory, logical_outgoing_from_inventory,
    prepare_logical_from_inventory,
};

/// One hierarchy item — proc identification plus its name and
/// definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    /// Sealed current source declaration; reporting item fields do not reissue it.
    pub identity: Option<crate::original_declaration::OriginalDeclarationIdentity>,
    /// Proc name (qualified).
    pub name: String,
    /// Detail (e.g. parameter list summary).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare call hierarchy" request to a single
/// item — the proc whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<CallHierarchyItem> {
    prepare_in_program(
        source,
        line,
        character,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`prepare`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn prepare_in_program(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<CallHierarchyItem> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    let offset =
        crate::definition::byte_offset_at(&LineIndex::new(source), source, line, character);
    if crate::definition::program_forced_import_abstains(analysis, resolution.program, offset) {
        return Vec::new();
    }

    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        let document = crate::original_call_hierarchy::OriginalCallDocument {
            uri: "",
            source,
            analysis,
        };
        return selected
            .and_then(|identity| crate::original_call_hierarchy::item(&identity, &document))
            .into_iter()
            .collect();
    }

    // The original selector can continue for variables and non-command
    // source positions. Those positions cannot borrow a same-named method
    // from the separate Logical compatibility maps.
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // Declaration-span hit, else the namespace-aware candidate resolution
    // (mirrors `references::proc_references` / `rename::rename_proc`) —
    // never a namespace-blind `p.name == word` first-`HashMap`-hit scan.
    let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(target) = LogicalCallTarget::at_declaration_offset(source, analysis, cursor_off) {
        return vec![match target.declaration() {
            LogicalCallDeclaration::Procedure {
                qualified,
                definition,
            } => item_for_proc(source, definition, qualified, &line_index),
            LogicalCallDeclaration::Member {
                class, definition, ..
            } => item_for_method(source, class, definition, &line_index),
        }];
    }
    let proc_match =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, &word, resolution);
    if let Some((qname, proc_def)) = proc_match {
        return vec![item_for_proc(source, proc_def, qname, &line_index)];
    }
    // Class-method fallback — cursor inside a class body on a
    // method / classmethod name.
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((class_def, method)) = enclosing_class_method(analysis, &word, cursor_offset) {
        return vec![item_for_method(source, class_def, method, &line_index)];
    }
    Vec::new()
}

fn method_item_name(class_def: &ClassDef, method: &MethodDef) -> String {
    tcl_syntax::naming::rooted_member_key(&class_def.qualified_name, &method.name)
}

/// Grouping identity for one end of a call-hierarchy edge — the caller
/// bucket of an incoming list, the callee bucket of an outgoing one.
///
/// The display name alone is **not** an identity.  A `TclOO` class may
/// define an instance `method make` and a `classmethod make` at the same
/// time: the two live in independent method tables, and tclsh 9.0.4 sends
/// `my make` to the instance copy and `C make` to the class copy.  Both
/// render as `::C::make`, so keying the grouping on the name text alone
/// would merge them into a single item whose ranges span both call sites and
/// whose declaration range is whichever entry was inserted first.  Pairing
/// the name with the declaration's own name-token span separates them while
/// keeping the emitted order name-first, hence reproducible.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CallItemKey {
    /// Qualified display name — the primary sort key.
    name: String,
    /// The declaration's name-token span, as `(start, end)`.  `(0, 0)` for
    /// the synthetic top-level item, which declares nothing.
    decl: (u32, u32),
}

impl CallItemKey {
    fn for_method(class_def: &ClassDef, method: &MethodDef) -> Self {
        Self {
            name: method_item_name(class_def, method),
            decl: (method.name_span.start(), method.name_span.end()),
        }
    }

    fn for_proc(qname: &str, proc_def: &ProcDef) -> Self {
        Self {
            name: qname.to_owned(),
            decl: (proc_def.name_span.start(), proc_def.name_span.end()),
        }
    }

    fn top_level() -> Self {
        Self {
            name: TOP_LEVEL_NAME.to_owned(),
            decl: (0, 0),
        }
    }
}

/// Display name of the synthetic item standing for a document's top-level
/// command stream.
const TOP_LEVEL_NAME: &str = "<top-level>";

/// Per-edge accumulator: the far end's item plus every call-site range
/// attributed to it, keyed by [`CallItemKey`].
type EdgeBuckets = std::collections::BTreeMap<CallItemKey, (CallHierarchyItem, Vec<LspRange>)>;

/// A current Logical source declaration selected by its exact name-token range.
/// Reporting labels and detail never reconstruct the target. Native and hosted
/// naming inputs use their independent original declaration identities.
pub struct LogicalCallTarget<'a> {
    current: crate::original_context::CurrentSourceContext<'a>,
    source_context: crate::workspace_index::WorkspaceDiagnosticSourceContext,
    input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    declaration: LogicalCallDeclaration<'a>,
    class_occurrence:
        Option<tcl_compiler::signature_scan::original_name::SourceOriginalNameOccurrence>,
    member: Option<&'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata>,
    logical_member: Option<tcl_compiler::analyser::types::LogicalSourceMemberDeclaration>,
}

/// Readonly metadata issued by [`LogicalCallTarget`]. This is source advice,
/// without Native dispatch, runtime method-table currency or editing permission.
#[derive(Clone, Copy)]
pub enum LogicalCallDeclaration<'a> {
    /// The actual procedure and its retained qualified source key.
    Procedure {
        /// Qualified source key retained by the selected current declaration.
        qualified: &'a str,
        /// Actual procedure declaration borrowed from the current analysis.
        definition: &'a ProcDef,
    },
    /// The actual class/member declaration and the table containing it.
    Member {
        /// Actual class containing the selected current source member.
        class: &'a ClassDef,
        /// Actual member declaration borrowed from the current analysis.
        definition: &'a MethodDef,
        /// Declared member table, without runtime dispatch or export authority.
        side: MemberSide,
    },
}

impl<'a> LogicalCallTarget<'a> {
    /// Select one unique declaration from the actual current source analysis.
    /// A missing, stale, ambiguous or nondeclaration range is terminal.
    #[must_use]
    pub fn capture(
        source: &str,
        analysis: &'a AnalysisResult,
        item: &CallHierarchyItem,
    ) -> Option<Self> {
        if item.identity.is_some() {
            return None;
        }
        let lines = LineIndex::new(source);
        Self::capture_matching(source, analysis, |span| {
            span_to_range(source, &lines, span) == item.selection_range
        })
    }

    fn at_declaration_offset(
        source: &str,
        analysis: &'a AnalysisResult,
        offset: u32,
    ) -> Option<Self> {
        Self::capture_matching(source, analysis, |span| {
            span.start() <= offset && offset < span.end()
        })
    }

    fn capture_matching(
        source: &str,
        analysis: &'a AnalysisResult,
        matches: impl Fn(tcl_lexer::Span) -> bool,
    ) -> Option<Self> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if !analysis.allows_retained_logical_declaration_advice() {
            return None;
        }
        let mut selected = None;
        for (qualified, definition) in &analysis.all_procs {
            if matches(definition.name_span) {
                if selected.is_some() {
                    return None;
                }
                selected = Some(LogicalCallDeclaration::Procedure {
                    qualified,
                    definition,
                });
            }
        }
        for class in analysis.all_classes.values() {
            for (side, members) in [
                (MemberSide::Instance, &class.methods),
                (MemberSide::ClassObject, &class.class_methods),
            ] {
                for definition in members.values() {
                    if matches(definition.name_span) {
                        if selected.is_some() {
                            return None;
                        }
                        selected = Some(LogicalCallDeclaration::Member {
                            class,
                            definition,
                            side,
                        });
                    }
                }
            }
        }
        let declaration = selected?;
        let (class_occurrence, member) = if let LogicalCallDeclaration::Member {
            class,
            definition,
            side,
        } = declaration
        {
            let mut classes = analysis.original_class_declarations().filter(|original| {
                original.metadata().name_span == class.name_span
                    && original.metadata().body_span == class.body_span
                    && class.source_name.as_ref() == Some(original.name())
            });
            let class_occurrence = classes
                .next()
                .filter(|_| classes.next().is_none())
                .map(|original| original.original_occurrence().clone());
            let mut members = class.original_members.declarations().filter(|original| {
                original.side() == side
                    && original.metadata().name_span == definition.name_span
                    && original.metadata().body_span == definition.body_span
            });
            let member = members.next().filter(|_| members.next().is_none());
            (class_occurrence, member)
        } else {
            (None, None)
        };
        let logical_member = if let LogicalCallDeclaration::Member {
            class,
            definition,
            side,
        } = declaration
        {
            analysis
                .logical_source_class_declaration(class)
                .and_then(|class| class.member(side, definition))
        } else {
            None
        };
        Some(Self {
            current,
            logical_member,
            source_context: crate::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                analysis,
            )?,
            input: analysis.resolved_input.as_ref()?,
            declaration,
            class_occurrence,
            member,
        })
    }

    pub(crate) fn matches_indexed_member_provider(
        &self,
        declaring_uri: &str,
        provider: &crate::workspace_index::WorkspaceClass,
        context: &crate::workspace_index::WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    ) -> bool {
        let LogicalCallDeclaration::Member { class, side, .. } = self.declaration else {
            return false;
        };
        if provider.uri != declaring_uri
            || context != &self.source_context
            || input != self.input
            || provider.name_span != class.name_span
            || provider.source_name_ambiguous
            || provider.source_name != class.source_name
            || provider.original_declaration != self.class_occurrence
        {
            return false;
        }
        if let Some(logical_member) = &self.logical_member {
            let LogicalCallDeclaration::Member { definition, .. } = self.declaration else {
                return false;
            };
            return provider
                .logical_declaration
                .as_ref()
                .and_then(|class| class.member(side, definition))
                .as_ref()
                == Some(logical_member);
        }
        let Some(member) = self.member else {
            return false;
        };
        let Some(members) = provider.original_members.as_ref() else {
            return false;
        };
        let mut selected = members.declarations().filter(|original| {
            original.side() == side && original.declaration() == member.declaration()
        });
        selected.next().is_some_and(|original| original == member) && selected.next().is_none()
    }

    /// Selected original metadata, with procedure/member and member-side axes.
    #[must_use]
    pub const fn declaration(&self) -> LogicalCallDeclaration<'a> {
        self.declaration
    }

    /// The actual supplied profile, independently of item and dialect labels.
    #[must_use]
    pub const fn profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.current.profile()
    }
}

/// Build a [`CallHierarchyItem`] for a class method.
fn item_for_method(
    source: &str,
    class_def: &ClassDef,
    method: &MethodDef,
    line_index: &LineIndex,
) -> CallHierarchyItem {
    let name_range = span_to_range(source, line_index, method.name_span);
    let body_range = span_to_range(source, line_index, method.body_span);
    let detail = Some(format!(
        "{} of {} ({} params)",
        method.kind,
        class_def.qualified_name,
        method.params.len(),
    ));
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    CallHierarchyItem {
        identity: None,
        name: method_item_name(class_def, method),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// `true` when `offset` falls within `span` (inclusive of both ends — a
/// cursor sitting right at either edge of a name token still counts).
fn span_contains_offset(span: tcl_lexer::Span, offset: u32) -> bool {
    span.start() <= offset && offset <= span.end()
}

/// Find the class + method whose body contains `cursor_offset`
/// and whose method name matches `word`.  Searches `methods`
/// then `class_methods` — except when `cursor_offset` sits inside a
/// `classmethod`'s own declaration or body, in which case `class_methods`
/// is tried first.
///
/// `my <word>` dispatch scope depends on *whose* body the cursor is
/// currently inside: a classmethod's body runs with `self` bound to the
/// class object (its own method table is `class_methods`); an ordinary
/// method's / constructor's / destructor's body runs with `self` bound to
/// the instance (`methods`) — the two tables are never merged (confirmed
/// against tclsh 9.0.4, see
/// [`tcl_compiler::analyser::diagnostics::var_command`]'s dispatch-scope
/// note). Preferring whichever table the surrounding body belongs to
/// resolves a name shared by both kinds (rare, but real) to the one
/// actually reachable from the cursor's own scope.
fn enclosing_class_method<'a>(
    analysis: &'a AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(&'a ClassDef, &'a MethodDef)> {
    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    let in_classmethod_territory = class_def.class_methods.values().any(|m| {
        span_contains_offset(m.name_span, cursor_offset)
            || span_contains_offset(m.body_span, cursor_offset)
    });
    if in_classmethod_territory && let Some(m) = class_def.class_methods.get(word) {
        return Some((class_def, m));
    }
    if let Some(m) = class_def.methods.get(word) {
        return Some((class_def, m));
    }
    if let Some(m) = class_def.class_methods.get(word) {
        return Some((class_def, m));
    }
    None
}

/// Conditional source calls in the same lexical method frame. Selected
/// helper traits exclude method dispatch without reparsing a written head.
fn segment_body_calls(
    source: &str,
    analysis: &AnalysisResult,
    body_span: tcl_lexer::Span,
) -> Vec<(String, tcl_lexer::Span)> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let Some(frame) =
        crate::definition::source_frame_extent(analysis, body_span.start().saturating_add(1))
    else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let Some(head) = command.argv.first() else {
                return false;
            };
            if !span_contains(body_span, head.span)
                || crate::definition::source_frame_extent(analysis, head.span.start())
                    != Some(frame)
            {
                return false;
            }
            let is_helper =
                crate::original_invocation::source_registry_words(source, analysis, command)
                    .and_then(|words| {
                        words.with_source_schema(&current.context(), |schema| {
                            schema.semantics.traits.intersects(
                                tcl_registry::Traits::TCLOO_SELF_DISPATCH
                                    | tcl_registry::Traits::TCLOO_NEXT_CHAIN
                                    | tcl_registry::Traits::TCLOO_INTROSPECTION,
                            )
                        })
                    })
                    .unwrap_or(false);
            if !is_helper {
                calls.push((command.name().to_owned(), head.span));
            }
            false
        },
    );
    calls
}

enum LogicalMethodProcedureSelection<'a> {
    Unavailable,
    Vacant {
        candidates: Option<Box<tcl_compiler::command_binding::OriginalLogicalCommandCandidates>>,
    },
    Occupied {
        procedure: Option<&'a ProcDef>,
        earlier_unknown_mutations: bool,
    },
}

/// Match conditional source coordinates against independent original headers.
/// A method's reporting class name supplies no caller namespace. Unknown
/// earlier effects and ambiguous occupied coordinates remain unresolved.
fn logical_method_procedure_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> LogicalMethodProcedureSelection<'a> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
        || !analysis.allows_lexical_declaration_advice()
    {
        return LogicalMethodProcedureSelection::Unavailable;
    }
    let ordinary =
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            source, analysis, offset,
        );
    let absolute = if ordinary.is_none() {
        tcl_compiler::registry_invocation::source_structure::source_logical_absolute_command_candidate_at(source, analysis, offset)
    } else {
        None
    };
    if ordinary
        .as_ref()
        .is_some_and(tcl_compiler::command_binding::OriginalLogicalCommandCandidates::has_lookup_operand_effects)
        || absolute
            .as_ref()
            .is_some_and(tcl_compiler::command_binding::OriginalLogicalAbsoluteCommandCandidate::has_lookup_operand_effects)
    {
        return LogicalMethodProcedureSelection::Unavailable;
    }
    let (coordinates, earlier_unknown_mutations) = if let Some(candidate) = &ordinary {
        (
            candidate.coordinates(),
            candidate.has_earlier_unknown_mutations(),
        )
    } else if let Some(candidate) = &absolute {
        (
            candidate.coordinates().as_slice(),
            candidate.has_earlier_unknown_mutations(),
        )
    } else {
        return LogicalMethodProcedureSelection::Unavailable;
    };
    for coordinate in coordinates {
        let mut selected: Option<&ProcDef> = None;
        let mut occupied = false;
        let mut ambiguous = false;
        for procedure in
            analysis
                .all_procs
                .values()
                .chain(analysis.original_procedure_declarations().map(
                tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::metadata,
            ))
        {
            if analysis.logical_source_procedure_declaration(procedure)
                .and_then(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::source_candidate_coordinate)
                .as_ref() != Some(coordinate)
            {
                continue;
            }
            occupied = true;
            if let Some(previous) = selected {
                if previous.name_span != procedure.name_span
                    || previous.body_span != procedure.body_span
                {
                    ambiguous = true;
                }
            } else {
                selected = Some(procedure);
            }
        }
        if analysis.all_classes.values().any(|class| {
            analysis.logical_source_class_declaration(class)
                .and_then(tcl_compiler::analyser::types::LogicalSourceClassDeclaration::source_candidate_coordinate)
                == Some(coordinate)
        }) {
            occupied = true;
            ambiguous = true;
        }
        if occupied {
            return LogicalMethodProcedureSelection::Occupied {
                procedure: if ambiguous { None } else { selected },
                earlier_unknown_mutations,
            };
        }
    }
    LogicalMethodProcedureSelection::Vacant {
        candidates: ordinary.map(Box::new),
    }
}

/// Build a [`CallHierarchyItem`] for a given proc definition.
fn item_for_proc(
    source: &str,
    proc_def: &ProcDef,
    _qname: &str,
    line_index: &LineIndex,
) -> CallHierarchyItem {
    let name_range = span_to_range(source, line_index, proc_def.name_span);
    let body_range = span_to_range(source, line_index, proc_def.body_span);
    let detail = if proc_def.params.is_empty() {
        None
    } else {
        Some(format!("({} params)", proc_def.params.len()))
    };
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    CallHierarchyItem {
        identity: None,
        // The short name is presentation; target selection retains the actual
        // declaration range and metadata independently.
        name: proc_def.name.clone(),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// `true` when `inv_span` lies inside `proc_body_span`.  Used
/// to bucket each call site into its enclosing proc.
fn span_contains(outer: tcl_lexer::Span, inner: tcl_lexer::Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Find the proc whose body span contains `inv_span` — i.e.
/// the proc the call site sits inside.  Returns `None` for
/// top-level call sites.
fn enclosing_proc<'a>(
    analysis: &'a AnalysisResult,
    inv_span: tcl_lexer::Span,
) -> Option<(&'a str, &'a ProcDef)> {
    let mut best: Option<(&'a str, &'a ProcDef)> = None;
    for (qname, proc_def) in &analysis.all_procs {
        if !span_contains(proc_def.body_span, inv_span) {
            continue;
        }
        // Pick the smallest enclosing body so nested procs
        // bucket calls into the inner proc rather than the
        // outer.
        let body_len = proc_def.body_span.end() - proc_def.body_span.start();
        let pick = match best {
            None => true,
            Some((_, prev)) => {
                let prev_len = prev.body_span.end() - prev.body_span.start();
                body_len < prev_len
            }
        };
        if pick {
            best = Some((qname.as_str(), proc_def));
        }
    }
    best
}

/// `true` when an execution site targets `proc_def`.
///
/// Retains the lookup-purpose distinction before delegating target identity
/// to [`crate::references::invocation_calls_proc`]. Introspection and
/// deferred registrations remain references without becoming calls. Actual
/// heads share the namespace and allocation checks used by references and
/// rename.
fn invocation_targets(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_def: &ProcDef,
    qname: &str,
    source: &str,
) -> bool {
    crate::references::invocation_calls_proc(analysis, inv, qname, proc_def, source)
}

/// One incoming-call entry: the caller proc plus the spans at
/// which it calls the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingCall {
    /// The proc that contains the call sites.
    pub from: CallHierarchyItem,
    /// Spans at which `from` calls the target proc.
    pub from_ranges: Vec<LspRange>,
}

/// One outgoing-call entry: the called proc plus the spans
/// from which it's called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingCall {
    /// The proc being called.
    pub to: CallHierarchyItem,
    /// Spans at which the source proc calls `to`.
    pub from_ranges: Vec<LspRange>,
}

/// A call site inside the queried item's body whose callee is
/// *not* defined in the current document.  The local
/// [`outgoing_calls`] pass can only resolve callees present in
/// `analysis.all_procs` / the enclosing class; cross-document
/// outgoing-call resolution feeds these unresolved heads to the
/// workspace index, which knows the sibling-file definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedOutgoingCall {
    /// Command head as written at the call site.
    pub name: String,
    /// Scope-resolved qualified name when the analyser inferred
    /// one for the call site, else `None`.
    pub resolved_qualified_name: Option<String>,
    /// Independent current original source recipes and ranges for every occurrence.
    /// Missing recipes remain unavailable; grouped head labels cannot replace them.
    pub logical_occurrences: Vec<LogicalOutgoingCallOccurrence>,
    /// Ranges (in the *current* document) of each call site,
    /// grouped under this callee head.
    pub from_ranges: Vec<LspRange>,
}

/// One original Logical call occurrence with its independent source recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalOutgoingCallOccurrence {
    /// Missing body/name/path ownership remains unavailable.
    pub candidates: Option<tcl_compiler::command_binding::OriginalLogicalCommandCandidates>,
    /// Actual occurrence range, independently of other same-head calls.
    pub range: LspRange,
}

/// Collect the call sites inside the queried item's body whose
/// callee is not resolvable within the current document — the
/// raw material for cross-document outgoing-call edges.
///
/// Mirrors [`outgoing_calls`]'s body-scan but inverts the filter:
/// it keeps the invocations that *don't* match a local proc /
/// sibling method, grouped by call-site head, so the server can
/// resolve each head against the workspace index.  Builtins and
/// unknown commands are returned too (the index lookup discards
/// the ones that aren't user-defined elsewhere).
#[must_use]
pub fn unresolved_outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<UnresolvedOutgoingCall> {
    unresolved_outgoing_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

fn unresolved_source_procedure_calls(
    source: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<UnresolvedOutgoingCall> {
    let Some(identity) = &item.identity else {
        return Vec::new();
    };
    if !identity.is_current(identity.uri(), source, analysis) {
        return Vec::new();
    }
    let Some(declaration) = identity.procedure_metadata(analysis) else {
        return Vec::new();
    };
    let index = LineIndex::new_lsp(source);
    let mut by_head: std::collections::BTreeMap<String, UnresolvedOutgoingCall> =
        std::collections::BTreeMap::new();
    for command in
        tcl_compiler::registry_invocation::source_structure::source_native_procedure_body_commands(
            source, analysis,
        )
        .into_iter()
        .flatten()
    {
        if !command
            .caller_declaration(analysis)
            .is_some_and(|caller| std::ptr::eq(caller, declaration))
        {
            continue;
        }
        let Some(head) = command.original_words().first() else {
            continue;
        };
        if tcl_compiler::registry_invocation::source_structure::source_original_procedure_call_at(
            source,
            analysis,
            head.span().start(),
        )
        .is_some()
        {
            continue;
        }
        let Some(name) = command
            .static_head()
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            continue;
        };
        let range = span_to_range(source, &index, head.span());
        let call = by_head
            .entry(name.clone())
            .or_insert_with(|| UnresolvedOutgoingCall {
                name,
                resolved_qualified_name: None,
                logical_occurrences: Vec::new(),
                from_ranges: Vec::new(),
            });
        call.logical_occurrences
            .push(LogicalOutgoingCallOccurrence {
                candidates: None,
                range,
            });
        if !call.from_ranges.contains(&range) {
            call.from_ranges.push(range);
        }
    }
    by_head.into_values().collect()
}

/// [`unresolved_outgoing_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn unresolved_outgoing_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<UnresolvedOutgoingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        return unresolved_source_procedure_calls(source, item, analysis);
    }

    let Some(target) = LogicalCallTarget::capture(source, analysis, item) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    if let LogicalCallDeclaration::Procedure {
        definition: source_proc,
        ..
    } = target.declaration()
    {
        let mut by_head: std::collections::BTreeMap<String, UnresolvedOutgoingCall> =
            std::collections::BTreeMap::new();
        for inv in &analysis.command_invocations {
            if !inv.lookup.is_execution_site() || !span_contains(source_proc.body_span, inv.range) {
                continue;
            }
            // Skip call sites the local pass already resolves.
            if analysis
                .all_procs
                .iter()
                .any(|(qname, proc_def)| invocation_targets(analysis, inv, proc_def, qname, source))
            {
                continue;
            }
            let range = span_to_range(source, &line_index, inv.range);
            let entry = by_head
                .entry(inv.name.clone())
                .or_insert_with(|| UnresolvedOutgoingCall {
                    name: inv.name.clone(),
                    resolved_qualified_name: inv.resolved_qualified_name.clone(),
                    logical_occurrences: Vec::new(),
                    from_ranges: Vec::new(),
                });
            entry.logical_occurrences.push(LogicalOutgoingCallOccurrence {
                candidates: tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(source, analysis, inv.range.start()),
                range,
            });
            entry.from_ranges.push(range);
        }
        return by_head.into_values().collect();
    }
    unresolved_method_outgoing_calls(source, dialect, &target, analysis, &line_index, resolution)
}

/// Method-body variant of [`unresolved_outgoing_calls`]: keeps
/// the call sites inside a class method that name neither a
/// local top-level proc nor a `TclOO` dispatch keyword.
///
/// A bare head matching a *sibling method* name is **not** excluded here —
/// unlike a proc, a method is never a bare-callable command (`greet` alone
/// errors "invalid command name"; only `my greet` dispatches), so a bare
/// `greet` site names no real target and is correctly surfaced as
/// unresolved, not silently treated as if it had resolved to the method.
fn unresolved_method_outgoing_calls(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    target: &LogicalCallTarget<'_>,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    _resolution: crate::definition::CallResolution<'_>,
) -> Vec<UnresolvedOutgoingCall> {
    let LogicalCallDeclaration::Member {
        definition: source_method,
        ..
    } = target.declaration()
    else {
        return Vec::new();
    };
    let mut by_head: std::collections::BTreeMap<String, UnresolvedOutgoingCall> =
        std::collections::BTreeMap::new();
    for (head, span) in segment_body_calls(source, analysis, source_method.body_span) {
        let selection = logical_method_procedure_at(source, analysis, span.start());
        if matches!(
            selection,
            LogicalMethodProcedureSelection::Occupied {
                procedure: Some(_),
                earlier_unknown_mutations: false,
            }
        ) {
            continue;
        }
        let candidates = match selection {
            LogicalMethodProcedureSelection::Vacant { candidates } => {
                candidates.map(|candidate| *candidate)
            }
            LogicalMethodProcedureSelection::Unavailable
            | LogicalMethodProcedureSelection::Occupied { .. } => None,
        };
        let range = span_to_range(source, line_index, span);
        let entry = by_head
            .entry(head.clone())
            .or_insert_with(|| UnresolvedOutgoingCall {
                name: head,
                resolved_qualified_name: None,
                logical_occurrences: Vec::new(),
                from_ranges: Vec::new(),
            });
        entry.from_ranges.push(range);
        entry
            .logical_occurrences
            .push(LogicalOutgoingCallOccurrence { candidates, range });
    }
    by_head.into_values().collect()
}

/// Enumerate incoming calls for the actual selected declaration. Walks
/// every eligible command invocation in
/// `analysis.command_invocations` that targets the proc and
/// groups by enclosing proc.  Top-level call sites are bucketed
/// under a synthetic `<top-level>` caller.
///
/// Returns an empty `Vec` when the current declaration owner is unavailable.
#[must_use]
pub fn incoming_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<IncomingCall> {
    incoming_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`incoming_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn incoming_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    _resolution: crate::definition::CallResolution<'_>,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        let Some(identity) = &item.identity else {
            return Vec::new();
        };
        return crate::original_call_hierarchy::incoming(
            identity,
            &[crate::original_call_hierarchy::OriginalCallDocument {
                uri: identity.uri(),
                source,
                analysis,
            }],
            None,
        )
        .into_iter()
        .map(|(_, call)| call)
        .collect();
    }

    let Some(target) = LogicalCallTarget::capture(source, analysis, item) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let LogicalCallDeclaration::Procedure {
        qualified: target_qname,
        definition: target_proc,
    } = target.declaration()
    else {
        return method_incoming_calls(source, dialect, &target, analysis, &line_index);
    };
    incoming_calls_for_target(
        source,
        analysis,
        &target_proc.name,
        target_qname,
        Some(target_proc.name_span),
    )
}

/// Enumerate incoming calls to a proc identified *externally*
/// by `(target_simple, target_qualified)`, within the document
/// described by `source` / `analysis`.  Unlike [`incoming_calls`],
/// the target needn't be defined in this document — used by the
/// server to gather cross-document callers (one call per
/// indexed document).  `target_name_span`, when `Some`, skips
/// an invocation that overlaps the proc's own declaration in
/// *this* document (avoids self-linking); pass `None` for
/// documents that don't define the proc.
#[must_use]
pub fn incoming_calls_for_target(
    source: &str,
    analysis: &AnalysisResult,
    target_simple: &str,
    target_qualified: &str,
    target_name_span: Option<tcl_lexer::Span>,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    let line_index = LineIndex::new(source);
    let mut by_caller: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        if !inv.lookup.is_execution_site() {
            continue;
        }
        // Delegate to the shared matching rule (`invocation_calls_proc`
        // takes a `ProcDef`; this caller may not have one — cross-document
        // callers are gathered from a target the calling document never
        // defines — so route through the string-keyed core directly). This
        // adds the namespace gate a bare simple-name match needs: without
        // it, a bare call in one namespace falsely credited *any* same-named
        // proc anywhere as an incoming caller.
        if !crate::references::invocation_calls_named(
            analysis,
            inv,
            target_qualified,
            target_simple,
            target_qualified,
            source,
        ) {
            continue;
        }
        // Skip the proc's own declaration site in this document.
        if let Some(decl) = target_name_span
            && (span_contains(decl, inv.range) || span_contains(inv.range, decl))
        {
            continue;
        }
        let inv_range = span_to_range(source, &line_index, inv.range);
        let caller_key = enclosing_proc(analysis, inv.range)
            .map_or_else(|| TOP_LEVEL_NAME.to_owned(), |(qn, _)| qn.to_owned());
        let entry = by_caller.entry(caller_key.clone()).or_insert_with(|| {
            let caller_item = if caller_key == TOP_LEVEL_NAME {
                top_level_item()
            } else {
                let proc = &analysis.all_procs[&caller_key];
                item_for_proc(source, proc, &caller_key, &line_index)
            };
            (caller_item, Vec::new())
        });
        entry.1.push(inv_range);
    }
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Enumerate outgoing calls from the actual selected declaration. Walks
/// every eligible command invocation whose span
/// sits inside the proc's body span, then resolves each to its
/// target proc (when the invocation names a user proc), and adds every bare
/// `ClassName <classmethod>` dispatch in the same body.  Calls
/// to built-in commands are dropped (they have no
/// [`CallHierarchyItem`] to point to).  Multiple call sites to
/// the same target group together.
///
/// The classmethod half matters for symmetry: a class command is an
/// ordinary global command, so a proc body may dispatch one, and
/// [`add_classmethod_incoming`] lists that proc under the classmethod's
/// Incoming Calls.  Without the matching collection here the edge would
/// exist in one direction only.
#[must_use]
pub fn outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<OutgoingCall> {
    outgoing_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`outgoing_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn outgoing_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<OutgoingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        let Some(identity) = &item.identity else {
            return Vec::new();
        };
        return crate::original_call_hierarchy::outgoing(
            identity,
            &[crate::original_call_hierarchy::OriginalCallDocument {
                uri: identity.uri(),
                source,
                analysis,
            }],
            None,
        )
        .into_iter()
        .map(|(_, call)| call)
        .collect();
    }

    let Some(target) = LogicalCallTarget::capture(source, analysis, item) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let LogicalCallDeclaration::Procedure {
        definition: source_proc,
        ..
    } = target.declaration()
    else {
        return method_outgoing_calls(source, dialect, &target, analysis, &line_index, resolution);
    };
    let mut by_target: EdgeBuckets = EdgeBuckets::new();
    for inv in &analysis.command_invocations {
        if !span_contains(source_proc.body_span, inv.range) {
            continue;
        }
        // Find the user-proc this invocation targets, if any.
        for (qname, proc_def) in &analysis.all_procs {
            if invocation_targets(analysis, inv, proc_def, qname, source) {
                let inv_range = span_to_range(source, &line_index, inv.range);
                let entry = by_target
                    .entry(CallItemKey::for_proc(qname, proc_def))
                    .or_insert_with(|| {
                        (
                            item_for_proc(source, proc_def, qname, &line_index),
                            Vec::new(),
                        )
                    });
                entry.1.push(inv_range);
                break;
            }
        }
    }
    add_classmethod_outgoing(
        source,
        dialect,
        analysis,
        source_proc.body_span,
        &line_index,
        &mut by_target,
    );
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Incoming calls for a class method — every `my <method>` dispatch site
/// inside any sibling method body, grouped by the enclosing method, plus
/// (for a `classmethod`) every bare `ClassName <method>` dispatch anywhere
/// in the document, grouped by whichever body it sits in.
///
/// Intra-class dispatch of a `TclOO` method is `my <method>`, never a bare
/// `<method>` call (a method is not a command in the body's namespace — a
/// bare head errors "invalid command name" at runtime), so this matches
/// through [`crate::references::scan_my_method_sites`] — the same
/// control-flow-recursing matcher Find-References / rename / the code lens
/// use — rather than comparing a bare head.
fn method_incoming_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    target: &LogicalCallTarget<'_>,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<IncomingCall> {
    let LogicalCallDeclaration::Member {
        class: class_def,
        definition: target_method,
        side,
    } = target.declaration()
    else {
        return Vec::new();
    };
    let mut by_caller: EdgeBuckets = EdgeBuckets::new();
    for caller in member_frames_iter(class_def, side) {
        let spans = crate::references::scan_method_sites(
            source,
            analysis,
            &[caller.body_span],
            &target_method.name,
            Some(target_method.name_span),
            target_method.visibility == "public",
        );
        if spans.is_empty() {
            continue;
        }
        let entry = by_caller
            .entry(CallItemKey::for_method(class_def, caller))
            .or_insert_with(|| {
                (
                    item_for_method(source, class_def, caller, line_index),
                    Vec::new(),
                )
            });
        entry.1.extend(
            spans
                .into_iter()
                .map(|s| span_to_range(source, line_index, s)),
        );
    }
    if side == MemberSide::ClassObject {
        add_classmethod_incoming(
            source,
            dialect,
            analysis,
            class_def,
            target_method,
            line_index,
            &mut by_caller,
        );
    }
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Incoming instance-method calls found in one known receiver class.
///
/// Hosts use this for a sibling document whose class inherits the target's
/// provider from the workspace MRO: the document has the caller bodies, while
/// the provider lives elsewhere. `external_callback_allowed` comes from the
/// workspace visibility chain; internal `my` callbacks remain eligible.
#[must_use]
pub fn incoming_instance_method_calls_in_class(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    receiver_class: &str,
    method: &str,
    external_callback_allowed: bool,
) -> Vec<IncomingCall> {
    incoming_member_calls_in_class(
        source,
        analysis,
        receiver_class,
        method,
        MemberSide::Instance,
        external_callback_allowed,
    )
}

/// Conditional incoming member calls in an actual caller class. The target's
/// original declaration selects the member name and table; the host supplies
/// the independently checked workspace provider and visibility relationship.
#[must_use]
pub fn incoming_calls_for_logical_member_in_class(
    source: &str,
    analysis: &AnalysisResult,
    receiver_class: &str,
    target: &LogicalCallTarget<'_>,
    external_callback_allowed: bool,
) -> Vec<IncomingCall> {
    let LogicalCallDeclaration::Member {
        definition, side, ..
    } = target.declaration()
    else {
        return Vec::new();
    };
    incoming_member_calls_in_class(
        source,
        analysis,
        receiver_class,
        &definition.name,
        side,
        external_callback_allowed,
    )
}

fn incoming_member_calls_in_class(
    source: &str,
    analysis: &AnalysisResult,
    receiver_class: &str,
    method: &str,
    side: MemberSide,
    external_callback_allowed: bool,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
        || !analysis.allows_retained_logical_declaration_advice()
    {
        return Vec::new();
    }
    let Some(class_def) = analysis.all_classes.get(receiver_class) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let frames = member_frames_iter(class_def, side);
    let mut incoming = frames
        .filter_map(|caller| {
            let spans = crate::references::scan_method_sites(
                source,
                analysis,
                &[caller.body_span],
                method,
                None,
                external_callback_allowed,
            );
            (!spans.is_empty()).then(|| IncomingCall {
                from: item_for_method(source, class_def, caller, &line_index),
                from_ranges: spans
                    .into_iter()
                    .map(|span| span_to_range(source, &line_index, span))
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    // The class's own command has a separate source receiver owner. Its sites
    // are added only when the actual class-side workspace visibility permits it.
    if side == MemberSide::ClassObject && external_callback_allowed {
        for span in crate::references::logical_class_command_method_sites(
            source, analysis, class_def, method,
        ) {
            let (_, from) = enclosing_dispatch_caller(source, analysis, span, &line_index);
            if let Some(existing) = incoming
                .iter_mut()
                .find(|call| call.from.selection_range == from.selection_range)
            {
                existing
                    .from_ranges
                    .push(span_to_range(source, &line_index, span));
            } else {
                incoming.push(IncomingCall {
                    from,
                    from_ranges: vec![span_to_range(source, &line_index, span)],
                });
            }
        }
    }
    incoming
}

/// Add direct class-command selector sites for the selected class-object
/// member, attributed to their actual enclosing source body. Its issued table
/// side and the external visibility check are independent requirements.
///
/// The shared Logical selector retains the actual original class factory,
/// complete current source context and direct written selector. A reporting
/// class name cannot supply its source receiver, and captured selectors cannot
/// supply a call-site span.
///
/// A class-command selector has its own original class receiver; the caller's
/// surrounding method table cannot redirect it. These are conditional source
/// edges and supply no entered Native handler or live dispatch.
fn add_classmethod_incoming(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_def: &ClassDef,
    target_method: &MethodDef,
    line_index: &LineIndex,
    by_caller: &mut EdgeBuckets,
) {
    if target_method.visibility != "public" {
        return;
    }
    for span in crate::references::logical_class_command_method_sites(
        source,
        analysis,
        class_def,
        &target_method.name,
    ) {
        if span_contains(target_method.name_span, span) {
            continue;
        }
        let (key, caller_item) = enclosing_dispatch_caller(source, analysis, span, line_index);
        let entry = by_caller
            .entry(key)
            .or_insert_with(|| (caller_item, Vec::new()));
        entry.1.push(span_to_range(source, line_index, span));
    }
}

/// The caller a bare class-command dispatch site is attributed to: the
/// innermost class-member body containing it, else the innermost proc body,
/// else the top level.
///
/// This one list mixes both kinds of caller, and a method caller has no
/// short name that could ever be unambiguous — `::C::make` is the only
/// thing to call it.  So the proc callers alongside it are named by their
/// qualified name too (`::util::helper`, not `helper`), rather than reading
/// as a different kind of label in the same list.  Elsewhere a proc item
/// keeps the short display name the
/// editor's call-hierarchy UI expects.
fn enclosing_dispatch_caller(
    source: &str,
    analysis: &AnalysisResult,
    span: tcl_lexer::Span,
    line_index: &LineIndex,
) -> (CallItemKey, CallHierarchyItem) {
    // Ranked by (body length, body start) so the innermost body wins and ties
    // never depend on `all_classes`' hash iteration order.
    let mut best: Option<((u32, u32), CallItemKey, CallHierarchyItem)> = None;
    let mut consider = |body: tcl_lexer::Span, key: CallItemKey, item: CallHierarchyItem| {
        if !span_contains(body, span) {
            return;
        }
        let rank = (body.end() - body.start(), body.start());
        if best
            .as_ref()
            .is_none_or(|(best_rank, _, _)| rank < *best_rank)
        {
            best = Some((rank, key, item));
        }
    };
    for class_def in analysis.all_classes.values() {
        for member in class_methods_iter(class_def) {
            consider(
                member.body_span,
                CallItemKey::for_method(class_def, member),
                item_for_method(source, class_def, member, line_index),
            );
        }
    }
    for (qname, proc_def) in &analysis.all_procs {
        let mut item = item_for_proc(source, proc_def, qname, line_index);
        item.name.clone_from(qname);
        consider(
            proc_def.body_span,
            CallItemKey::for_proc(qname, proc_def),
            item,
        );
    }
    best.map_or_else(
        || (CallItemKey::top_level(), top_level_item()),
        |(_, key, item)| (key, item),
    )
}

/// The synthetic item standing for the document's top-level command stream.
fn top_level_item() -> CallHierarchyItem {
    let origin = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    CallHierarchyItem {
        identity: None,
        name: TOP_LEVEL_NAME.to_owned(),
        detail: None,
        range: origin,
        selection_range: origin,
    }
}

/// Outgoing calls from a class method — every `my <method>` dispatch site
/// inside the method's body that names a sibling method (→ method item),
/// every bare `ClassName <classmethod>` dispatch in it (→ that
/// classmethod's item), and every bare-headed call to a
/// top-level user proc (→ proc item).
///
/// The sibling-method half matches through
/// [`crate::references::scan_my_method_sites`] (the same matcher
/// Find-References / rename / the code lens use): a bare `<method>` call is
/// never a valid `TclOO` dispatch (it
/// errors "invalid command name" at runtime), so bare-head comparison
/// against a sibling method name would never match real code.  The proc
/// half keeps bare-head matching — an ordinary proc call genuinely is
/// bare-headed — but skips `my` / `next` / `nextto` heads so a `TclOO`
/// dispatch keyword is never misread as a proc-call attempt.
fn method_outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    target: &LogicalCallTarget<'_>,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    _resolution: crate::definition::CallResolution<'_>,
) -> Vec<OutgoingCall> {
    let LogicalCallDeclaration::Member {
        class: class_def,
        definition: source_method,
        side,
    } = target.declaration()
    else {
        return Vec::new();
    };
    let mut by_target: EdgeBuckets = EdgeBuckets::new();
    let callees = class_def
        .methods
        .values()
        .filter(move |_| side == MemberSide::Instance)
        .chain(
            class_def
                .class_methods
                .values()
                .filter(move |_| side == MemberSide::ClassObject),
        );
    for callee in callees {
        let spans = crate::references::scan_method_sites(
            source,
            analysis,
            &[source_method.body_span],
            &callee.name,
            None,
            callee.visibility == "public",
        );
        if spans.is_empty() {
            continue;
        }
        let entry = by_target
            .entry(CallItemKey::for_method(class_def, callee))
            .or_insert_with(|| {
                (
                    item_for_method(source, class_def, callee, line_index),
                    Vec::new(),
                )
            });
        entry.1.extend(
            spans
                .into_iter()
                .map(|s| span_to_range(source, line_index, s)),
        );
    }
    add_classmethod_outgoing(
        source,
        dialect,
        analysis,
        source_method.body_span,
        line_index,
        &mut by_target,
    );
    for (_, span) in segment_body_calls(source, analysis, source_method.body_span) {
        if let LogicalMethodProcedureSelection::Occupied {
            procedure: Some(proc_def),
            ..
        } = logical_method_procedure_at(source, analysis, span.start())
        {
            let qname = &proc_def.qualified_name;
            let entry = by_target
                .entry(CallItemKey::for_proc(qname, proc_def))
                .or_insert_with(|| {
                    (
                        item_for_proc(source, proc_def, qname, line_index),
                        Vec::new(),
                    )
                });
            entry.1.push(span_to_range(source, line_index, span));
        }
    }
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Add every bare `ClassName <classmethod>` dispatch inside `body` to
/// `by_target`, keyed by the classmethod it names.
///
/// Every class the document declares is a candidate, not just the calling
/// method's own: a class command is global, so an instance method of one
/// class may perfectly well dispatch another class's classmethod
/// within the selected Logical source model. Sites come from the shared
/// original class-command selector, filtered to the calling body's span.
/// These source relationships do not establish Native dispatch.
fn add_classmethod_outgoing(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
    line_index: &LineIndex,
    by_target: &mut EdgeBuckets,
) {
    for class_def in analysis.all_classes.values() {
        for callee in class_def.class_methods.values() {
            if callee.visibility != "public" {
                continue;
            }
            let spans: Vec<tcl_lexer::Span> =
                crate::references::logical_class_command_method_sites(
                    source,
                    analysis,
                    class_def,
                    &callee.name,
                )
                .into_iter()
                .filter(|span| span_contains(body, *span))
                .collect();
            if spans.is_empty() {
                continue;
            }
            let entry = by_target
                .entry(CallItemKey::for_method(class_def, callee))
                .or_insert_with(|| {
                    (
                        item_for_method(source, class_def, callee, line_index),
                        Vec::new(),
                    )
                });
            entry.1.extend(
                spans
                    .into_iter()
                    .map(|s| span_to_range(source, line_index, s)),
            );
        }
    }
}

/// Iterate every method + classmethod of a class (the bodies
/// that can host intra-class method calls).
fn class_methods_iter(class_def: &ClassDef) -> impl Iterator<Item = &MethodDef> {
    class_def
        .methods
        .values()
        .chain(class_def.class_methods.values())
}

/// Every instance-side execution frame that can contain `my` dispatches.
/// Constructors and the destructor share the same object dispatch context as
/// ordinary methods and therefore participate in cross-document incoming
/// hierarchy edges too.
fn instance_method_frames_iter(class_def: &ClassDef) -> impl Iterator<Item = &MethodDef> {
    class_def
        .methods
        .values()
        .chain(class_def.constructors.iter())
        .chain(class_def.destructor.iter())
}

/// Actual source frames on the independently selected member table.
fn member_frames_iter(class_def: &ClassDef, side: MemberSide) -> impl Iterator<Item = &MethodDef> {
    instance_method_frames_iter(class_def)
        .filter(move |_| side == MemberSide::Instance)
        .chain(
            class_def
                .class_methods
                .values()
                .filter(move |_| side == MemberSide::ClassObject),
        )
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_source_context_tests;

#[cfg(test)]
mod logical_call_target_tests;

#[cfg(test)]
mod logical_workspace_occurrence_tests;

#[cfg(test)]
mod logical_method_source_candidate_tests;

#[cfg(test)]
mod method_original_lookup_operand_effect_tests;
