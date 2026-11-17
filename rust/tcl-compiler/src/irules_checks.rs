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

//! iRules-specific static checks (non-taint).
//!
//! Hosts:
//!
//! * **IRULE3102** — `HTTP::path` / `HTTP::uri` / `HTTP::query` getters
//!   used without the `-normalized` option (URL-evasion exposure).
//! * **IRULE5002** — `drop` / `reject` / `discard` without a subsequent
//!   `event disable all` or `return` (other iRules continue executing).
//! * **IRULE5004** — `DNS::return` without a subsequent `return`
//!   (iRule processing continues after `DNS::return`).
//!
//! The cross-event diagnostics — IRULE1005 / IRULE1006 / IRULE1007 /
//! IRULE1008 (collect/release/payload pairing across `when` events),
//! IRULE1201 / IRULE1202 (HTTP-after-respond and multi-respond), and
//! IRULE4004 (per-request set hoistable to once-per-connection) — build
//! on the richer cross-event analysis in `connection_scope.rs`.

#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;
#[cfg(test)]
use std::sync::OnceLock;
use tcl_core_types::DiagCode;
use tcl_dialect::model::Family;

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::events::{CollectionReleaseRequirement, PayloadCollectionRequirement};
#[cfg(test)]
use tcl_registry::side_effects::SideEffectTarget;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
#[cfg(test)]
use crate::ir::Statement;
#[cfg(test)]
use crate::value_shapes::parse_command_substitution_with_config;
use tcl_dialect::model::SurfaceQuery;

mod collect_flow;
mod conditional_flow;
mod drop_flow;
mod flow_inventory;
mod flow_walk;
mod generic_static_names;
mod hoistable_sets;
mod http_flow;
mod source_query;
mod unnormalised_getters;

#[cfg(test)]
use collect_flow::{collect_flow_state, event_anchor_span};
pub use collect_flow::{
    find_collect_flow_warnings, find_conditional_collection_source_pairing_advice,
};
use conditional_flow::conditional_irule_source_flow_advice;
pub use conditional_flow::find_conditional_irule_source_flow_advice;
pub use drop_flow::find_unguarded_drop_warnings;
pub use generic_static_names::{
    DEFAULT_GENERIC_VARIABLE_PATTERNS, find_generic_static_name_warnings,
};
#[cfg(test)]
use generic_static_names::{compile_generic_patterns, is_generic_static_name};
#[cfg(test)]
use hoistable_sets::event_mutation_counts;
pub use hoistable_sets::find_hoistable_set_warnings;
pub use http_flow::find_http_flow_warnings;
#[cfg(test)]
use source_query::leaf_handler_source_completion;
pub use unnormalised_getters::find_unnormalised_getter_warnings;
pub(crate) use unnormalised_getters::{format_message, is_unnormalised_getter};
#[cfg(test)]
use unnormalised_getters::{
    is_getter_form, supports_normalized_flag, unnormalised_getter_source_config,
};

/// Process-wide iRules command registry, used to derive the HTTP-flow command
/// sets below.  Built once (the data is static).
#[cfg(test)]
fn irules_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("f5-irules").commands()
}

/// Commands that commit an HTTP response — derived from the registry's
/// `ResponseCommit` side-effect, so the bare iRules `redirect` is
/// included alongside `HTTP::respond` / `HTTP::redirect` without a
/// hardcoded list.
#[cfg(test)]
fn commits_response_commands() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let reg = irules_registry();
        reg.command_names()
            .filter(|name| {
                reg.get(name).is_some_and(|spec| {
                    spec.side_effects
                        .lower_bound()
                        .iter()
                        .any(|h| h.target == SideEffectTarget::ResponseCommit && h.writes)
                })
            })
            .map(String::from)
            .collect()
    })
}

