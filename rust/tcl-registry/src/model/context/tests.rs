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

use super::*;
use crate::model::surface::declarations_for_spec;
use crate::spec::CommandSpec;
use tcl_dialect::model::EnvironmentRegistry;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_breadth;
use tcl_dialect::surface;

fn context(environment: &str) -> ResolvedContext {
    let registry = EnvironmentRegistry::compiled();
    let definition = registry.resolve(environment).expect(environment);
    ResolvedContext::resolve(definition, &KeyedVersions::default())
}

fn rows(surface: Option<&'static [SpecSurface]>) -> Vec<SurfaceDeclaration> {
    declarations_for_spec(&CommandSpec {
        name: "context-test",
        surface,
        ..CommandSpec::DEFAULT
    })
    .into_vec()
}

#[test]
fn selected_surface_reuses_the_same_admission_without_changing_resolution() {
    use tcl_dialect::model::InvocationRealm;
    let registry = crate::model::assembly::universe();
    let mut checks = 0usize;
    for profile in tcl_dialect::DialectProfile::all() {
        let ctx = context(profile.name);
        for realm in [
            InvocationRealm::RuleLoader,
            InvocationRealm::InterpreterRuntime,
        ] {
            let query = ctx.authoring_query().with_realm(realm);
            for name in registry
                .command_names()
                .chain(std::iter::once("not_a_surface_command"))
            {
                for spelling in [name.to_owned(), format!("::{name}")] {
                    let selected = registry.get_for_surface(&spelling, Some(query));
                    if let Some(spec) = selected {
                        assert!(crate::irules_policy::runtime_surface_admits(spec, query));
                    }
                    let full = selected.filter(|spec| {
                        ctx.spec_available_under(spec, query) && ctx.roster_admits(spec, &query)
                    });
                    let actual = ctx.resolve_spec_in_realm(registry, &spelling, realm);
                    assert!(
                        match (full, actual) {
                            (Some(left), Some(right)) => std::ptr::eq(left, right),
                            (None, None) => true,
                            _ => false,
                        },
                        "profile={} realm={realm:?} name={spelling}",
                        profile.name
                    );
                    checks += 1;
                }
            }
        }
    }
    assert!(checks > 1000);
}

#[test]
fn selected_surface_still_refuses_missing_package_without_wider_fallback() {
    // A hosted library (`Tk`) is assumed present in an open world, so the
    // missing package is one a vendor surface owns.
    let mut registry = CommandRegistry::build_default();
    for (surface, required_package) in [(None, None), (Some(SpecSurface::TCL86), Some("expect"))] {
        registry.insert(CommandSpec {
            name: "post-surface-package-control",
            surface,
            required_package,
            ..CommandSpec::DEFAULT
        });
    }
    let older = context("tcl8.6");
    let newer = context("tcl9.0");
    for name in [
        "post-surface-package-control",
        "::post-surface-package-control",
    ] {
        let winner = registry
            .get_for_surface(name, Some(older.authoring_query()))
            .unwrap();
        assert_eq!(winner.required_package, Some("expect"));
        assert!(!older.spec_available(winner));
        assert!(older.resolve_spec(&registry, name).is_none());
        assert_eq!(
            newer
                .resolve_spec(&registry, name)
                .unwrap()
                .required_package,
            None
        );
    }
}

#[test]
fn catalogue_case_advice_keeps_exact_names_availability_and_ambiguity() {
    let mut registry = CommandRegistry::build_default();
    for (name, surface) in [
        ("CaseItem", None),
        ("CASEITEM", Some(SpecSurface::TCL90_PLUS)),
        ("caseitem ", None),
        ("", None),
    ] {
        registry.insert(CommandSpec {
            name,
            surface,
            ..CommandSpec::DEFAULT
        });
    }
    let older = context("tcl8.6");
    let newer = context("tcl9.0");
    assert_eq!(
        older
            .resolve_spec_case_advice(&registry, "caseitem")
            .unwrap()
            .name,
        "CaseItem"
    );
    assert!(
        newer
            .resolve_spec_case_advice(&registry, "caseitem")
            .is_none()
    );
    for name in ["CaseItem", "CASEITEM", "caseitem ", ""] {
        assert_eq!(
            newer
                .resolve_spec_case_advice(&registry, name)
                .unwrap()
                .name,
            name
        );
    }
    assert!(
        older
            .resolve_spec_case_advice(&registry, " caseitem")
            .is_none()
    );
    assert!(
        older
            .resolve_spec_case_advice(&registry, "missing")
            .is_none()
    );
}

#[test]
// naming.core.original-math-function-selection
// docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
fn source_math_catalogue_retains_core_release_build_and_unknown() {
    assert_eq!(
        context("tcl8.6").source_math_function("isnan"),
        CapabilityAnswer::No
    );
    assert_eq!(
        context("tcl9.0").source_math_function("isnan"),
        CapabilityAnswer::Yes
    );
    assert_eq!(
        context("tcl8.6").source_math_function("sqrt"),
        CapabilityAnswer::Yes
    );
    assert_eq!(
        context("jim").source_math_function("min"),
        CapabilityAnswer::No
    );
    assert_eq!(
        context("jim").source_math_function("sqrt"),
        CapabilityAnswer::Yes
    );
    let mut minimal = context("jim");
    Arc::make_mut(&mut minimal.environment)
        .core
        .as_mut()
        .unwrap()
        .build = tcl_dialect::model::BuildProfileId::JimMinimal;
    assert_eq!(minimal.source_math_function("sqrt"), CapabilityAnswer::No);
    assert_eq!(minimal.source_math_function("int"), CapabilityAnswer::Yes);
    let mut compatibility = context("tcl8.6");
    compatibility.declare_targets(
        targets_from_clauses(&VersionAxisId::core(Family::Tcl), &["9.0"]).unwrap(),
    );
    assert_eq!(
        compatibility.source_math_function("isnan"),
        CapabilityAnswer::No
    );
    let mut unknown_build = context("jim");
    Arc::make_mut(&mut unknown_build.environment)
        .core
        .as_mut()
        .unwrap()
        .build = tcl_dialect::model::BuildProfileId::Unknown;
    assert_eq!(
        unknown_build.source_math_function("sqrt"),
        CapabilityAnswer::Unknown
    );
    let mut unknown = context("tcl8.6");
    Arc::make_mut(&mut unknown.environment).core = None;
    assert_eq!(
        unknown.source_math_function("sqrt"),
        CapabilityAnswer::Unknown
    );
}

