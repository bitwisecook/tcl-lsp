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

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use tcl_core_types::DiagCode;
use tcl_dialect::model::Family;

use tcl_lexer::Span;
use tcl_registry::events::{
    CollectionReleaseRequirement, DataCollectionAction, EventRegistry, PayloadCollectionRequirement,
};
use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget};
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg_builder::build_cfg;
use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use crate::lowering::lower_to_ir_with_config;
use crate::sccp::cfg_order;
use crate::value_shapes::parse_command_substitution;
use tcl_dialect::model::SurfaceQuery;

/// Process-wide iRules command registry, used to derive the HTTP-flow command
/// sets below.  Built once (the data is static).
fn irules_registry() -> &'static CommandRegistry {
    static REG: OnceLock<CommandRegistry> = OnceLock::new();
    REG.get_or_init(|| {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    })
}

/// Commands that commit an HTTP response — derived from the registry's
/// `ResponseCommit` side-effect, so the bare iRules `redirect` is
/// included alongside `HTTP::respond` / `HTTP::redirect` without a
/// hardcoded list.
fn commits_response_commands() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let reg = irules_registry();
        reg.command_names()
            .filter(|name| {
                reg.get(name).is_some_and(|spec| {
                    spec.side_effects
                        .iter()
                        .any(|h| h.target == SideEffectTarget::ResponseCommit && h.writes)
                })
            })
            .map(String::from)
            .collect()
    })
}

/// Registered commands that require a live HTTP transaction context.
fn requires_http_context_commands() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let registry = irules_registry();
        registry
            .command_names()
            .filter(|name| {
                registry
                    .get(name)
                    .is_some_and(|spec| spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT))
            })
            .map(String::from)
            .collect()
    })
}

/// Whether `cmd` is a registered iRules getter that carries the
/// `-normalized` option in its [`tcl_registry::CommandSpec`].
///
/// Replaces the previous hardcoded `NORMALISED_FLAG_COMMANDS` table:
/// the registry's own option list is the single source of truth, so
/// adding `-normalized` to a new command's spec automatically
/// extends the IRULE3102 surface.
fn supports_normalized_flag(registry: &CommandRegistry, cmd: &str) -> bool {
    registry
        .get(cmd)
        .is_some_and(|spec| spec.options.iter().any(|opt| opt.name == "-normalized"))
}

/// How much a [`CodeFix`] changes the behaviour of the program it rewrites.
///
/// A fix's *description* says what it does; this says what it costs.  The
/// distinction matters because "Fix All Safe Issues" applies fixes
/// unattended, in bulk, without the author reading each one: only a fix
/// that is **provably** semantics-preserving for the instance it was built
/// from belongs there.  A whitelist of diagnostic *codes* cannot express
/// that, because the same code can produce an equivalent rewrite in one
/// place and a behaviour-changing one in another (issue #1195).
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

/// An IRULE3102 / iRules-check diagnostic emitted by
/// [`find_unnormalised_getter_warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrulesCheckWarning {
    /// Source span of the offending call.
    pub span: Span,
    /// Diagnostic code (`"IRULE3102"`).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text (currently always `None`).
    pub replacement: Option<String>,
    /// Suggested fixes whose range is independent of `span` (insertions /
    /// out-of-span replacements).  Empty when the check supplies no fix.
    pub fixes: Vec<CodeFix>,
}

/// Return `true` when `args` suggests a *getter* invocation — no args,
/// or every arg starts with `-` (all flags, no positional value).
/// Resolving a getter form properly would need a form-kind model on the
/// command registry, which it does not have yet, so this is a
/// conservative approximation. `HTTP::path /foo` (setter) is excluded
/// because `/foo` is a non-flag first arg.
fn is_getter_form(args: &[String]) -> bool {
    args.iter().all(|a| a.starts_with('-'))
}

/// Format the IRULE3102 message for `cmd`.
pub(crate) fn format_message(cmd: &str) -> String {
    format!(
        "Use '{cmd} -normalized' for canonicalized request data; \
         non-normalized values may allow URL evasion patterns."
    )
}

/// Return `true` when `cmd` is one of the commands that carry the
/// `-normalized` option and `args` misses it in a getter form.
pub(crate) fn is_unnormalised_getter(
    registry: &CommandRegistry,
    cmd: &str,
    args: &[String],
) -> bool {
    if !supports_normalized_flag(registry, cmd) {
        return false;
    }
    if args.iter().any(|a| a == "-normalized") {
        return false;
    }
    is_getter_form(args)
}

/// Whether this point stands on the iRules surface.
///
/// Every check in this module is iRules-only and asked the question inline;
/// naming it once keeps the five gates reading the same way and keeps the
/// family comparison in one place.
fn is_irules(dialect: Option<SurfaceQuery<'_>>) -> bool {
    dialect.is_some_and(|q| q.core.is_some_and(|(family, _)| family == Family::F5Irules))
}

