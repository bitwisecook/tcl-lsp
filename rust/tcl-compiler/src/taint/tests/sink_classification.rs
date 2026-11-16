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

//! Sink classification from the registry: network and interpreter sinks, iRules sinks and their dialect gate.

use super::*;

#[test]
fn classify_network_interp_sinks_maps_socket_and_interp() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        classify_network_interp_sinks(&reg, "socket", &["host".into(), "80".into()]),
        vec![(DiagCode::T104, "socket".to_owned())]
    );
    assert_eq!(
        classify_network_interp_sinks(&reg, "interp", &["eval".into(), "$c".into()]),
        vec![(DiagCode::T105, "interp eval".to_owned())]
    );
    // A non-eval interp subcommand and a plain command map to nothing.
    assert_eq!(
        classify_network_interp_sinks(&reg, "interp", &["share".into()]).len(),
        0
    );
    assert_eq!(
        classify_network_interp_sinks(&reg, "puts", &["hi".into()]).len(),
        0
    );
}

#[test]
fn classify_irules_sink_http_respond() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::respond",
        &[],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3001));
}

#[test]
fn classify_irules_sink_http_header_insert() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::header",
        &["insert".to_owned(), "X-Foo".to_owned(), "bar".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::header insert");
}

#[test]
fn classify_irules_sink_http_cookie_replace() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::cookie",
        &["replace".to_owned(), "sid".to_owned(), "val".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie replace");
}

#[test]
fn classify_irules_sink_prefix_abbreviation() {
    // `HTTP::cookie ins` is a legal abbreviation of `insert`
    // and must still classify as the IRULE3002 output sink.
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::cookie",
        &["ins".to_owned(), "sid".to_owned(), "val".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    // The label reports the canonical subcommand name.
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie insert");
}

#[test]
fn classify_irules_sink_http_header_remove_is_none() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::header",
        &["remove".to_owned(), "X-Foo".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert!(hit.is_none(), "remove subcommand must not emit IRULE3002");
}

#[test]
fn classify_irules_sink_log_and_redirect() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    assert_eq!(
        classify_sink(
            reg,
            "log",
            &["local0.info".to_owned(), "x".to_owned()],
            Some(tcl_dialect::DialectProfile::irules())
        )
        .as_ref()
        .map(|(c, _)| *c),
        Some(DiagCode::Irule3003),
    );
    assert_eq!(
        classify_sink(
            reg,
            "HTTP::redirect",
            &["https://evil".to_owned()],
            Some(tcl_dialect::DialectProfile::irules())
        )
        .as_ref()
        .map(|(c, _)| *c),
        Some(DiagCode::Irule3004),
    );
}

#[test]
fn classify_sink_skips_irules_without_dialect() {
    let registry = CommandRegistry::build_default();
    let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
    assert!(hit.is_none(), "no dialect → no IRULE3001, got {hit:?}");
}

#[test]
fn classify_sink_skips_irules_without_dialect_even_when_specs_loaded() {
    // Stronger than `classify_sink_skips_irules_without_dialect`: the
    // registry here *does* have the iRules specs loaded (as a shared
    // multi-dialect registry might), so `registry.get("HTTP::respond")`
    // succeeds. The IRULE3001 classification must still be withheld
    // because the active *document* dialect (`None`) isn't iRules —
    // mirrors `find_setter_constraint_warnings`'s explicit
    // `is_irules_dialect` gate for IRULE3101.
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    assert!(registry.get("HTTP::respond").is_some());
    let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
    assert!(
        hit.is_none(),
        "no dialect → no IRULE3001 even with iRules specs loaded, got {hit:?}"
    );
}

#[test]
fn classify_sink_exec_stays_t100_under_irules_dialect() {
    // `exec` is universal spec data (`surface: None`); under f5-irules
    // it is *banned* by the profile's §9 disable list, not by its own
    // surface. T100
    // classification must not route through profile availability —
    // it's a plain trait fact, independent of the active document
    // dialect — or `exec` would lose its T100 sink status in exactly
    // the dialect where flagging it matters most.
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    let hit = classify_sink(
        &registry,
        "exec",
        &["$cmd".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::T100));
}

// registry-driven source / sink / setter-constraint coverage

/// The Tcl-core source classification flows from the registry's
/// [`Traits::TAINT_SOURCE`] flag: registry-side query and
/// end-to-end taint-warning emission must agree on `gets` /
/// `read` / `exec` / `socket`.
#[test]
fn arch3_tcl_core_source_is_registry_driven() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // Registry-side: TAINT_SOURCE trait is stamped on these.
    for cmd in ["gets", "read", "exec", "socket"] {
        let spec = registry.get(cmd).unwrap_or_else(|| panic!("{cmd} missing"));
        assert!(
            spec.traits.contains(tcl_registry::Traits::TAINT_SOURCE),
            "{cmd} must carry TAINT_SOURCE in the registry",
        );
    }
    // Same fact via the registry-side taint helper.
    assert!(tcl_registry::taint::is_taint_source(
        &registry,
        "gets",
        &["stdin"],
        None,
    ));

    // End-to-end: `gets` → `eval` raises T100. The fact reaches
    // the diagnostic via the registry, not via a compiler-side
    // command-name table.
    let cu = CompilationUnit::build_for("set x [gets stdin]\neval $x", &registry, false)
        .with_interprocedural(&registry, None);
    let mut warnings: Vec<TaintWarning> = Vec::new();
    for fu in cu.analysable_functions() {
        warnings.extend(find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        ));
    }
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 from gets→eval with registry-driven source, got {warnings:?}",
    );
}