/// How much a [`CodeFix`] changes the behaviour of the program it rewrites.
///
/// A fix's *description* says what it does; this says what it costs.  The
/// distinction matters because "Fix All Safe Issues" applies fixes
/// unattended, in bulk, without the author reading each one: only a fix
/// that is **provably** semantics-preserving for the instance it was built
/// from belongs there.  A whitelist of diagnostic *codes* cannot express
/// that, because the same code can produce an equivalent rewrite in one
/// place and a behaviour-changing one in another.
///
/// The classification is per **fix instance**, not per code.  An emitter
/// that can prove equivalence for some inputs and not others must classify
/// each fix it builds on its own evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum FixSafety {
    /// The rewrite is provably equivalent for this instance: every input
    /// produces the same completion code, result, output, and side effects
    /// before and after.  Whitespace and delimiter repairs, and rewrites
    /// whose operands the emitter has proven inert, live here.
    ///
    /// **This is the only class "Fix All Safe Issues" applies.**
    SemanticsEquivalent,
    /// The rewrite deliberately removes a hazard, and in doing so can change
    /// what the program does.  Bracing an `expr` operand (W100) removes
    /// Tcl's second round of substitution — the point of the fix, and the
    /// reason a program relying on that substitution changes behaviour.
    /// Offered as its own named action with an explanation; never applied
    /// unattended.
    BehaviourHardening,
    /// The rewrite changes only the spelling a human reads — comment
    /// layout, a suppression directive, an alternative documented form with
    /// identical semantics that is nonetheless a matter of taste.  Safe to
    /// apply, but not a *correctness* fix, so bulk application is a style
    /// decision the author makes rather than one the server makes for them.
    StyleOnly,
    /// Equivalence is not established: the fix is a suggestion whose
    /// correctness depends on context the emitter could not check (inferred
    /// types, run-time values, traces, aliases, a rename the analyser cannot
    /// see).  The author must read it.  This is the **default** so a new
    /// emitter that has not thought about safety cannot silently opt its
    /// fixes into unattended bulk application.
    #[default]
    RequiresReview,
}

impl FixSafety {
    /// Whether a fix in this class may be applied unattended, in bulk, by
    /// "Fix All Safe Issues".
    ///
    /// The single place that decision is encoded — the server asks the fix,
    /// rather than keeping a second list of diagnostic codes that drifts
    /// from what the emitters actually prove.
    #[must_use]
    pub const fn is_bulk_applicable(self) -> bool {
        matches!(self, Self::SemanticsEquivalent)
    }

    /// A short label for the class, for diagnostics, tooling, and the
    /// `applied` list the fix-all command reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SemanticsEquivalent => "semantics-equivalent",
            Self::BehaviourHardening => "behaviour-hardening",
            Self::StyleOnly => "style-only",
            Self::RequiresReview => "requires-review",
        }
    }
}

/// A suggested code-action fix — maps to an LSP `TextEdit`.
///
/// `span` is the **fix range** the edit applies to, *independent* of the
/// diagnostic's own span: an insertion is a zero-width span anchored at the
/// insertion point (e.g. the end of a `drop`), a replacement covers the text it
/// rewrites.  This is the lower-level twin of the analyser's `CodeFix`, which
/// re-exports this type (see `analyser::types`); it lives here because the
/// iRules-flow / compiler-checks layer is below the analyser.
///
/// `safety` records how much the rewrite changes behaviour — see
/// [`FixSafety`].  Prefer the classified constructors
/// ([`CodeFix::equivalent`], [`CodeFix::hardening`], [`CodeFix::style`],
/// [`CodeFix::review`]) over a struct literal: they make the classification
/// an explicit decision at every emit site.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeFix {
    /// Source span the `new_text` replaces (zero-width for an insertion).
    pub span: Span,
    /// Replacement / inserted text.
    pub new_text: String,
    /// Human-readable description (`"Add 'event disable all' + 'return'"`).
    pub description: String,
    /// How much this rewrite changes the program's behaviour.
    pub safety: FixSafety,
}

impl CodeFix {
    /// A fix the emitter has proven equivalent for this instance — the only
    /// class "Fix All Safe Issues" applies.  See
    /// [`FixSafety::SemanticsEquivalent`].
    #[must_use]
    pub fn equivalent(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::classified(span, new_text, description, FixSafety::SemanticsEquivalent)
    }

    /// A fix that removes a hazard and may change behaviour in doing so.
    /// See [`FixSafety::BehaviourHardening`].
    #[must_use]
    pub fn hardening(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::classified(span, new_text, description, FixSafety::BehaviourHardening)
    }

    /// A fix that changes only the spelling a human reads.  See
    /// [`FixSafety::StyleOnly`].
    #[must_use]
    pub fn style(span: Span, new_text: impl Into<String>, description: impl Into<String>) -> Self {
        Self::classified(span, new_text, description, FixSafety::StyleOnly)
    }

