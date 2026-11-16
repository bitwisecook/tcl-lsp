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

use super::DialectProfile;
use super::EvaluationEvidence;
use crate::grammar::{BracedVarStyle, EscapeSyntax, ExprCommentStyle, NumberSyntax};
use crate::library::{LibraryVersion, LibraryVersionOverrides, VersionKey};
use crate::model::{
    CorePoints, Family, PackageFloor, SpecProvider, SpecSurface, SurfaceQuery, surface_admits,
};
use crate::version::{TclVersion, Ternary};

#[test]
fn library_pins_follow_the_dialect_shape() {
    // §7 "Libraries" column invariants, derived not enumerated:
    for p in all_with_fallback() {
        // Every F5 profile keys its own surface on the BIG-IP release.
        if p.name.starts_with("f5-") {
            assert!(
                p.libraries.iter().any(|pin| pin.ambient
                    && pin.version == LibraryVersion::Keyed(VersionKey::BigipVersion)),
                "{}: F5 surfaces are keyed on BigipVersion",
                p.name
            );
        }
        // Every EDA shell keys sdc + upf + its tool; all ambient.
        if p.name.ends_with("-eda-tcl") {
            assert!(
                p.libraries.iter().any(|pin| pin.package == "sdc"
                    && pin.version == LibraryVersion::Keyed(VersionKey::SdcVersion)),
                "{}: sdc keyed on SdcVersion",
                p.name
            );
            assert!(
                p.libraries.iter().any(|pin| pin.package == "upf"
                    && pin.version == LibraryVersion::Keyed(VersionKey::UpfVersion)),
                "{}: upf keyed on UpfVersion",
                p.name
            );
            assert!(
                p.libraries
                    .iter()
                    .any(|pin| pin.version == LibraryVersion::Keyed(VersionKey::ToolVersion)),
                "{}: tool keyed on ToolVersion",
                p.name
            );
            assert!(p.libraries.iter().all(|pin| pin.ambient), "{}", p.name);
        }
        // Plain versioned Tcl hosts Tk (tracking base) + Itcl (pinned),
        // neither ambient — `package require` still applies.
        if p.name.starts_with("tcl") && p.name != "tcl" {
            let tk = p.library_pin("Tk").expect("plain Tcl hosts Tk");
            assert_eq!(tk.version, LibraryVersion::TracksBase, "{}", p.name);
            assert!(!tk.ambient, "{}: Tk still needs its require", p.name);
            assert!(
                p.library_pin("Itcl")
                    .is_some_and(|i| !i.ambient && matches!(i.version, LibraryVersion::Pinned(_))),
                "{}: Itcl is a pinned hosted library",
                p.name
            );
        }
        // Closed vendor worlds never host Tk (§2.2).
        if p.vendor_surface.is_some() {
            assert!(
                p.library_pin("Tk").is_none(),
                "{}: vendor shells never host Tk",
                p.name
            );
        }
        // The permissive fallback pins nothing.
        if p.is_fallback() {
            assert_eq!(p.libraries, []);
        }
    }
}

#[test]
fn library_floor_resolution_covers_all_pin_kinds() {
    let none = LibraryVersionOverrides::default();

    // TracksBase → the runtime base as a package version.
    let tcl86 = DialectProfile::find("tcl8.6").expect("catalogue profile");
    assert_eq!(tcl86.library_floor("Tk", &none), Some("8.6"));
    let tcl90 = DialectProfile::find("tcl9.0").expect("catalogue profile");
    assert_eq!(tcl90.library_floor("Tk", &none), Some("9.0"));

    // Pinned → the pinned string.
    assert_eq!(tcl86.library_floor("Itcl", &none), Some("4.2"));
    assert_eq!(
        DialectProfile::find("tcl8.4")
            .expect("catalogue profile")
            .library_floor("Itcl", &none),
        Some("3.4")
    );
    assert_eq!(
        DialectProfile::find("expect")
            .expect("catalogue profile")
            .library_floor("Expect", &none),
        Some("5.45.4")
    );

    // Keyed → override, else the oldest-supported default.
    let irules = DialectProfile::irules();
    assert_eq!(
        irules.library_floor("f5-irules-cmds", &none),
        Some("16.1.0"),
        "BigipVersion defaults to the oldest supported TMOS"
    );
    let pinned = LibraryVersionOverrides {
        bigip_version: Some("17.1.0".to_owned()),
        ..LibraryVersionOverrides::default()
    };
    assert_eq!(
        irules.library_floor("f5-irules-cmds", &pinned),
        Some("17.1.0"),
        "an explicit pin overrides the default"
    );
    // Keyed with no default and no override → permissive.
    let synopsys = DialectProfile::find("synopsys-eda-tcl").expect("catalogue profile");
    assert_eq!(synopsys.library_floor("synopsys-dc", &none), None);

    // Unpinned package → no profile floor.
    assert_eq!(tcl86.library_floor("no-such-lib", &none), None);
    // Ambience predicate: F5 surface ambient, Tk hosted.
    assert!(irules.is_ambient_package("f5-irules-cmds"));
    assert!(!tcl86.is_ambient_package("Tk"));
    assert!(!tcl86.is_ambient_package("no-such-lib"));
}