#[test]
fn core_floors_carry_the_environment_selection() {
    let ctx = context("tcl8.6");
    let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
    assert_eq!(
        ctx.floors.primary(&axis),
        Some(&Version::parse("8.6").expect("version"))
    );
    let targets = ctx.floors.targets(&axis).expect("core targets");
    assert!(targets.contains(&Version::parse("8.6.16").expect("version")));
    assert!(!targets.contains(&Version::parse("9.0").expect("version")));
}

#[test]
fn keyed_placements_resolve_through_keyed_versions() {
    let ctx = context("f5-irules");
    let axis = VersionAxisId::package("f5-irules-cmds");
    // The D5 default: oldest supported TMOS.
    assert_eq!(
        ctx.floors.primary(&axis),
        Some(&Version::parse("16.1.0").expect("version"))
    );
    let registry = EnvironmentRegistry::compiled();
    let pinned = ResolvedContext::resolve(
        registry.resolve("f5-irules").expect("irules"),
        &KeyedVersions {
            bigip: Some(Version::parse("17.1.0").expect("version")),
            ..KeyedVersions::default()
        },
    );
    assert_eq!(
        pinned.floors.primary(&axis),
        Some(&Version::parse("17.1.0").expect("version"))
    );
}

#[test]
fn keyed_versions_mirror_library_version_overrides() {
    let overrides = LibraryVersionOverrides {
        bigip_version: Some("17.1.0".to_owned()),
        ..LibraryVersionOverrides::default()
    };
    let keyed = KeyedVersions::from_overrides(&overrides).expect("valid overrides");
    assert_eq!(
        keyed.resolve(KeyedAxis::BigipVersion),
        Some(Version::parse("17.1.0").expect("version"))
    );
    assert_eq!(keyed.resolve(KeyedAxis::ToolVersion), None);
    let malformed = LibraryVersionOverrides {
        bigip_version: Some("not a version".to_owned()),
        ..LibraryVersionOverrides::default()
    };
    assert!(KeyedVersions::from_overrides(&malformed).is_err());
    // Distinct pins hash distinctly; the default is stable.
    assert_eq!(
        KeyedVersions::default().content_hash(),
        KeyedVersions::default().content_hash()
    );
    assert_ne!(
        keyed.content_hash(),
        KeyedVersions::default().content_hash()
    );
}

#[test]
fn core_rows_follow_the_environment_family_and_primary() {
    let tcl85_plus = rows(Some(SpecSurface::TCL85_PLUS));
    assert!(context("tcl8.6").is_available(&tcl85_plus));
    assert!(context("tcl9.1").is_available(&tcl85_plus));
    assert!(!context("tcl8.4").is_available(&tcl85_plus));
    // A core row never fires under another family or a coreless
    // environment.
    assert!(!context("f5-irules").is_available(&tcl85_plus));
    assert!(!context("f5-bigip").is_available(&tcl85_plus));
    let irules_only = rows(Some(SpecSurface::IRULES));
    assert!(context("f5-irules").is_available(&irules_only));
    assert!(!context("tcl8.4").is_available(&irules_only));
}

#[test]
fn world_policy_gates_package_rows() {
    let expect_only = rows(Some(SpecSurface::EXPECT));
    // The vendor surface is active in its own environment only.
    assert!(context("expect").is_available(&expect_only));
    for other in ["tcl8.6", "f5-irules", "f5-iapps", "spectcl", "f5-bigip"] {
        assert!(!context(other).is_available(&expect_only), "{other}");
    }
    // The bridge covers surfaces with no compiled placement at all.
    let spectcl_only = rows(Some(SpecSurface::SPECTCL));
    assert!(context("spectcl").is_available(&spectcl_only));
    assert!(!context("tcl9.0").is_available(&spectcl_only));
    let shared = rows(Some(surface![
        SpecSurface::package("iapps"),
        SpecSurface::package("tmsh")
    ]));
    assert!(context("f5-iapps").is_available(&shared));
    assert!(context("f5-tmsh").is_available(&shared));
    assert!(!context("tcl8.5").is_available(&shared));
}

#[test]
fn the_none_translation_is_available_everywhere() {
    let universal = rows(None);
    for environment in [
        "tcl8.4",
        "tcl9.1",
        "tcl",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "f5-bigip",
        "expect",
        "spectcl",
        "bpf",
        "cadence-eda-tcl",
    ] {
        assert!(
            context(environment).is_available(&universal),
            "{environment}"
        );
    }
}

#[test]
fn an_unplaced_hosted_package_row_needs_an_explicit_require() {
    // The `tcltest::bytestring` shape: an 8.x-gated spec whose hosted
    // package must not resurrect it under 9.x through the package row.
    let spec = CommandSpec {
        name: "context-test",
        surface: Some(SpecSurface::TCL8X),
        required_package: Some("tcltest"),
        tcllib_package: None,
        ..CommandSpec::DEFAULT
    };
    let declarations = declarations_for_spec(&spec);
    assert!(!context("tcl9.0").is_available(&declarations));
    assert!(context("tcl8.6").is_available(&declarations));
    // An explicit require puts the hosted package in the world.
    let mut required = context("tcl9.0");
    required.require_package("tcltest", None);
    assert!(required.is_available(&declarations));
    // …but not under a closed world.
    let mut closed = context("f5-irules");
    closed.require_package("tcltest", None);
    assert!(!closed.is_available(&declarations));
}

#[test]
fn a_closed_world_require_constrains_across_environments() {
    let spec = CommandSpec {
        name: "context-test",
        surface: Some(SpecSurface::IRULES),
        required_package: Some("f5-irules-cmds"),
        ..CommandSpec::DEFAULT
    };
    let declarations = declarations_for_spec(&spec);
    assert!(context("f5-irules").is_available(&declarations));
    assert!(!context("tcl8.4").is_available(&declarations));
    // Selection admits it; the package conjunct is the
    // winner filter, mirroring `get_for_surface → is_available`.
    assert!(
        declarations
            .iter()
            .any(|row| context("f5-irules").admits_for_selection(row))
    );
}

// Tk's placement model.

