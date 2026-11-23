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

//! IRULE3001 to IRULE3004 through the full pass: response bodies, header values, logs, redirects and mitigating colours.

use super::*;

// IRULE3001–3004 sink classifier + end-to-end detection

fn irules_warnings_for(source: &str) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = registry.project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(source, &registry, false, profile)
        .with_interprocedural(&registry, Some(profile));
    crate::taint::find_taint_warnings_for_cu(&cu, &registry, Some(profile))
}

#[test]
fn irule3001_fires_on_tainted_respond_body() {
    let w = irules_warnings_for("set u [HTTP::uri]\nHTTP::respond 200 content $u");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3001),
        "expected IRULE3001, got {w:?}"
    );
}

#[test]
fn irule3001_no_warning_for_literal_body() {
    let w = irules_warnings_for("HTTP::respond 200 content \"hello\"");
    assert!(
        w.iter().all(|x| x.code != DiagCode::Irule3001),
        "expected no IRULE3001 on literal body, got {w:?}"
    );
}

#[test]
fn irule3002_fires_on_tainted_header_value() {
    let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header insert X-Echo $v");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3002),
        "expected IRULE3002, got {w:?}"
    );
}

#[test]
fn irule3002_fires_in_command_sub_form() {
    // Regression: `set _ [HTTP::header insert X-Foo $v]` — sink is
    // inside a command substitution, so the AssignValue branch must
    // preserve the subcommand args so classify_irules_sink sees
    // "insert" at arg-index 0.
    let w =
        irules_warnings_for("set v [HTTP::header X-Src]\nset _ [HTTP::header insert X-Echo $v]");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3002),
        "expected IRULE3002 inside command-sub sink, got {w:?}"
    );
}

#[test]
fn irule3002_skipped_on_remove_subcommand() {
    let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header remove $v");
    assert!(
        w.iter().all(|x| x.code != DiagCode::Irule3002),
        "remove subcommand must not fire IRULE3002, got {w:?}"
    );
}

#[test]
fn irule3003_fires_on_tainted_log() {
    let w = irules_warnings_for("set u [HTTP::uri]\nlog local0.info $u");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3003),
        "expected IRULE3003, got {w:?}"
    );
}

#[test]
fn irule3004_fires_on_tainted_redirect() {
    let w = irules_warnings_for("set target [HTTP::header Location]\nHTTP::redirect $target");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3004),
        "expected IRULE3004, got {w:?}"
    );
}

#[test]
fn irule3004_redirect_safe_suppresses_via_lattice() {
    // Direct lattice check: tainted + REDIRECT_SAFE must suppress.
    // (Latent as an end-to-end test until iRules sources are tagged
    // with PATH_PREFIXED from their `taint_hints` — `HTTP::path`
    // carries PATH_PREFIXED on the getter form.)
    let lat = TaintLattice::tainted().with(TaintColour::PATH_PREFIXED);
    assert!(sink_colour_mitigated(DiagCode::Irule3004, lat));
    let lat = TaintLattice::tainted().with(TaintColour::PATH_NORMALISED);
    assert!(sink_colour_mitigated(DiagCode::Irule3004, lat));
    // Plain tainted should not suppress.
    assert!(!sink_colour_mitigated(
        DiagCode::Irule3004,
        TaintLattice::tainted()
    ));
}

#[test]
fn irule_sinks_do_not_fire_without_dialect() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // Without `with_interprocedural(Some(tcl_dialect::DialectProfile::irules()))`, no dialect
    // is active, and HTTP commands aren't sinks.
    let cu = CompilationUnit::build_for(
        "set u [HTTP::uri]\nHTTP::respond 200 content $u",
        &registry,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    );
    assert!(
        warnings
            .iter()
            .all(|w| !w.code.as_str().starts_with("IRULE")),
        "no IRULE warnings without dialect, got {warnings:?}"
    );
}

#[test]
fn sink_colour_mitigated_html_escaped_only_mitigates_3001() {
    let tainted_html = TaintLattice::tainted().with(TaintColour::HTML_ESCAPED);
    // IRULE3001 (HTTP response body) — HTML_ESCAPED directly mitigates.
    assert!(sink_colour_mitigated(DiagCode::Irule3001, tainted_html));
    // IRULE3002/3003 (header / log) — HTML_ESCAPED does NOT prove
    // CRLF-injection safety (the escape rewrites `<`/`>`/`&` but
    // leaves raw CR/LF untouched). The CRLF-safe mask excludes
    // `HTML_ESCAPED`.
    assert!(!sink_colour_mitigated(DiagCode::Irule3002, tainted_html));
    assert!(!sink_colour_mitigated(DiagCode::Irule3003, tainted_html));
    // IRULE3004 (redirect) — also not mitigated by HTML_ESCAPED.
    assert!(!sink_colour_mitigated(DiagCode::Irule3004, tainted_html));

    // `CRLF_FREE` does suppress IRULE3002/3003 (the one mitigation
    // accepted in the value position).
    let tainted_crlf_free = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
    assert!(sink_colour_mitigated(
        DiagCode::Irule3002,
        tainted_crlf_free
    ));
    assert!(sink_colour_mitigated(
        DiagCode::Irule3003,
        tainted_crlf_free
    ));
}

#[test]
fn irule3002_detects_the_measured_header_setter_forms() {
    for call in [
        "HTTP::header X-Echo $value",
        "HTTP::header value X-Echo $value",
        "HTTP::header insert X-Echo $value",
    ] {
        let warnings = irules_warnings_for(&format!("set value [HTTP::header X-Input]\n{call}"));
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == DiagCode::Irule3002),
            "{call}"
        );
    }
}

#[test]
fn irule3002_header_token_safe_name_position_suppresses() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let args = vec!["insert".to_owned(), "$name".to_owned(), "$value".to_owned()];
    let config = TaintSourceContext::standalone(registry, registry.profile()).config;
    let name_word = crate::value_shapes::value_word_with_config(&args[1], config).unwrap();
    let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
    // Var `name` occupies arg-index 1 (name position) → suppressed.
    assert!(irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "name",
        lat,
        Some(&name_word),
        config,
    ));
    // Var `value` occupies arg-index 2 (value position) → not suppressed.
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "value",
        lat,
        Some(&name_word),
        config,
    ));
    // Without HEADER_TOKEN_SAFE colour: never suppressed, even at name position.
    let plain = TaintLattice::tainted();
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "name",
        plain,
        Some(&name_word),
        config,
    ));
    // Wrong subcommand: not suppressed.
    let rm_args = vec!["remove".to_owned(), "$name".to_owned()];
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &rm_args,
        "name",
        lat,
        Some(&name_word),
        config,
    ));
}

/// The iRules sink classification is registry-driven (via
/// `TAINT_SINK` and `EVALUATES_CODE` traits): tainted data into
/// `expr` (which carries `TAINT_SINK`) raises T100. The trait
/// is the single source of truth — flipping it removes the
/// classification.
#[test]
fn arch3_irules_sink_is_registry_driven() {
    let registry = CommandRegistry::build_default();
    let expr_spec = registry.get("expr").expect("expr in registry");
    assert!(
        expr_spec.traits.contains(tcl_registry::Traits::TAINT_SINK),
        "expr must carry TAINT_SINK in the registry",
    );

    // End-to-end: tainted iRules data (`HTTP::uri`) into expr
    // surfaces T100 under the iRules dialect.
    let w = irules_warnings_for("set u [HTTP::uri]\nexpr $u");
    assert!(
        w.iter().any(|d| d.code == DiagCode::T100),
        "expected T100 on tainted expr sink under iRules, got {w:?}",
    );
}
