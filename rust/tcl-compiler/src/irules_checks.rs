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
//! C44-irules-flow status (audited at port time): IRULE3102, IRULE5002,
//! IRULE5004 land here.  IRULE1005 / IRULE1006 / IRULE1007 / IRULE1008
//! (collect/release/payload pairing across `when` events), IRULE1201 /
//! IRULE1202 (HTTP-after-respond and multi-respond), IRULE4004
//! (per-request set hoistable to once-per-connection) require richer
//! cross-event analysis built on `connection_scope.rs` —  ported as
//! their own follow-up sub-strips per the chunk-log row's
//! "each diagnostic is its own sub-strip" sequencing.

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::compilation_unit::CompilationUnit;
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::taint::is_irules_dialect;
use crate::value_shapes::parse_command_substitution;

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

/// An IRULE3102 / iRules-check diagnostic emitted by
/// [`find_unnormalised_getter_warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrulesCheckWarning {
    /// Source span of the offending call.
    pub span: Span,
    /// Diagnostic code (`"IRULE3102"`).
    pub code: String,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text (currently always `None`).
    pub replacement: Option<String>,
}

/// Return `true` when `args` suggests a *getter* invocation — no args,
/// or every arg starts with `-` (all flags, no positional value). The
/// Python canonical form resolves a `FormKind::GETTER` via the command
/// registry; the Rust registry has no form-kind model yet, so this is
/// a conservative approximation. `HTTP::path /foo` (setter) is excluded
/// because `/foo` is a non-flag first arg.
fn is_getter_form(args: &[String]) -> bool {
    args.iter().all(|a| a.starts_with('-'))
}

/// Format the IRULE3102 message for `cmd`.
fn format_message(cmd: &str) -> String {
    format!(
        "Use '{cmd} -normalized' for canonicalized request data; \
         non-normalized values may allow URL evasion patterns."
    )
}