/// The central claim, stated as one table: `Tk` is one
/// package whose availability is decided by **placement plus policy**,
/// and the three answers a caller can want are three distinct queries
/// that disagree in exactly the right places.
#[test]
fn tk_is_one_package_with_three_placement_answers() {
    // (environment, in the world, ambient — no require needed, hostable)
    for (environment, active, ambient, hostable) in [
        // `wish`: ambient. Everything true, and W120 owns the silence.
        ("tk", true, true, true),
        // A release-pinned `tclsh`: hosted. Visible under the open
        // world (§5.3's lenient default), but not shipped, so W120 nags
        // and Tk stays out of the authoring point.
        ("tcl8.6", true, false, true),
        ("tcl9.0", true, false, true),
        // The lenient sink declares the same hosted placement.
        ("tcl", true, false, true),
        // A closed world is exactly its ambient closure: `package
        // require` is not part of the language, so Tk is neither
        // active nor hostable.
        ("f5-irules", false, false, false),
        ("bpf", false, false, false),
        ("spectcl", false, false, false),
        // Ambient-plus-require shells host nothing they did not place,
        // and only what they required joins the world (Q7).
        ("f5-iapps", false, false, false),
        ("f5-tmsh", false, false, false),
        // An EDA shell is open, so the lenient hosted rule applies —
        // but it declares no Tk placement, so it cannot host one.
        ("xilinx-eda-tcl", true, false, false),
    ] {
        let ctx = context(environment);
        assert_eq!(ctx.package_active("Tk"), active, "{environment} active");
        assert_eq!(ctx.ambient_package("Tk"), ambient, "{environment} ambient");
        assert_eq!(
            ctx.can_host_package("Tk"),
            hostable,
            "{environment} hostable"
        );
    }
}

/// `Tk` is ambient somewhere **and** hosted elsewhere, which is what
/// keeps it out of the closed-world vocabulary: reading ambience alone
/// would classify it as a vendor runtime, and every Tk command would
/// vanish from plain Tcl.
#[test]
fn tk_is_a_library_with_an_ambient_host_not_a_vendor_surface() {
    use crate::model::surface::{is_closed_world_package, is_placement_gated_package};
    assert!(!is_closed_world_package("Tk"));
    assert!(is_placement_gated_package("Tk"));
    assert!(context("tk").placement_is_ambient("Tk"));
    assert!(context("tcl8.6").can_host_package("Tk"));
    assert!(!context("tcl8.6").placement_is_ambient("Tk"));
    // The vendor runtimes stay closed-world; the unplaced libraries
    // stay ungated (W120 owns their nag).
    for vendor in ["f5-irules-cmds", "f5-iapps-cmds", "Expect"] {
        assert!(is_closed_world_package(vendor), "{vendor}");
    }
    for library in ["Itcl", "csv", "struct::graph", "tcltest"] {
        assert!(!is_closed_world_package(library), "{library}");
        assert!(!is_placement_gated_package(library), "{library}");
    }
}

/// Every `required_package: Some("Tk")` spec now carries the package
/// conjunct, so the Tk surface resolves through
/// [`ResolvedContext::package_active`] rather than off its
/// `Core(Tcl)` row. The surviving core row is specificity data — the
/// breadth `get_for_surface`'s specificity ordering needs —
/// which is why it is asserted here rather than deleted.
#[test]
fn tk_declarations_are_gated_on_the_package_provider() {
    use crate::model::surface::{CapabilityPredicate, PackageId, Provider};
    let spec = CommandSpec {
        name: "context-test",
        surface: Some(SpecSurface::TK_AND_TCL),
        required_package: Some("Tk"),
        ..CommandSpec::DEFAULT
    };
    let declarations = declarations_for_spec(&spec);
    assert!(
        declarations
            .iter()
            .all(|row| row.predicate == CapabilityPredicate::RequiresPackage(PackageId::new("Tk"))),
        "{declarations:?}"
    );
    assert!(
        declarations
            .iter()
            .any(|row| row.provider == Provider::Package(PackageId::new("Tk")))
    );
    // Specificity is unchanged: five Tcl ladder releases + the Tk
    // vendor package = the 6 `TK_AND_TCL` states.
    assert_eq!(specificity_breadth(&declarations), 6);
    // …and availability follows the placement query exactly: open
    // worlds resolve the hosted rows, an ambient-plus-require shell
    // that never required Tk does not.
    for environment in ["tk", "tcl8.6", "tcl"] {
        assert!(
            context(environment).is_available(&declarations),
            "{environment}"
        );
    }
    assert!(!context("f5-iapps").is_available(&declarations));
}

/// The **one enumerated delta** from the old model, pinned directly rather
/// than only as an allowlist in the parity sweeps: a world that is not open
/// stops resolving the Tk surface on its own. `package require` is not
/// part of the `bpf`, `spectcl` or `f5-irules` language, and an iApp gets
/// only what it requires, so `wm` was never callable in any of them; the
/// retired profile point admitted it only because `TK_AND_TCL` unions the
/// whole Tcl ladder.
///
/// The two policies part company on the require: it cannot open a
/// closed world, but it is exactly how a package joins an
/// ambient-plus-require one (Q7).
#[test]
fn tk_needs_an_open_world_or_an_explicit_require() {
    let spec = CommandSpec {
        name: "context-test",
        surface: Some(SpecSurface::TK_AND_TCL),
        required_package: Some("Tk"),
        ..CommandSpec::DEFAULT
    };
    let declarations = declarations_for_spec(&spec);
    for shut in ["bpf", "spectcl", "f5-irules", "f5-iapps", "f5-tmsh"] {
        assert!(!context(shut).is_available(&declarations), "{shut}");
        assert!(
            !context(shut).required_package_available(Some("Tk")),
            "{shut}"
        );
    }
    // An explicit require cannot open a closed world…
    let mut required = context("spectcl");
    required.require_package("Tk", None);
    assert!(!required.is_available(&declarations));
    // …but it is the whole point of an ambient-plus-require one.
    let mut required = context("f5-iapps");
    required.require_package("Tk", None);
    assert!(required.is_available(&declarations));
    // Nothing else moves: an unplaced library keeps the old lenient
    // answer under the same closed worlds.
    for closed in ["bpf", "spectcl"] {
        assert!(
            context(closed).required_package_available(Some("Itcl")),
            "{closed}"
        );
        assert!(
            context(closed).required_package_available(Some("csv")),
            "{closed}"
        );
    }
}