    /// A suggestion whose correctness the emitter could not establish.  See
    /// [`FixSafety::RequiresReview`].
    #[must_use]
    pub fn review(span: Span, new_text: impl Into<String>, description: impl Into<String>) -> Self {
        Self::classified(span, new_text, description, FixSafety::RequiresReview)
    }

    /// Shared constructor behind the four classified builders.
    #[must_use]
    pub fn classified(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
        safety: FixSafety,
    ) -> Self {
        Self {
            span,
            new_text: new_text.into(),
            description: description.into(),
            safety,
        }
    }
}

/// Typed subject of one iRules check. The message is rendered from these
/// fields; consumers read them, never the message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrulesCheckPayload {
    /// IRULE3102: a getter whose declared normalising option is not selected.
    UnnormalisedGetter {
        /// Getter command as selected from the registry.
        command: String,
    },
    /// IRULE5002: a drop with no following `event disable all` / `return`.
    UnguardedDrop {
        /// Drop command as written.
        command: String,
    },
    /// IRULE5004: `DNS::return` with no following `return`.
    UnguardedDnsReturn,
    /// IRULE1005: an event whose collection requirement has no candidate.
    EventWithoutCollect {
        /// Event name.
        event: String,
        /// Required collection side.
        side: String,
        /// Registry collect commands that would satisfy the requirement.
        collect_commands: Vec<String>,
    },
    /// IRULE1006: a payload read with no matching collect candidate.
    PayloadWithoutCollect {
        /// Data-collection protocol.
        protocol: String,
        /// Collection side.
        side: String,
    },
    /// IRULE1007: a collect with no matching release candidate.
    CollectWithoutRelease {
        /// Data-collection protocol.
        protocol: String,
        /// Collection side.
        side: String,
    },
    /// IRULE1008: a release with no matching collect candidate.
    ReleaseWithoutCollect {
        /// Data-collection protocol.
        protocol: String,
        /// Collection side.
        side: String,
    },
    /// IRULE1201: an HTTP-context command after a committed response.
    ContextAfterResponse {
        /// Command that needs HTTP context.
        command: String,
        /// Event whose body contains it.
        event: String,
    },
    /// IRULE1202: a second response command on one path.
    RepeatedResponse {
        /// Response command.
        command: String,
        /// Event whose body contains it.
        event: String,
    },
    /// IRULE4002: a `static::` name matching a generic-name pattern.
    GenericStaticName {
        /// Source spelling of the variable reference.
        source_name: String,
        /// Namespace-relative name the pattern matched.
        pattern_name: String,
    },
    /// IRULE4004: a per-request constant assignment that could be hoisted.
    HoistableAssignment {
        /// Assigned connection variable.
        variable: String,
    },
}

/// One iRules-check diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrulesCheckWarning {
    /// Source span of the offending call.
    pub span: Span,
    /// Diagnostic code.
    pub code: DiagCode,
    /// Rendered message; presentation only.
    pub message: String,
    /// Typed subject the message is rendered from.
    pub payload: IrulesCheckPayload,
    /// Optional replacement text (currently always `None`).
    pub replacement: Option<String>,
    /// Suggested fixes whose range is independent of `span` (insertions /
    /// out-of-span replacements).  Empty when the check supplies no fix.
    pub fixes: Vec<CodeFix>,
}

/// Whether this point stands on the iRules surface.
///
/// Every check in this module is iRules-only and asked the question inline;
/// naming it once keeps the five gates reading the same way and keeps the
/// family comparison in one place.
fn is_irules(dialect: Option<SurfaceQuery<'_>>) -> bool {
    dialect.is_some_and(|q| {
        q.core
            .nearest()
            .is_some_and(|(family, _)| family == Family::F5Irules)
    })
}

/// One source-owner query shared by secondary iRules syntax consumers.
/// Supplied input and positively tagged standalone analysis are distinct routes.
struct IrulesSourceQuery<'a> {
    unit: &'a CompilationUnit,
    registry: &'a CommandRegistry,
    standalone: bool,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
}

/// Closed leaf-source census under its original handler and schema premises.
/// Runtime lookup, observers, activation and completion remain obligations.
struct ConditionalSourcePairingAdvice<'query, 'unit> {
    _source: &'query IrulesSourceQuery<'unit>,
    handlers: Vec<(
        crate::ir::ProcedureId,
        crate::registry_invocation::OriginalSourceScriptBody,
    )>,
    schemas: Vec<crate::registry_invocation::RegistryInvocationShape>,
    completions: Vec<tcl_registry::registry::InvocationCompletion>,
    flow_commands: Vec<(crate::ir::ProcedureId, Vec<OriginalIrulesFlowCommand>)>,
    _obligations: &'query HashSet<CollectionSourceObligationKind>,
}