#[test]
fn help_terms_cover_every_real_dialect() {
    // §5.4: every catalogue profile carries help-filter terms; only the
    // permissive fallback filters nothing. The versioned-Tcl profiles
    // (tcl9.1 included) share the
    // tcl/tk terms; every vendor profile's terms include a
    // vendor-identifying string.
    for p in DialectProfile::all() {
        assert!(
            !p.help_terms.is_empty(),
            "{}: catalog profiles carry help terms",
            p.name
        );
        if p.name.starts_with("tcl") {
            assert_eq!(p.help_terms, &["tcl", "tk"], "{}", p.name);
        }
    }
    assert!(
        DialectProfile::plain_tcl().help_terms.is_empty(),
        "the fallback applies no filter"
    );
    // Alias canonicalisation (§2.4): the legacy `irules` spelling
    // resolves to the same terms as the canonical profile — a plain
    // string-keyed table would silently apply no filter to it.
    assert_eq!(
        DialectProfile::find("irules")
            .expect("catalogue profile")
            .help_terms,
        DialectProfile::find("f5-irules")
            .expect("catalogue profile")
            .help_terms
    );
    assert!(
        DialectProfile::find("f5-tmsh")
            .expect("catalogue profile")
            .help_terms
            .contains(&"tmsh")
    );
    assert!(
        DialectProfile::find("bpf")
            .expect("catalogue profile")
            .help_terms
            .contains(&"bpf")
    );
}

