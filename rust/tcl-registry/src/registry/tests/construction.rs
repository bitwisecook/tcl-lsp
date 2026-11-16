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

//! Building and loading: the default catalogue, dialect loading, profile projection, reference bodies and ambient packages.

use super::*;

fn body_backed(name: &'static str, backing: crate::RuntimeBacking) -> crate::CommandSpec {
    crate::CommandSpec {
        name,
        runtime_backing: backing,
        ..crate::CommandSpec::DEFAULT
    }
}

fn text_backing(text: &'static str) -> crate::RuntimeBacking {
    crate::RuntimeBacking::pack_text(text)
}

/// The reference bodies a registry offers are the specs a pack installed,
/// whose backing is a Tcl body this registry holds the text of, and which
/// are still the answer for their name: the text a `PackText` carries or a
/// `PackageSource` resolved to at load, never an embedder's own spec, a
/// backing of another kind, a `PackageSource` nobody read, or a spec a later
/// insertion shadowed.
#[test]
fn a_registry_offers_the_live_pack_installed_bodies_it_holds_text_for() {
    let installed = |registry: &mut CommandRegistry, spec: crate::CommandSpec| {
        let spec: &'static crate::CommandSpec = Box::leak(Box::new(spec));
        registry.insert_static(spec);
        registry.insert_pack_origin(spec, origin());
        spec
    };
    let mut registry = CommandRegistry::build_default();
    let in_text = installed(
        &mut registry,
        body_backed("vendor::text", text_backing("proc vendor::text {} {}")),
    );
    let from_file = installed(
        &mut registry,
        body_backed(
            "vendor::file",
            crate::RuntimeBacking::package_source("lib.tcl"),
        ),
    );
    registry.insert_reference_text(from_file, Arc::from("proc vendor::file {} {}"));
    // Not offered: nobody read the file; the host owns the command; the
    // embedder inserted it; and a later spec shadowed it.
    installed(
        &mut registry,
        body_backed(
            "vendor::unread",
            crate::RuntimeBacking::package_source("absent.tcl"),
        ),
    );
    installed(
        &mut registry,
        body_backed("vendor::native", crate::RuntimeBacking::HostNative),
    );
    registry.insert(body_backed(
        "embedder::text",
        text_backing("proc embedder::text {} {}"),
    ));
    installed(
        &mut registry,
        body_backed(
            "vendor::shadowed",
            text_backing("proc vendor::shadowed {} {}"),
        ),
    );
    registry.insert(body_backed(
        "vendor::shadowed",
        crate::RuntimeBacking::HostNative,
    ));

    let offered: Vec<(&str, &str)> = registry
        .reference_bodies()
        .map(|(spec, text)| (spec.name, text))
        .collect();
    assert_eq!(
        offered,
        vec![
            ("vendor::text", "proc vendor::text {} {}"),
            ("vendor::file", "proc vendor::file {} {}"),
        ]
    );
    assert_eq!(
        registry.reference_body(in_text),
        Some("proc vendor::text {} {}")
    );
    assert_eq!(
        registry.reference_body(from_file),
        Some("proc vendor::file {} {}")
    );
    // A projection for a profile carries the text with the spec.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    assert_eq!(
        registry
            .project_for_profile(profile)
            .reference_bodies()
            .count(),
        2
    );
}

// Ambient packages.

/// Two packs naming the same package are two claims that the runtime
/// provides at least that version. The strongest claim is the one that
/// holds, so the floor is the greatest — never the last declared, which
/// would make the answer depend on pack load order.
#[test]
fn the_highest_declared_ambient_version_is_the_floor() {
    let mut registry = CommandRegistry::build_default();
    assert_eq!(
        registry.ambient_package_floor("Tk"),
        None,
        "a compiled-in registry declares none"
    );

    registry.insert_ambient_package("Tk", "8.6");
    registry.insert_ambient_package("Tk", "8.5");
    registry.insert_ambient_package("Itcl", "4.0");
    assert_eq!(registry.ambient_package_floor("Tk"), Some("8.6"));
    assert_eq!(registry.ambient_package_floor("Itcl"), Some("4.0"));
    assert_eq!(registry.ambient_package_floor("Expect"), None);

    // Order-independent: declaring the higher one last changes nothing.
    let mut reversed = CommandRegistry::build_default();
    reversed.insert_ambient_package("Tk", "8.5");
    reversed.insert_ambient_package("Tk", "8.6");
    assert_eq!(reversed.ambient_package_floor("Tk"), Some("8.6"));
}