/// Return `true` when `cmd` is one of the commands that carry the
/// `-normalized` option and `args` misses it in a getter form.
fn is_unnormalised_getter(registry: &CommandRegistry, cmd: &str, args: &[String]) -> bool {
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

    for fu in cu.functions() {
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
                        ..
                    } if is_unnormalised_getter(registry, command, args) => {
                        out.push(IrulesCheckWarning {
                            span: *span,
                            code: "IRULE3102".to_owned(),
                            message: format_message(command),
                            replacement: None,
                        });
                    }
                    Statement::AssignValue { value, span, .. } => {
                        let Some((cmd, sub_args)) = parse_command_substitution(value.trim()) else {
                            continue;
                        };
                        if is_unnormalised_getter(registry, &cmd, &sub_args) {
                            out.push(IrulesCheckWarning {
                                span: *span,
                                code: "IRULE3102".to_owned(),
                                message: format_message(&cmd),
                                replacement: None,
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

// ---------------------------------------------------------------------------
// IRULE5002 + IRULE5004 — unguarded drop / DNS::return
// ---------------------------------------------------------------------------
//
// Mirrors `core/compiler/irules_flow.py::_check_unguarded_drops`
// (lines 807-1050).  The Python implementation is path-sensitive
// (walks each branch of every `if` / `switch` / `try` separately
// and emits at branch joins where the drop survived).  This Rust
// minimum-viable port linear-scans each `::when::*` proc body in
// CFG order and emits when no terminator follows the drop in the
// same block run.  Catches the most common shape (top-level
// `drop` / `reject` without a guard); branch-sensitive coverage
// is the C44 follow-up sub-strip.

fn is_drop_command(cmd: &str) -> bool {
    matches!(cmd, "drop" | "reject" | "discard")
}

fn is_dns_return(cmd: &str, args: &[String]) -> bool {
    cmd == "DNS::return" && (args.is_empty() || args.iter().all(|a| !a.starts_with('-')))
}

/// `event disable all` is a drop guard: the subsequent statements
/// run but no other iRule does.
fn is_event_disable_all(cmd: &str, args: &[String]) -> bool {
    cmd == "event"
        && args.len() >= 2
        && args[0] == "disable"
        && args[1] == "all"
}

/// Scan each `::when::*` proc body for IRULE5002 / IRULE5004
/// shapes.  Linear analysis only — see the module-level note.
#[must_use]
pub fn find_unguarded_drop_warnings(
    cu: &CompilationUnit,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }

    for fu in cu.functions() {
        // Only `::when::EVENT` proc bodies are iRules event handlers.
        if !fu.name.starts_with("::when::") {
            continue;
        }
        out.extend(scan_when_body_for_drops(fu));
    }
    out
}

fn scan_when_body_for_drops(fu: &crate::compilation_unit::FunctionUnit) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    // Linear scan: track the most-recent drop / DNS::return spans.
    // A `return` or `event disable all` clears them.
    let mut pending_drop: Option<(Span, String)> = None;
    let mut pending_dns: Option<Span> = None;

    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            let Statement::Call {
                command,
                args,
                span,
                ..
            } = stmt
            else {
                continue;
            };
            if is_drop_command(command) {
                pending_drop = Some((*span, command.clone()));
                continue;
            }
            if is_dns_return(command, args) {
                pending_dns = Some(*span);
                continue;
            }
            if is_event_disable_all(command, args) {
                // Guards an earlier drop.
                pending_drop = None;
            }
            // `return` is a Terminator on the block, not a Call —
            // handled below per-block.
        }
        // After the block's statements: check the terminator.  A
        // `Return` clears all pending drops; otherwise pending state
        // carries across.
        if let Some(term) = &block.terminator {
            if matches!(term, crate::cfg::Terminator::Return { .. }) {
                pending_drop = None;
                pending_dns = None;
            }
        }
    }

    if let Some((span, cmd)) = pending_drop {
        out.push(IrulesCheckWarning {
            span,
            code: "IRULE5002".to_owned(),
            message: format!(
                "`{cmd}` without a subsequent `event disable all` or `return` — other iRules continue executing on this connection.",
            ),
            replacement: None,
        });
    }
    if let Some(span) = pending_dns {
        out.push(IrulesCheckWarning {
            span,
            code: "IRULE5004".to_owned(),
            message: "`DNS::return` without a subsequent `return` — iRule processing continues after `DNS::return`.".to_owned(),
            replacement: None,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// IRULE1005 / IRULE1006 / IRULE1007 / IRULE1008 — collect/release/payload
// ---------------------------------------------------------------------------
//
// Mirrors `irules_flow.py::_find_collect_flow_warnings` (lines 680-806).
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
// Side awareness mirrors Python's `_default_collect_side`: events
// starting with SERVER prefer the server side; CLIENT prefer client;
// `_RESPONSE` events default to server, `_REQUEST` to client; the
// registry's `EventProps.client_side` / `server_side` flags override
// when set exclusively.

const DATA_EVENT_REQUIREMENTS: &[(&str, &[&str], &str)] = &[
    ("CLIENT_DATA", &["TCP", "UDP"], "client"),
    ("SERVER_DATA", &["TCP", "UDP"], "server"),
    ("HTTP_REQUEST_DATA", &["HTTP"], "client"),
    ("HTTP_RESPONSE_DATA", &["HTTP"], "server"),
    ("CLIENTSSL_DATA", &["SSL"], "client"),
    ("SERVERSSL_DATA", &["SSL"], "server"),
];

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
) {
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            match stmt {
                Statement::Call { command, span, .. }
                | Statement::Barrier { command, span, .. } => {
                    classify_collect_command(command, side, *span, state);
                }
                Statement::AssignValue { value, span, .. } => {
                    let Some((cmd, _)) = parse_command_substitution(value.trim()) else {
                        continue;
                    };
                    classify_collect_command(&cmd, side, *span, state);
                }
                _ => {}
            }
        }
    }
}

/// Collect/release/payload pairing across all `when` events.  Mirrors
/// `irules_flow.py::_find_collect_flow_warnings`.
#[must_use]
pub fn find_collect_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    // Pass 1: classify across all when bodies.
    let mut state = CollectFlowState::default();
    let mut events_seen: Vec<String> = Vec::new();
    for fu in cu.functions() {
        if let Some(event) = fu.name.strip_prefix("::when::") {
            let bare = event.split('#').next().unwrap_or(event).to_string();
            events_seen.push(bare.clone());
            let side = default_collect_side(event);
            scan_when_body_for_collect_flow(fu, side, &mut state);
        }
    }

    // Pass 2: emit per-event IRULE1005 (using the first when proc as
    // anchor span; finer span resolution requires the `when` event-token
    // span which the analyser layer carries).
    for (event, protocols, required_side) in DATA_EVENT_REQUIREMENTS {
        if !events_seen.iter().any(|e| e == event) {
            continue;
        }
        let satisfied = protocols.iter().any(|p| {
            state
                .collected
                .get(&p.to_ascii_uppercase())
                .is_some_and(|s| s.contains(*required_side))
        });
        if satisfied {
            continue;
        }
        // Anchor on the first statement of the event's CFG.
        let qname = format!("::when::{event}");
        let anchor_span = cu
            .functions()
            .find(|fu| fu.name == qname || fu.name.starts_with(&format!("{qname}#")))
            .and_then(|fu| fu.cfg.blocks.get(&fu.cfg.entry))
            .and_then(|b| b.statements.first().map(crate::ir::Statement::span))
            .unwrap_or(Span::new(0, 0));
        let proto_hint: Vec<String> = protocols
            .iter()
            .map(|p| format!("{p}::collect"))
            .collect();
        out.push(IrulesCheckWarning {
            span: anchor_span,
            code: "IRULE1005".to_owned(),
            message: format!(
                "'{event}' will never fire without a {required_side} {} call in another event.",
                proto_hint.join(" or "),
            ),
            replacement: None,
        });
    }

    // IRULE1006: payload without matching collect on same side.
    for (proto, side, span) in &state.payload_calls {
        let matched = state
            .collected
            .get(proto)
            .is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: "IRULE1006".to_owned(),
                message: format!(
                    "'{proto}::payload' without a {side} {proto}::collect call. The payload buffer will be empty.",
                ),
                replacement: None,
            });
        }
    }

    // IRULE1007: collect without matching release on same side.
    for (proto, side, span) in &state.collect_calls {
        let matched = state
            .released
            .get(proto)
            .is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: "IRULE1007".to_owned(),
                message: format!(
                    "{proto}::collect without matching {proto}::release on the {side} side; collected data is never released",
                ),
                replacement: None,
            });
        }
    }

    // IRULE1008: release without matching collect on same side.
    for (proto, side, span) in &state.release_calls {
        let matched = state
            .collected
            .get(proto)
            .is_some_and(|s| s.contains(side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: *span,
                code: "IRULE1008".to_owned(),
                message: format!(
                    "{proto}::release without matching {proto}::collect on the {side} side; no data was collected",
                ),
                replacement: None,
            });
        }
    }

    out
}