fn conditional_source_pairing_message(
    _advice: &ConditionalSourcePairingAdvice<'_, '_>,
    source_pairing: &str,
) -> String {
    format!("Conditional source pairing (runtime applicability unproved): {source_pairing}")
}

/// Conditional pairing under a closed retained handler-source census.
/// Its immutable owners and unresolved runtime premises remain separate from
/// actual collection, absence, handler completion and edit authority.
pub struct ConditionalCollectionSourcePairingAdvice<'unit> {
    owner: ConditionalIruleSourceOwner<'unit>,
    pairings: Vec<ConditionalCollectionSourcePairing>,
}

#[derive(Clone)]
struct ConditionalIruleSourceOwner<'unit> {
    unit: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    _handlers: Vec<(
        crate::ir::ProcedureId,
        crate::registry_invocation::OriginalSourceScriptBody,
    )>,
    _schemas: Vec<crate::registry_invocation::RegistryInvocationShape>,
    obligations: HashSet<CollectionSourceObligationKind>,
    _completions: Vec<tcl_registry::registry::InvocationCompletion>,
    flow_commands: Vec<(crate::ir::ProcedureId, Vec<OriginalIrulesFlowCommand>)>,
}

impl<'unit> ConditionalCollectionSourcePairingAdvice<'unit> {
    /// Source-only pairing candidates; these supply no runtime outcome or fix.
    #[must_use]
    pub fn pairings(&self) -> &[ConditionalCollectionSourcePairing] {
        &self.pairings
    }

    pub(crate) fn diagnostic_source_owner_for(
        &self,
        pairing: &ConditionalCollectionSourcePairing,
    ) -> Option<(&CompilationUnit, &CommandRegistry)> {
        self.pairings
            .iter()
            .any(|current| std::ptr::eq(current, pairing))
            .then_some((self.owner.unit, self.owner.registry))
    }

    /// Independent premises that every conditional source pairing retains.
    pub fn obligations(&self) -> impl Iterator<Item = &CollectionSourceObligationKind> {
        self.owner.obligations.iter()
    }

    /// Project source flow from this same complete census and schema receipts.
    /// No second source inventory or command resolution is performed.
    #[must_use]
    pub fn conditional_source_flow(&self) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
        conditional_irule_source_flow_advice(&self.owner)
    }
}

/// Conditional flow among complete original leaf-handler source commands.
/// Source order and descriptor shapes establish no executed continuation.
pub struct ConditionalIruleSourceFlowAdvice<'unit> {
    owner: ConditionalIruleSourceOwner<'unit>,
    findings: Vec<ConditionalIruleSourceFlowFinding>,
}

impl ConditionalIruleSourceFlowAdvice<'_> {
    /// Source-order hazards with runtime applicability explicitly unresolved.
    #[must_use]
    pub fn findings(&self) -> &[ConditionalIruleSourceFlowFinding] {
        &self.findings
    }

    /// Independent premises, including normal continuation, remain unproved.
    pub fn obligations(&self) -> impl Iterator<Item = &CollectionSourceObligationKind> {
        self.owner.obligations.iter()
    }

    pub(crate) fn diagnostic_source_owner_for(
        &self,
        finding: &ConditionalIruleSourceFlowFinding,
    ) -> Option<(&CompilationUnit, &CommandRegistry)> {
        self.findings
            .iter()
            .any(|current| std::ptr::eq(current, finding))
            .then_some((self.owner.unit, self.owner.registry))
    }
}

/// A possible source-order hazard, without a reached runtime path or fix.
pub struct ConditionalIruleSourceFlowFinding {
    warning: IrulesCheckWarning,
}

impl ConditionalIruleSourceFlowFinding {
    /// Original source advice anchor.
    #[must_use]
    pub fn span(&self) -> Span {
        self.warning.span
    }

    /// Diagnostic family; it supplies no action subject.
    #[must_use]
    pub fn code(&self) -> DiagCode {
        self.warning.code
    }

    /// Conditional source scope and unresolved runtime continuation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.warning.message
    }

    /// Typed subject of the finding.
    #[must_use]
    pub fn payload(&self) -> &IrulesCheckPayload {
        &self.warning.payload
    }
}

