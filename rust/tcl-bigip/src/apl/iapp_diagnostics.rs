//! Cross-file diagnostics for iApp presentation ↔ implementation — Rust
//! port of `dialects/f5/bigip/iapp_diagnostics.py`.
//!
//! Validates that:
//! - implementation variables reference fields that exist in the presentation
//! - presentation fields are actually used by the implementation
//! - `#include` directives point to files that resolved
//!
//! Diagnostic codes (all internal):
//!
//! - **IAPP7001** (WARNING): implementation references a variable not
//!   defined in the presentation
//! - **IAPP7002** (HINT): presentation field never referenced in the
//!   implementation
//! - **IAPP7003** (WARNING): `#include` file not found

use std::collections::HashSet;

use super::iapp_vars::IappVarRef;
use super::model::AplModel;
use crate::range::{Position, Range};
use crate::validator::{ConfigDiagnostic, DiagSeverity};

/// Dialects in which iApp presentation/implementation cross-validation
/// makes sense (iApps live under `f5-iapps`; `f5-tmsh` / `f5-bigip` share
/// the iApp template format). All other dialects are unrelated, so the
/// IAPP diagnostics must not fire there. Mirrors Python `IAPP_DIALECTS`.
pub const IAPP_DIALECTS: [&str; 3] = ["f5-iapps", "f5-tmsh", "f5-bigip"];

/// Whether `dialect` is one where iApp checks apply (port of
/// `_iapp_dialect_active`).
#[must_use]
pub fn iapp_dialect_active(dialect: &str) -> bool {
    IAPP_DIALECTS.contains(&dialect)
}

/// Validate an APL presentation model, optionally cross-checked against
/// implementation variable references. Returns an empty list outside the
/// iApp dialects (defense-in-depth, exactly as in Python) so iApp advice
/// never leaks into plain Tcl / iRules / EDA dialects.
///
/// IAPP7001 (undefined variable) is emitted by
/// [`validate_iapp_implementation`] so the diagnostic lands in the
/// implementation file; it is intentionally not emitted here.
#[must_use]
pub fn validate_iapp_presentation(
    model: &AplModel,
    impl_var_refs: Option<&[IappVarRef]>,
    dialect: &str,
) -> Vec<ConfigDiagnostic> {
    if !iapp_dialect_active(dialect) {
        return Vec::new();
    }
    let mut out: Vec<ConfigDiagnostic> = Vec::new();

    // IAPP7003: #include not found.
    for inc in &model.includes {
        if !inc.resolved {
            // AplInclude only tracks the line number; offset/char are 0,
            // mirroring the Python placeholder.
            let pos = Position {
                line: inc.line,
                character: 0,
                offset: 0,
            };
            out.push(ConfigDiagnostic {
                code: "IAPP7003".to_owned(),
                message: format!("#include file not found: \"{}\"", inc.path),
                severity: DiagSeverity::Warning,
                range: Range {
                    start: pos,
                    end: pos,
                },
            });
        }
    }

    let Some(refs) = impl_var_refs else {
        return out;
    };

    let referenced: HashSet<&str> = refs.iter().map(|r| r.apl_name.as_str()).collect();

    // IAPP7002: unused presentation field (message fields are display-only).
    for (qname, field) in &model.all_fields {
        if !referenced.contains(qname.as_str()) {
            if field.field_type == "message" {
                continue;
            }
            out.push(ConfigDiagnostic {
                code: "IAPP7002".to_owned(),
                message: format!(
                    "Presentation field '{qname}' is never referenced in the implementation \
                     (expected $::{})",
                    field.qualified_name.replace('.', "__")
                ),
                severity: DiagSeverity::Hint,
                range: field.range,
            });
        }
    }

    out
}

/// Validate iApp implementation references against the presentation,
/// producing diagnostics positioned in the implementation source. Returns
/// an empty list outside the iApp dialects, or when no presentation model
/// is available. Mirrors `validate_iapp_implementation`.
#[must_use]
pub fn validate_iapp_implementation(
    impl_var_refs: &[IappVarRef],
    model: Option<&AplModel>,
    dialect: &str,
) -> Vec<ConfigDiagnostic> {
    if !iapp_dialect_active(dialect) {
        return Vec::new();
    }
    let Some(model) = model else {
        return Vec::new();
    };

    let known: HashSet<&str> = model.all_fields.iter().map(|(q, _)| q.as_str()).collect();

    let mut out: Vec<ConfigDiagnostic> = Vec::new();
    for r in impl_var_refs {
        if !known.contains(r.apl_name.as_str()) {
            out.push(ConfigDiagnostic {
                code: "IAPP7001".to_owned(),
                message: format!(
                    "Variable ${} references presentation field '{}' which is not defined \
                     in the presentation",
                    r.tcl_name, r.apl_name
                ),
                severity: DiagSeverity::Warning,
                range: r.range,
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::super::iapp_vars::extract_iapp_var_refs;
    use super::super::parser::parse_apl;
    use super::*;

    const PRESENTATION: &str = "section basic {\n  string addr\n  string port\n}\n";

    fn codes(diags: &[ConfigDiagnostic]) -> Vec<&str> {
        diags.iter().map(|d| d.code.as_str()).collect()
    }

    #[test]
    fn iapp7001_fires_for_undefined_variable() {
        let model = parse_apl(PRESENTATION);
        let refs = extract_iapp_var_refs("set x $::basic__missing");
        let diags = validate_iapp_implementation(&refs, Some(&model), "f5-iapps");
        assert!(codes(&diags).contains(&"IAPP7001"));
    }

    #[test]
    fn iapp7001_quiet_for_defined_variable() {
        let model = parse_apl(PRESENTATION);
        let refs = extract_iapp_var_refs("set x $::basic__addr");
        let diags = validate_iapp_implementation(&refs, Some(&model), "f5-iapps");
        assert!(diags.is_empty());
    }

    #[test]
    fn iapp7002_fires_for_unused_field() {
        let model = parse_apl(PRESENTATION);
        // Implementation references only `basic.addr`, leaving `basic.port`.
        let refs = extract_iapp_var_refs("set x $::basic__addr");
        let diags = validate_iapp_presentation(&model, Some(&refs), "f5-iapps");
        assert!(codes(&diags).contains(&"IAPP7002"));
    }

    #[test]
    fn no_iapp_checks_outside_iapp_dialects() {
        let model = parse_apl(PRESENTATION);
        let refs = extract_iapp_var_refs("set x $::basic__missing");
        assert!(validate_iapp_implementation(&refs, Some(&model), "tcl").is_empty());
        assert!(validate_iapp_presentation(&model, Some(&refs), "f5-irules").is_empty());
    }
}
