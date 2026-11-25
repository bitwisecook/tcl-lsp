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

//! Lookup and visibility: rooted names, the most specific visible spec, floors, providers and declared commands.

use super::*;

/// Tcl 9.0.4: inside a method the bare words resolve to the receiving
/// object's `my` command or the `::oo::Helpers` namespace path, while every
/// global `::keyword` spelling is absent. Generic registry lookup must not
/// turn those contextual bare specs into nonexistent global commands.
#[test]
fn rooted_contextual_commands_do_not_fall_back_to_bare_specs() {
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    for (head, expected) in [
        ("my", MethodDispatchKind::SelfDispatch),
        ("next", MethodDispatchKind::NextChain),
        ("nextto", MethodDispatchKind::NextChain),
        ("self", MethodDispatchKind::Introspection),
    ] {
        assert_eq!(registry.method_dispatch_keyword(head), Some(expected));
        assert_eq!(
            registry.method_dispatch_keyword(&format!("::{head}")),
            None,
            "tclsh9.0.4 has no global ::{head} command"
        );
    }
    for qualified in [
        "oo::Helpers::next",
        "::oo::Helpers::next",
        "oo::Helpers::nextto",
        "::oo::Helpers::nextto",
        "oo::Helpers::self",
        "::oo::Helpers::self",
    ] {
        assert_eq!(registry.method_dispatch_keyword(qualified), None);
    }

    assert!(registry.is_self_receiver_call("self", None));
    assert!(registry.is_self_receiver_call("self", Some("object")));
    assert!(!registry.is_self_receiver_call("::self", None));
    assert!(!registry.is_self_receiver_call("::self", Some("object")));
    for qualified in ["oo::Helpers::self", "::oo::Helpers::self"] {
        assert!(
            registry.is_self_receiver_call(qualified, None),
            "no-argument {qualified} is the real qualified helper command"
        );
        assert!(
            registry.is_self_receiver_call(qualified, Some("object")),
            "{qualified} is the real qualified helper command"
        );
    }

    for head in [
        "callback",
        "classvariable",
        "link",
        "my",
        "mymethod",
        "next",
        "nextto",
        "self",
    ] {
        let rooted = format!("::{head}");
        assert!(registry.get(head).is_some(), "bare {head}");
        assert!(registry.get(&rooted).is_none(), "rooted {head}");
        assert!(
            registry
                .get_for_surface(&rooted, registry.own_surface_query())
                .is_none(),
            "profiled rooted {head}"
        );
        assert!(!registry.has_command_in_this_dialect(&rooted), "{rooted}");
        assert_eq!(
            registry.known_in_any_dialect(&rooted),
            head == "next",
            "only BPF-Tcl independently declares a non-contextual global {rooted}"
        );
        assert!(!registry.declares_command_at(&rooted), "{rooted}");
        assert!(
            !registry.resolves_only_in_method_context(&rooted),
            "{rooted}"
        );
        assert!(!registry.requires_oo_method_frame(&rooted), "{rooted}");
    }
    assert!(!registry.binds_method_alias("::link"));

    for (rooted, canonical) in [
        ("::set", "set"),
        ("::oo::Helpers::self", "oo::Helpers::self"),
    ] {
        assert_eq!(registry.get(rooted).map(|spec| spec.name), Some(canonical));
        assert!(registry.has_command_in_this_dialect(rooted), "{rooted}");
        assert!(registry.known_in_any_dialect(rooted), "{rooted}");
        assert!(registry.declares_command_at(rooted), "{rooted}");
    }
}

#[test]
fn get_for_dialect_picks_the_most_specific_visible_spec() {
    // §5.3/D6 — the single selection rule. Three specs of one name,
    // registered deliberately most-specific-FIRST so the old
    // last-match rule would pick the catch-all:
    //   1. a TCL86-scoped spec (tightest),
    //   2. a wider TCL86|TCL90 spec,
    //   3. a catch-all (`surface: None`).
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: Some(SpecSurface::TCL86),
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: Some(surface![SpecSurface::core_in(
            Family::Tcl,
            &[("8.6", Some("9.1"))]
        )]),
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_probe",
        surface: None,
        ..CommandSpec::DEFAULT
    });

    // Scoped beats catch-all, tighter beats wider — even though the
    // catch-all was registered last.
    let under_86 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert_eq!(under_86.and_then(|s| s.surface), Some(SpecSurface::TCL86));
    // Under 9.0 the tightest visible spec is the two-bit one.
    let under_90 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert_eq!(
        under_90.and_then(|s| s.surface),
        Some(surface![SpecSurface::core_in(
            Family::Tcl,
            &[("8.6", Some("9.1"))]
        )])
    );
    // Where no scoped spec is visible, the catch-all still resolves.
    let under_84 = reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
    assert!(under_84.is_some_and(|s| s.surface.is_none()));
}