// ---------------------------------------------------------------------------
// IRULE1201 / IRULE1202 — HTTP-after-respond / multi-respond
// ---------------------------------------------------------------------------
//
// Mirrors `irules_flow.py::_analyse_when_body` (lines 296-450).
// Linear CFG scan within each `::when::HTTP*` proc body:
//
//   * IRULE1202 — second `HTTP::respond` / `HTTP::redirect` on the
//     same path; only the first response takes effect.
//   * IRULE1201 — any `HTTP::*` command issued after a response is
//     committed; HTTP context is invalid post-respond.
//
// Path-sensitivity (separate state per branch of `if` / `switch` /
// `try`) is the C44 follow-up.  This linear shape catches the
// straight-line cases (`HTTP::respond ...; HTTP::header ...`).
//
// Response-committing commands: hardcoded to `HTTP::respond` /
// `HTTP::redirect` (the canonical pair).  When the registry grows
// a `SideEffectTarget::ResponseCommit` category the lookup
// switches to the registry-driven query.

fn commits_http_response(cmd: &str) -> bool {
    matches!(cmd, "HTTP::respond" | "HTTP::redirect")
}

fn is_http_namespace(cmd: &str) -> bool {
    cmd.starts_with("HTTP::")
}

/// Per-event HTTP-flow warnings.  Emits IRULE1201 + IRULE1202 for
/// each `::when::HTTP*` proc body.
#[must_use]
pub fn find_http_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<&str>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    for fu in cu.functions() {
        let Some(event) = fu.name.strip_prefix("::when::") else {
            continue;
        };
        // Strip priority index (`HTTP_REQUEST#1` → `HTTP_REQUEST`).
        let bare_event = event.split('#').next().unwrap_or(event);
        if !bare_event.starts_with("HTTP") {
            continue;
        }
        out.extend(scan_when_body_for_http_flow(fu, bare_event));
    }
    out
}

