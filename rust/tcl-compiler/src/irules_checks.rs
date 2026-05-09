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
}