#[test]
fn get_for_dialect_breaks_specificity_ties_by_last_registration() {
    // Curated pack overrides re-register a name at the same scope; the
    // later registration must keep winning (the old `.rev()` guarantee,
    // preserved as the D6 tie-break).
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "d6_tie",
        surface: Some(SpecSurface::TCL86),
        arity: Arity::exact(1), // base data
        ..CommandSpec::DEFAULT
    });
    reg.insert(CommandSpec {
        name: "d6_tie",
        surface: Some(SpecSurface::TCL86),
        arity: Arity::exact(2), // curated override
        ..CommandSpec::DEFAULT
    });
    let won = reg
        .get_for_surface("d6_tie", Some(SurfaceQuery::core(Family::Tcl, "8.6")))
        .expect("d6_tie resolves");
    assert_eq!(won.arity, Arity::exact(2), "later registration wins ties");
}

fn reference_best_visible(
    registry: &CommandRegistry,
    specs: &[&'static CommandSpec],
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<&'static CommandSpec> {
    specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| registry.spec_visible(spec, dialect))
        .max_by_key(|&(index, spec)| {
            let nearness = std::cmp::Reverse(
                spec.surface
                    .zip(dialect)
                    .and_then(|(rows, query)| surface_nearness(rows, &query))
                    .unwrap_or(usize::MAX),
            );
            let scope_tightness = std::cmp::Reverse(spec.surface.map_or(u32::MAX, surface_breadth));
            (spec.surface.is_some(), nearness, scope_tightness, index)
        })
        .map(|(_, spec)| *spec)
}

fn synthetic_spec(
    name: &'static str,
    surface: Option<&'static [SpecSurface]>,
    traits: Traits,
) -> &'static CommandSpec {
    Box::leak(Box::new(CommandSpec {
        name,
        traits,
        surface,
        ..CommandSpec::DEFAULT
    }))
}