/// Find IRULE3102 warnings across every function in `cu`.
///
/// Gated: returns an empty vector unless the point stands on the iRules
/// surface.
///
/// Scan targets:
///
/// * `Statement::Call` whose `command` is a normalised-flag command —
///   direct call site, typically as part of a larger expression: e.g.
///   `if {[HTTP::uri] eq "/foo"}` after lowering becomes a call on
///   `HTTP::uri`.
/// * `Statement::AssignValue` whose RHS is a pure command substitution
///   `[HTTP::uri …]` — covers `set u [HTTP::uri]`.
#[must_use]
pub fn find_unnormalised_getter_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules(dialect) {
        return out;
    }

    for fu in cu.analysable_functions() {
        for bn in cfg_order(&fu.cfg) {
            if !fu.sccp.executable_blocks.contains(&bn) {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            for stmt in &block.statements {
                match stmt {
                    Statement::Call {
                        command,
                        args,
                        span,
                        tokens,
                        ..
                    } => {
                        // Direct getter call — e.g. `if {[HTTP::uri] eq …}`
                        // lowers to a `Call` whose command *is* the getter.
                        if is_unnormalised_getter(registry, command, args) {
                            out.push(IrulesCheckWarning {
                                span: fu.abs_span(*span),
                                code: DiagCode::Irule3102,
                                message: format_message(command),
                                replacement: None,
                                fixes: Vec::new(),
                            });
                        }
                        // Getter nested as a command-substitution argument —
                        // e.g. the `[HTTP::uri]` fed into an HTTP sink in
                        // `HTTP::respond 200 content [HTTP::uri]`.  The
                        // substitution stays a literal `Cmd` arg on the outer
                        // call rather than a separate statement, so flag it
                        // here.  Prefer the argument token's own span so the
                        // squiggle lands on the getter, not the whole call.
                        for (i, arg) in args.iter().enumerate() {
                            let trimmed = arg.trim();
                            if !trimmed.starts_with('[') {
                                continue;
                            }
                            let Some((cmd, sub_args)) = parse_command_substitution(trimmed) else {
                                continue;
                            };
                            if is_unnormalised_getter(registry, &cmd, &sub_args) {
                                let arg_span = tokens
                                    .as_ref()
                                    .and_then(|t| t.argv.get(i + 1).copied())
                                    .unwrap_or(*span);
                                out.push(IrulesCheckWarning {
                                    span: fu.abs_span(arg_span),
                                    code: DiagCode::Irule3102,
                                    message: format_message(&cmd),
                                    replacement: None,
                                    fixes: Vec::new(),
                                });
                            }
                        }
                    }
                    Statement::AssignValue { value, span, .. } => {
                        let Some((cmd, sub_args)) = parse_command_substitution(value.trim()) else {
                            continue;
                        };
                        if is_unnormalised_getter(registry, &cmd, &sub_args) {
                            out.push(IrulesCheckWarning {
                                span: fu.abs_span(*span),
                                code: DiagCode::Irule3102,
                                message: format_message(&cmd),
                                replacement: None,
                                fixes: Vec::new(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    out
}

// IRULE5002 + IRULE5004 — unguarded drop / DNS::return
//
// Path-sensitive: each `::when::*` proc body is walked with each branch
// of every `if` / `switch` / `try` tracked separately, emitting at
// branch joins where the drop survived unguarded.

fn is_drop_command(cmd: &str) -> bool {
    matches!(cmd, "drop" | "reject" | "discard")
}

fn is_dns_return(cmd: &str, _args: &[String]) -> bool {
    // Any `DNS::return` (with or without options) requires a
    // following `return`; option-bearing forms (`DNS::return -...`)
    // are valid Tcl and equally affected by the missing-`return`
    // hazard (no option-prefix gate).
    cmd == "DNS::return"
}

/// `event disable all` is a drop guard: the subsequent statements
/// run but no other iRule does.
fn is_event_disable_all(cmd: &str, args: &[String]) -> bool {
    cmd == "event" && args.len() >= 2 && args[0] == "disable" && args[1] == "all"
}

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

/// Deduplicate drop states by `(dropped, dns_returned)`, keeping the first.
fn dedupe_drop_states(states: Vec<DropFlowState>) -> Vec<DropFlowState> {
    let mut seen: HashSet<(bool, bool)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        if seen.insert((s.dropped, s.dns_returned)) {
            out.push(s);
        }
    }
    out
}

/// Leaf transfer for the drop-flow walk: `drop`/`reject`/`discard` set the
/// dropped fact, `DNS::return` sets the DNS fact, and `event disable all`
/// clears the dropped fact (it guards the drop).
fn drop_flow_leaf(st: &DropFlowState, stmt: &Statement) -> DropFlowState {
    let Statement::Call {
        command,
        args,
        span,
        ..
    } = stmt
    else {
        return st.clone();
    };
    if is_event_disable_all(command, args) {
        return DropFlowState {
            dropped: false,
            drop_cmd: String::new(),
            drop_at: None,
            dns_returned: st.dns_returned,
            dns_at: st.dns_at,
        };
    }
    if is_drop_command(command) {
        return DropFlowState {
            dropped: true,
            drop_cmd: command.clone(),
            drop_at: Some(*span),
            dns_returned: st.dns_returned,
            dns_at: st.dns_at,
        };
    }
    if is_dns_return(command, args) {
        return DropFlowState {
            dns_returned: true,
            dns_at: Some(*span),
            ..st.clone()
        };
    }
    st.clone()
}

/// Path-sensitive IRULE5002 / IRULE5004 — an unguarded `drop`/`reject`/
/// `discard` (no following `event disable all` / `return`) or a `DNS::return`
/// without a following `return`, on at least one path through a `::when::*`
/// body.
#[must_use]
pub fn find_unguarded_drop_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let leaf = |st: &DropFlowState, stmt: &Statement, _out: &mut Vec<IrulesCheckWarning>| {
        drop_flow_leaf(st, stmt)
    };
    // Honour the complexity guard but walk the structured
    // IR body of each analysable `::when::*` event handler.
    for fu in cu.analysable_functions() {
        if !fu.name.starts_with("::when::") {
            continue;
        }
        let Some(proc) = cu.ir_module.procedures.get(&fu.name) else {
            continue;
        };
        let finals = walk_flow(
            &proc.body,
            &[DropFlowState::default()],
            &mut FlowDispatch {
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_drop_states,
            },
            None,
        );
        for st in finals {
            if st.dropped
                && let Some(span) = st.drop_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5002,
                    message: format!(
                        "'{}' without 'event disable all' or 'return' — other iRules and later \
                         priorities in this event will still execute, which may cause TCL errors.",
                        st.drop_cmd,
                    ),
                    replacement: None,
                    // Insert `event disable all` + `return` right after the drop
                    // (a zero-width edit at the drop command's end).
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    event disable all\n    return".to_owned(),
                        description: "Add 'event disable all' + 'return'".to_owned(),
                        // IRULE5002: adding `event disable all` + `return` stops later
                        // iRules and priorities running — the fix, and a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
            if st.dns_returned
                && let Some(span) = st.dns_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5004,
                    message: "'DNS::return' must be followed by 'return' to stop iRule processing."
                        .to_owned(),
                    replacement: None,
                    // Insert `return` right after `DNS::return`.
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    return".to_owned(),
                        description: "Add 'return' after DNS::return".to_owned(),
                        // IRULE5004: adding `return` stops iRule processing after
                        // `DNS::return` — a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
        }
    }
    out
}

// IRULE1005 / IRULE1006 / IRULE1007 / IRULE1008 — collect/release/payload
//
// Cross-event analysis: classifies registry-declared collection operations
// across every `::when::*` proc body, then emits:
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

/// Record a registry-declared data-collection operation.
///
/// This deliberately does not infer semantics from a `::payload` suffix:
/// `UDP::payload` and `ASM::payload` are immediately available, while TCP,
/// HTTP, and SSL payloads require collection.  New protocol commands join the
/// analysis by declaring [`tcl_registry::DataCollectionOperation`] in their
/// command specs.
fn classify_collect_command(
    command: &str,
    args: &[String],
    event: &str,
    side: &str,
    span: Span,
    state: &mut CollectFlowState,
    registry: &CommandRegistry,
) {
    let Some(operation) = registry.data_collection_operation(command) else {
        return;
    };
    let arg_words: Vec<&str> = args.iter().map(String::as_str).collect();
    let protocol = operation.protocol.name.to_string();
    match operation.action {
        DataCollectionAction::Collect => {
            state
                .collected
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.collect_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Release => {
            state
                .released
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.release_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Payload => {
            let Some(collection_requirement) = operation.payload_requirement_for_args(&arg_words)
            else {
                // Dynamic/unknown call form: it may select a payload form
                // whose availability differs, so abstain rather than guess.
                return;
            };
            state.payload_calls.push(PayloadCall {
                protocol,
                side: side.to_string(),
                span,
                collection_requirement,
            });
        }
    }
}

fn scan_when_body_for_collect_flow(
    fu: &crate::compilation_unit::FunctionUnit,
    event: &str,
    side: &str,
    state: &mut CollectFlowState,
    registry: &CommandRegistry,
) {
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(stmt, event, side, state, registry, fu.base_offset);
        }
    }
}

/// Classify one IR statement for collect/release/payload flow,
/// descending into iRules side-switch bodies (`clientside` /
/// `serverside` / `peer`) with the switched side.  `clientside` /
/// `serverside` carry a fixed side; `peer` evaluates under the
/// *opposite* of the current side (so `peer { TCP::collect }` in a
/// client-side event issues a server-side collect, and vice-versa).
/// These commands carry an `ArgRole::Body` arg so they lower to a
/// `Statement::Barrier` (or a `Statement::Call` with the body in
/// `args[0]`); both shapes keep the body text in `args[0]`.
fn classify_stmt_for_collect_flow(
    stmt: &Statement,
    event: &str,
    side: &str,
    state: &mut CollectFlowState,
    registry: &CommandRegistry,
    base_offset: i64,
) {
    // The flow-state spans land in cross-event warnings emitted where the
    // producing `fu` is out of scope, so absolutise them here (no-op when
    // `base_offset == 0`).
    let abs = |sp: Span| -> Span {
        if base_offset == 0 {
            return sp;
        }
        // Spans are `u32`; the absolutised offset is clamped to `0`, and a
        // source larger than 4 GiB is not representable, so saturate to
        // `u32::MAX` on the (impossible-in-practice) overflow.
        let s = (i64::from(sp.start()) + base_offset).max(0);
        let e = (i64::from(sp.end()) + base_offset).max(0);
        Span::new(
            u32::try_from(s).unwrap_or(u32::MAX),
            u32::try_from(e).unwrap_or(u32::MAX),
        )
    };
    match stmt {
        Statement::Call {
            command,
            canonical_command,
            span,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            span,
            args,
            tokens,
            ..
        } => {
            // Normalise to the resolved/canonical head and strip a leading
            // `::`: IR can legally carry a global spelling or an alias. The
            // registry owns both the side-switch membership and the exact
            // client/server/peer transition, so this walker has no command
            // spelling knowledge.
            let head = canonical_command
                .as_deref()
                .unwrap_or(command)
                .trim_start_matches(':');
            if let Some(target) = registry.side_switch_target(head) {
                let inner_side = target.execution_side(side);
                if let Some(body) = args.first()
                    && tokens
                        .as_ref()
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(0))
                    && let Some(body_span) = tokens
                        .as_ref()
                        .and_then(|tokens| tokens.argv.get(1).copied())
                {
                    let body_base = base_offset + i64::from(body_span.start()) + 1;
                    scan_side_switch_body(body, event, inner_side, state, registry, body_base);
                }
            } else {
                classify_collect_command(head, args, event, side, abs(*span), state, registry);
            }
        }
        Statement::AssignValue { value, span, .. } => {
            if let Some((cmd, sub_args)) = parse_command_substitution(value.trim()) {
                classify_collect_command(&cmd, &sub_args, event, side, abs(*span), state, registry);
            }
        }
        _ => {}
    }
}

/// Lower a side-switch body (the brace-script text in `args[0]`) and
/// classify its commands under `side`, recursing through any nested
/// side-switches.  The body is re-lowered to its own CFG so nested
/// control flow is flattened the same way the enclosing `when` body is.
fn scan_side_switch_body(
    body_text: &str,
    event: &str,
    side: &str,
    state: &mut CollectFlowState,
    registry: &CommandRegistry,
    base_offset: i64,
) {
    // iRules side-switch flow — lower under the iRules dialect so `{*}` is
    // literal and `}{` splits words.
    let module = lower_to_ir_with_config(
        body_text,
        registry,
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
    );
    let cfg_module = build_cfg(&module, false);
    for bn in cfg_order(&cfg_module.top_level) {
        let Some(block) = cfg_module.top_level.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(stmt, event, side, state, registry, base_offset);
        }
    }
}

/// Collect/release/payload pairing across all `when` events.
#[must_use]
pub fn find_collect_flow_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    if !is_irules(dialect) {
        return Vec::new();
    }
    let (state, events_seen) = collect_flow_state(cu, registry);
    let events = EventRegistry::build();
    let mut out = Vec::new();
    emit_missing_data_collect_warnings(cu, registry, &events, &events_seen, &state, &mut out);
    emit_payload_without_collect_warnings(&state, &mut out);
    emit_collect_without_release_warnings(&events, &events_seen, &state, &mut out);
    emit_release_without_collect_warnings(&state, &mut out);
    out
}

/// Classify each iRules event handler and retain its bare event name.
fn collect_flow_state(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> (CollectFlowState, Vec<String>) {
    let mut state = CollectFlowState::default();
    let mut events_seen: Vec<String> = Vec::new();
    let events = EventRegistry::build();
    for fu in cu.analysable_functions() {
        if let Some(event) = fu.name.strip_prefix("::when::") {
            let bare = event.split('#').next().unwrap_or(event).to_string();
            events_seen.push(bare.clone());
            let Some(side) = events
                .get_props(&bare)
                .and_then(tcl_registry::events::EventProps::command_side)
                .and_then(|side| match side {
                    ConnectionSide::Client => Some("client"),
                    ConnectionSide::Server => Some("server"),
                    _ => None,
                })
            else {
                // Unknown or genuinely both-sided event context is not proof
                // of either lifecycle direction. Abstain instead of deriving
                // a side from the event's spelling.
                continue;
            };
            scan_when_body_for_collect_flow(fu, &bare, side, &mut state, registry);
        }
    }
    (state, events_seen)
}

/// Emit IRULE1005 for data-event protocols that are certainly not collected.
fn emit_missing_data_collect_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    out: &mut Vec<IrulesCheckWarning>,
) {
    // The event table owns protocol and side requirements. The first event
    // statement is a fallback anchor until the analyser exposes event-token
    // spans directly to this flow pass.
    let mut emitted_events: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for event in events_seen {
        let Some((protocols, required_side)) = events.data_collect_requirement(event) else {
            continue;
        };
        if !emitted_events.insert(event.as_str()) {
            continue;
        }
        let satisfied = protocols.iter().any(|protocol_name| {
            let Some(protocol) = registry.data_collection_protocol(protocol_name) else {
                // The event table must not be treated as proof for a protocol
                // whose command surface is absent from this registry profile.
                return false;
            };
            protocol.payload_requirement == PayloadCollectionRequirement::NotRequired
                || state
                    .collected
                    .get(protocol.name)
                    .is_some_and(|s| s.contains(required_side))
        });
        if satisfied {
            continue;
        }
        let proto_hint: Vec<&str> = protocols
            .iter()
            .filter_map(|protocol| registry.data_collection_collect_command(protocol))
            .map(|spec| spec.name)
            .collect();
        if proto_hint.is_empty() {
            // The registry says each applicable protocol supplies payload
            // without an explicit collect, so the source cannot be proved to
            // miss one. This is intentionally an abstention, not a warning.
            continue;
        }
        out.push(IrulesCheckWarning {
            span: event_anchor_span(cu, event),
            code: DiagCode::Irule1005,
            message: format!(
                "'{event}' will never fire without a {required_side} {} call in another event.",
                proto_hint.join(" or "),
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Return the first statement span in `event`, or a stable zero-width fallback.
fn event_anchor_span(cu: &CompilationUnit, event: &str) -> Span {
    let qname = format!("::when::{event}");
    cu.functions()
        .find(|fu| fu.name == qname || fu.name.starts_with(&format!("{qname}#")))
        .and_then(|fu| {
            let span = fu
                .cfg
                .blocks
                .get(&fu.cfg.entry)
                .and_then(|b| b.statements.first().map(crate::ir::Statement::span))?;
            Some(fu.abs_span(span))
        })
        .unwrap_or(Span::new(0, 0))
}

/// Emit IRULE1006 for payload commands that explicitly require collection.
fn emit_payload_without_collect_warnings(
    state: &CollectFlowState,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for payload in &state.payload_calls {
        let matched = state
            .collected
            .get(&payload.protocol)
            .is_some_and(|s| s.contains(&payload.side));
        if payload.collection_requirement == PayloadCollectionRequirement::ExplicitCollect
            && !matched
        {
            out.push(IrulesCheckWarning {
                span: payload.span,
                code: DiagCode::Irule1006,
                message: format!(
                    "'{}::payload' without a {} {}::collect call. The payload buffer will be empty.",
                    payload.protocol, payload.side, payload.protocol,
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Emit IRULE1007 for registered collection lifecycles not released by code or event.
fn emit_collect_without_release_warnings(
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for collect in &state.collect_calls {
        let needs_release =
            collect.release_requirement != CollectionReleaseRequirement::NotApplicable;
        let released = state
            .released
            .get(&collect.protocol)
            .is_some_and(|s| s.contains(&collect.side));
        let implicitly_released = collect.release_requirement
            == CollectionReleaseRequirement::ImplicitInDataEvent
            && events_seen.iter().any(|event| {
                let side = match collect.side.as_str() {
                    "client" => ConnectionSide::Client,
                    "server" => ConnectionSide::Server,
                    _ => return false,
                };
                events.data_event_releases_collection(
                    event,
                    &collect.event,
                    &collect.protocol,
                    side,
                )
            });
        if needs_release && !released && !implicitly_released {
            out.push(IrulesCheckWarning {
                span: collect.span,
                code: DiagCode::Irule1007,
                message: format!(
                    "{}::collect without matching {}::release on the {} side; collected data is never released",
                    collect.protocol, collect.protocol, collect.side,
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Emit IRULE1008 for a release whose registered protocol was never collected.
fn emit_release_without_collect_warnings(
    state: &CollectFlowState,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for release in &state.release_calls {
        let matched = state
            .collected
            .get(&release.protocol)
            .is_some_and(|s| s.contains(&release.side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: release.span,
                code: DiagCode::Irule1008,
                message: format!(
                    "{}::release without matching {}::collect on the {} side; no data was collected",
                    release.protocol, release.protocol, release.side,
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

// IRULE1201 / IRULE1202 — HTTP-after-respond / multi-respond
//
// Path-sensitive analysis of the *structured* IR body of each
// `::when::HTTP*` procedure, tracking a set of per-path response states
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

/// Per-event HTTP-flow warnings. Emits IRULE1201 + IRULE1202 for each handler
/// whose event registry entry implies an HTTP profile, path-sensitively.
#[must_use]
pub fn find_http_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let events = EventRegistry::build();
    // Iterate the *analysable* functions (honouring the complexity
    // guard) but walk their structured IR body.
    for fu in cu.analysable_functions() {
        let Some(event) = fu.name.strip_prefix("::when::") else {
            continue;
        };
        // Strip priority index (`HTTP_REQUEST#1` → `HTTP_REQUEST`).
        let bare_event = event.split('#').next().unwrap_or(event);
        if !events
            .get_props(bare_event)
            .is_some_and(|props| props.implied_profiles.contains(&"HTTP"))
        {
            continue;
        }
        let Some(proc) = cu.ir_module.procedures.get(&fu.name) else {
            continue;
        };
        let leaf = |st: &HttpFlowState, stmt: &Statement, out: &mut Vec<IrulesCheckWarning>| {
            match http_flow_stmt_command(stmt) {
                Some((cmd, span)) => apply_http_flow_command(*st, cmd, span, bare_event, out),
                None => *st,
            }
        };
        walk_flow(
            &proc.body,
            &[HttpFlowState {
                responded: false,
                respond_at: None,
            }],
            &mut FlowDispatch {
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_flow_states,
            },
            None,
        );
    }
    // Distinct predecessor states can prove the same source command unsafe.
    // The proof multiplicity is internal; the editor should receive one
    // diagnostic per code/range/message.
    let mut seen = HashSet::new();
    out.retain(|warning| {
        seen.insert((
            warning.code,
            warning.span.start(),
            warning.span.end(),
            warning.message.clone(),
        ))
    });
    out
}

/// Deduplicate flow states by `(responded, respond_at)`.
fn dedupe_flow_states(states: Vec<HttpFlowState>) -> Vec<HttpFlowState> {
    let mut seen: HashSet<(bool, i64)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        let key = (s.responded, s.respond_at.map_or(-1, i64::from));
        if seen.insert(key) {
            out.push(s);
        }
    }
    out
}

/// The command word + span a statement applies to the response flow, or `None`.
/// Direct `Call`/`Barrier` heads, plus a whole-value `[cmd …]` command
/// substitution assigned by `set`.
fn http_flow_stmt_command(stmt: &Statement) -> Option<(&str, Span)> {
    match stmt {
        Statement::Call { command, span, .. } | Statement::Barrier { command, span, .. } => {
            Some((command.as_str(), *span))
        }
        Statement::AssignValue { value, span, .. } => {
            let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
            inner.split_whitespace().next().map(|cmd| (cmd, *span))
        }
        _ => None,
    }
}

/// Apply one command to a flow state, emitting IRULE1201/1202 as warranted.
fn apply_http_flow_command(
    state: HttpFlowState,
    cmd: &str,
    span: Span,
    event: &str,
    out: &mut Vec<IrulesCheckWarning>,
) -> HttpFlowState {
    if commits_response_commands().contains(cmd) {
        if state.responded {
            out.push(IrulesCheckWarning {
                span,
                code: DiagCode::Irule1202,
                message: format!(
                    "Multiple '{cmd}' calls possible in {event}. Only the first response takes effect.",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
            return state;
        }
        return HttpFlowState {
            responded: true,
            respond_at: Some(span.start()),
        };
    }
    if state.responded && requires_http_context_commands().contains(cmd) {
        out.push(IrulesCheckWarning {
            span,
            code: DiagCode::Irule1201,
            message: format!(
                "'{cmd}' used after response is committed. HTTP context is invalid after HTTP::respond/HTTP::redirect.",
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
    state
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
    out: &'a mut Vec<IrulesCheckWarning>,
    leaf: &'a L,
    dedupe: &'a D,
}

fn walk_flow<S, L, D>(
    script: &Script,
    in_states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let mut states = in_states.to_vec();
    for stmt in &script.statements {
        // `None` ⇒ the statement terminates every current path (`return`).
        match flow_step(stmt, &states, fd, return_sink.as_deref_mut()) {
            None => {
                // Hand the at-return states to the enclosing catch (if any) so
                // flow resumes past it, then terminate this script's path.
                if let Some(sink) = return_sink {
                    sink.extend(states);
                }
                return Vec::new();
            }
            Some(next) => states = next,
        }
    }
    states
}

/// Transfer one structured statement over the flow-state set, or `None` when
/// it (`return`) terminates the current paths.  Control-flow arms recurse
/// through [`walk_flow`]; leaf statements go through `leaf`.
fn flow_step<S, L, D>(
    stmt: &Statement,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Option<Vec<S>>
where
    S: Clone,
    L: Fn(&S, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    Some(match stmt {
        Statement::Return { .. } => return None,
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut next = Vec::new();
            for st in states {
                for clause in clauses {
                    next.extend(walk_flow(
                        &clause.body,
                        one(st),
                        fd,
                        return_sink.as_deref_mut(),
                    ));
                }
                if let Some(eb) = else_body {
                    next.extend(walk_flow(eb, one(st), fd, return_sink.as_deref_mut()));
                } else {
                    // condition-false path keeps the incoming state.
                    next.push(st.clone());
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Switch {
            arms, default_body, ..
        } => flow_step_switch(arms, default_body.as_ref(), states, fd, return_sink),
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                next.push(st.clone()); // zero iterations
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
            (fd.dedupe)(next)
        }
        Statement::Catch { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                // `catch` swallows TCL_RETURN: both the body's normal exits and
                // any at-return states continue past the catch.  Fall back to
                // the incoming state if the body yields nothing.
                let mut catch_returns = Vec::new();
                let mut body_out = walk_flow(body, one(st), fd, Some(&mut catch_returns));
                body_out.append(&mut catch_returns);
                if body_out.is_empty() {
                    next.push(st.clone());
                } else {
                    next.extend(body_out);
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => flow_step_try(
            body,
            handlers,
            finally_body.as_ref(),
            states,
            fd,
            return_sink,
        ),
        // A grouping block (namespace/uplevel body) runs in sequence.
        Statement::Block { body, .. } => walk_flow(body, states, fd, return_sink),
        _ => {
            let mut next = Vec::with_capacity(states.len());
            for st in states {
                next.push((fd.leaf)(st, stmt, fd.out));
            }
            (fd.dedupe)(next)
        }
    })
}

/// The `switch` arm of [`flow_step`]: every arm body (and the default body) is
/// a feasible path, plus the no-match fall-through that keeps the incoming
/// state.
fn flow_step_switch<S, L, D>(
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        // no-match fall-through path.
        next.push(st.clone());
        for arm in arms {
            if let Some(body) = &arm.body {
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
        }
        if let Some(db) = default_body {
            next.extend(walk_flow(db, one(st), fd, return_sink.as_deref_mut()));
        }
    }
    (fd.dedupe)(next)
}

/// The `try` arm of [`flow_step`]: the body and each handler are feasible
/// paths; a `finally` body runs in sequence after each; an empty result keeps
/// the incoming state.
fn flow_step_try<S, L, D>(
    body: &Script,
    handlers: &[crate::ir::TryHandler],
    finally_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        let mut branch = walk_flow(body, one(st), fd, return_sink.as_deref_mut());
        for handler in handlers {
            branch.extend(walk_flow(
                &handler.body,
                one(st),
                fd,
                return_sink.as_deref_mut(),
            ));
        }
        if let Some(fb) = finally_body {
            let mut final_out = Vec::new();
            for mid in &branch {
                final_out.extend(walk_flow(fb, one(mid), fd, return_sink.as_deref_mut()));
            }
            branch = final_out;
        }
        if branch.is_empty() {
            next.push(st.clone());
        } else {
            next.extend(branch);
        }
    }
    (fd.dedupe)(next)
}

// IRULE4004 — `set var value` in per-request event hoistable to once-per-connection
//
// The event registry classifies per-request and once-per-connection events.
// An `AssignConst` whose value does not depend on per-request data could be
// hoisted to a once-per-connection event, saving work per request.
//
// A warning is only safe when the literal scalar assignment is unconditional
// and is the variable's sole write across the rule. Repeated initialisation is
// commonly a deliberate per-request reset; moving it changes connection-state
// semantics after the first request. Array elements and namespace/static
// variables likewise need a different destination and are excluded.

/// Hoistable-set warnings for IRULE4004.
#[must_use]
pub fn find_hoistable_set_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let events = EventRegistry::build();
    let mut write_counts = HashMap::<String, usize>::new();
    // Count every lowered event, including one whose deeper analyses hit the
    // complexity guard. A skipped handler is not evidence that no second write
    // exists there.
    for fu in cu.functions() {
        if !fu.name.starts_with("::when::") {
            continue;
        }
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                match stmt {
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => {
                        *write_counts.entry(name.clone()).or_default() += 1;
                    }
                    Statement::Call { defs, .. } => {
                        for name in defs {
                            *write_counts.entry(name.clone()).or_default() += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    for fu in cu.analysable_functions() {
        let Some(event) = fu.name.strip_prefix("::when::") else {
            continue;
        };
        let event = event.split('#').next().unwrap_or(event);
        if !events.is_per_request(event) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&fu.cfg.entry) else {
            continue;
        };
        for stmt in &block.statements {
            let (name, value, span) = match stmt {
                Statement::AssignConst {
                    name, value, span, ..
                }
                | Statement::AssignValue {
                    name, value, span, ..
                } => (name, value, *span),
                _ => continue,
            };
            if name.is_empty()
                || value.is_empty()
                || name.contains('(')
                || name.contains("::")
                || write_counts.get(name) != Some(&1)
            {
                continue;
            }
            // Skip dynamic values — `$x` / `[cmd]` interpolation.
            if value.contains('$') || value.contains('[') {
                continue;
            }
            out.push(IrulesCheckWarning {
                span: fu.abs_span(span),
                code: DiagCode::Irule4004,
                message: format!(
                    "`set {name} ...` runs on every request — consider hoisting to a once-per-connection event.",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
    out
}

// IRULE4002 — generic `static::` variable name that will collide
//
// Walks every `::when::` CFG body for statements that define a
// `static::` variable and flags bare names matching a set of generic
// regex patterns. Deduplicated across events — the first occurrence's
// span wins. The pattern set is configurable (`tclLsp.diagnostics.
// genericVariablePatterns`): pass `None` for the built-in default set,
// `Some(&[])` to disable the check, or `Some(custom)` to replace the
// defaults.

/// Default generic `static::` variable-name patterns. Each is matched case-insensitively
/// (the bare name is lowercased) against the part after the `static::`
/// prefix. A user-supplied list replaces this set wholesale.
pub const DEFAULT_GENERIC_VARIABLE_PATTERNS: &[&str] = &[
    // Debug / logging
    r"^debug(_level|_enabled)?$",
    r"^dbg$",
    r"^log_(level|server|enabled)$",
    r"^logging$",
    r"^verbose$",
    r"^trace$",
    // Configuration
    r"^(response_)?timeout$",
    r"^(max_)?retr(y|ies)$",
    r"^config$",
    r"^(enabled|disabled|active)$",
    r"^mode$",
    r"^(port|host|server|pool)$",
    // Counters / limits
    r"^count(er)?$",
    r"^(limit|max_connections|threshold|rate|interval)$",
    // Generic state
    r"^(flag|level|status|state|version|name|value|data|result|test|init|default)$",
];

/// Compile the generic-name patterns for IRULE4002. `None` selects the
/// built-in [`DEFAULT_GENERIC_VARIABLE_PATTERNS`]; `Some(custom)` uses the
/// caller's list verbatim (an empty slice compiles to no patterns, which
/// disables the check). Invalid regexes are skipped.
fn compile_generic_patterns(patterns: Option<&[String]>) -> Vec<regex::Regex> {
    let build = |src: &str| {
        regex::RegexBuilder::new(src)
            .case_insensitive(true)
            .build()
            .ok()
    };
    match patterns {
        None => DEFAULT_GENERIC_VARIABLE_PATTERNS
            .iter()
            .filter_map(|p| build(p))
            .collect(),
        Some(pats) => pats.iter().filter_map(|p| build(p)).collect(),
    }
}

/// `is_generic_static_name` — `var_name` (with the `static::` prefix) is
/// generic when its lowercased bare name matches any `compiled` pattern.
fn is_generic_static_name(var_name: &str, compiled: &[regex::Regex]) -> bool {
    let bare = var_name
        .strip_prefix("static::")
        .unwrap_or(var_name)
        .to_ascii_lowercase();
    compiled.iter().any(|re| re.is_match(&bare))
}

/// Generic-static-name warnings for IRULE4002. `generic_patterns` follows the
/// [`compile_generic_patterns`] convention (`None` → default set, `Some(&[])`
/// → disabled, `Some(custom)` → replace defaults).
#[must_use]
pub fn find_generic_static_name_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
    generic_patterns: Option<&[String]>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let compiled = compile_generic_patterns(generic_patterns);
    if compiled.is_empty() {
        return out;
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for fu in cu.analysable_functions() {
        if !fu.name.starts_with("::when::") {
            continue;
        }
        for bn in cfg_order(&fu.cfg) {
            let Some(block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            for stmt in &block.statements {
                let span = stmt.span();
                match stmt {
                    Statement::AssignConst { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => {
                        check_generic_static(name, span, &compiled, &mut seen, &mut out);
                    }
                    Statement::Call { defs, .. } => {
                        for name in defs {
                            check_generic_static(name, span, &compiled, &mut seen, &mut out);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

fn check_generic_static(
    var_name: &str,
    span: Span,
    compiled: &[regex::Regex],
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    if !var_name.starts_with("static::") {
        return;
    }
    if seen.contains(var_name) {
        return;
    }
    if !is_generic_static_name(var_name, compiled) {
        return;
    }
    seen.insert(var_name.to_owned());
    let bare = var_name.strip_prefix("static::").unwrap_or(var_name);
    out.push(IrulesCheckWarning {
        span,
        code: DiagCode::Irule4002,
        message: format!(
            "'{var_name}' is a generic name that will collide with other iRules. \
             static:: variables are shared across every iRule on the BIG-IP system — \
             prefix with the application or rule name (e.g. 'static::<app>_{bare}')."
        ),
        replacement: None,
        fixes: Vec::new(),
    });
}

// Tests

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::*;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    fn warnings_for_irules(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_unnormalised_getter_warnings(
            &cu,
            &registry(),
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
    }

    #[test]
    fn irule3102_warns_on_bare_http_uri_getter() {
        let w = warnings_for_irules("set u [HTTP::uri]");
        assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
        assert_eq!(w[0].code, DiagCode::Irule3102);
        assert!(w[0].message.contains("HTTP::uri -normalized"));
    }

    #[test]
    fn irule3102_clean_when_normalized_flag_present() {
        let w = warnings_for_irules("set u [HTTP::uri -normalized]");
        assert!(w.is_empty(), "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_setter_form_not_flagged() {
        // `HTTP::path /x` — first arg `/x` is non-flag → setter form → no warning.
        let w = warnings_for_irules("HTTP::path /x");
        assert!(w.is_empty(), "expected no IRULE3102 on setter, got {w:?}");
    }

    #[test]
    fn irule3102_warns_on_getter_nested_in_sink_argument() {
        // `HTTP::respond 200 content [HTTP::uri]` — the unnormalised
        // `[HTTP::uri]` getter is a command-substitution *argument* of the
        // HTTP sink, not a top-level call/assignment, but must still fire.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri]\n}\n",
        );
        assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
        assert_eq!(w[0].code, DiagCode::Irule3102);
        assert!(w[0].message.contains("HTTP::uri -normalized"));
    }

    #[test]
    fn irule3102_silent_for_constant_sink_argument() {
        // A constant fed into the same sink stays silent.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content \"static body\"\n}\n",
        );
        assert!(w.is_empty(), "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_silent_for_normalized_getter_in_sink_argument() {
        // The nested getter with `-normalized` is canonicalised → silent.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri -normalized]\n}\n",
        );
        assert!(w.is_empty(), "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_non_irules_dialect_returns_empty() {
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry(), false);
        assert!(find_unnormalised_getter_warnings(&cu, &registry(), None).is_empty());
        assert!(
            find_unnormalised_getter_warnings(
                &cu,
                &registry(),
                Some(SurfaceQuery::any_release(Family::Tcl))
            )
            .is_empty()
        );
    }

    #[test]
    fn irule3102_also_fires_inside_assign_value_cmd_sub() {
        // Plain `set u [HTTP::query]` — command-sub form, must still fire.
        let w = warnings_for_irules("set u [HTTP::query]");
        assert_eq!(w.len(), 1, "expected IRULE3102 for HTTP::query, got {w:?}");
        assert!(w[0].message.contains("HTTP::query"));
    }

    #[test]
    fn irule3102_fires_on_http_path_without_flag() {
        let w = warnings_for_irules("set u [HTTP::path]");
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("HTTP::path"));
    }

    #[test]
    fn is_getter_form_recognises_flags_only() {
        assert!(is_getter_form(&[]));
        assert!(is_getter_form(&["-normalized".to_owned()]));
        assert!(!is_getter_form(&["/foo".to_owned()]));
        assert!(!is_getter_form(&[
            "foo".to_owned(),
            "-normalized".to_owned()
        ]));
    }

    /// IRULE3102 must derive its command set from the
    /// registry's `OptionSpec` table (not a hardcoded list). When
    /// the registry-side option is present, the check fires; when
    /// the registered command does not declare `-normalized`, no
    /// diagnostic is produced.
    #[test]
    fn arch3_normalised_option_is_registry_driven() {
        let registry = registry();
        // Registry-side: HTTP::uri / HTTP::path / HTTP::query carry
        // the `-normalized` option in their command spec.
        for name in ["HTTP::uri", "HTTP::path", "HTTP::query"] {
            assert!(
                supports_normalized_flag(&registry, name),
                "{name} should declare -normalized in its registry OptionSpec",
            );
        }
        // A registered iRules command without `-normalized` is not a
        // candidate (e.g. `HTTP::header`, which has no
        // `-normalized` option).
        assert!(
            !supports_normalized_flag(&registry, "HTTP::header"),
            "HTTP::header has no -normalized option in the registry",
        );

        // End-to-end proof: HTTP::uri without -normalized fires
        // IRULE3102; HTTP::header (no -normalized in registry) does not.
        let with_uri = warnings_for_irules("set u [HTTP::uri]");
        assert!(
            with_uri.iter().any(|w| w.code == DiagCode::Irule3102),
            "expected IRULE3102 on HTTP::uri, got {with_uri:?}",
        );
        let with_header = warnings_for_irules("set u [HTTP::header Content-Type]");
        assert!(
            with_header.is_empty(),
            "no IRULE3102 expected on HTTP::header (no -normalized), got {with_header:?}",
        );
    }

    // IRULE5002 / IRULE5004

    fn drop_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_unguarded_drop_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)))
    }

    #[test]
    fn irule5002_drop_without_return_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "expected IRULE5002, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_return_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; return }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected when `return` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_event_disable_all_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; event disable all }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected when `event disable all` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_reject_also_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { reject }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "expected IRULE5002 on `reject`, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { drop }", &registry(), false);
        let none_dialect = find_unguarded_drop_warnings(&cu, None);
        assert!(none_dialect.is_empty(), "got {none_dialect:?}");
        let tcl_dialect =
            find_unguarded_drop_warnings(&cu, Some(SurfaceQuery::any_release(Family::Tcl)));
        assert!(tcl_dialect.is_empty(), "got {tcl_dialect:?}");
    }

    #[test]
    fn irule5004_dns_return_without_return_fires() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5004),
            "expected IRULE5004, got {ws:?}",
        );
    }

    #[test]
    fn irule5004_dns_return_followed_by_return_clean() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return; return }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5004),
            "no IRULE5004 expected when `return` follows `DNS::return`, got {ws:?}",
        );
    }

    #[test]
    fn no_drop_warnings_for_clean_when_body() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { log local0. \"connection open\" }");
        assert!(ws.is_empty(), "got {ws:?}");
    }

    fn drop_codes(source: &str) -> Vec<String> {
        let mut c: Vec<String> = drop_warnings(source)
            .iter()
            .map(|w| w.code.to_string())
            .collect();
        c.sort();
        c
    }

    #[test]
    fn irule5002_fires_when_only_one_branch_guarded() {
        // The `drop` path has no following `event disable all` / `return`, so
        // it is unguarded — the path-sensitive walk keeps that path's state and
        // flags it even though the other branch returns.
        assert!(
            drop_codes("when CLIENT_ACCEPTED { if {$x} { drop } else { return } }")
                .contains(&"IRULE5002".to_string())
        );
    }

    #[test]
    fn irule5002_quiet_when_every_branch_guarded() {
        // Both branches guard their drop with `return`, so no surviving path
        // is left unguarded.
        assert!(
            drop_codes("when CLIENT_ACCEPTED { if {$x} { drop; return } else { reject; return } }")
                .is_empty()
        );
    }

    #[test]
    fn irule5002_drop_points_at_the_drop_command() {
        // The diagnostic span must land on the `drop` word itself (offset 23
        // in this source), not past it — the structured-IR walk carries the
        // command's absolute span.
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5002)
            .expect("IRULE5002");
        assert_eq!(w.span.start(), 23, "span should point at `drop`");
    }

    #[test]
    fn irule5002_carries_insertion_fix() {
        // The quick-fix inserts `event disable all` + `return` *after* the drop
        // (a zero-width edit anchored at the drop command's end), independent of
        // the diagnostic span.
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5002)
            .expect("IRULE5002");
        assert_eq!(w.fixes.len(), 1, "expected one fix, got {:?}", w.fixes);
        let fix = &w.fixes[0];
        assert_eq!(fix.new_text, "\n    event disable all\n    return");
        assert_eq!(fix.description, "Add 'event disable all' + 'return'");
        // Zero-width insertion at the drop command's end.
        assert_eq!(fix.span.start(), w.span.end());
        assert_eq!(fix.span.start(), fix.span.end());
    }

    #[test]
    fn irule5004_carries_insertion_fix() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5004)
            .expect("IRULE5004");
        assert_eq!(w.fixes.len(), 1);
        let fix = &w.fixes[0];
        assert_eq!(fix.new_text, "\n    return");
        assert_eq!(fix.description, "Add 'return' after DNS::return");
        assert_eq!(fix.span.start(), fix.span.end());
    }

    #[test]
    fn irule5004_fires_through_catch_body() {
        // A `DNS::return` inside `catch { … }` still leaves the path needing a
        // following `return` (the linear scan missed the catch body).
        assert!(
            drop_codes("when DNS_REQUEST { catch { DNS::return } }")
                .contains(&"IRULE5004".to_string())
        );
    }

    #[test]
    fn irule5002_drop_and_return_inside_catch_still_warns() {
        // `catch` swallows the `return`, so it does NOT stop iRule processing —
        // the unguarded `drop` still leaks past the catch and must be flagged.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop; return }\n}");
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule5002)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE5002, got {ws:?}");
        assert!(
            hits[0].message.contains("drop"),
            "expected the unguarded drop to be flagged, got {:?}",
            hits[0].message,
        );
    }

    #[test]
    fn irule5002_catch_body_without_return_still_warns_drop() {
        // A catch whose body does not return continues normally and the
        // unguarded drop still leaks out.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop }\n}");
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule5002).count(),
            1,
            "expected one IRULE5002, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_catch_without_drop_no_warning() {
        // A benign catch body must not spuriously warn.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { set x 1; return }\n}");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected for a benign catch, got {ws:?}",
        );
    }

    #[test]
    fn irule5004_dns_return_and_return_inside_catch_still_warns() {
        // `catch` swallows the `return` so it does not stop processing — the
        // unguarded `DNS::return` still leaks out.
        let ws = drop_warnings("when DNS_REQUEST {\n    catch { DNS::return; return }\n}");
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule5004)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE5004, got {ws:?}");
        assert!(
            hits[0].message.contains("DNS::return"),
            "expected the unguarded DNS::return to be flagged, got {:?}",
            hits[0].message,
        );
    }

    // IRULE1005 / 1006 / 1007 / 1008

    fn flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        find_collect_flow_warnings(&cu, &reg, Some(SurfaceQuery::any_release(Family::F5Irules)))
    }

    #[test]
    fn irule1005_http_data_event_without_collect_fires() {
        // HTTP_REQUEST_DATA has no transport alternative: HTTP::collect is
        // required for it to fire.
        let ws = flow_warnings("when HTTP_REQUEST_DATA { log local0. \"data\" }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "expected IRULE1005, got {ws:?}",
        );
    }

    #[test]
    fn irule1005_satisfied_by_matching_collect() {
        // HTTP_REQUEST issues a client-side HTTP::collect, satisfying the
        // HTTP_REQUEST_DATA requirement.
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { log local0. \"data\" }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "no IRULE1005 expected — CLIENT_ACCEPTED supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn irule1006_payload_without_collect_fires() {
        let ws = flow_warnings("when HTTP_REQUEST { HTTP::payload }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1006),
            "expected IRULE1006 on HTTP::payload without HTTP::collect, got {ws:?}",
        );
    }

    #[test]
    fn udp_and_asm_payloads_do_not_require_collect() {
        // F5 exposes every UDP datagram to UDP::payload, and ASM owns the
        // payload in ASM events. Neither protocol has a corresponding
        // `::collect` command, so suffix-based classification was a false
        // positive for both.
        let udp = flow_warnings("when CLIENT_DATA { UDP::payload }");
        assert!(
            !udp.iter()
                .any(|w| w.code == DiagCode::Irule1005 || w.code == DiagCode::Irule1006),
            "UDP data is available without collect, got {udp:?}",
        );
        let asm = flow_warnings("when ASM_REQUEST_VIOLATION { ASM::payload }");
        assert!(
            !asm.iter().any(|w| w.code == DiagCode::Irule1006),
            "ASM owns its payload, got {asm:?}",
        );
    }

    #[test]
    fn mqtt_payload_call_forms_have_distinct_collection_requirements() {
        let bare = flow_warnings("when MQTT_CLIENT_DATA { set bytes [MQTT::payload] }");
        assert!(
            bare.iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "bare MQTT payload requires a prior MQTT::collect, got {bare:?}",
        );

        let append = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload append bytes }");
        assert!(
            append
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "MQTT payload append requires collected bytes, got {append:?}",
        );

        for form in ["length", "replace bytes", "prepend bytes"] {
            let source = format!("when MQTT_CLIENT_INGRESS {{ MQTT::payload {form} }}");
            let warnings = flow_warnings(&source);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "MQTT payload {form} uses the current message without collect, got {warnings:?}",
            );
        }

        let dynamic = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload $operation bytes }");
        assert!(
            !dynamic
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "a dynamic MQTT payload form must abstain, got {dynamic:?}",
        );
    }

    #[test]
    fn mqtt_collection_lifecycle_balances_across_registry_events() {
        let warnings = flow_warnings(
            "when MQTT_CLIENT_INGRESS { MQTT::collect 200 }
             when MQTT_CLIENT_DATA { set bytes [MQTT::payload]; MQTT::release }",
        );
        assert!(
            !warnings.iter().any(|warning| matches!(
                warning.code,
                DiagCode::Irule1005
                    | DiagCode::Irule1006
                    | DiagCode::Irule1007
                    | DiagCode::Irule1008
            )),
            "balanced MQTT collection should be clean, got {warnings:?}",
        );
    }

    #[test]
    fn every_explicit_payload_family_warns_without_collect() {
        for (protocol, event) in [
            ("MR", "MR_DATA"),
            ("RTSP", "RTSP_REQUEST_DATA"),
            ("SCTP", "CLIENT_DATA"),
            ("WS", "WS_CLIENT_DATA"),
        ] {
            let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
            let warnings = flow_warnings(&source);
            assert!(
                warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "{protocol} payload without collect must warn, got {warnings:?}",
            );
        }
    }

    #[test]
    fn event_owned_payload_families_do_not_invent_collect_lifecycles() {
        for (protocol, event) in [
            ("CACHE", "CACHE_RESPONSE"),
            ("DIAMETER", "DIAMETER_INGRESS"),
            ("GTP", "GTP_SIGNALLING_INGRESS"),
            ("REWRITE", "REWRITE_REQUEST_DONE"),
            ("SIP", "SIP_REQUEST"),
            ("XML", "XML_CONTENT_BASED_ROUTING"),
        ] {
            let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
            let warnings = flow_warnings(&source);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "{protocol} payload is event-owned, got {warnings:?}",
            );
        }
    }

    #[test]
    fn irule1007_collect_without_release_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::collect }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "expected IRULE1007, got {ws:?}",
        );
    }

    #[test]
    fn irule1008_release_without_collect_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::release }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1008),
            "expected IRULE1008, got {ws:?}",
        );
    }

    #[test]
    fn irule1007_satisfied_by_matching_release_same_side() {
        let ws = flow_warnings(
            "when CLIENT_ACCEPTED { TCP::collect }
             when CLIENT_DATA { TCP::release }",
        );
        // Both sides — same side — should NOT fire 1007 or 1008.
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "1007 unexpected, got {ws:?}",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1008),
            "1008 unexpected, got {ws:?}",
        );
    }

    #[test]
    fn http_data_event_implicitly_releases_collection() {
        // F5 releases HTTP collection when the matching DATA event completes
        // unless another HTTP::collect starts. An explicit HTTP::release here
        // is optional, unlike TCP and SSL.
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { set payload [HTTP::payload] }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "HTTP data event supplies the release, got {ws:?}",
        );
    }

    #[test]
    fn http_response_data_implicitly_releases_server_side_collection() {
        let ws = flow_warnings(
            "when HTTP_RESPONSE { HTTP::collect }
             when HTTP_RESPONSE_DATA { set payload [HTTP::payload] }",
        );
        assert!(
            !ws.iter().any(|warning| matches!(
                warning.code,
                DiagCode::Irule1005 | DiagCode::Irule1006 | DiagCode::Irule1007
            )),
            "registry side and implicit release should satisfy response collection, got {ws:?}",
        );
    }

    #[test]
    fn recollect_inside_http_data_event_still_needs_a_later_release() {
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { HTTP::collect }",
        );
        let hits = ws
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule1007)
            .count();
        assert_eq!(
            hits, 1,
            "the setup collect is implicit-release safe, but the DATA-event recollect is not: {ws:?}",
        );
    }

    // -- side-switch body descent + peer flip -----

    #[test]
    fn collect_inside_clientside_body_is_seen() {
        // The collect lives inside a `clientside { ... }` nesting script.
        // With the BODY role set, the flow check descends into the body
        // and registers the (client-side) collect, so CLIENTSSL_DATA's
        // IRULE1005 requirement is satisfied.  Without the descent the
        // collect was invisible and CLIENTSSL_DATA wrongly fired IRULE1005.
        let ws = flow_warnings(
            "when CLIENTSSL_HANDSHAKE { clientside { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"data\" }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "no IRULE1005 expected — the clientside body supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn peer_flips_side_for_collect_flow() {
        // `peer { SSL::collect }` evaluates under the *opposite* side of
        // the surrounding client-side event. It therefore satisfies
        // SERVERSSL_DATA but not CLIENTSSL_DATA — the inverse of a plain
        // client-side collection.
        let ws = flow_warnings(
            "when CLIENTSSL_HANDSHAKE { peer { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"c\" }
             when SERVERSSL_DATA { log local0. \"s\" }",
        );
        let irule1005: Vec<&str> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule1005)
            .map(|w| w.message.as_str())
            .collect();
        assert_eq!(
            irule1005.len(),
            1,
            "expected exactly one IRULE1005 (CLIENTSSL_DATA only), got {ws:?}",
        );
        assert!(
            irule1005[0].contains("CLIENTSSL_DATA"),
            "the surviving IRULE1005 must be CLIENTSSL_DATA (peer flipped the collect to the server side), got {irule1005:?}",
        );
    }

    #[test]
    fn nested_side_switch_warning_keeps_its_absolute_source_span() {
        let source = "when CLIENT_ACCEPTED {\n  serverside {\n    TCP::payload\n  }\n}";
        let warning = flow_warnings(source)
            .into_iter()
            .find(|warning| warning.code == DiagCode::Irule1006)
            .expect("server-side payload without collect");
        let start = usize::try_from(warning.span.start()).unwrap();
        let end = usize::try_from(warning.span.end()).unwrap();
        assert_eq!(&source[start..end], "TCP::payload");
    }

    #[test]
    fn side_switch_specs_have_body_role_and_arity() {
        // Registry-level guard for the spec edits.
        let r = registry();
        for cmd in ["clientside", "serverside", "peer"] {
            assert!(r.is_side_switch(cmd), "{cmd} must be a side-switch");
            assert!(
                r.side_switch_target(cmd).is_some(),
                "{cmd} must declare its side-switch target",
            );
            assert_eq!(
                r.arg_indices_for_role(cmd, &["{ TCP::collect }"], tcl_registry::ArgRole::Body),
                vec![0],
                "{cmd} body script must be ArgRole::Body at index 0",
            );
        }
        // clientside/serverside accept the bare query form or one body;
        // peer requires its body; after's timer body is the trailing arg.
        let cs = r.get("clientside").unwrap().arity;
        assert_eq!((cs.min, cs.max), (0, 1), "clientside arity");
        let ss = r.get("serverside").unwrap().arity;
        assert_eq!((ss.min, ss.max), (0, 1), "serverside arity");
        let pr = r.get("peer").unwrap().arity;
        assert_eq!((pr.min, pr.max), (1, 1), "peer arity");
        // `after`'s deferred timer body runs in its own dispatch context.
        assert_eq!(
            r.get("after").unwrap().body_kind,
            tcl_registry::BodyKind::Structural,
            "after timer body must be Structural",
        );
    }

    #[test]
    fn collect_flow_only_in_irules_dialect() {
        let reg = registry();
        let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { TCP::collect }", &reg, false);
        let none = find_collect_flow_warnings(&cu, &reg, None);
        assert!(none.is_empty(), "got {none:?}");
    }

    // IRULE1201 / 1202

    fn http_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_http_flow_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)))
    }

    #[test]
    fn irule1202_double_respond_fires() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content x; HTTP::respond 404 content y }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1202),
            "expected IRULE1202, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_http_after_respond_fires() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "expected IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_uses_the_event_profile_instead_of_its_name() {
        // FN regression: ASM_RESPONSE_VIOLATION does not start with `HTTP`,
        // but its event descriptor implies the HTTP profile.
        let ws = http_warnings(
            "when ASM_RESPONSE_VIOLATION { HTTP::respond 200 content ok; HTTP::header insert X-Test yes }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "expected profile-driven IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_allows_the_registered_response_state_query() {
        // FP regression: checking whether a response has been sent is the
        // purpose of HTTP::has_responded, so its spec does not require the
        // still-live transaction context.
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; set sent [HTTP::has_responded] }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "HTTP::has_responded must remain valid after commit, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_clean_when_respond_followed_by_return_only() {
        let ws = http_warnings("when HTTP_REQUEST { HTTP::respond 200 content ok; return }");
        assert!(
            !ws.iter()
                .any(|w| w.code == DiagCode::Irule1201 || w.code == DiagCode::Irule1202),
            "no IRULE1201/1202 expected, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_respond_then_return_inside_catch_warns() {
        // `catch` swallows the `return`, so control continues past the catch
        // with the response already committed — the post-catch HTTP::header
        // insert must still be flagged, and the at-catch-return `responded`
        // state must propagate out.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok; return }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
        );
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule1201)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE1201, got {ws:?}");
        assert!(
            hits[0].message.contains("HTTP::header"),
            "expected the post-catch HTTP::header to be flagged, got {:?}",
            hits[0].message,
        );
    }

    #[test]
    fn irule1201_catch_body_without_return_still_continues() {
        // A catch whose body does not return continues normally, carrying the
        // responded state to flag the post-catch command.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
        );
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule1201).count(),
            1,
            "expected one IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_catch_without_respond_no_warning() {
        // A benign catch body must not spuriously flag post-catch HTTP
        // commands.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { set x 1; return }\n\
            \x20   HTTP::header insert X-Debug 1\n\
            }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "no IRULE1201 expected for a benign catch, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_only_in_http_events() {
        // CLIENT_ACCEPTED is non-HTTP; HTTP::respond there is silly
        // but IRULE1201/1202 doesn't apply to non-HTTP events.
        let ws = http_warnings(
            "when CLIENT_ACCEPTED { HTTP::respond 200 content x; HTTP::respond 404 content y }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1202),
            "IRULE1202 should not fire outside HTTP events, got {ws:?}",
        );
    }

    fn http_codes(source: &str) -> Vec<String> {
        let mut c: Vec<String> = http_warnings(source)
            .iter()
            .map(|w| w.code.to_string())
            .collect();
        c.sort();
        c
    }

    #[test]
    fn irule1202_quiet_on_mutually_exclusive_branches() {
        // A respond in each arm of an if/else is NOT a double-respond — only
        // one executes per path.  The path-sensitive walk keeps the branch
        // states separate (the linear scan wrongly fired IRULE1202 here).
        assert!(
            http_codes(
                "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b } }"
            )
            .is_empty()
        );
        // Same for mutually-exclusive switch arms.
        assert!(
            http_codes(
                "when HTTP_REQUEST { switch $x { a { HTTP::respond 200 content a } \
             b { HTTP::respond 404 content b } } }"
            )
            .is_empty()
        );
    }

    #[test]
    fn irule1202_reports_one_diagnostic_for_multiple_predecessor_proofs() {
        let ws = http_warnings(
            "when HTTP_REQUEST {\
             if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b }; \
             HTTP::respond 500 content c \
             }",
        );
        let final_respond: Vec<_> = ws
            .iter()
            .filter(|warning| {
                warning.code == DiagCode::Irule1202
                    && warning.message.contains("Multiple 'HTTP::respond'")
            })
            .collect();
        assert_eq!(
            final_respond.len(),
            1,
            "one source command must produce one diagnostic, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_quiet_when_respond_and_use_are_exclusive() {
        // respond in one branch, an HTTP command in the *other* — no path
        // both responds and then uses HTTP, so nothing fires.
        assert!(
            http_codes(
                "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::header insert Foo bar } }"
            )
            .is_empty()
        );
    }

    #[test]
    fn irule1201_fires_through_catch_body() {
        // A respond inside `catch { … }` still commits the response, so a
        // following HTTP command is flagged (the linear scan missed this).
        assert!(http_codes(
            "when HTTP_REQUEST { catch { HTTP::respond 200 content a }; HTTP::header insert Foo bar }"
        )
        .contains(&"IRULE1201".to_string()));
    }

    #[test]
    fn irule1201_quiet_on_unregistered_http_subcommand() {
        // `HTTP::log` is not a registered HTTP:: command, so it must not trip
        // IRULE1201 (it carries no registry context trait).
        assert!(
            !http_codes("when HTTP_REQUEST { HTTP::respond 200 content a; HTTP::log foo }")
                .contains(&"IRULE1201".to_string())
        );
    }

    #[test]
    fn irule1202_fires_after_loop_respond() {
        // A respond in a loop body, then a respond after the loop: when the
        // loop runs at least once, the second respond is a double-respond.
        assert!(
            http_codes(
                "when HTTP_REQUEST { foreach i $list { HTTP::respond 200 content a }; \
             HTTP::respond 404 content b }"
            )
            .contains(&"IRULE1202".to_string())
        );
    }

    #[test]
    fn bare_redirect_commits_response() {
        // The bare iRules `redirect` (not just `HTTP::redirect`) commits a
        // response — derived from the registry's ResponseCommit side-effect.
        assert!(commits_response_commands().contains("redirect"));
        assert!(commits_response_commands().contains("HTTP::respond"));
    }

    // IRULE4004

    fn hoist_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)))
    }

    #[test]
    fn irule4004_literal_set_in_per_request_event_fires() {
        let ws = hoist_warnings(r#"when HTTP_REQUEST { set svc "foo" }"#);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected IRULE4004, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_registry_only_per_request_event_fires() {
        // TP: the registry's per-request classification is the sole oracle —
        // a compiler-local event list would not carry ASM_REQUEST_DONE.
        let ws = hoist_warnings(r#"when ASM_REQUEST_DONE { set svc "foo" }"#);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected registry-driven IRULE4004, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_dynamic_set_clean() {
        // Value depends on per-request data — can't hoist.
        let ws = hoist_warnings("when HTTP_REQUEST { set svc [HTTP::host] }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — value depends on request, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_var_substitution_clean() {
        let ws = hoist_warnings("when HTTP_REQUEST { set svc $session }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — value uses $session, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_per_request_reset_is_not_hoistable() {
        // FP regression from the third-party corpus: moving the initial zero
        // changes later requests after a branch sets the connection variable.
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set is_reject 0; if {[HTTP::path] eq \"/deny\"} { set is_reject 1 } }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a per-request reset must stay in the request event, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_conditional_literal_set_is_not_hoistable() {
        let ws = hoist_warnings(
            "when HTTP_REQUEST { if {[HTTP::path] eq \"/health\"} { set health_check 1 } }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "hoisting a conditional assignment would make it unconditional, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_cross_event_write_is_not_hoistable() {
        let ws =
            hoist_warnings("when HTTP_REQUEST { set state 0 }\nwhen HTTP_RESPONSE { set state 1 }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a connection variable written by another event is not invariant, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_counts_writes_in_complexity_guarded_events() {
        let padding = "x".repeat(crate::ssa::DEEP_ANALYSIS_BODY_BYTES);
        let source = format!(
            "when HTTP_REQUEST {{ set state 0 }}\n\
             when HTTP_RESPONSE {{ # {padding}\n set state 1 }}"
        );
        let cu = CompilationUnit::build_for(&source, &registry(), false);
        let response = cu
            .functions()
            .find(|fu| fu.name.starts_with("::when::HTTP_RESPONSE"))
            .expect("HTTP_RESPONSE event is lowered");
        assert!(
            response.complexity_guarded,
            "the control event must be guarded"
        );

        let ws =
            find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)));
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a guarded event's second write must suppress the hoist warning, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_array_and_static_destinations_abstain() {
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set state(current) 0; set static::feature_enabled 1 }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "array and static variables need different hoist destinations, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_set_in_once_per_connection_clean() {
        // FP control: the registry classifies CLIENT_ACCEPTED as
        // once-per-connection, so there is nothing to hoist.
        let ws = hoist_warnings(r#"when CLIENT_ACCEPTED { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — already once-per-connection, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_unknown_event_abstains() {
        // TN: an event absent from the registry is not guessed to be
        // per-request from its spelling.
        let ws = hoist_warnings(r#"when CUSTOM_REQUEST { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "unknown event should abstain, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            r#"when HTTP_REQUEST { set svc "foo" }"#,
            &registry(),
            false,
        );
        let none = find_hoistable_set_warnings(&cu, None);
        assert!(none.is_empty(), "got {none:?}");
    }

    // IRULE4002

    fn generic_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_generic_static_name_warnings(
            &cu,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            None,
        )
    }

    fn generic_warnings_with(source: &str, patterns: &[String]) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_generic_static_name_warnings(
            &cu,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            Some(patterns),
        )
    }

    #[test]
    fn irule4002_fires_for_generic_static_name() {
        let ws = generic_warnings("when RULE_INIT { set static::debug 1 }");
        assert!(
            ws.iter()
                .any(|w| w.code == DiagCode::Irule4002 && w.message.contains("'static::debug'")),
            "expected IRULE4002 for static::debug, got {ws:?}",
        );
    }

    #[test]
    fn irule4002_quiet_for_specific_static_name() {
        let ws = generic_warnings("when RULE_INIT { set static::myapp_cache_ttl 60 }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4002),
            "no IRULE4002 expected for a specific name, got {ws:?}",
        );
    }

    #[test]
    fn irule4002_deduplicates_across_events() {
        let src = "when RULE_INIT { set static::timeout 30 }\n\
                   when HTTP_REQUEST { set static::timeout 30 }";
        let ws = generic_warnings(src);
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule4002).count(),
            1,
            "expected a single deduplicated IRULE4002, got {ws:?}",
        );
    }

    #[test]
    fn irule4002_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            "when RULE_INIT { set static::debug 1 }",
            &registry(),
            false,
        );
        assert!(find_generic_static_name_warnings(&cu, None, None).is_empty());
    }

    #[test]
    fn is_generic_static_name_matches_default_set() {
        let compiled = compile_generic_patterns(None);
        assert!(is_generic_static_name("static::debug", &compiled));
        assert!(is_generic_static_name("static::DEBUG", &compiled));
        assert!(is_generic_static_name("static::max_retries", &compiled));
        assert!(!is_generic_static_name("static::myapp_token", &compiled));
        assert!(!is_generic_static_name("static::debugger", &compiled));
    }

    #[test]
    fn irule4002_custom_patterns_replace_defaults() {
        // A custom pattern set replaces the defaults wholesale: `myapp_token`
        // now fires, while the previously-generic `debug` goes quiet.
        let patterns = vec![r"^myapp_".to_owned()];
        let ws = generic_warnings_with("when RULE_INIT { set static::myapp_token 1 }", &patterns);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4002),
            "expected IRULE4002 for static::myapp_token under custom patterns, got {ws:?}",
        );
        let quiet = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &patterns);
        assert!(
            !quiet.iter().any(|w| w.code == DiagCode::Irule4002),
            "static::debug should be quiet once defaults are replaced, got {quiet:?}",
        );
    }

    #[test]
    fn irule4002_empty_patterns_disable_check() {
        // An explicit empty pattern list disables IRULE4002 entirely.
        let ws = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &[]);
        assert!(
            ws.iter().all(|w| w.code != DiagCode::Irule4002),
            "empty pattern list should disable IRULE4002, got {ws:?}",
        );
    }
}