#[test]
fn available_at_targets_names_the_covered_subset() {
    let ctx = context("tcl");
    let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
    let tcl86_plus = rows(Some(SpecSurface::TCL86_PLUS));
    let covered = ctx.available_at_targets(&tcl86_plus, &axis);
    assert!(covered.contains(&Version::parse("8.6").expect("version")));
    assert!(covered.contains(&Version::parse("9.1").expect("version")));
    assert!(!covered.contains(&Version::parse("8.5.19").expect("version")));
    // The full-set check: targets ⊆ covered fails for a gated spec…
    let targets = ctx.floors.targets(&axis).expect("targets");
    assert!(!targets.subset(&covered).expect("same axis"));
    // …and holds for an everywhere spec.
    let everywhere = ctx.available_at_targets(&rows(Some(SpecSurface::ALL_TCL)), &axis);
    assert!(targets.subset(&everywhere).expect("same axis"));
    // Another provider's rows never contribute to this axis.
    let vendor = ctx.available_at_targets(&rows(Some(SpecSurface::EXPECT)), &axis);
    assert!(vendor.is_empty());
}

/// A `jim` context resolves the shared core surface through
/// its ancestry edge instead of through 76 re-authored specs: the
/// `Core(Tcl)` provider is active, the Tcl-axis primary is the 8.6
/// anchor, and the derived point is jim's own family followed by the
/// 8.6 line — so `lassign` (8.5+) and `lmap` (8.6+) both resolve while
/// an 8.4-only shape does not.
#[test]
fn a_jim_context_inherits_the_tcl_core_surface() {
    let ctx = context("jim");
    let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
    let tcl_axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);

    assert!(ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::Jim)));
    assert!(
        ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::Tcl)),
        "the ancestry edge is what admits the shared core specs"
    );
    assert!(!ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::F5Irules)));

    // The lineage floor is the edge's anchor, on the *Tcl* axis.
    assert_eq!(
        ctx.floors.primary(&tcl_axis).map(Version::as_str),
        Some(tcl_dialect::model::Family::JIM_SURFACE_ANCHOR)
    );
    // Jim's own axis spans the ladder and takes no point primary.
    assert!(ctx.floors.primary(&jim_axis).is_none());
    assert!(
        ctx.floors
            .targets(&jim_axis)
            .expect("jim targets")
            .contains(&Version::parse("0.76").expect("version"))
    );

    // The derived point, and what it admits.
    assert_eq!(
        ctx.authoring_query(),
        SurfaceQuery {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            core: CorePoints::two((Family::Jim, None), (Family::Tcl, Some("8.6"))),
            packages: tcl_dialect::model::PackageFloors::Borrowed(&[]),
        }
    );
    for gate in [
        SpecSurface::ALL_TCL,
        SpecSurface::TCL85_PLUS,
        SpecSurface::TCL86_PLUS,
    ] {
        assert!(
            ctx.spec_available(&CommandSpec {
                name: "context-test",
                surface: Some(gate),
                ..CommandSpec::DEFAULT
            }),
            "{gate:?}"
        );
    }
    assert!(
        !ctx.spec_available(&CommandSpec {
            name: "context-test",
            surface: Some(SpecSurface::TCL84),
            ..CommandSpec::DEFAULT
        }),
        "an 8.4-only shape is not part of the 8.6 command set"
    );
    // A real command, resolved from the shared catalogue: this is
    // the 76-commands-by-hand line item, gone.
    let registry = crate::model::assembly::universe();
    for name in ["set", "if", "proc", "lassign", "lmap", "dict"] {
        assert!(ctx.resolve_spec(registry, name).is_some(), "{name}");
    }
}

const JIM_FROM_080: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.80", None)])];

/// A registry holding the shipped surface plus a spec of `name` for each
/// of `surfaces`, registered in that order.
fn registry_with(name: &'static str, surfaces: &[&'static [SpecSurface]]) -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for surface in surfaces {
        registry.insert(CommandSpec {
            name,
            surface: Some(*surface),
            ..CommandSpec::DEFAULT
        });
    }
    registry
}

/// A spec available only from jim 0.80 is admitted at a `jim`
/// document's own point and at no Tcl point, and the inherited Tcl
/// surface still resolves there.
#[test]
fn a_jim_context_resolves_a_command_only_jim_provides() {
    let registry = registry_with("loop", &[JIM_FROM_080]);
    let jim = context("jim");

    let looped = jim
        .resolve_spec(&registry, "loop")
        .expect("a jim document resolves a Core(Jim) command");
    assert_eq!(looped.surface, Some(JIM_FROM_080));
    assert!(
        jim.resolve_spec(&registry, "puts").is_some(),
        "the inherited Tcl surface still resolves"
    );

    for environment in ["tcl8.6", "tcl9.0", "tcl", "tk", "f5-iapps"] {
        assert!(
            context(environment)
                .resolve_spec(&registry, "loop")
                .is_none(),
            "{environment}: a Core(Jim) command is not part of a Tcl surface"
        );
    }
}

/// Where a jim-only row and an inherited Tcl row both offer one
/// command, a `jim` document sees the jim row and a Tcl document the
/// Tcl row — whichever was registered last, and even when the Tcl row
/// is the narrower of the two.
#[test]
fn a_jim_row_shadows_the_inherited_tcl_row_for_a_jim_context() {
    let jim = context("jim");
    let tcl = context("tcl8.6");

    // `proc` has an inherited Tcl row in the shipped surface.
    let registry = registry_with("proc", &[JIM_FROM_080]);
    assert_eq!(
        jim.resolve_spec(&registry, "proc")
            .and_then(|spec| spec.surface),
        Some(JIM_FROM_080)
    );
    let inherited = tcl.resolve_spec(&registry, "proc").expect("Tcl's proc");
    assert_ne!(inherited.surface, Some(JIM_FROM_080));

    // A single-release Tcl row is as narrow as a jim row can be, so
    // breadth ties and only the point's order can decide.
    for surfaces in [
        [SpecSurface::TCL86, JIM_FROM_080],
        [JIM_FROM_080, SpecSurface::TCL86],
    ] {
        let registry = registry_with("shadow_probe", &surfaces);
        assert_eq!(
            jim.resolve_spec(&registry, "shadow_probe")
                .and_then(|spec| spec.surface),
            Some(JIM_FROM_080),
            "{surfaces:?}: jim"
        );
        assert_eq!(
            tcl.resolve_spec(&registry, "shadow_probe")
                .and_then(|spec| spec.surface),
            Some(SpecSurface::TCL86),
            "{surfaces:?}: tcl8.6"
        );
    }
}