fn assert_best_visible_matches_reference(
    registry: &CommandRegistry,
    specs: &[&'static CommandSpec],
    dialect: Option<SurfaceQuery<'_>>,
) {
    let expected = reference_best_visible(registry, specs, dialect);
    let actual = registry.best_visible(specs, dialect);
    assert_eq!(
        actual.map(|spec| spec.name),
        expected.map(|spec| spec.name),
        "optimised selector changed the selected name"
    );
    assert_eq!(
        actual.map(std::ptr::from_ref),
        expected.map(std::ptr::from_ref),
        "optimised selector changed the selected spec"
    );
}

#[test]
fn best_visible_matches_reference_for_surface_and_registration_cases() {
    let registry = CommandRegistry::build_default();
    let scoped = synthetic_spec(
        "d6_scoped_singleton",
        Some(SpecSurface::TCL86),
        Traits::empty(),
    );
    let catch_all = synthetic_spec("d6_catch_all_singleton", None, Traits::empty());

    // A singleton scoped row and a singleton catch-all row are both
    // visible exactly as the reference selector expects.
    assert_best_visible_matches_reference(
        &registry,
        &[scoped],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
    assert_best_visible_matches_reference(&registry, &[catch_all], None);

    let narrow = synthetic_spec("d6_narrow", Some(SpecSurface::TCL86), Traits::empty());
    let wide = synthetic_spec("d6_wide", Some(SpecSurface::TCL86_PLUS), Traits::empty());
    let disjoint_old = synthetic_spec("d6_disjoint_old", Some(SpecSurface::TCL85), Traits::empty());
    let disjoint_new = synthetic_spec("d6_disjoint_new", Some(SpecSurface::TCL90), Traits::empty());

    // Overlapping rows choose the narrow window, while disjoint rows
    // produce no result when neither row admits the queried release.
    assert_best_visible_matches_reference(
        &registry,
        &[wide, narrow],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
    assert_best_visible_matches_reference(
        &registry,
        &[disjoint_old, disjoint_new],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );

    let tie_first = synthetic_spec("d6_tie_first", Some(SpecSurface::TCL86), Traits::empty());
    let tie_last = synthetic_spec("d6_tie_last", Some(SpecSurface::TCL86), Traits::empty());
    assert_best_visible_matches_reference(
        &registry,
        &[tie_first, tie_last],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
}

#[test]
fn an_unscoped_pack_override_beats_a_scoped_shipped_spec() {
    let mut registry = CommandRegistry::build_default();
    let shipped = synthetic_spec(
        "d6_pack_override_precedence",
        Some(SpecSurface::TCL86),
        Traits::empty(),
    );
    let override_spec = synthetic_spec("d6_pack_override_precedence", None, Traits::empty());
    registry.insert_pack_origin(
        override_spec,
        crate::pack_origin::PackOrigin {
            pack: "precedence-test".to_owned(),
            content_hash: 1,
            vocabulary_version: "2.0".to_owned(),
            provenance: tcl_dialect::model::Provenance::WorkspaceTrusted,
        },
    );

    let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
    assert_eq!(
        registry
            .best_visible(&[override_spec, shipped], query)
            .map(std::ptr::from_ref),
        Some(std::ptr::from_ref(override_spec)),
        "an installed pack override has provenance precedence over core scope"
    );
    assert_eq!(
        registry
            .best_visible(&[shipped, override_spec], query)
            .map(std::ptr::from_ref),
        Some(std::ptr::from_ref(override_spec)),
        "candidate registration order does not let a scoped core row take precedence"
    );
}

#[test]
fn best_visible_prefers_the_nearer_core_point() {
    use tcl_dialect::model::CorePoints;

    let registry = CommandRegistry::build_default();
    let jim_rows = surface![SpecSurface::core_in(Family::Jim, &[("0.80", None)])];
    let own = synthetic_spec("near_own", Some(jim_rows), Traits::empty());
    let inherited = synthetic_spec("near_inherited", Some(SpecSurface::TCL86), Traits::empty());
    let inherited_wide = synthetic_spec(
        "near_inherited_wide",
        Some(SpecSurface::ALL_TCL),
        Traits::empty(),
    );
    let catch_all = synthetic_spec("near_catch_all", None, Traits::empty());
    let own_first = SurfaceQuery {
        realm: tcl_dialect::model::InvocationRealm::RuleLoader,
        core: CorePoints::two((Family::Jim, None), (Family::Tcl, Some("8.6"))),
        packages: tcl_dialect::model::PackageFloors::Borrowed(&[]),
    };

    // The own-family row wins over an inherited row registered after
    // it and just as narrow, and over a wider one registered before it.
    for specs in [
        [own, inherited],
        [inherited, own],
        [inherited_wide, own],
        [own, inherited_wide],
    ] {
        assert_best_visible_matches_reference(&registry, &specs, Some(own_first));
        assert_eq!(
            registry
                .best_visible(&specs, Some(own_first))
                .map(|spec| spec.name),
            Some("near_own")
        );
    }
    // A scoped row still beats a catch-all, and the inherited row is
    // still what a query without the own-family point selects.
    assert_best_visible_matches_reference(&registry, &[catch_all, inherited], Some(own_first));
    assert_eq!(
        registry
            .best_visible(
                &[own, inherited],
                Some(SurfaceQuery::core(Family::Tcl, "8.6"))
            )
            .map(|spec| spec.name),
        Some("near_inherited")
    );
}

#[test]
fn best_visible_matches_reference_for_package_rows() {
    let registry = CommandRegistry::build_default();
    let package = synthetic_spec("d6_package", Some(SpecSurface::EXPECT), Traits::empty());
    let core = synthetic_spec("d6_core", Some(SpecSurface::ALL_TCL), Traits::empty());
    let packages = [PackageFloor::named("expect")];
    let with_package = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&packages);

    // Package rows are visible through the package set and are narrower
    // than the whole core row, so they win the same reference decision
    // used by the unoptimised implementation.
    assert_best_visible_matches_reference(&registry, &[core, package], Some(with_package));
    assert_eq!(
        registry
            .best_visible(&[core, package], Some(with_package))
            .map(|spec| spec.name),
        Some("d6_package")
    );
    assert_best_visible_matches_reference(
        &registry,
        &[package],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
}

#[test]
fn best_visible_matches_reference_for_profile_operator_visibility() {
    let operator_and_core = surface![
        SpecSurface::core(Family::Tcl),
        SpecSurface::core(Family::F5Irules),
    ];
    let ordinary = synthetic_spec(
        "d6_profile_ordinary",
        Some(operator_and_core),
        Traits::empty(),
    );
    let operator = synthetic_spec(
        "d6_profile_operator",
        Some(operator_and_core),
        Traits::OPERATOR_COMMAND,
    );
    let specs = [ordinary, operator];

    // iRules does not treat math operators as command heads, so the
    // earlier ordinary row must remain selected despite the operator row
    // being registered later. Plain Tcl permits those heads, preserving
    // the later-registration tie-break.
    let irules = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::irules());
    assert_best_visible_matches_reference(irules, &specs, irules.own_surface_query());
    let plain = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert_best_visible_matches_reference(plain, &specs, plain.own_surface_query());
    assert_eq!(
        irules
            .best_visible(&specs, irules.own_surface_query())
            .map(|spec| spec.name),
        Some("d6_profile_ordinary")
    );
    assert_eq!(
        plain
            .best_visible(&specs, plain.own_surface_query())
            .map(|spec| spec.name),
        Some("d6_profile_operator")
    );
}

/// A registry whose pack declared a floor for a package its point carries
/// asks under a query that is not `==` its profile's — and is still that
/// profile's own point, so the profile's operator exclusion applies to it.
#[test]
fn a_floor_on_the_own_query_leaves_it_the_profiles_own_point() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let iapps = tcl_dialect::DialectProfile::find("f5-iapps").expect("catalogue profile");
    assert!(!iapps.operators_as_commands);
    let mut registry = CommandRegistry::build_default().project_for_profile(iapps);
    registry.insert_ambient_package("iapps", "1.0");
    let own = registry.own_surface_query();
    let query = own.expect("a profiled registry has a point");
    assert_eq!(
        query.package("iapps"),
        Some(PackageFloor::at("iapps", "1.0"))
    );
    assert_ne!(query, iapps.surface_query());
    assert!(query.same_point(&iapps.surface_query()));

    let operator = synthetic_spec("floor_probe_operator", None, Traits::OPERATOR_COMMAND);
    let ordinary = synthetic_spec("floor_probe_ordinary", None, Traits::empty());
    assert!(!registry.spec_visible(operator, own));
    assert!(registry.spec_visible(ordinary, own));
    assert!(
        registry.spec_visible(operator, Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        "asked at another point, the exclusion is not this profile's to apply"
    );
}

/// Whether a name is a command in this registry's own dialect is asked at
/// the point the registry's floors make, however late a floor arrives.
#[test]
fn a_command_is_known_here_only_where_the_floored_point_admits_it() {
    // The `tk` environment runs on its 8.6 core and ships the Tk that
    // tracks it: its own Tk 8.6 floor admits a Tk 8.6 command whatever
    // weaker floor is declared, and its core never admits a Tk 9 one.
    const FROM_8_6: &[tcl_dialect::model::SpecWindow] = &[("8.6", None)];
    const FROM_9_0: &[tcl_dialect::model::SpecWindow] = &[("9.0", None)];
    let probes = [
        synthetic_spec(
            "floor_probe_from_86",
            Some(surface![SpecSurface::package_in("Tk", FROM_8_6)]),
            Traits::empty(),
        ),
        synthetic_spec(
            "floor_probe_from_90",
            Some(surface![SpecSurface::package_in("Tk", FROM_9_0)]),
            Traits::empty(),
        ),
    ];
    for floor in [None, Some("8.5"), Some("9.0")] {
        let mut authored = CommandRegistry::build_default();
        for probe in probes {
            authored.insert_static(probe);
        }
        let mut registry = authored.project_for_profile(tcl_dialect::DialectProfile::tk());
        if let Some(floor) = floor {
            registry.insert_ambient_package("Tk", floor);
        }
        assert!(
            registry.has_command_in_this_dialect("floor_probe_from_86"),
            "under a floor of {floor:?}"
        );
        assert!(
            !registry.has_command_in_this_dialect("floor_probe_from_90"),
            "under a floor of {floor:?}"
        );
    }
}

#[test]
fn get_resolves_global_qualifier_to_builtin() {
    let reg = CommandRegistry::build_default();
    assert!(reg.get("::foreach").is_some());
    assert_eq!(
        reg.get("::for").map(|s| s.name),
        reg.get("for").map(|s| s.name)
    );
    assert!(reg.get("::nonexistent_command").is_none());
}

#[test]
fn switch_names_is_dialect_filtered() {
    let reg = CommandRegistry::build_default();
    let regsub = reg.get("regsub").expect("regsub spec");
    // `-command` is Tcl 9.0+ (TIP 463); the always-available
    // switches appear in every dialect.
    let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(in_86.contains(&"-all"), "{in_86:?}");
    assert!(in_86.contains(&"-nocase"), "{in_86:?}");
    assert!(
        !in_86.contains(&"-command"),
        "9.0-only -command leaked into 8.6: {in_86:?}",
    );
    let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert!(
        in_90.contains(&"-command"),
        "-command missing under 9.0: {in_90:?}",
    );
    // No filter → every declared option, no duplicates.
    let all = regsub.switch_names(None);
    assert!(all.contains(&"-command"));
    let mut dedup = all.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(dedup.len(), all.len(), "switch_names returned duplicates");
}

#[test]
fn lookup_for_command() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get("for").unwrap();
    assert_eq!(spec.name, "for");
    assert!(spec.traits.contains(Traits::CONTROL_FLOW));
    assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
    assert_eq!(spec.arity, crate::arity::Arity::exact(4));
}

#[test]
fn ttk_widgets_require_tk_8_5() {
    // ttk (themed Tk) widgets were introduced in Tk 8.5, so they must be
    // gated out when only an older Tk is guaranteed by `package require`.
    let reg = CommandRegistry::build_default();
    for name in [
        "ttk::button",
        "ttk::treeview",
        "ttk::notebook",
        "ttk::style",
    ] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            !spec.available_for_version(Some("8.4")),
            "{name} must not be available under Tk 8.4",
        );
        assert!(
            spec.available_for_version(Some("8.5")),
            "{name} must be available under Tk 8.5",
        );
        assert!(
            spec.available_for_version(None),
            "{name} must be permissive when no Tk version is pinned",
        );
    }
}

