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

//! IRULE3101: setter constraints on literals, tainted values and command substitutions, gated by dialect.

use super::*;

// IRULE3101 — setter-constraint violations

/// Default helper: run the setter check under the `f5-irules` dialect
/// (which is the only dialect that can surface IRULE3101 post-internal-gate).
fn setter_warnings_for(source: &str) -> Vec<TaintWarning> {
    setter_warnings_for_dialect(source, Some(tcl_dialect::DialectProfile::irules()))
}

fn setter_warnings_for_dialect(
    source: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    let mut registry = CommandRegistry::build_default();
    // The IRULE3101 setter constraints live on the iRules specs, so the
    // registry must have them loaded for the dialect-gated check to see
    // anything.
    registry.load_irules();
    let mut cu = CompilationUnit::build_for(source, &registry, false);
    if dialect.is_some() {
        cu = cu.with_interprocedural(&registry, dialect);
    }
    let mut out: Vec<TaintWarning> = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_setter_constraint_warnings(
            &registry,
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            Some(&fu.sccp.values),
            &fu.sccp.executable_blocks,
            dialect,
        ));
    }
    out
}

#[test]
fn irule3101_literal_missing_slash_warns() {
    let w = setter_warnings_for("HTTP::uri foo");
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].code, DiagCode::Irule3101);
    assert!(w[0].message.contains("HTTP::uri value must start"));
}

#[test]
fn irule3101_literal_with_slash_clean() {
    let w = setter_warnings_for("HTTP::uri /foo");
    assert_eq!(w.len(), 0, "literal /foo must be clean, got {w:?}");
}

#[test]
fn irule3101_http_path_literal_variants() {
    let bad = setter_warnings_for("HTTP::path bar");
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].code, DiagCode::Irule3101);
    let good = setter_warnings_for("HTTP::path /bar");
    assert_eq!(good.len(), 0);
}

#[test]
fn irule3101_generic_taint_warns() {
    // `HTTP::header X-Foo` is a tainted source; reusing it as a path
    // has generic taint (no PATH_PREFIXED / _NORMALISED / _BOUNDED).
    let w = setter_warnings_for("set v [HTTP::header X-Foo]\nHTTP::uri $v");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "generic taint must fire IRULE3101, got {w:?}"
    );
}

#[test]
fn irule3101_pure_var_ref_always_warns_without_safe_colour() {
    // A plain `$p` setter value (no taint + no provable path colour)
    // that the lattice cannot pin — two arms set two values — cannot be
    // proved `/`-prefixed by the static analyser, so IRULE3101 fires.
    // Latent suppression paths via tainted-with-PATH_PREFIXED /
    // _NORMALISED / _BOUNDED colours will light up once iRules source
    // `taint_hints` reach the lattice. A value the lattice proves is
    // checked as a literal is (#2055): `set p /safe` is clean.
    let w = setter_warnings_for(
        "if {[HTTP::has_responded]} { set p /safe } else { set p safe }\nHTTP::uri $p",
    );
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "pure var-ref setter value must warn without tainted-safe-colour, got {w:?}"
    );
    let w = setter_warnings_for("set p /safe\nHTTP::uri $p");
    assert!(
        !w.iter().any(|x| x.code == DiagCode::Irule3101),
        "a proven `/` path is clean, got {w:?}"
    );
}

#[test]
fn irule3101_dynamic_command_sub_warns() {
    // RHS is a command sub `[foo]` → hits the dynamic branch, which
    // always warns. The literal-check also bails on `[`.
    let w = setter_warnings_for("HTTP::uri [something]");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "command-sub setter value must warn, got {w:?}"
    );
}

#[test]
fn irule3101_internally_gated_on_dialect() {
    // Defence-in-depth: even if a caller invokes
    // `find_setter_constraint_warnings` directly (bypassing
    // `run_all_checks`), IRULE3101 must not fire under a non-iRules
    // dialect against user-defined commands named `HTTP::uri` /
    // `HTTP::path`.
    let under_irules =
        setter_warnings_for_dialect("HTTP::uri foo", Some(tcl_dialect::DialectProfile::irules()));
    assert_eq!(under_irules.len(), 1);
    assert_eq!(under_irules[0].code, DiagCode::Irule3101);

    let under_none = setter_warnings_for_dialect("HTTP::uri foo", None);
    assert_eq!(
        under_none.len(),
        0,
        "no IRULE3101 under None dialect, got {under_none:?}"
    );

    let under_tcl = setter_warnings_for_dialect(
        "HTTP::uri foo",
        Some(tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()),
    );
    assert_eq!(
        under_tcl.len(),
        0,
        "no IRULE3101 under tcl dialect, got {under_tcl:?}"
    );
}

/// IRULE3101 setter-constraint violations are gated by the
/// iRules dialect filter — the diagnostic must not fire under
/// the plain Tcl dialect because the setter constraint comes
/// from the iRules-only registry entry.
#[test]
fn arch3_setter_constraint_is_dialect_driven() {
    // `HTTP::uri foo` (no leading slash) under iRules produces
    // an IRULE3101 setter-constraint violation.
    let irules =
        setter_warnings_for_dialect("HTTP::uri foo", Some(tcl_dialect::DialectProfile::irules()));
    assert!(
        irules.iter().any(|w| w.code == DiagCode::Irule3101),
        "expected IRULE3101 under iRules dialect, got {irules:?}",
    );

    // The same source under plain Tcl: a user-defined proc named
    // `HTTP::uri` is legal and the setter check must stay silent.
    let plain = setter_warnings_for_dialect("HTTP::uri foo", None);
    assert!(
        plain.iter().all(|w| w.code != DiagCode::Irule3101),
        "no IRULE3101 outside iRules, got {plain:?}",
    );
}