/// **Invariant I2.** A declared `jim` range gates on the jim
/// axis and says nothing on the Tcl axis — and vice versa. The
/// axis machinery needed no jim-specific code: `targets_from_clauses`,
/// `ladder_releases_in` and `ladder_coverage` all read the family's
/// own ladder.
#[test]
fn a_declared_jim_range_gates_on_the_jim_axis_only() {
    let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
    let tcl_axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
    let mut ctx = context("jim");

    // The targets grammar spells jim ranges off the jim ladder: a
    // bare release is that release's line, an inclusive max covers
    // the whole line of the named release, and an open end clamps to
    // the modelled ladder.
    let declared = targets_from_clauses(&jim_axis, &["0.76-0.79"]).expect("well-formed");
    assert_eq!(
        ladder_releases_in(&declared),
        vec!["0.76", "0.77", "0.78", "0.79"]
    );
    assert!(!declared.contains(&Version::parse("0.80").expect("version")));
    let open = targets_from_clauses(&jim_axis, &["0.81-"]).expect("well-formed");
    assert_eq!(
        ladder_releases_in(&open),
        vec!["0.81", "0.82", "0.83", "0.84"]
    );
    assert!(
        !open.contains(&Version::parse("0.85").expect("version")),
        "an open end clamps to the modelled ladder"
    );

    ctx.declare_targets(declared);
    assert!(ctx.declared_targets(&jim_axis).is_some());
    assert!(
        ctx.declared_targets(&tcl_axis).is_none(),
        "I2: a jim declaration is not a Tcl declaration"
    );

    // The window query answers on the jim axis. `lt`/`ge` arrive at
    // Jim 0.80, so a 0.76-0.79 project is told so.
    let outside = ctx
        .targets_outside_window(&jim_axis, Some("0.80"), None)
        .expect("the whole declaration is below 0.80");
    assert_eq!(
        ladder_releases_in(&outside),
        vec!["0.76", "0.77", "0.78", "0.79"]
    );
    assert_eq!(requirement_spelling(&outside), "0.76-0.80");
    // A window the declaration sits inside abstains.
    assert!(
        ctx.targets_outside_window(&jim_axis, Some("0.76"), None)
            .is_none()
    );
    // …and the same question on the Tcl axis abstains entirely: the
    // jim declaration is not evidence about Tcl releases.
    assert!(
        ctx.targets_outside_window(&tcl_axis, Some("8.6"), None)
            .is_none(),
        "I2: the jim range must not leak onto the Tcl axis"
    );

    // The leak is unrepresentable, not merely untested: the two sets
    // cannot be compared at all.
    let tcl_targets = targets_from_clauses(&tcl_axis, &["8.5-9.0"]).expect("well-formed");
    let jim_targets = targets_from_clauses(&jim_axis, &["0.81-"]).expect("well-formed");
    assert!(
        jim_targets.intersect(&tcl_targets).is_err(),
        "I2: cross-axis operations are typed errors"
    );

    // A declared *Tcl* range on a jim context gates the Tcl axis and
    // leaves jim's alone — the mirror direction.
    let mut mirror = context("jim");
    mirror.declare_targets(tcl_targets);
    assert!(mirror.declared_targets(&tcl_axis).is_some());
    assert!(mirror.declared_targets(&jim_axis).is_none());
    assert!(
        mirror
            .targets_outside_window(&jim_axis, Some("0.80"), None)
            .is_none()
    );
}

/// A `Core(Jim)` declaration restricted to part of the jim
/// ladder reports exactly the covered subset — the same
/// `available_at_targets` machinery Tk and the tcllib modules use,
/// on a core family's own axis.
#[test]
fn available_at_targets_answers_on_the_jim_axis() {
    let ctx = context("jim");
    let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
    let modern = vec![SurfaceDeclaration {
        provider: Provider::Core(tcl_dialect::model::Family::Jim),
        applicable: targets_from_clauses(&jim_axis, &["0.80-"]).expect("well-formed"),
        predicate: crate::model::surface::CapabilityPredicate::None,
        history: tcl_dialect::model::ItemHistory::default(),
        provenance: tcl_dialect::model::Provenance::BuiltIn,
    }];
    let covered = ctx.available_at_targets(&modern, &jim_axis);
    assert!(covered.contains(&Version::parse("0.84").expect("version")));
    assert!(!covered.contains(&Version::parse("0.79").expect("version")));
    // The declaration does not cover the environment's whole ladder.
    let targets = ctx.floors.targets(&jim_axis).expect("targets");
    assert!(!targets.subset(&covered).expect("same axis"));
    // A Tcl-provider row contributes nothing to the jim axis.
    assert!(
        ctx.available_at_targets(&rows(Some(SpecSurface::ALL_TCL)), &jim_axis)
            .is_empty()
    );
}

#[test]
fn parsed_family_ladders_keep_the_original_successes_and_order() {
    for family in Family::ALL {
        let expected: Vec<_> = family
            .releases()
            .iter()
            .filter_map(|release| Version::parse(release.as_str()).ok())
            .collect();
        let expected_text: Vec<_> = expected.iter().map(Version::as_str).collect();
        let actual_text: Vec<_> = parsed_family_ladder(family)
            .iter()
            .map(Version::as_str)
            .collect();
        assert_eq!(actual_text, expected_text, "{family:?}");
    }
}