/// One possible source-pairing hazard, without an observed runtime outcome.
pub struct ConditionalCollectionSourcePairing {
    warning: IrulesCheckWarning,
}

impl ConditionalCollectionSourcePairing {
    /// Original source advice anchor, not an evaluated operation.
    #[must_use]
    pub fn span(&self) -> Span {
        self.warning.span
    }

    /// Diagnostic family for this conditional source advice.
    #[must_use]
    pub fn code(&self) -> DiagCode {
        self.warning.code
    }

    /// The explicit source scope and unresolved runtime applicability.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.warning.message
    }

    /// Typed subject of the finding.
    #[must_use]
    pub fn payload(&self) -> &IrulesCheckPayload {
        &self.warning.payload
    }
}

/// Original conditional source event, without an executing event or worker.
struct OriginalEvent<'a> {
    body: &'a crate::ir::SourceIrulesEventBody,
    procedure: &'a crate::ir::Procedure,
}

enum OriginalEventPresence<'a> {
    Ordinary,
    Event(OriginalEvent<'a>),
}

impl<'a> OriginalEventPresence<'a> {
    fn into_event(self) -> Option<OriginalEvent<'a>> {
        match self {
            Self::Ordinary => None,
            Self::Event(event) => Some(event),
        }
    }
}

/// Select the actual function's complete config before choosing parser purpose.
/// Standalone assistance requires explicit standalone Module and CFG owners.
fn irules_function_source_config(
    module: &crate::ir::Module,
    function: &FunctionUnit,
    registry: &CommandRegistry,
    standalone_module: bool,
) -> Option<tcl_lexer::LexerConfig> {
    if standalone_module {
        if !module.source_entry.metadata_context.is_standalone()
            || module.source_metadata_input.is_some()
            || !function.cfg.metadata_context.is_standalone()
            || function.source_metadata_input().is_some()
        {
            return None;
        }
    } else {
        function.invocation_metadata_context_for_module(registry, module)?;
    }
    Some(function.source_lexer_config())
}

// IRULE5002 + IRULE5004 — unguarded drop / DNS::return
//
// Path-sensitive: each original event body is walked with each branch
// of every `if` / `switch` / `try` tracked separately, emitting at
// branch joins where the drop survived unguarded.

/// One drop / DNS-return flow fact along a path.  Dedup keys on the
/// `(dropped, dns_returned)` pair only, so the first unguarded occurrence
/// on any surviving path wins.
#[derive(Clone, Default)]
struct DropFlowState {
    dropped: bool,
    drop_cmd: String,
    drop_at: Option<Span>,
    dns_returned: bool,
    dns_at: Option<Span>,
}

#[derive(Clone)]
struct OriginalIrulesPacketActions {
    drops_connection: bool,
    returns_dns_packet: bool,
}

#[derive(Clone)]
struct OriginalIrulesFlowCommand {
    command: String,
    span: Span,
    packet_actions: OriginalIrulesPacketActions,
    disables_all_events: bool,
    may_commit_response: bool,
    requires_http_context: bool,
}

// IRULE1005 / IRULE1006 / IRULE1007 / IRULE1008 — collect/release/payload
//
// Cross-event analysis: classifies registry-declared collection operations
// across every original event body, then emits:
//
//   * IRULE1005 — `*_DATA` event handler exists but no matching
//     `*::collect` for the corresponding protocol.
//   * IRULE1006 — a payload that requires collection without a matching
//     collect (not UDP or ASM payloads, which are immediately available).
//   * IRULE1007 — a collection without an explicit or registry-declared
//     implicit release on the same connection side.
//   * IRULE1008 — a release without matching collection on the same
//     connection side.
//
// Side awareness: `EventProps::command_side` declares the event's execution
// side. Nested side-switch bodies apply their registry-declared
// `CommandSpec::side_switch_target`; no command or event spelling is decoded
// here.

#[derive(Default)]
struct CollectFlowState {
    /// Runtime lookup and widening remain independent of positive source calls.
    obligations: HashSet<CollectionSourceObligationKind>,
    source_argv: Vec<(crate::ir::ProcedureId, Vec<tcl_lexer::NativeWord>)>,
    /// `protocol -> {side, ...}` for each protocol with a collect call.
    collected: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Same shape for release calls.
    released: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Each collect call, including the release policy declared by its spec.
    collect_calls: Vec<CollectionCall>,
    /// Each release call.
    release_calls: Vec<CollectionCall>,
    /// Each payload call, including whether its spec requires a collect.
    payload_calls: Vec<PayloadCall>,
}