#[test]
fn profile_projection_keeps_surface_less_authored_override_of_core_name() {
    let mut registry = CommandRegistry::build_default();
    let mut override_spec = registry.get("set").expect("core set spec").clone();
    override_spec.surface = None;
    override_spec.arity = Arity::exact(17);
    registry.insert(override_spec);

    let profile = crate::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let projected = registry.project_for_profile(profile);
    assert_eq!(
        projected.get("set").map(|spec| spec.arity),
        Some(Arity::exact(17)),
        "the authored catch-all is the effective binding, not Tcl's scoped row"
    );
    assert!(
        projected.get("throw").is_none(),
        "preserving the overlay must not leak Tcl 8.6+ shipped commands into Tcl 8.4"
    );
}

#[test]
fn build_default_has_commands() {
    let reg = CommandRegistry::build_default();
    assert!(!reg.is_empty());
    assert!(reg.get("for").is_some());
    assert!(reg.get("set").is_some());
    assert!(reg.get("nonexistent_command").is_none());
}

#[test]
fn leading_zero_is_octal_tracks_tcl_version() {
    // Plain default registry (no Tcl release row) defaults to octal.
    assert!(CommandRegistry::build_default().leading_zero_is_octal());
    // tcl9.0 (TIP 114) reads leading zeros as decimal; everything else
    // (8.4/8.5/8.6 and the 8.x-derived F5 dialects) stays octal.
    let octal_cases = [
        SurfaceLayer::Core(Family::Tcl, "8.4"),
        SurfaceLayer::Core(Family::Tcl, "8.5"),
        SurfaceLayer::Core(Family::Tcl, "8.6"),
        SurfaceLayer::Core(Family::F5Irules, ""),
        SurfaceLayer::Package("iapps"),
    ];
    for layer in octal_cases {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(layer);
        assert!(reg.leading_zero_is_octal(), "{layer:?} should be octal");
    }
    let mut reg90 = CommandRegistry::build_default();
    reg90.load_surface(SurfaceLayer::Core(Family::Tcl, "9.0"));
    assert!(!reg90.leading_zero_is_octal(), "tcl9.0 should be decimal");
    // tcl9.1 keeps the TIP 114 decimal rule; a tcl9.1-only
    // registry (loads TCL91, not TCL90) must still read leading zeros as
    // decimal.
    let mut reg91 = CommandRegistry::build_default();
    reg91.load_surface(SurfaceLayer::Core(Family::Tcl, "9.1"));
    assert!(!reg91.leading_zero_is_octal(), "tcl9.1 should be decimal");
}

#[test]
fn tcl9_commands_from_pr_433_are_registered() {
    let reg = CommandRegistry::build_default();
    for name in [
        "foreachLine",
        "readFile",
        "writeFile",
        "lpop",
        "const",
        "tcl::idna",
        "::tcl::idna",
        "tcl::process",
        "::tcl::process",
    ] {
        assert!(
            reg.get(name).is_some(),
            "{name} not registered after SYNC-MAY19-tcl9-commands",
        );
    }
}

#[test]
fn coroinject_coroprobe_registered() {
    // Verify these two commands remain registered and visible to
    // the LSP.
    let reg = CommandRegistry::build_default();
    assert!(reg.get("coroinject").is_some());
    assert!(reg.get("coroprobe").is_some());
}

#[test]
fn tcl9_commands_gated_to_tcl90() {
    let reg = CommandRegistry::build_default();
    // `const` (Tcl 9.0, TIP 677) joins the list, gated `TCL90_PLUS`: it
    // does not exist in iRules' embedded Tcl 8.4.6, so it is (correctly)
    // neither pre-9.0 nor iRules-visible.
    for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
        let spec = reg.get(name).expect("registered");
        // A 9.0 addition is available in 9.0 *and* 9.1 (a `.1` release is
        // additive — verified against C Tcl 9.1b0 doc/*.n), so it is gated
        // `TCL90_PLUS`, not `TCL90`-only.
        assert_eq!(
            spec.surface,
            Some(SpecSurface::TCL90_PLUS),
            "{name} should be Tcl 9.0+",
        );
        assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
        assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
        assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
        assert!(!spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))));
    }
}