#[test]
fn catalog_is_one_profile_per_canonical_name_in_sorted_order() {
    let names: Vec<&str> = DialectProfile::all().iter().map(|p| p.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}

#[test]
fn find_resolves_canonical_names_to_themselves() {
    for profile in DialectProfile::all() {
        assert_eq!(
            DialectProfile::find(profile.name)
                .expect("catalogue profile")
                .name,
            profile.name
        );
    }
}

#[test]
fn unknown_and_ingress_only_names_are_not_catalogue_entries() {
    // The catalogue lookup answers `None` for anything that is not a
    // canonical name or alias — the lenient-sink behaviour for a
    // user-written string lives in the
    // environment seam (`tcl_registry::model::ingress`), where its
    // tests pin it.
    for unknown in ["", "nonsense", "tcl8.7", "TCL8.6", "tk", "tcl"] {
        assert!(DialectProfile::find(unknown).is_none(), "{unknown:?}");
    }
}

#[test]
fn tk_ingress_profile_keeps_the_typed_library_bit() {
    // Tk deliberately remains a library pin rather than an
    // editor-visible catalogue profile, but an explicit `tk` / wish
    // document must retain the typed command-surface fact — carried by
    // the dedicated ingress profile the environment seam promotes.
    assert_eq!(DialectProfile::tk().name, "tk");
    assert_eq!(
        DialectProfile::tk().surface_query(),
        SurfaceQuery {
            realm: crate::model::InvocationRealm::RuleLoader,
            core: DialectProfile::plain_tcl().surface_query().core,
            packages: crate::model::PackageFloors::Borrowed(&[PackageFloor::named("Tk")]),
        }
    );
    // A canonical profile and a legacy alias keep their profile-owned
    // security scope: the iRules sandbox point is the same under either
    // spelling.
    assert_eq!(
        DialectProfile::find("f5-irules")
            .expect("catalogue profile")
            .surface_query(),
        SurfaceQuery::any_release(Family::F5Irules)
    );
    assert_eq!(
        DialectProfile::find("irules")
            .expect("registered alias")
            .surface_query(),
        SurfaceQuery::any_release(Family::F5Irules)
    );
}

#[test]
fn irules_handle_is_the_catalog_entry() {
    let via_handle = DialectProfile::irules();
    let via_name = DialectProfile::find("f5-irules").expect("catalogue profile");
    assert!(std::ptr::eq(via_handle, via_name));
    assert_eq!(via_handle.name, "f5-irules");
    assert!(via_handle.is_irules());
    assert!(
        !DialectProfile::find("tcl8.4")
            .expect("catalogue profile")
            .is_irules()
    );
}

#[test]
fn profiles_are_interned_pointer_identities() {
    assert!(std::ptr::eq(
        DialectProfile::find("tcl8.6").expect("catalogue profile"),
        DialectProfile::find("tcl8.6").expect("catalogue profile")
    ));
    assert!(!std::ptr::eq(
        DialectProfile::find("tcl8.6").expect("catalogue profile"),
        DialectProfile::find("tcl9.0").expect("catalogue profile")
    ));
}

#[test]
fn irules_aliases_canonicalise_to_the_same_profile() {
    // §2.4: `irules` / `tcl-irule` must resolve to the f5-irules
    // profile so profile predicates can never disagree with the
    // canonical spelling.
    for alias in ["irules", "tcl-irule"] {
        let p = DialectProfile::find(alias).expect("catalogue profile");
        assert!(p.is_irules(), "{alias:?} must canonicalise to f5-irules");
    }
}

#[test]
fn the_irules_point_names_the_family_and_no_release() {
    // A command is available under iRules iff its own surface names the
    // family. Every core command states a surface, so there is no
    // catch-all and no subtractive ban list — a sandbox-banned command
    // such as `exec` simply does not name iRules and falls out. The
    // command-level checks (`exec`/`file`/`socket` unavailable,
    // `pool`/`set` available) live in tcl-registry's
    // `dialect_profile.rs` suite, which has the registry; here we pin
    // only the point shape the scheme rests on.
    let p = DialectProfile::irules();
    assert_eq!(
        p.surface_query(),
        SurfaceQuery::any_release(Family::F5Irules)
    );
    assert_eq!(p.vendor_surface, Some(SpecProvider::Core(Family::F5Irules)));
}

#[test]
fn an_additive_vendor_point_carries_its_base_release_and_its_package() {
    // §7: an *additive* vendor profile is a base release plus its own
    // package. The EDA shells moved to the packaged model (a plain base
    // release plus a `required_package` gate — see
    // eda-library-packages.md), so only the F5 iApps host shell and
    // Expect remain additive here.
    // f5-iapps composes the **8.4** line: it rides the `f5-tcl` trunk
    // (fork of Tcl at 8.4.6) — measured, bigip-irule-parser-measurements.md
    // §4a; the 8.5 hypothesis is falsified.
    let cases: &[(&str, &str, &[PackageFloor<'_>])] = &[
        ("f5-iapps", "8.4", &[PackageFloor::named("iapps")]),
        ("expect", "8.6", &[PackageFloor::named("expect")]),
    ];
    for &(name, base, packages) in cases {
        let p = DialectProfile::find(name).expect("catalogue profile");
        assert_eq!(
            p.surface_query(),
            SurfaceQuery::core(Family::Tcl, base).with_packages(packages),
            "{name}"
        );
    }
}

#[test]
fn eda_shells_are_packaged_vendors_with_a_plain_release_point() {
    // Each EDA shell's point is its plain
    // base Tcl release, with no vendor surface — the vendor command
    // surface is gated by `required_package` (ambient in the profile).
    // Base versions follow the tools' embedded cores (owner decisions):
    // Cadence 8.4-safe, Xilinx/Quartus 8.5, Synopsys + modern Questa 8.6.
    for (name, base) in [
        ("xilinx-eda-tcl", "8.5"),
        ("synopsys-eda-tcl", "8.6"),
        ("cadence-eda-tcl", "8.4"),
        ("intel-quartus-eda-tcl", "8.5"),
        ("mentor-eda-tcl", "8.6"),
    ] {
        let p = DialectProfile::find(name).expect("catalogue profile");
        assert_eq!(
            p.surface_query(),
            SurfaceQuery::core(Family::Tcl, base),
            "{name}: a pure base-release point with no vendor package"
        );
    }
}

#[test]
fn plain_tcl_versions_keep_their_exact_release() {
    for (name, release) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
    ] {
        assert_eq!(
            DialectProfile::find(name)
                .expect("catalogue profile")
                .surface_query(),
            SurfaceQuery::core(Family::Tcl, release)
        );
    }
}

#[test]
fn first_class_vendor_masks_are_precise() {
    // The first-class vendor masks are precise (D7/D8). f5-tmsh: the
    // tmsh shell hosts the `f5-tcl` trunk (fork of Tcl at 8.4.6 —
    // measured, bigip-irule-parser-measurements.md §4a) plus its own
    // surface.
    assert_eq!(
        DialectProfile::find("f5-tmsh")
            .expect("catalogue profile")
            .surface_query(),
        SurfaceQuery::core(Family::Tcl, "8.4").with_packages(&[PackageFloor::named("tmsh")])
    );
    // f5-bigip: identity only — a config parser with no Tcl surface;
    // BIG-IP documents route to the tcl-bigip validator, never the Tcl
    // analyser.
    assert_eq!(
        DialectProfile::find("f5-bigip")
            .expect("catalogue profile")
            .surface_query(),
        SurfaceQuery {
            realm: crate::model::InvocationRealm::RuleLoader,
            core: CorePoints::NONE,
            packages: crate::model::PackageFloors::Borrowed(&[PackageFloor::named("bigip")]),
        }
    );
    // bpf embeds a genuine Tcl 9.0.
    assert_eq!(
        DialectProfile::find("bpf")
            .expect("catalogue profile")
            .surface_query(),
        SurfaceQuery::core(Family::Tcl, "9.0").with_packages(&[PackageFloor::named("bpf")])
    );
}

#[test]
fn grammar_union_covers_every_provider_the_point_names() {
    // §10: the static-grammar union over-approximates (or equals, for
    // the iRules profile) the precise point — never under-approximates.
    for p in DialectProfile::all() {
        let query = p.surface_query();
        for (family, _) in query.core.iter() {
            assert!(
                p.grammar_union.contains(&SpecProvider::Core(family)),
                "{}: grammar_union must cover the point's core family",
                p.name
            );
        }
        for carried in query.packages {
            let package = carried.name;
            assert!(
                p.grammar_union.contains(&SpecProvider::Package(package)),
                "{}: grammar_union must cover the point's `{package}` package",
                p.name
            );
        }
    }
}

#[test]
fn irules_grammar_union_is_the_shipped_bare_bit_fix() {
    // The shipped iRules highlight fix is literally "the grammar union
    // is the iRules surface and nothing else" (§9.1).
    let p = DialectProfile::irules();
    assert_eq!(p.grammar_union, &[SpecProvider::Core(Family::F5Irules)]);
}

// Behaviour axis — the §7.1 derivation rules as
// invariants, so the hand-laid table can never drift from the model.

fn all_with_fallback() -> impl Iterator<Item = &'static DialectProfile> {
    DialectProfile::all()
        .iter()
        .chain(std::iter::once(DialectProfile::plain_tcl()))
}

#[test]
fn surface_packages_carry_the_vendor_surface() {
    // The point's package half is the vendor package wherever there is
    // one; `tk` is the single documented addition (a library, not a
    // closed-world vendor surface).
    for p in all_with_fallback() {
        match p.vendor_surface {
            Some(SpecProvider::Package(package)) => assert_eq!(
                p.surface_packages,
                [PackageFloor::named(package)],
                "{}: the point carries exactly its vendor package, with no floor",
                p.name
            ),
            _ => assert!(
                p.surface_packages.is_empty() || p.name == "tk",
                "{}: only `tk` adds a package without a vendor surface",
                p.name
            ),
        }
    }
}

#[test]
fn the_vendor_surface_composes_the_point() {
    // §2.2: a profile's point is one of
    // - a bare core family (iRules, §9);
    // - a Tcl release plus the vendor package (iApps, Expect, tmsh,
    //   bpf) or the bigip identity; or
    // - a bare Tcl release for a *packaged* vendor (the EDA shells),
    //   whose `vendor_surface` is a loading marker only — the vendor
    //   command surface is gated by `required_package`, not the point.
    for p in all_with_fallback() {
        let query = p.surface_query();
        match p.vendor_surface {
            Some(SpecProvider::Core(family)) => {
                assert_eq!(
                    query,
                    SurfaceQuery::any_release(family),
                    "{}: a core vendor surface asks as that family alone (§9)",
                    p.name
                );
            }
            Some(SpecProvider::Package(package)) => {
                assert!(
                    query.carries(package),
                    "{}: the point must carry the vendor package",
                    p.name
                );
            }
            None => {
                assert!(
                    query.packages.is_empty(),
                    "{}: no vendor surface — the point carries no package",
                    p.name
                );
            }
        }
        assert!(
            query
                .core
                .iter()
                .all(|(family, _)| family == Family::Tcl || p.is_irules()),
            "{}: a non-iRules point asks on the Tcl ladder",
            p.name
        );
    }
}

#[test]
fn version_ceiling_tracks_the_signature_base() {
    // §7: the option-gating ceiling is the signature base everywhere —
    // including the first-class vendor profiles (f5-tmsh V8_5;
    // f5-bigip has neither, being a non-Tcl surface). Only the
    // permissive fallback stays unceilinged by design.
    for p in all_with_fallback() {
        assert_eq!(
            p.version_ceiling, p.signature_base,
            "{}: ceiling == signature base",
            p.name
        );
    }
}

#[test]
fn octal_policy_is_derived_from_the_runtime_base() {
    // §7.1: octal = runtime_base < V9_0; Inert exactly when there is
    // no Tcl runtime (never a silent default — §11.1).
    for p in all_with_fallback() {
        let expected = match p.runtime_base {
            None => Ternary::Inert,
            Some(v) if v < TclVersion::V9_0 => Ternary::Yes,
            Some(_) => Ternary::No,
        };
        assert_eq!(p.leading_zero_is_octal, expected, "{}", p.name);
    }
}

#[test]
fn expr_grammar_base_equals_runtime_base() {
    // §7.1: the expr grammar (TIP 201/461, mathfunc tiers) follows the
    // runtime, not the signature surface.
    for p in all_with_fallback() {
        assert_eq!(p.expr_grammar_base, p.runtime_base, "{}", p.name);
    }
}

#[test]
fn vm_runtime_version_tracks_the_profile_runtime_base() {
    // The VM must not silently execute a vendor or versioned profile with
    // Tcl 9 semantics.  A non-Tcl / permissive profile has no runtime base,
    // so its deliberately documented fallback stays the C Tcl 9.0 oracle.
    for p in all_with_fallback() {
        assert_eq!(
            p.vm_runtime_version,
            p.runtime_base.unwrap_or(TclVersion::V9_0),
            "{}",
            p.name
        );
    }
}

#[test]
fn signature_and_runtime_base_agree_everywhere_today() {
    // Every profile in the current catalogue has signature == runtime
    // (iRules has both at V8_4 — D3). The fields stay structurally
    // separate because future dialects may split them; this test
    // documents that no current entry does.
    for p in all_with_fallback() {
        assert_eq!(p.signature_base, p.runtime_base, "{}", p.name);
    }
}

#[test]
fn tcloo_agrees_with_what_the_point_resolves() {
    // §11.2: the hand-filled tcloo bool must agree with what the point
    // resolves for `oo::*` (8.6 and later) — otherwise hover and the oo
    // handler would contradict each other. Documented exception:
    // f5-bigip has NO Tcl surface at all (tcloo false is the model
    // truth, §7), so it is asserted directly.
    for p in all_with_fallback() {
        if p.name == "f5-bigip" {
            assert!(!p.tcloo, "f5-bigip has no Tcl surface at all");
            continue;
        }
        assert_eq!(
            p.tcloo,
            surface_admits(SpecSurface::TCL86_PLUS, Some(&p.surface_query())),
            "{}: tcloo must match what the point resolves for oo::*",
            p.name
        );
    }
}

#[test]
fn fixed_ensembles_cover_exactly_the_f5_family() {
    // §7.1: {f5-irules, f5-iapps, f5-bigip} — NOT f5-tmsh (a wrong
    // `true` mis-minifies tmsh scripts).
    for p in all_with_fallback() {
        let expected = matches!(p.name, "f5-irules" | "f5-iapps" | "f5-bigip");
        assert_eq!(p.has_fixed_ensembles, expected, "{}", p.name);
    }
}

#[test]
fn operators_are_commands_everywhere_but_irules_bigip_tcl84_and_cadence() {
    // §9: the math-operator heads (`::tcl::mathop`, TIP 174) exist in every
    // command dialect on a Tcl 8.5+ core. The pre-8.5 cores have none — the
    // `::tcl::` namespace itself is 8.5+: the F5 trunk profiles (all three
    // ride the fork of Tcl at 8.4.6, and `::tcl::mathop` is measured
    // absent in every BIG-IP execution context —
    // bigip-irule-parser-measurements.md §4a), plain tcl8.4, and Cadence
    // Innovus/Genus (8.4-safe, owner decision). f5-bigip has no command
    // surface at all; `tk` is a library pin, not a profile.
    for p in all_with_fallback() {
        let expected = !(p.is_irules()
            || p.name == "f5-bigip"
            || p.name == "f5-iapps"
            || p.name == "f5-tmsh"
            || p.name == "tcl8.4"
            || p.name == "cadence-eda-tcl");
        assert_eq!(p.operators_as_commands, expected, "{}", p.name);
    }
}

#[test]
fn lexer_grammar_follows_the_runtime_base() {
    // 8.x runtimes lex `${…}` to the FIRST close brace; 9.x (and the
    // no-runtime profiles) use the modern nesting rule. `{*}` expansion
    // is 8.5+; the `}{` ghost separator is iRules-only.
    for p in all_with_fallback() {
        let expected_braced = match p.runtime_base {
            Some(v) if v < TclVersion::V9_0 => BracedVarStyle::FirstClose,
            _ => BracedVarStyle::Tcl9Nesting,
        };
        assert_eq!(p.grammar.braced_var, expected_braced, "{}", p.name);
        let expected_expand = p.runtime_base.is_none_or(|v| v >= TclVersion::V8_5);
        assert_eq!(p.grammar.expand_syntax, expected_expand, "{}", p.name);
        // The implicit word break (R-rules) is an `f5-tcl` **trunk**
        // fact, measured byte-identical in all three BIG-IP execution
        // contexts (bigip-irule-parser-measurements.md §1, §4a) — so
        // every trunk-riding profile carries it, not just iRules.
        let expected_separator = matches!(p.name, "f5-irules" | "f5-iapps" | "f5-tmsh");
        assert_eq!(
            p.grammar.irules_brace_separator, expected_separator,
            "{}",
            p.name
        );
        // TIP 582 `expr` comments: the `COMMENT` lexeme and
        // `ParseLexeme`'s `case '#':` appear in `tclCompExpr.c` from the
        // 8.7/9.0 cycle and are absent at core-8-4-20 / core-8-5-19 /
        // core-8-6-16, so `>= V9_0` is the exact gate for the versions
        // modelled here.
        let expected_comments = if p.runtime_base.is_none_or(|v| v >= TclVersion::V9_0) {
            ExprCommentStyle::Hash
        } else {
            ExprCommentStyle::None
        };
        assert_eq!(p.grammar.expr_comments, expected_comments, "{}", p.name);
        // Numeric-literal grammar: `tclStrToD.c` does not exist at
        // core-8-4-20 (8.4 scans integers strtoul-style, knowing only `0x`
        // and leading-zero octal); core-8-5-19 / core-8-6-16 add the
        // `ZERO_B`/`ZERO_O` states but no `ZERO_D` and keep
        // `#undef KILL_OCTAL`; 9.0.4 adds `ZERO_D` plus `_` separators and
        // drops leading-zero octal (`changes.md`). So the gates are exactly
        // `>= V8_5` for `0b`/`0o` and `>= V9_0` for `0d` / `_` / decimal
        // leading zero.
        let expected_numbers = match p.runtime_base {
            Some(TclVersion::V8_4) => NumberSyntax::Tcl84,
            Some(TclVersion::V8_5 | TclVersion::V8_6) => NumberSyntax::Tcl85,
            _ => NumberSyntax::Tcl90,
        };
        assert_eq!(p.grammar.numbers, expected_numbers, "{}", p.name);
        // Backslash-escape grammar: TIP 388 (8.6) capped `\x` at two hex
        // digits, added `\U`, and guarded the octal third digit, so 8.4 and
        // 8.5 share one rule and 8.6 starts another. 9.0 keeps 8.6's
        // widths and raises `TCL_UTF_MAX` to 4, so a decoded scalar past
        // U+FFFF stops degrading to U+FFFD.
        let expected_escapes = match p.runtime_base {
            Some(TclVersion::V8_4 | TclVersion::V8_5) => EscapeSyntax::Tcl84,
            Some(TclVersion::V8_6) => EscapeSyntax::Tcl86,
            _ => EscapeSyntax::Tcl90,
        };
        assert_eq!(p.grammar.escapes, expected_escapes, "{}", p.name);
    }
}

#[test]
fn escape_grammar_splits_85_from_86() {
    // The one axis on which 8.5 and 8.6 differ — they share a numeral
    // grammar, a `${…}` rule, and an `expr` grammar, so a single shared
    // 8.x `LexerGrammar` constant would silently give 8.5 TIP 388's rules.
    assert_eq!(
        DialectProfile::find("tcl8.5")
            .expect("catalogue profile")
            .grammar
            .escapes,
        EscapeSyntax::Tcl84
    );
    assert_eq!(
        DialectProfile::find("tcl8.6")
            .expect("catalogue profile")
            .grammar
            .escapes,
        EscapeSyntax::Tcl86
    );
    assert_eq!(
        DialectProfile::find("tcl8.5")
            .expect("catalogue profile")
            .grammar
            .numbers,
        DialectProfile::find("tcl8.6")
            .expect("catalogue profile")
            .grammar
            .numbers
    );
    // iRules is a genuine embedded 8.4.6, so it takes the 8.4 escapes.
    assert_eq!(
        DialectProfile::find("f5-irules")
            .expect("catalogue profile")
            .grammar
            .escapes,
        EscapeSyntax::Tcl84
    );
}

#[test]
fn expect_and_tmsh_lex_braced_vars_with_the_8x_rule() {
    // The 8.x rule a plain string-keyed lexer table would miss:
    // expect (8.6) and f5-tmsh (8.5) are 8.x runtimes, so `${a{b}c}`
    // names `a{b` — not the Tcl 9 nesting read.
    for name in ["expect", "f5-tmsh"] {
        assert_eq!(
            DialectProfile::find(name)
                .expect("catalogue profile")
                .grammar
                .braced_var,
            BracedVarStyle::FirstClose,
            "{name}"
        );
    }
    // bpf embeds Tcl 9.0: nesting, unchanged.
    assert_eq!(
        DialectProfile::find("bpf")
            .expect("catalogue profile")
            .grammar
            .braced_var,
        BracedVarStyle::Tcl9Nesting
    );
}

#[test]
fn const_fold_version_is_the_evaluation_point_and_the_release_the_name_parses_to() {
    for p in all_with_fallback() {
        assert_eq!(p.const_fold_version(), p.evaluation_point(), "{}", p.name);
        assert_eq!(
            p.const_fold_version(),
            TclVersion::from_dialect(Some(p.name)),
            "{}",
            p.name
        );
    }
    assert_eq!(
        DialectProfile::find("tcl8.4")
            .expect("catalogue profile")
            .const_fold_version(),
        Some(TclVersion::V8_4)
    );
}

/// A vendor profile whose release was measured folds under it: iRules, iApps
/// and tmsh are an 8.4 fork that every F5 context reports as 8.4.6 and that
/// fails every 8.5 discriminator probed, so a versioned fold answers as 8.4.
#[test]
fn a_measured_vendor_profile_has_an_evaluation_point() {
    for name in ["f5-irules", "f5-iapps", "f5-tmsh"] {
        let p = DialectProfile::find(name).expect("catalogue profile");
        let EvaluationEvidence::MeasuredFork { note } = p.evaluation_evidence else {
            panic!("{name} carries no measured-fork note");
        };
        assert!(
            note.contains("bigip-irule-parser-measurements.md"),
            "{name}: {note}"
        );
        assert_eq!(p.evaluation_point(), Some(TclVersion::V8_4), "{name}");
        assert_eq!(p.evaluation_point(), p.runtime_base, "{name}");
        assert!(!p.is_tcl_release(), "{name} is a fork and not a release");
    }
    assert_eq!(
        TclVersion::from_dialect(Some("irules")),
        Some(TclVersion::V8_4),
        "an alias resolves through the catalogue to the same point"
    );
}

/// A profile nothing measured has no evaluation point, whatever base it
/// models: the evidence is the gate, and the base alone is not.
#[test]
fn an_unmeasured_profile_has_none() {
    let mut modelled = 0;
    for p in all_with_fallback() {
        if p.evaluation_evidence != EvaluationEvidence::Unmeasured {
            continue;
        }
        assert_eq!(p.evaluation_point(), None, "{}", p.name);
        assert_eq!(TclVersion::from_dialect(Some(p.name)), None, "{}", p.name);
        modelled += usize::from(p.runtime_base.is_some());
    }
    assert!(
        modelled >= 5,
        "the gate is not vacuous: unmeasured profiles still model a base ({modelled})"
    );
    for name in ["expect", "bpf", "cadence-eda-tcl", "spectcl"] {
        let p = DialectProfile::find(name).expect("catalogue profile");
        assert!(p.runtime_base.is_some(), "{name} models a base");
        assert_eq!(p.evaluation_point(), None, "{name}");
    }
    assert_eq!(DialectProfile::plain_tcl().evaluation_point(), None);
    assert_eq!(DialectProfile::tk().evaluation_point(), None);
}

/// An evaluation point is the runtime base and nothing else, and a Tcl release
/// claims its reference toolchain exactly where the manifest pins one.
#[test]
fn an_evaluation_point_is_the_runtime_base_and_a_release_claims_a_pinned_toolchain() {
    for p in all_with_fallback() {
        if let Some(point) = p.evaluation_point() {
            assert_eq!(Some(point), p.runtime_base, "{}", p.name);
        }
        if p.is_tcl_release() {
            let base = p.runtime_base.expect("a Tcl release has a base");
            assert_eq!(Some(base), p.signature_base, "{}", p.name);
            assert!(p.vendor_surface.is_none(), "{}", p.name);
            assert!(
                !base.patchlevel().is_empty(),
                "{}: no pinned patchlevel",
                p.name
            );
            assert_eq!(p.name, format!("tcl{}", base.version_string()));
        }
    }
    let releases = all_with_fallback().filter(|p| p.is_tcl_release()).count();
    assert_eq!(
        releases,
        TclVersion::ALL.len(),
        "one profile per pinned release"
    );
}

#[test]
fn presentation_fields_follow_the_catalog_shape() {
    let mut seen_ext: Vec<&str> = Vec::new();
    let mut seen_lang: Vec<&str> = Vec::new();
    for p in all_with_fallback() {
        assert!(!p.display_name.is_empty(), "{}: display_name", p.name);
        assert!(!p.short_name.is_empty(), "{}: short_name", p.name);
        if let Some(lang) = p.editor_language_id {
            // Undotted by contract, and unique: two
            // dialects can't claim the same editor language.
            assert!(
                !lang.contains('.'),
                "{}: language id {lang:?} has a dot",
                p.name
            );
            assert!(
                !seen_lang.contains(&lang),
                "{}: language id {lang:?} reused",
                p.name
            );
            seen_lang.push(lang);
        }
        for row in p.file_extensions {
            assert!(
                !row.extension.is_empty()
                    && !row.extension.starts_with('.')
                    && row.extension.chars().all(|c| c.is_ascii_lowercase()),
                "{}: extension {:?} must be lower-case with no dot",
                p.name,
                row.extension
            );
            assert!(
                !row.display_name.is_empty(),
                "{}: {} name",
                p.name,
                row.extension
            );
            // Extension routing is a function: one owner per extension
            // across the whole catalogue.
            assert!(
                !seen_ext.contains(&row.extension),
                "{}: extension {:?} owned twice",
                p.name,
                row.extension
            );
            seen_ext.push(row.extension);
            // A dialect that owns file extensions must give the editors
            // somewhere to register them.
            assert!(
                p.editor_language_id.is_some() || p.name == "tcl",
                "{}: owns extensions but has no editor language",
                p.name
            );
        }
    }
}

/// The `filenames` axis obeys the same shape rules as `file_extensions`:
/// lower-case whole basenames, one owner apiece, and an editor language
/// to register them under.
#[test]
fn owned_filenames_follow_the_catalog_shape() {
    let mut seen: Vec<&str> = Vec::new();
    for p in all_with_fallback() {
        for name in p.filenames {
            assert!(
                !name.is_empty() && *name == name.to_ascii_lowercase(),
                "{}: filename {name:?} must be lower-case and non-empty",
                p.name
            );
            assert!(
                !name.contains('/') && !name.contains('\\'),
                "{}: filename {name:?} must be a bare basename",
                p.name
            );
            assert!(
                !seen.contains(name),
                "{}: filename {name:?} owned twice",
                p.name
            );
            seen.push(name);
            assert!(
                p.editor_language_id.is_some(),
                "{}: owns filenames but has no editor language",
                p.name
            );
        }
    }
    // The axis is not vacuous — BIG-IP's config basenames are its whole
    // reason to exist, and an empty catalogue would pass every rule above.
    assert!(
        seen.contains(&"bigip.conf"),
        "the BIG-IP config basenames must be catalogued"
    );
}

/// F5's character model must agree with
/// `model::family::CoreProfileId::character_model`, which keeps
/// `Utf16CodeUnits` for F5 pending a TMOS
/// measurement (#2151).
///
/// The two answers must agree, or a compile-time fold and the runtime
/// disagree for the same profile.
#[test]
fn f5_profiles_do_not_inherit_the_tcl_8_4_character_model() {
    use crate::StringCharacterModel;

    for name in ["f5-irules", "f5-tmsh", "f5-iapps"] {
        let profile = DialectProfile::find(name).expect("a known F5 profile");
        assert_eq!(
            profile.runtime_base,
            Some(TclVersion::V8_4),
            "{name} forks from 8.4 — that is what made this reachable",
        );
        assert_eq!(
            profile.character_model(),
            Some(StringCharacterModel::Utf16CodeUnits),
            "{name} must not inherit 8.4's model while F5's own value is \
                 unmeasured (#2151)",
        );
    }

    // Plain 8.4 is unaffected: it is the release the model was measured
    // on, so it keeps the new value.
    assert_eq!(
        DialectProfile::find("tcl8.4")
            .expect("tcl8.4")
            .character_model(),
        Some(StringCharacterModel::BmpCharsElseUtf8Bytes),
    );
}
