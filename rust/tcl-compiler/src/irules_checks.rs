//! iRules-specific static checks (non-taint).
//!
//! Currently hosts **IRULE3102** — `HTTP::path` / `HTTP::uri` /
//! `HTTP::query` getters used without the `-normalized` option, which
//! leaves them susceptible to URL-evasion patterns (double-encoding,
//! path-traversal escapes). Ported from
//! `core/analysis/checks/_domain.py::check_irules_unnormalized_http_getter`.
//!
//! Unlike [`crate::taint`] which drives off per-SSA-value lattices,
//! IRULE3102 is a pure AST scan: it walks every `Statement::Call` and
//! every `Statement::AssignValue` whose RHS is a bare command
//! substitution, and reports getters that omit `-normalized` under the
//! `"f5-irules"` / `"irules"` dialect.

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
}