#[test]
fn breadth_counts_one_per_provider_release() {
    // One per Tcl
    // ladder release a core row covers, one per other core row, one per
    // package row.
    let cases: &[(&[SpecSurface], u32)] = &[
        (SpecSurface::TCL84, 1),
        (SpecSurface::TCL8X, 3),
        (SpecSurface::TCL85_PLUS, 4),
        (SpecSurface::ALL_TCL, 5),
        (SpecSurface::IRULES, 1),
        (SpecSurface::ALL_TCL_AND_IRULES, 6),
        (SpecSurface::TK_AND_TCL, 6),
        (
            surface![SpecSurface::package("iapps"), SpecSurface::package("tmsh")],
            2,
        ),
        (SpecSurface::EXPECT, 1),
    ];
    for &(rows_of, expected) in cases {
        assert_eq!(
            specificity_breadth(&rows(Some(rows_of))),
            expected,
            "{rows_of:?}"
        );
        assert_eq!(surface_breadth(rows_of), expected, "{rows_of:?} breadth");
    }
    // The universal translation is strictly wider than any explicit
    // gate (old rule: a catch-all loses to every scoped spec). 24 =
    // 5 Tcl releases + 1 f5-tcl + 1 f5-irules + 9 jim + 8 vendor
    // packages — the `f5-tcl` trunk family (measurements §4a) added
    // its row in the F5 reclassification, and `sslictcl` added its
    // vendor package with the `.sslictcl` authoring dialect.
    assert_eq!(specificity_breadth(&rows(None)), 24);
    // A hosted attribution row adds no specificity, mirroring the old
    // specificity, which never counted `required_package`.
    let hosted = declarations_for_spec(&CommandSpec {
        name: "context-test",
        surface: Some(SpecSurface::ALL_TCL),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    });
    assert_eq!(specificity_breadth(&hosted), 5);
}

/// **Parity sweep 1**: every context-derived authoring fact —
/// the point, the option ceiling, the operator-head rule, the vendor
/// provider, and every placement's ambience and static floor —
/// reproduces the old profile's value for every catalogue profile,
/// and the lenient `tcl`/`tk` environments reproduce the permissive
/// fallback profile the analyser resolved those names to.
#[test]
fn authoring_facts_reproduce_every_catalogue_profile() {
    use tcl_dialect::DialectProfile;
    for profile in DialectProfile::all() {
        let ctx = context(profile.name);
        // documented deltas (point, ceiling, operator heads).
        assert!(
            crate::model::surface::points_answer_alike(
                &ctx.authoring_query(),
                &profile.surface_query()
            ),
            "{} point: {:?} vs {:?}",
            profile.name,
            ctx.authoring_query(),
            profile.surface_query()
        );
        assert_eq!(
            ctx.tcl_version_ceiling(),
            profile.version_ceiling,
            "{} ceiling",
            profile.name
        );
        assert_eq!(
            ctx.operator_heads_are_commands(),
            profile.operators_as_commands,
            "{} operator heads",
            profile.name
        );
        assert_eq!(
            ctx.vendor_authoring_provider(),
            profile.vendor_surface,
            "{} vendor bit",
            profile.name
        );
        for pin in profile.libraries {
            assert_eq!(
                ctx.placement_is_ambient(pin.package),
                profile.is_ambient_package(pin.package),
                "{} ambience of {}",
                profile.name,
                pin.package
            );
            assert_eq!(
                ctx.placement_floor(pin.package).map(Version::as_str),
                profile.library_floor_default(pin.package),
                "{} floor of {}",
                profile.name,
                pin.package
            );
        }
        // The keyed axes honour a session pin exactly as
        // `library_floor` with overrides does.
        let overrides = LibraryVersionOverrides {
            bigip_version: Some("17.1.0".to_owned()),
            ..LibraryVersionOverrides::default()
        };
        let keyed = KeyedVersions::from_overrides(&overrides).expect("valid overrides");
        let registry = EnvironmentRegistry::compiled();
        let pinned =
            ResolvedContext::resolve(registry.resolve(profile.name).expect(profile.name), &keyed);
        for pin in profile.libraries {
            assert_eq!(
                pinned.placement_floor(pin.package).map(Version::as_str),
                profile.library_floor(pin.package, &overrides),
                "{} pinned floor of {}",
                profile.name,
                pin.package
            );
        }
    }
    // The analyser resolved `tcl` (and every unknown name) to the
    // permissive fallback profile; its environment derives the same
    // permissive facts.
    let plain = DialectProfile::plain_tcl();
    let ctx = context("tcl");
    assert_eq!(ctx.authoring_query(), plain.surface_query());
    assert_eq!(ctx.tcl_version_ceiling(), None);
    assert!(ctx.operator_heads_are_commands());
    assert_eq!(ctx.vendor_authoring_provider(), None);
    // `tk` derives the same permissive facts **plus** Tk itself, off
    // its ambient placement. It is still not a *vendor* environment —
    // Tk is a library — so no vendor provider and no ceiling.
    let tk = context("tk");
    assert_eq!(
        tk.authoring_query(),
        SurfaceQuery {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            core: plain.surface_query().core,
            packages: tcl_dialect::model::PackageFloors::Borrowed(&[PackageFloor::named("Tk")]),
        }
    );
    assert_eq!(tk.tcl_version_ceiling(), None);
    assert!(tk.operator_heads_are_commands());
    assert_eq!(tk.vendor_authoring_provider(), None);
    assert!(tk.placement_is_ambient("Tk"));
}

/// The floor a pack declares for an ambient package reaches the query of
/// a context whose point carries that package — and only that one — so a
/// spec whose package row is windowed past the floor is not available.
#[test]
fn a_pack_declared_floor_is_the_floor_of_a_carried_package_only() {
    const FROM_8_6: &[tcl_dialect::model::SpecWindow] = &[("8.6", None)];
    let gated = CommandSpec {
        name: "tk-from-8.6",
        surface: Some(surface![SpecSurface::package_in("Tk", FROM_8_6)]),
        ..CommandSpec::DEFAULT
    };

    let mut tk = context("tk");
    assert_eq!(
        tk.authoring_query().package("Tk"),
        Some(PackageFloor::named("Tk"))
    );
    assert!(tk.spec_available(&gated), "no floor declared: permissive");

    tk.record_pack_ambient("Tk", "8.5");
    assert_eq!(
        tk.authoring_query().package("Tk"),
        Some(PackageFloor::at("Tk", "8.5"))
    );
    assert!(!tk.spec_available(&gated), "introduced after the floor");

    tk.record_pack_ambient("Tk", "8.6");
    assert_eq!(
        tk.authoring_query().package("Tk"),
        Some(PackageFloor::at("Tk", "8.6")),
        "the strongest claim holds"
    );
    assert!(tk.spec_available(&gated));

    tk.record_pack_ambient("Itcl", "4.0");
    assert!(
        !tk.authoring_query().carries("Itcl"),
        "a floor does not put a package into the point"
    );

    let mut plain = context("tcl8.6");
    plain.record_pack_ambient("Tk", "9.0");
    assert!(!plain.authoring_query().carries("Tk"));
    assert!(!plain.spec_available(&gated));
}