/// The providers of a name across the compiled-in universe: Expect's
/// `system` is offered by that package alone, `dict` by a core Tcl
/// window, a name nothing defines by nobody.
#[test]
fn providers_in_any_dialect_names_who_offers_a_command() {
    let reg = CommandRegistry::build_default();
    let system = reg
        .providers_in_any_dialect("system")
        .expect("system is Expect's");
    assert!(!system.unrestricted);
    assert_eq!(system.providers, [SpecProvider::Package("expect")]);

    let dict = reg.providers_in_any_dialect("dict").expect("dict is Tcl's");
    assert!(
        dict.providers.contains(&SpecProvider::Core(Family::Tcl)),
        "{dict:?}"
    );

    assert!(
        reg.providers_in_any_dialect("no_such_command_anywhere")
            .is_none()
    );
    assert_eq!(
        reg.providers_in_any_dialect("::dict"),
        reg.providers_in_any_dialect("dict"),
        "a rooted spelling names the same providers"
    );
}

/// `declares_command_at` answers "is this in a fresh
/// interpreter's command table", which is strictly narrower than `get`'s
/// "is this a spelling a call site may write".
///
/// Every row is oracle-verified on tclsh 9.0.4 and 8.6.14 via
/// `info commands <name>` in a fresh `interp create`.
#[test]
fn declares_command_at_is_the_fresh_interpreter_command_table() {
    let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
    // `info commands ::set` -> ::set
    for name in ["set", "::set", "puts", "::puts", "if", "foreach"] {
        assert!(reg.declares_command_at(name), "{name} is a global builtin");
    }
    // `info commands ::tcl::mathop::+` -> ::tcl::mathop::+
    for name in [
        "tcl::mathop::+",
        "::tcl::mathop::+",
        "oo::define",
        "::oo::define",
    ] {
        assert!(
            reg.declares_command_at(name),
            "{name} is a namespaced builtin"
        );
    }
    // `info commands ::+` -> {} and `+ 1 2` -> invalid command name "+".
    // Only qualified mathop names are registered by the Tcl command pack.
    for name in ["+", "::+", "eq", "::eq", "in", "::in"] {
        assert!(
            !reg.declares_command_at(name),
            "{name} is only callable after `namespace import ::tcl::mathop::*`",
        );
    }
    // A name nothing declares.
    for name in ["notACommand", "::notACommand", "define"] {
        assert!(!reg.declares_command_at(name), "{name}");
    }
}

/// The answer is per dialect, because the command table is: `HTTP::uri`
/// exists for iRules and nowhere else.
#[test]
fn declares_command_at_is_dialect_specific() {
    assert!(
        crate::model::ingress::static_context_for("f5-irules")
            .commands()
            .declares_command_at("HTTP::uri"),
        "iRules declares HTTP::uri",
    );
    assert!(
        !crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .declares_command_at("HTTP::uri"),
        "plain Tcl does not",
    );
}

#[test]
fn smoke_lookup_set_command() {
    let reg = CommandRegistry::build_default();
    let spec = reg.get("set").expect("`set` must be a known command");
    assert!(
        spec.arity.min <= 1 && spec.arity.max >= 1,
        "set must accept at least a variable name: {:?}",
        spec.arity
    );
}
