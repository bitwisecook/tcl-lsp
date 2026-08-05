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

use std::collections::HashSet;
use std::sync::OnceLock;
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;
use tcl_registry::side_effects::SideEffectTarget;

use crate::cfg_builder::build_cfg;
use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use crate::lowering::lower_to_ir_with_config;
use crate::sccp::cfg_order;
use crate::taint::is_irules_dialect;
use crate::value_shapes::parse_command_substitution;

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

/// Registered `HTTP::` namespace commands.
/// Only *registered* commands count, so an unregistered `HTTP::log` does not
/// trip IRULE1201 (the set of 32 registered commands).
fn http_namespace_commands() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        irules_registry()
            .command_names()
            .filter(|name| name.starts_with("HTTP::"))
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

/// Find IRULE3102 warnings across every function in `cu`.
///
/// Dialect-gated: returns an empty vector unless `dialect` is
/// `"f5-irules"` / `"irules"`.
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
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules_dialect(dialect) {
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
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules_dialect(dialect) {
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
// Cross-event analysis: classifies every `*::collect` / `*::release` /
// `*::payload` call across every `::when::*` proc body, then emits:
//
//   * IRULE1005 — `*_DATA` event handler exists but no matching
//     `*::collect` for the corresponding protocol.
//   * IRULE1006 — `*::payload` access without matching `*::collect`.
//   * IRULE1007 — `*::collect` without matching `*::release` on the
//     same connection side.
//   * IRULE1008 — `*::release` without matching `*::collect` on the
//     same connection side.
//
// Side awareness: events starting with SERVER prefer the server side;
// CLIENT prefer client; `_RESPONSE` events default to server, `_REQUEST`
// to client; the registry's `EventProps.client_side` / `server_side`
// flags override when set exclusively.

fn default_collect_side(event_name: &str) -> &'static str {
    let upper = event_name.to_ascii_uppercase();
    // Strip any priority index suffix (`HTTP_REQUEST#1` → `HTTP_REQUEST`).
    let base = upper.split('#').next().unwrap_or(upper.as_str());
    if base.starts_with("SERVER") {
        return "server";
    }
    if base.starts_with("CLIENT") {
        return "client";
    }
    if base.contains("_RESPONSE") {
        return "server";
    }
    if base.contains("_REQUEST") {
        return "client";
    }
    "client"
}

#[derive(Default)]
struct CollectFlowState {
    /// `protocol -> {side, ...}` for each protocol with a collect call.
    collected: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Same shape for release calls.
    released: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// `(protocol, side, span)` for every collect call.
    collect_calls: Vec<(String, String, Span)>,
    release_calls: Vec<(String, String, Span)>,
    payload_calls: Vec<(String, String, Span)>,
}

fn classify_collect_command(cmd: &str, side: &str, span: Span, state: &mut CollectFlowState) {
    if let Some(proto) = cmd.strip_suffix("::collect") {
        let p = proto.to_ascii_uppercase();
        state
            .collected
            .entry(p.clone())
            .or_default()
            .insert(side.to_string());
        state.collect_calls.push((p, side.to_string(), span));
    } else if let Some(proto) = cmd.strip_suffix("::release") {
        let p = proto.to_ascii_uppercase();
        state
            .released
            .entry(p.clone())
            .or_default()
            .insert(side.to_string());
        state.release_calls.push((p, side.to_string(), span));
    } else if let Some(proto) = cmd.strip_suffix("::payload") {
        let p = proto.to_ascii_uppercase();
        state.payload_calls.push((p, side.to_string(), span));
    }
}

fn scan_when_body_for_collect_flow(
    fu: &crate::compilation_unit::FunctionUnit,
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
            classify_stmt_for_collect_flow(stmt, side, state, registry, fu.base_offset);
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
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            span,
            args,
            ..
        } => {
            // Normalise to the resolved/canonical head and strip a leading
            // `::`: IR can legally carry `::clientside` or an alias, and the
            // bare-name `is_side_switch` lookup + the side-flip `match` would
            // otherwise miss it (and the `match`'s `_ => peer` arm would treat
            // an unrecognised side-switch head as `peer`).
            let head = canonical_command
                .as_deref()
                .unwrap_or(command)
                .trim_start_matches(':');
            if registry.is_side_switch(head) {
                let inner_side = match head {
                    "clientside" => "client",
                    "serverside" => "server",
                    // peer — the opposite-side context.
                    _ if side == "client" => "server",
                    _ => "client",
                };
                if let Some(body) = args.first() {
                    scan_side_switch_body(body, inner_side, state, registry);
                }
            } else {
                classify_collect_command(command, side, abs(*span), state);
            }
        }
        Statement::AssignValue { value, span, .. } => {
            if let Some((cmd, _)) = parse_command_substitution(value.trim()) {
                classify_collect_command(&cmd, side, abs(*span), state);
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
    side: &str,
    state: &mut CollectFlowState,
    registry: &CommandRegistry,
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
            // Side-switch bodies are re-lowered to their own CFG, so spans are
            // already relative to this fresh build's origin (0) — independent of
            // the enclosing unit's `base_offset`.
            classify_stmt_for_collect_flow(stmt, side, state, registry, 0);
        }
    }
}