/// **Parity sweep 2**: the spec/subcommand/option availability
/// queries answer exactly as the old `ProfileQueries` for every spec
/// in the compiled universe under every catalogue profile — commands,
/// each subcommand, each sub-subcommand (at no version and at a pinned
/// one), and every option at its inherited parent gate.
#[test]
fn spec_queries_reproduce_profile_queries_for_every_profile() {
    use crate::profile_queries::{LegacyProfileOracle, ProfileQueries};
    use tcl_dialect::DialectProfile;
    let universe = crate::model::assembly::universe();
    let mut checks = 0usize;
    for profile in DialectProfile::all() {
        let ctx = context(profile.name);
        for name in universe.command_names() {
            for spec in universe.specs(name) {
                assert_eq!(
                    ctx.spec_available(spec),
                    crate::model::assembly::tests::old_available_after_p3(profile, spec),
                    "`{name}` under `{}`",
                    profile.name
                );
                assert_eq!(
                    ctx.keyed_ambient_placement(spec)
                        .map(|placement| placement.package.as_ref().to_owned()),
                    profile
                        .keyed_pin_for(spec)
                        .map(|pin| pin.package.to_owned()),
                    "`{name}` keyed pin under `{}`",
                    profile.name
                );
                assert_eq!(
                    ctx.keyed_version_range(spec),
                    profile.keyed_version_range(spec),
                    "`{name}` keyed range under `{}`",
                    profile.name
                );
                for sub in spec.subcommands {
                    assert_eq!(
                        ctx.subcommand_available(spec, sub),
                        profile.is_subcommand_available(spec, sub),
                        "`{name} {}` under `{}`",
                        sub.name,
                        profile.name
                    );
                    for sub_sub in sub.sub_subcommands {
                        for version in [None, Some("1.0"), Some("99.0")] {
                            assert_eq!(
                                ctx.sub_subcommand_available(spec, sub, sub_sub, version),
                                profile.is_sub_subcommand_available(spec, sub, sub_sub, version),
                                "`{name} {} {}` under `{}` at {version:?}",
                                sub.name,
                                sub_sub.name,
                                profile.name
                            );
                        }
                    }
                    assert_eq!(
                        ctx.available_sub_option_names(spec, sub),
                        profile.available_sub_option_names(spec, sub),
                        "`{name} {}` options under `{}`",
                        sub.name,
                        profile.name
                    );
                    checks += 1;
                }
                let parent = spec.surface;
                for opt in spec
                    .options
                    .iter()
                    .chain(spec.command_forms.iter().flat_map(|form| form.options))
                {
                    assert_eq!(
                        ctx.option_available(opt, parent),
                        profile.is_option_available(opt, parent),
                        "`{name} {}` under `{}`",
                        opt.name,
                        profile.name
                    );
                    checks += 1;
                }
                assert_eq!(
                    ctx.available_option_names(spec),
                    profile.available_option_names(spec),
                    "`{name}` option names under `{}`",
                    profile.name
                );
                checks += 1;
            }
        }
    }
    println!("profile-query parity sweep: {checks} item checks, 0 divergences");
}

/// **Wave-4 parity pin**: the context's vendor-surface summary
/// reproduces `ProfileQueries::vendor_surface` for every catalogue
/// profile, over that profile's own registry generation — the store
/// the generated AI prompt has always read it from. At least one
/// profile must actually have a surface, or the sweep would pass
/// vacuously on a pair of `None`s.
#[test]
fn vendor_surface_groups_constructed_rooted_descriptors() {
    use crate::profile_queries::LegacyProfileOracle;
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "::VENDOR_ADAPTER::nested::command",
        surface: Some(SpecSurface::IRULES),
        ..CommandSpec::DEFAULT
    });
    let summary = context("f5-irules")
        .vendor_command_surface(&registry)
        .unwrap();
    assert_eq!(
        summary
            .namespaces
            .iter()
            .find(|(namespace, _)| namespace == "VENDOR_ADAPTER"),
        Some(&("VENDOR_ADAPTER".to_owned(), 1))
    );
    let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
    assert_eq!(Some(summary), profile.vendor_surface(&registry));
}

#[test]
fn vendor_surface_matches_the_profile_query() {
    use crate::profile_queries::LegacyProfileOracle;
    use tcl_dialect::DialectProfile;
    let mut with_surface = 0usize;
    for profile in DialectProfile::all() {
        let ctx = context(profile.name);
        let store = crate::model::ingress::static_context_for(profile.name).commands();
        let ported = ctx.vendor_command_surface(store);
        assert_eq!(
            ported,
            profile.vendor_surface(store),
            "{} vendor surface",
            profile.name
        );
        if ported.is_some() {
            with_surface += 1;
        }
    }
    assert!(
        with_surface > 0,
        "no catalogue profile has a vendor surface — the sweep is vacuous"
    );
    // The lenient sink and the additive `tk` ingress author under no
    // vendor provider, so neither has a surface of its own.
    for lenient in ["tcl", "tk"] {
        let store = crate::model::ingress::static_context_for(lenient).commands();
        assert!(
            context(lenient).vendor_command_surface(store).is_none(),
            "{lenient} has no vendor surface"
        );
    }
}