fn scan_when_body_for_http_flow(
    fu: &crate::compilation_unit::FunctionUnit,
    event: &str,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    let mut responded = false;
    let mut respond_command: Option<String> = None;

    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            let (cmd, span) = match stmt {
                Statement::Call { command, span, .. }
                | Statement::Barrier { command, span, .. } => (command.as_str(), *span),
                _ => continue,
            };
            if commits_http_response(cmd) {
                if responded {
                    out.push(IrulesCheckWarning {
                        span,
                        code: "IRULE1202".to_owned(),
                        message: format!(
                            "Multiple '{cmd}' calls possible in {event}. Only the first response takes effect.",
                        ),
                        replacement: None,
                    });
                } else {
                    responded = true;
                    respond_command = Some(cmd.to_string());
                }
                continue;
            }
            if responded && is_http_namespace(cmd) {
                let _ = respond_command.as_ref();
                out.push(IrulesCheckWarning {
                    span,
                    code: "IRULE1201".to_owned(),
                    message: format!(
                        "'{cmd}' used after response is committed. HTTP context is invalid after HTTP::respond/HTTP::redirect.",
                    ),
                    replacement: None,
                });
            }
        }
        // `return` Terminator clears the response state (the rule
        // exits before any "after" code runs).
        if let Some(crate::cfg::Terminator::Return { .. }) = &block.terminator {
            responded = false;
            respond_command = None;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// IRULE4004 — `set var value` in per-request event hoistable to once-per-connection
// ---------------------------------------------------------------------------
//
// Mirrors `irules_flow.py::_check_hoistable_sets` (lines 1075-1280).
// Per-request events (HTTP_REQUEST, HTTP_REQUEST_DATA, etc.) run on
// every request; once-per-connection events (CLIENT_ACCEPTED,
// CLIENTSSL_HANDSHAKE) run once.  An `AssignConst` whose value
// doesn't depend on per-request data could be hoisted to the
// once-per-connection event, saving work per request.
//
// Minimum-viable port: flag every `set var value` in a per-request
// event whose value is a literal (no `$` substitutions, no
// `[cmd]` substitutions) and whose variable name is also literal.
// The Python implementation additionally checks that the variable
// isn't reassigned later in the same body (otherwise hoisting
// changes semantics) — that branch-aware check is the follow-up.

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
    for fu in cu.functions() {
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
                    Statement::AssignConst { name, value, span }
                    | Statement::AssignValue { name, value, span, .. } => (name, value, *span),
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
                    span,
                    code: "IRULE4004".to_owned(),
                    message: format!(
                        "`set {name} ...` runs on every request — consider hoisting to a once-per-connection event (e.g. CLIENT_ACCEPTED).",
                    ),
                    replacement: None,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        assert_eq!(w[0].code, "IRULE3102");
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

    /// ARCH3 — IRULE3102 must derive its command set from the
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
            with_uri.iter().any(|w| w.code == "IRULE3102"),
            "expected IRULE3102 on HTTP::uri, got {with_uri:?}",
        );
        let with_header = warnings_for_irules("set u [HTTP::header Content-Type]");
        assert!(
            with_header.is_empty(),
            "no IRULE3102 expected on HTTP::header (no -normalized), got {with_header:?}",
        );
    }

    // -- IRULE5002 / IRULE5004 ----------------------------------------

    fn drop_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_unguarded_drop_warnings(&cu, Some("f5-irules"))
    }

    #[test]
    fn irule5002_drop_without_return_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE5002"),
            "expected IRULE5002, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_return_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; return }");
        assert!(
            !ws.iter().any(|w| w.code == "IRULE5002"),
            "no IRULE5002 expected when `return` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_event_disable_all_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; event disable all }");
        assert!(
            !ws.iter().any(|w| w.code == "IRULE5002"),
            "no IRULE5002 expected when `event disable all` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_reject_also_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { reject }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE5002"),
            "expected IRULE5002 on `reject`, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            "when CLIENT_ACCEPTED { drop }",
            &registry(),
            false,
        );
        let none_dialect = find_unguarded_drop_warnings(&cu, None);
        assert!(none_dialect.is_empty(), "got {none_dialect:?}");
        let tcl_dialect = find_unguarded_drop_warnings(&cu, Some("tcl"));
        assert!(tcl_dialect.is_empty(), "got {tcl_dialect:?}");
    }

    #[test]
    fn irule5004_dns_return_without_return_fires() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE5004"),
            "expected IRULE5004, got {ws:?}",
        );
    }

    #[test]
    fn irule5004_dns_return_followed_by_return_clean() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return; return }");
        assert!(
            !ws.iter().any(|w| w.code == "IRULE5004"),
            "no IRULE5004 expected when `return` follows `DNS::return`, got {ws:?}",
        );
    }

    #[test]
    fn no_drop_warnings_for_clean_when_body() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { log local0. \"connection open\" }");
        assert!(ws.is_empty(), "got {ws:?}");
    }

    // -- IRULE1005 / 1006 / 1007 / 1008 -------------------------------

    fn flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_collect_flow_warnings(&cu, Some("f5-irules"))
    }

    #[test]
    fn irule1005_data_event_without_collect_fires() {
        // CLIENT_DATA needs a TCP::collect or UDP::collect somewhere.
        let ws = flow_warnings("when CLIENT_DATA { log local0. \"data\" }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE1005"),
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
            !ws.iter().any(|w| w.code == "IRULE1005"),
            "no IRULE1005 expected — CLIENT_ACCEPTED supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn irule1006_payload_without_collect_fires() {
        let ws = flow_warnings("when HTTP_REQUEST { HTTP::payload }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE1006"),
            "expected IRULE1006 on HTTP::payload without HTTP::collect, got {ws:?}",
        );
    }

    #[test]
    fn irule1007_collect_without_release_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::collect }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE1007"),
            "expected IRULE1007, got {ws:?}",
        );
    }

    #[test]
    fn irule1008_release_without_collect_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::release }");
        assert!(
            ws.iter().any(|w| w.code == "IRULE1008"),
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
            !ws.iter().any(|w| w.code == "IRULE1007"),
            "1007 unexpected, got {ws:?}",
        );
        assert!(
            !ws.iter().any(|w| w.code == "IRULE1008"),
            "1008 unexpected, got {ws:?}",
        );
    }

    #[test]
    fn collect_flow_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            "when CLIENT_ACCEPTED { TCP::collect }",
            &registry(),
            false,
        );
        let none = find_collect_flow_warnings(&cu, None);
        assert!(none.is_empty(), "got {none:?}");
    }

    // -- IRULE1201 / 1202 ---------------------------------------------

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
            ws.iter().any(|w| w.code == "IRULE1202"),
            "expected IRULE1202, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_http_after_respond_fires() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache }",
        );
        assert!(
            ws.iter().any(|w| w.code == "IRULE1201"),
            "expected IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_clean_when_respond_followed_by_return_only() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; return }",
        );
        assert!(
            !ws.iter().any(|w| w.code == "IRULE1201" || w.code == "IRULE1202"),
            "no IRULE1201/1202 expected, got {ws:?}",
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
            !ws.iter().any(|w| w.code == "IRULE1202"),
            "IRULE1202 should not fire outside HTTP events, got {ws:?}",
        );
    }

    // -- IRULE4004 ----------------------------------------------------

    fn hoist_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_hoistable_set_warnings(&cu, Some("f5-irules"))
    }

    #[test]
    fn irule4004_literal_set_in_per_request_event_fires() {
        let ws = hoist_warnings(r#"when HTTP_REQUEST { set svc "foo" }"#);
        assert!(
            ws.iter().any(|w| w.code == "IRULE4004"),
            "expected IRULE4004, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_dynamic_set_clean() {
        // Value depends on per-request data — can't hoist.
        let ws = hoist_warnings("when HTTP_REQUEST { set svc [HTTP::host] }");
        assert!(
            !ws.iter().any(|w| w.code == "IRULE4004"),
            "no IRULE4004 expected — value depends on request, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_var_substitution_clean() {
        let ws = hoist_warnings("when HTTP_REQUEST { set svc $session }");
        assert!(
            !ws.iter().any(|w| w.code == "IRULE4004"),
            "no IRULE4004 expected — value uses $session, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_set_in_once_per_connection_clean() {
        // CLIENT_ACCEPTED already runs once-per-connection; nothing to hoist.
        let ws = hoist_warnings(r#"when CLIENT_ACCEPTED { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == "IRULE4004"),
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
}