/// Unresolved runtime premises retained independently of source pairings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectionSourceObligationKind {
    /// A genuine no-own-argv widening phase retains unknown effects.
    EffectOnly(crate::ir::SyntheticMarker),
    /// Runtime command selection retains an unknown alternative.
    UnknownTarget,
    /// Runtime lookup may select no command.
    MayBeAbsent,
    /// A retained event declaration does not establish activation.
    EventActivation,
    /// Authored source descriptors do not establish their actual handlers.
    SourceSchemaApplicability,
    /// Source ordering does not establish child or parent normal completion.
    NormalContinuation,
}

/// One collect or release operation observed in a `when` body.
struct CollectionCall {
    protocol: String,
    event: String,
    side: String,
    span: Span,
    release_requirement: CollectionReleaseRequirement,
}

/// One payload operation observed in a `when` body.
struct PayloadCall {
    protocol: String,
    side: String,
    span: Span,
    collection_requirement: PayloadCollectionRequirement,
}

// IRULE1201 / IRULE1202 — HTTP-after-respond / multi-respond
//
// Path-sensitive analysis of the *structured* IR body of each
// original HTTP-profile event body, tracking a set of per-path response states
// so that:
//
//   * IRULE1202 — a second `HTTP::respond` / `HTTP::redirect` / `redirect`
//     on a path that already responded; only the first response takes effect.
//   * IRULE1201 — a registry-declared HTTP-context command issued on a path
//     where a response is already committed.
//
// Mutually-exclusive branches keep separate states, so `if {…} { respond }
// else { respond }` is *not* a double-respond (avoids the linear-scan false
// positive), `catch { respond }; HTTP::…` *is* flagged, and a loop body runs
// zero-or-more times.  `return` terminates the current path.

/// A single response-flow fact along one path.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HttpFlowState {
    responded: bool,
    /// Start offset of the committing response (for dedup / parity), or `None`.
    respond_at: Option<u32>,
}

/// Path-sensitive forward walk of a structured IR script, threading a set of
/// per-path flow states `S`.  Control-flow statements fan the states into each
/// branch and merge (`dedupe`) at join points; `return` terminates the current
/// paths.  Leaf statements are transferred by `leaf` (which may emit warnings
/// into `out`).  Shared by the IRULE1201/1202 and IRULE5002/5004 analyses —
/// they differ only in their state type, `leaf`, and `dedupe`.
///
/// `return_sink` is the nearest enclosing `catch` body's collection point.
/// Tcl `catch` swallows `TCL_RETURN`, so a `return` inside a catch body does
/// not stop the rule — control continues past the catch carrying the state as
/// of the swallowed `return`.  When `return_sink` is `Some`, a `return` hands
/// its at-return states there (so e.g. a committed response, or a pending
/// drop, survives past the catch) instead of discarding them; the [`Catch`]
/// arm folds them back into the continuing paths.  It is threaded through every
/// nested walk so a `return` buried in an `if`/`switch`/loop/`try` inside the
/// catch body still propagates out.  At event/proc scope it is `None` and a
/// `return` terminates the path.
///
/// [`Catch`]: Statement::Catch
/// The fixed dispatch state threaded through the flow walk: the warning sink
/// `out`, the per-leaf transfer `leaf`, and the state-set `dedupe`.  Bundled so
/// the mutually-recursive walk helpers stay under the argument limit.
struct FlowDispatch<'a, L, D> {
    registry: &'a CommandRegistry,
    out: &'a mut Vec<IrulesCheckWarning>,
    leaf: &'a L,
    dedupe: &'a D,
    /// The statements the solver proved never run ([`unreached_statements`]):
    /// a step over one leaves the states as they were.
    unreached: &'a HashSet<Span>,
    /// Whether a statement ends its path, for a walk that has no converged
    /// handler to ask: conditional advice answers from the statement's
    /// possible source schemas. `None` asks the handler's completion route.
    terminates: Option<&'a dyn Fn(&crate::ir::CommandTokens) -> bool>,
}

// Tests

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_getter_source_config_tests;

#[cfg(test)]
mod original_collection_source_tests;

#[cfg(test)]
mod original_event_owner_tests;