/// §5.4 range targeting: the targets grammar, the window/gate
/// remainder queries, and the diagnostic spellings.
#[test]
fn declared_targets_answer_the_range_queries() {
    let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
    let mut ctx = context("tcl8.6");
    // No declaration ⇒ every range query abstains (the no-range pin).
    assert!(ctx.declared_targets(&axis).is_none());
    assert!(
        ctx.targets_outside_window(&axis, Some("8.6"), None)
            .is_none()
    );
    assert!(
        ctx.targets_uncovered_by_gate(Some(SpecSurface::TCL86_PLUS))
            .is_none()
    );

    // The §5.4 canonical declaration: `tcl 8.5-9.0` covers the whole
    // 9.0 line (inclusive-max reading), not the vsatisfies exclusive
    // one.
    let declared = targets_from_clauses(&axis, &["8.5-9.0"]).expect("well-formed");
    assert!(declared.contains(&Version::parse("9.0.1").expect("version")));
    assert!(declared.contains(&Version::parse("8.5").expect("version")));
    assert!(!declared.contains(&Version::parse("9.1").expect("version")));
    assert_eq!(ladder_releases_in(&declared), vec!["8.5", "8.6", "9.0"]);
    ctx.declare_targets(declared.clone());
    assert_eq!(ctx.declared_targets(&axis), Some(&declared));

    // An item introduced at 8.6: the remainder is exactly the 8.5
    // line, spelled and named for the message.
    let outside = ctx
        .targets_outside_window(&axis, Some("8.6"), None)
        .expect("8.5 is outside");
    assert_eq!(ladder_releases_in(&outside), vec!["8.5"]);
    assert_eq!(requirement_spelling(&outside), "8.5-8.6");
    // An item retired at 9.0: the remainder is the 9.0 line.
    let retired = ctx
        .targets_outside_window(&axis, None, Some("9.0"))
        .expect("9.0 is outside");
    assert_eq!(ladder_releases_in(&retired), vec!["9.0"]);
    // A window covering the whole declaration abstains.
    assert!(
        ctx.targets_outside_window(&axis, Some("8.5"), None)
            .is_none()
    );

    // Gate coverage: an 8.6+ gate misses the 8.5 target; ALL_TCL and
    // an absent gate cover everything; a vendor-only gate is another
    // provider's item and abstains.
    let uncovered = ctx
        .targets_uncovered_by_gate(Some(SpecSurface::TCL86_PLUS))
        .expect("8.5 uncovered");
    assert_eq!(ladder_releases_in(&uncovered), vec!["8.5"]);
    assert!(
        ctx.targets_uncovered_by_gate(Some(SpecSurface::ALL_TCL))
            .is_none()
    );
    assert!(ctx.targets_uncovered_by_gate(None).is_none());
    assert!(
        ctx.targets_uncovered_by_gate(Some(SpecSurface::TK))
            .is_none()
    );
    // An 8.x-only gate misses the 9.0 end.
    let gone = ctx
        .targets_uncovered_by_gate(Some(SpecSurface::TCL8X))
        .expect("9.0 uncovered");
    assert_eq!(ladder_releases_in(&gone), vec!["9.0"]);

    // Redeclaring an axis replaces, never accumulates.
    ctx.declare_targets(targets_from_clauses(&axis, &["9.0"]).expect("well-formed"));
    assert!(
        ctx.targets_uncovered_by_gate(Some(SpecSurface::TCL86_PLUS))
            .is_none(),
        "a single 9.0 target is covered by an 8.6+ gate"
    );
    assert_eq!(ctx.declared_target_sets().len(), 1);
}

/// The targets grammar's clause forms, including the package-axis
/// line rule and the ladder clamp.
#[test]
fn the_targets_grammar_spells_lines_and_ranges() {
    let core = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
    // Bare release: the line alone, not the vsatisfies next-major
    // window.
    let line = targets_from_clauses(&core, &["8.5"]).expect("well-formed");
    assert!(line.contains(&Version::parse("8.5.19").expect("version")));
    assert!(!line.contains(&Version::parse("8.6").expect("version")));
    // Open-ended: clamped to the modelled ladder.
    let open = targets_from_clauses(&core, &["8.5-"]).expect("well-formed");
    assert!(open.contains(&Version::parse("9.1").expect("version")));
    assert!(!open.contains(&Version::parse("9.2").expect("version")));
    // Multi-clause union (the `8.5 9.0` spelling).
    let union = targets_from_clauses(&core, &["8.5", "9.0"]).expect("well-formed");
    assert!(union.contains(&Version::parse("8.5").expect("version")));
    assert!(union.contains(&Version::parse("9.0.1").expect("version")));
    assert!(!union.contains(&Version::parse("8.6").expect("version")));
    // Package axis: bare release bumps the last component.
    let tk = VersionAxisId::package("Tk");
    let tk_line = targets_from_clauses(&tk, &["8.6"]).expect("well-formed");
    assert!(tk_line.contains(&Version::parse("8.6.12").expect("version")));
    assert!(!tk_line.contains(&Version::parse("8.7").expect("version")));
    let tk_range = targets_from_clauses(&tk, &["8.5-8.6"]).expect("well-formed");
    assert!(tk_range.contains(&Version::parse("8.6.12").expect("version")));
    assert!(!tk_range.contains(&Version::parse("8.7").expect("version")));
    // Malformed clauses error rather than guess.
    assert!(targets_from_clauses(&core, &["not-a-version"]).is_err());
    assert!(targets_from_clauses(&core, &["8.5-9.0-9.1"]).is_err());
}

#[test]
fn capability_predicates_resolve_through_the_core_build() {
    let mut declaration = rows(Some(SpecSurface::ALL_TCL)).remove(0);
    declaration.predicate = CapabilityPredicate::RequiresCapability(BuildCapability::MathExtension);
    let ctx = context("tcl8.6");
    assert!(ctx.predicate_passes(&declaration.predicate));
    assert!(ctx.is_available(std::slice::from_ref(&declaration)));
    // No core ⇒ no build to answer ⇒ fail, never silently pass.
    assert!(!context("f5-bigip").predicate_passes(&declaration.predicate));
}
#[test]
fn genuine_nonvendor_requirements_retain_owned_source_package_floors() {
    let mut ctx = context("tcl8.6");
    let name = String::from("AcmeSourceKit");
    ctx.record_pack_ambient(&name, "2.0");
    assert!(
        !ctx.authoring_query().carries(&name),
        "a floor alone cannot activate a provider"
    );
    ctx.require_package(&name, None);
    drop(name);
    assert_eq!(
        ctx.authoring_query().package("AcmeSourceKit"),
        Some(PackageFloor::at("AcmeSourceKit", "2.0"))
    );
    let source_shape = CommandSpec {
        name: "acme_source_shape",
        surface: Some(surface![SpecSurface::package("AcmeSourceKit")]),
        ..CommandSpec::DEFAULT
    };
    assert!(ctx.spec_available(&source_shape));
    assert!(!ctx.authoring_query().carries("ForeignSourceKit"));
}
