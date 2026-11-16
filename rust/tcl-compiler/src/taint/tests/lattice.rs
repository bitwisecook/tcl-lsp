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

//! `TaintLattice` operations: constructors, joins, safe masks, rendered-property colours and the registry colour bridge.

use super::*;

#[test]
fn clean_and_tainted_constructors() {
    let c = TaintLattice::clean();
    assert!(!c.is_tainted());
    let t = TaintLattice::tainted();
    assert!(t.is_tainted());
}

#[test]
fn join_propagates_taint_intersects_mitigations() {
    // Two TAINTED operands: taint unions, mitigations are must-have so only
    // the colour present on both survives the intersection.
    let a = TaintLattice::from_colours(
        TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED,
    );
    let b = TaintLattice::from_colours(
        TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::NON_DASH_PREFIXED,
    );
    let j = a.join(b);
    assert!(j.colours.contains(TaintColour::TAINTED));
    assert!(j.colours.contains(TaintColour::CRLF_FREE));
    assert!(!j.colours.contains(TaintColour::PATH_PREFIXED));
    assert!(!j.colours.contains(TaintColour::NON_DASH_PREFIXED));
}

#[test]
fn join_with_untainted_is_identity() {
    // A clean/untainted operand is the join identity: it contributes no
    // taint, so it must not dilute the tainted operand's mitigation
    // colours: joining with the annihilating empty set would strip
    // PATH_PREFIXED.
    let tainted = TaintLattice::from_colours(TaintColour::TAINTED | TaintColour::PATH_PREFIXED);
    assert_eq!(tainted.join(TaintLattice::clean()).colours, tainted.colours);
    assert_eq!(TaintLattice::clean().join(tainted).colours, tainted.colours);
    // An untainted operand that happens to carry colours is still the
    // identity — its taint contribution is nil, so it cannot remove a
    // mitigation from the tainted side.
    let untainted_coloured = TaintLattice::from_colours(TaintColour::CRLF_FREE);
    assert_eq!(tainted.join(untainted_coloured).colours, tainted.colours);
}

#[test]
fn with_and_sanitised() {
    let v = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
    assert!(v.is_tainted());
    assert!(v.colours.contains(TaintColour::CRLF_FREE));
    let s = v.sanitised();
    assert!(!s.is_tainted());
    assert!(s.colours.contains(TaintColour::CRLF_FREE));
}

#[test]
fn t102_safe_mask_excludes_tainted() {
    assert!(!TaintColour::T102_SAFE.contains(TaintColour::TAINTED));
    assert!(TaintColour::T102_SAFE.contains(TaintColour::PATH_PREFIXED));
}

#[test]
fn crlf_safe_mask_composition() {
    // `CRLF_SAFE = CRLF_FREE | IP_ADDRESS | PORT | FQDN`.
    // HEADER_TOKEN_SAFE only suppresses IRULE3002 in the name position
    // (handled by `irule3002_name_position_safe`); HTML_ESCAPED / URL_ENCODED
    // do not prove CRLF-injection safety in the value position.
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::CRLF_FREE));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::IP_ADDRESS));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::PORT));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::FQDN));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HEADER_TOKEN_SAFE));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HTML_ESCAPED));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::URL_ENCODED));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::TAINTED));
}

/// Rendered-property colouring: a value starting with `/` should
/// pick up `PATH_PREFIXED` (implying `NON_DASH_PREFIXED`).
#[test]
fn rendered_props_colour_slash_value_path_prefixed() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for("set path /etc/hosts", &registry, false);
    let fu = cu.function("::top").unwrap();
    let entry = fu
        .taints
        .iter()
        .find(|((n, _), _)| fu.ssa.var_name(*n) == "path")
        .expect("path taint entry");
    assert!(
        entry.1.colours.contains(TaintColour::PATH_PREFIXED),
        "expected PATH_PREFIXED colour on /-prefixed literal",
    );
}

#[test]
fn redirect_safe_mask_includes_path_prefixed() {
    assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_PREFIXED));
    assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_NORMALISED));
    // Must not spill into unrelated colours.
    assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::TAINTED));
    assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::CRLF_FREE));
}

/// The compiler and registry colour domains are a bijection.
/// Keep both directions pinned so a future colour cannot be silently
/// dropped by a permissive bitflags conversion.
#[test]
fn registry_taint_colour_bridge_preserves_every_bit_in_both_directions() {
    let registry_all = tcl_registry::TaintColour::all();
    let compiler_all = TaintColour::all();
    assert_eq!(reg_colour(registry_all), compiler_all);
    assert_eq!(compiler_colour(compiler_all), registry_all);
    for colour in [
        tcl_registry::TaintColour::TAINTED,
        tcl_registry::TaintColour::PATH_PREFIXED,
        tcl_registry::TaintColour::NON_DASH_PREFIXED,
        tcl_registry::TaintColour::CRLF_FREE,
        tcl_registry::TaintColour::SHELL_ATOM,
        tcl_registry::TaintColour::LIST_CANONICAL,
        tcl_registry::TaintColour::REGEX_LITERAL,
        tcl_registry::TaintColour::PATH_NORMALISED,
        tcl_registry::TaintColour::PATH_BOUNDED,
        tcl_registry::TaintColour::HEADER_TOKEN_SAFE,
        tcl_registry::TaintColour::HTML_ESCAPED,
        tcl_registry::TaintColour::URL_ENCODED,
        tcl_registry::TaintColour::IP_ADDRESS,
        tcl_registry::TaintColour::PORT,
        tcl_registry::TaintColour::FQDN,
        tcl_registry::TaintColour::PATH_JOINED,
        tcl_registry::TaintColour::CHANNEL,
    ] {
        let compiler = reg_colour(colour);
        assert_eq!(compiler_colour(compiler), colour, "round-trip {colour:?}");
    }
}