#[test]
fn every_command_has_hover_with_manpage_source() {
    // Every registered command must carry a hover snippet with a non-empty
    // summary and a manpage/source attribution. A short allowlist covers
    // internal pseudo-commands and dialect placeholders that have no user
    // documentation.
    const HOVERLESS_OK: &[&str] = &["disabled_in_irules"];
    let reg = CommandRegistry::build_default();
    let mut missing_hover = Vec::new();
    let mut missing_source = Vec::new();
    for name in reg.command_names() {
        if HOVERLESS_OK.contains(&name) {
            continue;
        }
        let spec = reg.get(name).expect("registered");
        match &spec.hover {
            None => missing_hover.push(name.to_string()),
            Some(h) => {
                if h.summary.trim().is_empty() || h.source.trim().is_empty() {
                    missing_source.push(name.to_string());
                }
            }
        }
    }
    missing_hover.sort();
    missing_source.sort();
    assert!(
        missing_hover.is_empty(),
        "commands without a hover snippet: {missing_hover:?}",
    );
    assert!(
        missing_source.is_empty(),
        "commands with an empty hover summary or manpage source: {missing_source:?}",
    );
}

#[test]
fn timerate_registered_with_body_and_int_hint() {
    // `timerate` measures the rate of execution of a script.
    use crate::arg_role::ArgRole;
    use crate::side_effects::SideEffectTarget;
    use crate::types::TclType;
    let reg = CommandRegistry::build_default();
    let spec = reg.get("timerate").expect("timerate registered");
    assert_eq!(spec.name, "timerate");
    // BODY role on arg 0, INT type hint (shimmers) on arg 1.
    assert_eq!(spec.arg_role_at(0), Some(ArgRole::Body));
    assert_eq!(
        spec.arg_types
            .iter()
            .find(|(i, _)| *i == 1)
            .map(|(_, h)| h.expected),
        Some(Some(TclType::Int)),
    );
    // Unbounded arity: at least the command word, no upper bound.
    assert!(!spec.arity.accepts(0));
    assert!(spec.arity.accepts(1));
    assert!(spec.arity.accepts(6));
    assert_eq!(spec.return_type, Some(TclType::String));
    // The body runs arbitrary code → an UNKNOWN-target read+write effect.
    assert!(
        spec.side_effects
            .lower_bound()
            .iter()
            .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
        "timerate should declare an UNKNOWN read+write side effect",
    );
}

#[test]
fn load_irules_dialect() {
    let mut reg = CommandRegistry::build_default();
    assert!(reg.get("HTTP::header").is_none()); // not loaded yet
    reg.load_irules();
    assert!(reg.get("HTTP::header").is_some());
    assert!(reg.len() > 200); // should have 1000+ commands now
}

#[test]
fn irules_command_has_irules_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let spec = reg.get("HTTP::header").unwrap();
    assert_eq!(spec.surface, Some(SpecSurface::IRULES));
}

#[test]
fn irules_idempotent_load() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let count1 = reg.len();
    reg.load_irules(); // second load should be no-op
    assert_eq!(reg.len(), count1);
}

#[test]
fn default_includes_stdlib_and_tcllib() {
    let reg = CommandRegistry::build_default();
    // stdlib and tcllib are loaded by default
    assert!(reg.len() > 200);
}

#[test]
fn load_tk_dialect() {
    // Tk is part of the always-known base registry now, so it is present
    // by default and a later `load_surface(Tk)` is an idempotent no-op.
    let reg = CommandRegistry::build_default();
    assert!(
        reg.get("grid").is_some(),
        "Tk commands are loaded by default"
    );
    let base_count = reg.len();
    let mut reg2 = CommandRegistry::build_default();
    reg2.load_surface(SurfaceLayer::Package("Tk"));
    assert_eq!(
        reg2.len(),
        base_count,
        "loading the Tk surface is a no-op after the default load"
    );
}

#[test]
fn load_iapps_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Package("iapps"));
    assert!(reg.len() > 100);
}

#[test]
fn load_expect_dialect() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Package("expect"));
    assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
}