/// Collect/release/payload pairing across all `when` events.
#[must_use]
pub fn find_collect_flow_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    // Pass 1: classify across all when bodies.
    let mut state = CollectFlowState::default();
    let mut events_seen: Vec<String> = Vec::new();
    for fu in cu.analysable_functions() {
        if let Some(event) = fu.name.strip_prefix("::when::") {
            let bare = event.split('#').next().unwrap_or(event).to_string();
            events_seen.push(bare.clone());
            let side = default_collect_side(event);
            scan_when_body_for_collect_flow(fu, side, &mut state, registry);
        }
    }

    // Pass 2: emit per-event IRULE1005 (using the first when proc as
    // anchor span; finer span resolution requires the `when` event-token
    // span which the analyser layer carries). The collect requirement of
    // each `*_DATA` event (protocols + side) comes from the event
    // registry's `EventProps`, not a table here.
    let events = EventRegistry::build();
    let mut emitted_events: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for event in &events_seen {
        let Some((protocols, required_side)) = events.data_collect_requirement(event) else {
            continue;
        };
        if !emitted_events.insert(event.as_str()) {
            continue;
        }
        let satisfied = protocols.iter().any(|p| {
            state
                .collected
                .get(&p.to_ascii_uppercase())
                .is_some_and(|s| s.contains(required_side))
        });
        if satisfied {
            continue;
        }
        // Anchor on the first statement of the event's CFG.
        let qname = format!("::when::{event}");
        let anchor_span = cu
            .functions()
            .find(|fu| fu.name == qname || fu.name.starts_with(&format!("{qname}#")))
            .and_then(|fu| {
                let s = fu
                    .cfg
                    .blocks
                    .get(&fu.cfg.entry)
                    .and_then(|b| b.statements.first().map(crate::ir::Statement::span))?;
                Some(fu.abs_span(s))
            })
            .unwrap_or(Span::new(0, 0));
        let proto_hint: Vec<String> = protocols.iter().map(|p| format!("{p}::collect")).collect();
        out.push(IrulesCheckWarning {
            span: anchor_span,
            code: DiagCode::Irule1005,
            message: format!(
                "'{event}' will never fire without a {required_side} {} call in another event.",
                proto_hint.join(" or "),
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }

    // IRULE1006: payload without matching collect on same side.
    for (proto, side, span) in &state.payload_calls {
        let matched = state.collected.get(proto).is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: DiagCode::Irule1006,
                message: format!(
                    "'{proto}::payload' without a {side} {proto}::collect call. The payload buffer will be empty.",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }

    // IRULE1007: collect without matching release on same side.
    for (proto, side, span) in &state.collect_calls {
        let matched = state.released.get(proto).is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: DiagCode::Irule1007,
                message: format!(
                    "{proto}::collect without matching {proto}::release on the {side} side; collected data is never released",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }

    // IRULE1008: release without matching collect on same side.
    for (proto, side, span) in &state.release_calls {
        let matched = state.collected.get(proto).is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: DiagCode::Irule1008,
                message: format!(
                    "{proto}::release without matching {proto}::collect on the {side} side; no data was collected",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }

    out
}

// IRULE1201 / IRULE1202 — HTTP-after-respond / multi-respond
//
// Path-sensitive analysis of the *structured* IR body of each
// `::when::HTTP*` procedure, tracking a set of per-path response states
// so that:
//
//   * IRULE1202 — a second `HTTP::respond` / `HTTP::redirect` / `redirect`
//     on a path that already responded; only the first response takes effect.
//   * IRULE1201 — a registered `HTTP::*` command issued on a path where a
//     response is already committed; HTTP context is invalid post-respond.
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

/// Per-event HTTP-flow warnings.  Emits IRULE1201 + IRULE1202 for each
/// `::when::HTTP*` proc body, path-sensitively.
#[must_use]
pub fn find_http_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    // Iterate the *analysable* functions (honouring the complexity
    // guard) but walk their structured IR body.
    for fu in cu.analysable_functions() {
        let Some(event) = fu.name.strip_prefix("::when::") else {
            continue;
        };
        // Strip priority index (`HTTP_REQUEST#1` → `HTTP_REQUEST`).
        let bare_event = event.split('#').next().unwrap_or(event);
        if !bare_event.starts_with("HTTP") {
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
    if state.responded && http_namespace_commands().contains(cmd) {
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
// Per-request events (HTTP_REQUEST, HTTP_REQUEST_DATA, etc.) run on
// every request; once-per-connection events (CLIENT_ACCEPTED,
// CLIENTSSL_HANDSHAKE) run once.  An `AssignConst` whose value
// doesn't depend on per-request data could be hoisted to the
// once-per-connection event, saving work per request.
//
// Flags every `set var value` in a per-request event whose value is a
// literal (no `$` substitutions, no `[cmd]` substitutions) and whose
// variable name is also literal.  It does not check that the variable
// isn't reassigned later in the same body (which would change the
// hoisting semantics).

fn is_per_request_event(event: &str) -> bool {
    // Strip priority index suffix.
    let base = event.split('#').next().unwrap_or(event);
    matches!(
        base,
        "HTTP_REQUEST"
            | "HTTP_REQUEST_DATA"
            | "HTTP_REQUEST_SEND"
            | "HTTP_REQUEST_RELEASE"
            | "HTTP_RESPONSE"
            | "HTTP_RESPONSE_DATA"
            | "HTTP_RESPONSE_RELEASE"
            | "DNS_REQUEST"
            | "DNS_RESPONSE",
    )
}

/// Hoistable-set warnings for IRULE4004.
#[must_use]
pub fn find_hoistable_set_warnings(
    cu: &CompilationUnit,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    for fu in cu.analysable_functions() {
        let Some(event) = fu.name.strip_prefix("::when::") else {
            continue;
        };
        if !is_per_request_event(event) {
            continue;
        }
        for bn in cfg_order(&fu.cfg) {
            if !fu.sccp.executable_blocks.contains(&bn) {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&bn) else {
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
                if name.is_empty() || value.is_empty() {
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
                        "`set {name} ...` runs on every request — consider hoisting to a once-per-connection event (e.g. CLIENT_ACCEPTED).",
                    ),
                    replacement: None,
                    fixes: Vec::new(),
                });
            }
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
    dialect: Option<&str>,
    generic_patterns: Option<&[String]>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
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
    use super::*;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    fn warnings_for_irules(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_unnormalised_getter_warnings(&cu, &registry(), Some("f5-irules"))
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
        assert!(find_unnormalised_getter_warnings(&cu, &registry(), Some("tcl")).is_empty());
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
        find_unguarded_drop_warnings(&cu, Some("f5-irules"))
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
        let tcl_dialect = find_unguarded_drop_warnings(&cu, Some("tcl"));
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
        find_collect_flow_warnings(&cu, &reg, Some("f5-irules"))
    }

    #[test]
    fn irule1005_data_event_without_collect_fires() {
        // CLIENT_DATA needs a TCP::collect or UDP::collect somewhere.
        let ws = flow_warnings("when CLIENT_DATA { log local0. \"data\" }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "expected IRULE1005, got {ws:?}",
        );
    }

    #[test]
    fn irule1005_satisfied_by_matching_collect() {
        // CLIENT_ACCEPTED issues TCP::collect (client side); that
        // satisfies CLIENT_DATA's IRULE1005.
        let ws = flow_warnings(
            "when CLIENT_ACCEPTED { TCP::collect }
             when CLIENT_DATA { log local0. \"data\" }
             when SERVER_CONNECTED { TCP::release }",
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

    // -- side-switch body descent + peer flip -----

    #[test]
    fn collect_inside_clientside_body_is_seen() {
        // The collect lives inside a `clientside { ... }` nesting script.
        // With the BODY role set, the flow check descends into the body
        // and registers the (client-side) collect, so CLIENT_DATA's
        // IRULE1005 requirement is satisfied.  Without the descent the
        // collect was invisible and CLIENT_DATA wrongly fired IRULE1005.
        let ws = flow_warnings(
            "when CLIENT_ACCEPTED { clientside { TCP::collect } }
             when CLIENT_DATA { log local0. \"data\" }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "no IRULE1005 expected — the clientside body supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn peer_flips_side_for_collect_flow() {
        // `peer { TCP::collect }` evaluates under the *opposite* side of
        // the surrounding event.  In the client-side CLIENT_ACCEPTED it
        // issues a *server*-side collect, so it satisfies SERVER_DATA but
        // NOT CLIENT_DATA — the inverse of a plain `clientside` collect.
        let ws = flow_warnings(
            "when CLIENT_ACCEPTED { peer { TCP::collect } }
             when CLIENT_DATA { log local0. \"c\" }
             when SERVER_DATA { log local0. \"s\" }",
        );
        let irule1005: Vec<&str> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule1005)
            .map(|w| w.message.as_str())
            .collect();
        assert_eq!(
            irule1005.len(),
            1,
            "expected exactly one IRULE1005 (CLIENT_DATA only), got {ws:?}",
        );
        assert!(
            irule1005[0].contains("CLIENT_DATA"),
            "the surviving IRULE1005 must be CLIENT_DATA (peer flipped the collect to the server side), got {irule1005:?}",
        );
    }

    #[test]
    fn side_switch_specs_have_body_role_and_arity() {
        // Registry-level guard for the spec edits.
        let r = registry();
        for cmd in ["clientside", "serverside", "peer"] {
            assert!(r.is_side_switch(cmd), "{cmd} must be a side-switch");
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
        find_http_flow_warnings(&cu, Some("f5-irules"))
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
        // IRULE1201 (matches the registry-derived namespace set).
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
        find_hoistable_set_warnings(&cu, Some("f5-irules"))
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
    fn irule4004_set_in_once_per_connection_clean() {
        // CLIENT_ACCEPTED already runs once-per-connection; nothing to hoist.
        let ws = hoist_warnings(r#"when CLIENT_ACCEPTED { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — already once-per-connection, got {ws:?}",
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
        find_generic_static_name_warnings(&cu, Some("f5-irules"), None)
    }

    fn generic_warnings_with(source: &str, patterns: &[String]) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_generic_static_name_warnings(&cu, Some("f5-irules"), Some(patterns))
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
