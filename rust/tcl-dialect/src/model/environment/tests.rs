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
use crate::DialectProfile;
use crate::TclVersion;

/// A stand-in for a pack-declared environment: `bpf`'s definition under
/// `id`, with none of `bpf`'s language-id claims.
fn pack_environment(id: &str) -> EnvironmentDefinition {
    let mut definition = bpf_environment();
    definition.id = EnvironmentId::new(id);
    definition.selecting_identities = Vec::new();
    definition
}

/// #2139: one predicate owns the untrusted class, and every variant has
/// a stated answer.
///
/// The inner `match` is the point: it is exhaustive, so a new
/// [`Provenance`] variant does not compile here until someone decides
/// its trust class. (The array still has to be extended by hand — the
/// match forces the decision, it cannot force the coverage.)
#[test]
fn provenance_states_a_trust_class_for_every_variant_issue_2139() {
    for provenance in [
        Provenance::BuiltIn,
        Provenance::BundledPack,
        Provenance::User,
        Provenance::WorkspaceTrusted,
        Provenance::WorkspaceUntrusted,
        Provenance::StudioOverride,
        Provenance::Document,
    ] {
        let expected = match provenance {
            Provenance::BuiltIn
            | Provenance::BundledPack
            | Provenance::User
            | Provenance::WorkspaceTrusted => false,
            Provenance::WorkspaceUntrusted | Provenance::StudioOverride | Provenance::Document => {
                true
            }
        };
        assert_eq!(
            provenance.is_untrusted(),
            expected,
            "{provenance:?} changed trust class"
        );
    }
}

/// Explicit editor trust determines workspace provenance; silence retains the default.
#[test]
fn a_workspace_is_trusted_until_the_editor_says_otherwise_issue_2139() {
    assert!(!Provenance::WorkspaceTrusted.is_untrusted());
    assert!(Provenance::WorkspaceUntrusted.is_untrusted());
    assert_eq!(WorkspaceTrust::default(), WorkspaceTrust::Trusted);
    assert!(
        !WorkspaceTrust::Trusted
            .workspace_provenance()
            .is_untrusted()
    );
    assert_eq!(
        WorkspaceTrust::Untrusted.workspace_provenance(),
        Provenance::WorkspaceUntrusted
    );
}

#[test]
fn every_old_name_and_alias_resolves() {
    let registry = EnvironmentRegistry::compiled();
    for profile in DialectProfile::all() {
        let resolved = registry.resolve(profile.name).expect(profile.name);
        for &alias in profile.aliases {
            let via_alias = registry
                .resolve(alias)
                .unwrap_or_else(|| panic!("alias `{alias}` of `{}` must resolve", profile.name));
            assert_eq!(via_alias.id, resolved.id, "{alias}");
        }
    }
    // The two off-catalogue profiles become real environments.
    assert_eq!(registry.resolve("tk").expect("tk").id.as_str(), "tk");
    assert_eq!(registry.resolve("tcl").expect("tcl").id.as_str(), "tcl");
    assert_eq!(registry.resolve("wish").expect("wish").id.as_str(), "tk");
}

/// The single-resolver property (§7 gates): `resolve` accepts exactly
/// {canonical ids} ∪ {aliases} ∪ {editor identities}, each input
/// resolving deterministically to the one environment that declares
/// it at the highest tier, and nothing else resolves.
#[test]
fn resolve_accepts_exactly_the_declared_names() {
    let registry = EnvironmentRegistry::compiled();
    let mut accepted: HashMap<String, String> = HashMap::new();
    for definition in registry.definitions() {
        let owner = definition.id.as_str().to_owned();
        accepted.insert(owner.clone(), owner.clone());
        for alias in &definition.aliases {
            accepted.insert(alias.as_ref().to_owned(), owner.clone());
        }
        if let Some(identity) = definition.editor_identity {
            accepted
                .entry(identity.as_str().to_owned())
                .or_insert_with(|| owner.clone());
        }
    }
    for (name, owner) in &accepted {
        let resolved = registry.resolve(name).unwrap_or_else(|| {
            panic!("declared name `{name}` must resolve");
        });
        assert_eq!(resolved.id.as_str(), owner, "{name}");
        // Deterministic: a second resolution gives the same value.
        assert_eq!(
            registry.resolve(name).expect("still resolves").id,
            resolved.id
        );
    }
    // Perturbations of every accepted name do not resolve, and
    // resolution is case-sensitive.
    for name in accepted.keys() {
        let padded = format!("{name}x");
        if !accepted.contains_key(&padded) {
            assert!(registry.resolve(&padded).is_none(), "{padded}");
        }
        let upper = name.to_uppercase();
        if upper != *name && !accepted.contains_key(&upper) {
            assert!(registry.resolve(&upper).is_none(), "{upper}");
        }
    }
    for unknown in ["", "nonsense", "tcl8.7", "jim0.85", "jim0.84"] {
        assert!(registry.resolve(unknown).is_none(), "{unknown}");
    }
}

/// One `jim` environment for a nine-release ladder: the releases are
/// targets on the family's own axis, not nine catalogue rows, and no
/// editor identity is minted for it.
#[test]
fn one_jim_environment_covers_the_whole_ladder() {
    let registry = EnvironmentRegistry::compiled();
    let jim = registry.resolve("jim").expect("jim");
    for alias in ["jimsh", "jimtcl"] {
        assert_eq!(registry.resolve(alias).expect(alias).id, jim.id, "{alias}");
    }
    // Exactly one compiled environment names the jim family.
    let jim_rows: Vec<&str> = registry
        .definitions()
        .iter()
        .filter(|definition| {
            definition
                .core
                .is_some_and(|core| core.family == Family::Jim)
        })
        .map(|definition| definition.id.as_str())
        .collect();
    assert_eq!(jim_rows, ["jim"], "nine profiles became one environment");

    let core = jim.core.expect("core");
    assert_eq!(core.default_release, Release::JIM_0_84);
    assert_eq!(core.build, BuildProfileId::Canonical);
    assert_eq!(jim.targets.axis(), &VersionAxisId::core(Family::Jim));
    // The whole ladder, so no release is implied.
    for release in Family::Jim.releases() {
        let point = Version::parse(release.as_str()).expect("jim releases spell versions");
        assert!(jim.targets.contains(&point), "{release}");
    }
    assert_eq!(
        jim.editor_identity.map(EditorLanguageIdentityId::as_str),
        Some("tcl-jim"),
        "jim opens under its own contributed language id"
    );
    assert_eq!(jim.short_name.as_ref(), "Jim");
    assert!(
        jim.expected_packages.is_empty(),
        "the jim surface rides the ancestry edge, not a placement"
    );
    assert_eq!(jim.policy_defaults.closed_world, WorldPolicy::Open);
    assert_eq!(
        jim.policy_defaults.version_ceiling, None,
        "the ceiling is a Tcl-ladder concept; jim has its own axis"
    );
}

#[test]
fn seeded_policies_translate_the_catalogue() {
    let registry = EnvironmentRegistry::compiled();
    let irules = registry.resolve("f5-irules").expect("irules");
    assert_eq!(irules.policy_defaults.closed_world, WorldPolicy::Closed);
    assert!(irules.policy_defaults.fixed_ensembles);
    assert_eq!(irules.core.expect("core").family, Family::F5Irules);
    assert!(irules.expected_packages.iter().any(|p| p.ambient
        && *p.package == *"f5-irules-cmds"
        && p.version == Placement::Keyed(KeyedAxis::BigipVersion)));

    let iapps = registry.resolve("f5-iapps").expect("iapps");
    assert_eq!(
        iapps.policy_defaults.closed_world,
        WorldPolicy::AmbientPlusRequire
    );
    assert!(iapps.policy_defaults.strict_ascii, "the W108 rule");
    assert!(iapps.policy_defaults.fixed_ensembles);
    // F5 reclassification (measurements §4a): the iApps core rides
    // the `f5-tcl` trunk under the 32-bit scriptd build, not
    // tcl@8.5.
    let iapps_core = iapps.core.expect("core");
    assert_eq!(iapps_core.family, Family::F5Tcl);
    assert_eq!(iapps_core.default_release, Release::F5_TCL_TMOS);
    assert_eq!(iapps_core.build, BuildProfileId::F5Scriptd32);

    let tmsh = registry.resolve("f5-tmsh").expect("tmsh");
    assert!(!tmsh.policy_defaults.fixed_ensembles);
    // F5 reclassification (measurements §4a): the tmsh core rides
    // the `f5-tcl` trunk at its canonical build, not tcl@8.5.
    let tmsh_core = tmsh.core.expect("core");
    assert_eq!(tmsh_core.family, Family::F5Tcl);
    assert_eq!(tmsh_core.default_release, Release::F5_TCL_TMOS);
    assert_eq!(tmsh_core.build, BuildProfileId::Canonical);

    let expect_env = registry.resolve("expect").expect("expect");
    assert!(expect_env.expected_packages.iter().any(
            |p| p.ambient && p.version == Placement::Pinned(Version::parse("5.45.4").unwrap())
        ));

    let bigip = registry.resolve("f5-bigip").expect("bigip");
    assert!(bigip.core.is_none(), "identity-only: no Tcl core");

    for id in ["spectcl", "sslictcl", "bpf"] {
        let env = registry.resolve(id).expect(id);
        assert_eq!(
            env.policy_defaults.closed_world,
            WorldPolicy::Closed,
            "{id}"
        );
        assert_eq!(
            env.core.expect("core").default_release,
            Release::TCL_9_0,
            "{id}"
        );
    }
}

/// The `tk` environment places Tk **ambient** (a `wish` shell has
/// already loaded it — no `package
/// require Tk` exists to write) on Tk's **own** version axis, never
/// `tracks-base`. Every plain-Tcl environment places the same package
/// **hosted**, which is what makes `Tk` a library with an ambient
/// host rather than a closed-world vendor surface.
#[test]
fn tk_environment_uses_tks_own_axis() {
    let registry = EnvironmentRegistry::compiled();
    let tk = registry.resolve("tk").expect("tk");
    assert_eq!(tk.core.expect("core").default_release, Release::TCL_8_6);
    let placement = tk
        .expected_packages
        .iter()
        .find(|p| *p.package == *"Tk")
        .expect("Tk placement");
    assert!(placement.ambient, "wish ships Tk: no require to write");
    let Placement::Requirement(set) = &placement.version else {
        panic!("Tk must be floored on its own axis, got {placement:?}");
    };
    assert_eq!(set.axis().package_name(), Some("Tk"));
    // The alias is the shebang word too — one identity, two ingresses.
    assert_eq!(registry.resolve("wish").expect("wish").id.as_str(), "tk");
    assert!(
        tk.server_detection
            .shebang_words
            .iter()
            .any(|word| &**word == "wish")
    );
}

/// The hosted half of the same placement: every plain-Tcl environment
/// declares that it *can* host Tk without shipping it, so "can this
/// environment host Tk?" is a placement query with no lenient special
/// case, and a release-pinned host derives the Tk point from its own
/// release (the one named exemption) while the unpinned ones do not.
#[test]
fn plain_tcl_environments_host_tk_without_shipping_it() {
    let registry = EnvironmentRegistry::compiled();
    for (id, expected) in [
        ("tcl", None),
        ("tcl8.4", Some("8.4")),
        ("tcl8.6", Some("8.6")),
        ("tcl9.0", Some("9.0")),
    ] {
        let definition = registry.resolve(id).expect(id);
        let placement = definition
            .expected_packages
            .iter()
            .find(|p| *p.package == *"Tk")
            .unwrap_or_else(|| panic!("{id} declares a Tk placement"));
        assert!(!placement.ambient, "{id}: hosted, so W120 still nags");
        match (expected, &placement.version) {
            (Some(release), Placement::TracksBase) => {
                assert_eq!(
                    definition.core.expect("core").default_release.as_str(),
                    release,
                    "{id}"
                );
            }
            (None, Placement::Requirement(set)) => {
                assert_eq!(set.axis().package_name(), Some("Tk"), "{id}");
            }
            (_, other) => panic!("{id}: unexpected Tk placement {other:?}"),
        }
    }
    // A closed vendor shell declares none, so it cannot host Tk at all.
    for id in ["f5-irules", "bpf", "spectcl", "sslictcl", "xilinx-eda-tcl"] {
        let definition = registry.resolve(id).expect(id);
        assert!(
            !definition
                .expected_packages
                .iter()
                .any(|p| *p.package == *"Tk"),
            "{id}"
        );
    }
}

#[test]
fn ladder_targets_are_single_release_lines() {
    let registry = EnvironmentRegistry::compiled();
    let tcl86 = registry.resolve("tcl8.6").expect("tcl8.6");
    let v = |text: &str| Version::parse(text).expect("version");
    assert!(tcl86.targets.contains(&v("8.6")));
    assert!(tcl86.targets.contains(&v("8.6.16")));
    assert!(!tcl86.targets.contains(&v("8.7")));
    assert!(!tcl86.targets.contains(&v("9.0")));
    assert_eq!(tcl86.targets.axis().core_family(), Some(Family::Tcl));
    // The lenient fallback spans the whole ladder.
    let plain = registry.resolve("tcl").expect("tcl");
    assert!(plain.targets.contains(&v("8.4")));
    assert!(plain.targets.contains(&v("9.1.2")));
    assert!(!plain.targets.contains(&v("9.2")));
}

/// A selecting language id is contributed, resolves to the environment
/// that lists it, and is no other environment's id or editor identity: an
/// environment's own identity and canonical id resolve as themselves.
#[test]
fn selecting_language_ids_resolve_to_the_environment_that_lists_them() {
    let registry = EnvironmentRegistry::compiled();
    let mut selecting: Vec<(&str, &str)> = Vec::new();
    for definition in registry.definitions() {
        for identity in &definition.selecting_identities {
            selecting.push((identity.as_str(), definition.id.as_str()));
        }
    }
    selecting.sort_unstable();
    assert_eq!(
        selecting,
        [
            ("tcl-apl", "f5-iapps"),
            ("tcl-bpf", "bpf"),
            ("tcl-libero", "microchip-libero-eda-tcl"),
            ("tcl-spec", "spectcl"),
        ]
    );
    for (spelling, owner) in selecting {
        assert_eq!(
            registry.resolve(spelling).map(|found| found.id.to_string()),
            Some(owner.to_owned()),
            "`{spelling}` selects `{owner}`"
        );
        assert!(
            !registry.definitions().iter().any(|definition| {
                definition.id.as_str() == spelling
                    || definition
                        .editor_identity
                        .is_some_and(|identity| identity.as_str() == spelling)
            }),
            "`{spelling}` is no environment's own id or identity"
        );
    }
}

#[test]
fn every_seeded_editor_identity_is_contributed() {
    // I8's model-side half: the seeds can only reference the
    // contributed set — the newtype makes the violation
    // unrepresentable, so this pins the expected selections.
    let registry = EnvironmentRegistry::compiled();
    let expected: &[(&str, Option<&str>)] = &[
        ("tcl", Some("tcl")),
        ("tcl8.4", Some("tcl84")),
        ("tcl9.1", Some("tcl91")),
        ("f5-irules", Some("tcl-irule")),
        ("jim", Some("tcl-jim")),
        ("spectcl", Some("tclspec")),
        ("sslictcl", Some("sslictcl")),
        ("tk", None),
        ("bpf", None),
    ];
    for &(env, editor) in expected {
        let definition = registry.resolve(env).expect(env);
        assert_eq!(
            definition
                .editor_identity
                .map(EditorLanguageIdentityId::as_str),
            editor,
            "{env}"
        );
    }
    assert!(EditorLanguageIdentityId::new("tcl-apl").is_some());
    assert!(EditorLanguageIdentityId::new("not-a-language").is_none());
}

/// The kind of every compiled environment is a stated judgement:
/// `bpf` is a language over a Tcl 9.0 core, `tk` is a package over a
/// Tcl 8.x core, and nothing derives one from the other fields.
#[test]
fn every_compiled_environment_states_its_kind() {
    const LANGUAGES: &[&str] = &[
        "tcl",
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "f5-bigip",
        "jim",
        "bpf",
        "expect",
        "spectcl",
        "sslictcl",
    ];
    const PACKAGES: &[&str] = &[
        "tk",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "microchip-libero-eda-tcl",
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
    ];
    let registry = EnvironmentRegistry::compiled();
    for definition in registry.definitions() {
        let id = definition.id.as_str();
        let expected = match (LANGUAGES.contains(&id), PACKAGES.contains(&id)) {
            (true, false) => EnvironmentKind::Language,
            (false, true) => EnvironmentKind::Packages,
            other => panic!("{id}: not stated as exactly one kind ({other:?})"),
        };
        assert_eq!(definition.kind, expected, "{id}");
    }
    assert_eq!(
        registry.definitions().len(),
        LANGUAGES.len() + PACKAGES.len()
    );
}

#[test]
fn the_kind_words_round_trip() {
    for kind in [EnvironmentKind::Language, EnvironmentKind::Packages] {
        assert_eq!(EnvironmentKind::from_word(kind.word()), Some(kind));
    }
    assert_eq!(EnvironmentKind::from_word("Packages"), None);
    assert_eq!(EnvironmentKind::from_word(""), None);
}

/// A `Language` describes itself by its display name alone.
#[test]
fn a_language_describes_itself_by_name() {
    let registry = EnvironmentRegistry::compiled();
    for (id, description) in [
        ("jim", "Jim Tcl"),
        ("f5-irules", "F5 iRules"),
        ("tcl8.6", "Tcl 8.6"),
        ("expect", "Expect"),
    ] {
        assert_eq!(registry.resolve(id).expect(id).description(), description);
    }
}

/// A `Packages` environment names its core release and its ambient
/// packages in declaration order; hosted packages are not ambient and
/// are left out.
#[test]
fn a_tool_shell_describes_its_core_and_ambient_packages() {
    let registry = EnvironmentRegistry::compiled();
    assert_eq!(
        registry
            .resolve("xilinx-eda-tcl")
            .expect("xilinx")
            .description(),
        "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf"
    );
    let tk = registry.resolve("tk").expect("tk");
    assert_eq!(tk.description(), "Tk — Tcl 8.6 + Tk");
    assert!(
        tk.expected_packages
            .iter()
            .any(|placement| !placement.ambient && &*placement.package == "Itcl"),
        "Itcl is hosted on tk, so the description omits it"
    );

    let mut bare = (*tk).clone();
    bare.expected_packages.clear();
    assert_eq!(bare.description(), "Tk — Tcl 8.6");
    bare.core = None;
    assert_eq!(bare.description(), "Tk");
}

/// The core label and the ambient packages are what the description is
/// made of, so anything else that quotes them agrees with it.
#[test]
fn the_description_is_the_core_label_and_the_ambient_packages() {
    let registry = EnvironmentRegistry::compiled();
    let vivado = registry.resolve("xilinx-eda-tcl").expect("xilinx");
    assert_eq!(vivado.core_label().as_deref(), Some("Tcl 8.5"));
    assert_eq!(
        vivado.ambient_packages().collect::<Vec<_>>(),
        ["vivado", "sdc", "upf"]
    );
    assert_eq!(
        vivado.description(),
        "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf"
    );
    let tk = registry.resolve("tk").expect("tk");
    assert_eq!(tk.ambient_packages().collect::<Vec<_>>(), ["Tk"]);

    let mut no_core = (*vivado).clone();
    no_core.core = None;
    assert_eq!(no_core.core_label(), None);
}

/// Kind is presentation: two definitions that differ only in kind
/// resolve, and describe their core, identically.
#[test]
fn kind_does_not_change_what_an_environment_resolves_to() {
    let registry = EnvironmentRegistry::compiled();
    let tk = registry.resolve("tk").expect("tk");
    let mut as_language = (*tk).clone();
    as_language.kind = EnvironmentKind::Language;
    assert_eq!(as_language.core, tk.core);
    assert_eq!(as_language.point(), tk.point());
    assert_eq!(as_language.expected_packages, tk.expected_packages);
    assert_eq!(as_language.policy_defaults, tk.policy_defaults);
}

/// An alias selects an environment; it never spells a package another
/// environment places, ambient or hosted, in any case, so a name read as
/// either resolves one way. An alias may equal a package its own
/// environment places: the tool's shell and the tool's package share the
/// tool's name.
#[test]
fn an_alias_never_spells_another_environments_package() {
    let placed_by_other = |alias: &str| {
        let mut definitions = compiled_definitions();
        let mut extra = pack_environment("runtime-pack-env");
        extra.provenance = Provenance::WorkspaceTrusted;
        extra.aliases = arcs(&[alias]);
        definitions.push(extra);
        EnvironmentRegistry::new(definitions, 1).err()
    };
    // `sdc` is ambient in the Vivado shell; `TK` is hosted by the ladder.
    for (alias, package) in [("sdc", "sdc"), ("TK", "Tk")] {
        match placed_by_other(alias) {
            Some(EnvironmentRegistryError::AliasSpellsPackage {
                alias: named,
                claimed_by,
                placed_by,
            }) => {
                assert_eq!(named, alias);
                assert_eq!(claimed_by, "runtime-pack-env");
                assert!(
                    registry_places(&placed_by, package),
                    "`{placed_by}` places `{package}`"
                );
            }
            other => panic!("`{alias}` must be rejected as a package, got {other:?}"),
        }
    }
    assert_eq!(placed_by_other("not-a-package"), None);
    // The owner is exempt: `vivado` is the Vivado shell's alias and package.
    let registry = EnvironmentRegistry::compiled();
    let vivado = registry.resolve("vivado").expect("the Vivado alias");
    assert!(
        vivado
            .expected_packages
            .iter()
            .any(|placement| placement.package.eq_ignore_ascii_case("vivado"))
    );
}

fn registry_places(environment: &str, package: &str) -> bool {
    EnvironmentRegistry::compiled()
        .resolve(environment)
        .is_some_and(|definition| {
            definition
                .expected_packages
                .iter()
                .any(|placement| placement.package.as_ref() == package)
        })
}

/// The compiled environments select by these interpreter words.
#[test]
fn the_compiled_shebang_words_name_their_environments() {
    let registry = EnvironmentRegistry::compiled();
    for (word, owner) in [
        ("jimsh", "jim"),
        ("wish", "tk"),
        ("expect", "expect"),
        ("tclsh8.5", "tcl8.5"),
        ("wish9.0", "tcl9.0"),
    ] {
        let claimants: Vec<&str> = registry
            .definitions()
            .iter()
            .filter(|definition| {
                definition
                    .server_detection
                    .shebang_words
                    .iter()
                    .any(|claimed| claimed.as_ref() == word)
            })
            .map(|definition| definition.id.as_str())
            .collect();
        assert_eq!(claimants, [owner], "{word}");
    }
}

#[test]
fn collisions_are_typed_construction_errors() {
    let base = compiled_definitions();
    // Duplicate canonical id.
    let mut dup = base.clone();
    dup.push(plain_tcl_environment());
    assert_eq!(
        EnvironmentRegistry::new(dup, 1).err(),
        Some(EnvironmentRegistryError::DuplicateCanonicalId(
            "tcl".to_owned()
        ))
    );
    // An alias shadowing a canonical id (the cycle shape).
    let mut shadowing = base.clone();
    let mut extra = pack_environment("my-env");
    extra.aliases = arcs(&["tcl8.6"]);
    shadowing.push(extra);
    assert_eq!(
        EnvironmentRegistry::new(shadowing, 1).err(),
        Some(EnvironmentRegistryError::AliasShadowsCanonical {
            alias: "tcl8.6".to_owned(),
            canonical: "tcl8.6".to_owned(),
        })
    );
    // Two environments claiming one alias.
    let mut dup_alias = base.clone();
    let mut a = pack_environment("env-a");
    a.aliases = arcs(&["shared-alias"]);
    let mut b = pack_environment("env-b");
    b.aliases = arcs(&["shared-alias"]);
    dup_alias.push(a);
    dup_alias.push(b);
    assert_eq!(
        EnvironmentRegistry::new(dup_alias, 1).err(),
        Some(EnvironmentRegistryError::DuplicateAlias(
            "shared-alias".to_owned()
        ))
    );
    // Two environments selecting one editor identity.
    let mut dup_editor = base.clone();
    let mut c = pack_environment("env-c");
    c.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
    let mut d = pack_environment("env-d");
    d.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
    dup_editor.push(c);
    dup_editor.push(d);
    assert_eq!(
        EnvironmentRegistry::new(dup_editor, 1).err(),
        Some(EnvironmentRegistryError::DuplicateEditorIdentity(
            "tcl-apl".to_owned()
        ))
    );
    // A language id another environment already selects, or owns as an
    // identity, alias or id.
    // (`tcl-bpf` is selected by `bpf`, `tcl-jim` is Jim's editor identity,
    // `tcl-spec` is an alias of `spectcl`, `sslictcl` is a canonical id.)
    for taken in ["tcl-bpf", "tcl-jim", "tcl-spec", "sslictcl"] {
        let mut selecting = base.clone();
        let mut claimant = pack_environment("selecting-claimant");
        claimant.selecting_identities = identities(&[taken]);
        selecting.push(claimant);
        assert_eq!(
            EnvironmentRegistry::new(selecting, 1).err(),
            Some(EnvironmentRegistryError::DuplicateSelectingIdentity(
                taken.to_owned()
            )),
            "{taken}"
        );
    }
    // Two environments claiming one shebang word, in any case.
    let mut dup_shebang = base;
    let mut claimant = pack_environment("shebang-claimant");
    claimant.server_detection.shebang_words = arcs(&["JimSH"]);
    dup_shebang.push(claimant);
    assert_eq!(
        EnvironmentRegistry::new(dup_shebang, 1).err(),
        Some(EnvironmentRegistryError::DuplicateShebangWord(
            "JimSH".to_owned()
        ))
    );
}

/// The environments the bundled packs declare seed the compiled
/// registry at `Provenance::BundledPack` (redesign D17), and their
/// names are reserved one step below the built-in ones — restatable by
/// the bundled tier, refused from every other.
#[test]
fn bundled_pack_names_are_reserved_below_the_bundled_tier() {
    let seeded = bundled_pack_definitions();
    assert!(!seeded.is_empty(), "the bundled packs declare environments");
    for definition in &seeded {
        assert_eq!(definition.provenance, Provenance::BundledPack);
        let id = definition.id.as_str();
        assert_eq!(
            reserved_against(id, Provenance::BundledPack),
            None,
            "{id}: the bundled tier restates its own environment"
        );
        for provenance in [
            Provenance::User,
            Provenance::WorkspaceTrusted,
            Provenance::WorkspaceUntrusted,
            Provenance::StudioOverride,
            Provenance::Document,
        ] {
            assert_eq!(
                reserved_against(id, provenance).as_deref(),
                Some(id),
                "{id}: reserved against {provenance:?}"
            );
        }
    }
    // A built-in name stays reserved against the bundled tier too.
    assert_eq!(
        reserved_against("tcl8.6", Provenance::BundledPack).as_deref(),
        Some("tcl8.6")
    );
    assert_eq!(reserved_against("tcl8.6", Provenance::BuiltIn), None);

    // The registry enforces the same rule at construction: a workspace
    // definition claiming a bundled name is a typed error, while a
    // bundled restatement of a *different* id is fine.
    let bundled_id = seeded[0].id.as_str().to_owned();
    let mut hijack = compiled_definitions();
    hijack.retain(|definition| definition.id.as_str() != bundled_id);
    let mut intruder = pack_environment(&bundled_id);
    intruder.provenance = Provenance::WorkspaceTrusted;
    hijack.push(intruder);
    assert_eq!(
        EnvironmentRegistry::new(hijack, 1).err(),
        Some(EnvironmentRegistryError::ReservedName {
            name: bundled_id.clone(),
            claimed_by: bundled_id,
        })
    );
}

/// The six vendor `DialectProfile` rows stay compiled as the lexer's
/// grammar key and the editor catalogues' key, so each must agree with
/// the pack-declared environment on every identity fact both carry.
#[test]
fn every_bundled_environment_agrees_with_its_catalogue_profile() {
    let seeded = bundled_pack_definitions();
    for definition in &seeded {
        let id = definition.id.as_str();
        let profile = DialectProfile::find(id)
            .unwrap_or_else(|| panic!("{id}: a bundled environment keeps its profile row"));
        assert_eq!(
            definition.display_name.as_ref(),
            profile.display_name,
            "{id}"
        );
        assert_eq!(definition.short_name.as_ref(), profile.short_name, "{id}");
        let aliases: Vec<&str> = definition.aliases.iter().map(AsRef::as_ref).collect();
        assert_eq!(aliases, profile.aliases, "{id}");
        assert_eq!(
            definition.kind,
            EnvironmentKind::Packages,
            "{id}: a vendor shell is a Tcl release with packages"
        );
        assert_eq!(
            definition
                .editor_identity
                .map(EditorLanguageIdentityId::as_str),
            profile.editor_language_id,
            "{id}"
        );
        let extensions: Vec<(&str, &str)> = definition
            .server_detection
            .file_extensions
            .iter()
            .map(|claim| (claim.extension.as_ref(), claim.display_name.as_ref()))
            .collect();
        let profile_extensions: Vec<(&str, &str)> = profile
            .file_extensions
            .iter()
            .map(|row| (row.extension, row.display_name))
            .collect();
        assert_eq!(extensions, profile_extensions, "{id}");
        let terms: Vec<&str> = definition.help_terms.iter().map(AsRef::as_ref).collect();
        assert_eq!(terms, profile.help_terms, "{id}");
        let core = definition.core.expect("a vendor shell selects a Tcl core");
        assert_eq!(core.family, Family::Tcl, "{id}");
        assert_eq!(
            Some(core.default_release.as_str()),
            profile.runtime_base.map(TclVersion::version_string),
            "{id}: base release"
        );
        assert_eq!(
            definition
                .policy_defaults
                .version_ceiling
                .map(Release::as_str),
            profile.version_ceiling.map(TclVersion::version_string),
            "{id}: ceiling"
        );
        for pin in profile.libraries {
            let placement = definition
                .expected_packages
                .iter()
                .find(|placement| placement.package.as_ref() == pin.package)
                .unwrap_or_else(|| panic!("{id}: the block places `{}`", pin.package));
            assert_eq!(placement.ambient, pin.ambient, "{id}: `{}`", pin.package);
        }
        assert_eq!(
            definition.expected_packages.len(),
            profile.libraries.len(),
            "{id}: the block places exactly the profile's libraries"
        );
    }
    let profiles: Vec<&str> = DialectProfile::all()
        .iter()
        .filter(|profile| profile.name.ends_with("-eda-tcl"))
        .map(|profile| profile.name)
        .collect();
    let mut seeded_ids: Vec<&str> = seeded.iter().map(|d| d.id.as_str()).collect();
    seeded_ids.sort_unstable();
    assert_eq!(seeded_ids, profiles, "every vendor shell is pack-declared");
}

/// The selectable set is the registry without the lenient sink,
/// languages before tool shells, canonical id ascending within a kind.
#[test]
fn selectable_lists_languages_then_packages_by_id_without_the_sink() {
    let selectable = EnvironmentRegistry::compiled().selectable();
    let ids: Vec<&str> = selectable
        .iter()
        .map(|definition| definition.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "bpf",
            "expect",
            "f5-bigip",
            "f5-iapps",
            "f5-irules",
            "f5-tmsh",
            "jim",
            "spectcl",
            "sslictcl",
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "cadence-eda-tcl",
            "intel-quartus-eda-tcl",
            "mentor-eda-tcl",
            "microchip-libero-eda-tcl",
            "synopsys-eda-tcl",
            "tk",
            "xilinx-eda-tcl",
        ]
    );
    assert!(!ids.contains(&LENIENT_ENVIRONMENT_ID));
    let languages = selectable
        .iter()
        .take_while(|definition| definition.kind == EnvironmentKind::Language)
        .count();
    assert!(
        selectable[languages..]
            .iter()
            .all(|definition| definition.kind == EnvironmentKind::Packages),
        "no language follows a tool shell"
    );
}

/// A pack-declared environment joins the selectable set of the registry
/// it was built into, among the tool shells.
#[test]
fn a_pack_declared_environment_is_selectable_beside_the_tool_shells() {
    let mut declared = pack_environment("spicegentcl/ngspice");
    declared.kind = EnvironmentKind::Packages;
    declared.provenance = Provenance::WorkspaceTrusted;
    let mut definitions = compiled_definitions();
    definitions.push(declared);
    let registry = EnvironmentRegistry::new(definitions, 1).expect("collision-free");
    let ids: Vec<String> = registry
        .selectable()
        .iter()
        .map(|definition| definition.id.to_string())
        .collect();
    let acme = ids.iter().position(|id| id == "spicegentcl/ngspice");
    let cadence = ids.iter().position(|id| id == "cadence-eda-tcl");
    let xilinx = ids.iter().position(|id| id == "xilinx-eda-tcl");
    assert!(acme.is_some(), "{ids:?}");
    assert!(
        cadence < acme && acme < xilinx,
        "`spicegentcl/ngspice` sorts among the tool shells: {ids:?}"
    );
}

/// The starting environment is a choice a user can also make: it is
/// selectable and is not the lenient sink.
#[test]
fn the_default_environment_is_selectable() {
    assert_ne!(DEFAULT_ENVIRONMENT_ID, LENIENT_ENVIRONMENT_ID);
    assert!(
        EnvironmentRegistry::compiled_selectable()
            .iter()
            .any(|definition| definition.id.as_str() == DEFAULT_ENVIRONMENT_ID)
    );
}

/// The compiled form is the generation-0 registry's selectable set.
#[test]
fn compiled_selectable_is_the_compiled_registrys_selectable_set() {
    let compiled = EnvironmentRegistry::compiled().selectable();
    assert_eq!(EnvironmentRegistry::compiled_selectable(), compiled);
}

/// The machine-readable provenance words a status payload carries.
#[test]
fn provenance_words_name_where_an_environment_came_from() {
    let registry = EnvironmentRegistry::compiled();
    let word = |id: &str| registry.resolve(id).expect(id).provenance.word();
    assert_eq!(word("jim"), "built-in");
    assert_eq!(word("tk"), "built-in");
    assert_eq!(word("xilinx-eda-tcl"), "bundled-pack");
    assert_eq!(Provenance::User.word(), "user-pack");
    assert_eq!(Provenance::WorkspaceTrusted.word(), "workspace-pack");
    assert_eq!(Provenance::WorkspaceUntrusted.word(), "workspace-pack");
}

#[test]
fn compiled_names_are_reserved_for_non_builtins() {
    let mut definitions = compiled_definitions();
    let mut intruder = pack_environment("workspace-env");
    intruder.aliases = arcs(&["irules"]);
    intruder.provenance = Provenance::WorkspaceTrusted;
    definitions.retain(|d| d.id.as_str() != "f5-irules");
    definitions.push(intruder);
    // Even with the compiled irules definition absent from this
    // registry, its names stay reserved.
    assert_eq!(
        EnvironmentRegistry::new(definitions, 1).err(),
        Some(EnvironmentRegistryError::ReservedName {
            name: "irules".to_owned(),
            claimed_by: "workspace-env".to_owned(),
        })
    );
    // A namespaced third-party id passes.
    let mut fine = compiled_definitions();
    let mut third_party = pack_environment("mypack/mytool");
    third_party.provenance = Provenance::User;
    fine.push(third_party);
    assert!(EnvironmentRegistry::new(fine, 1).is_ok());
}

#[test]
fn overlays_derive_without_mutating_the_base() {
    let registry = EnvironmentRegistry::compiled();
    let base_before = registry.resolve("tcl8.6").expect("tcl8.6");
    let overlay = EnvironmentOverlay {
        base: EnvironmentId::new("tcl8.6"),
        target_changes: TargetChanges {
            targets: Some(
                VersionSet::from_requirements(VersionAxisId::core(Family::Tcl), &["8.6-9.1"])
                    .expect("targets"),
            ),
        },
        package_changes: PackageChanges {
            add: vec![PackagePlacement {
                package: arc("json"),
                version: Placement::Requirement(
                    VersionSet::from_requirements(VersionAxisId::package("json"), &["1.0"])
                        .expect("requirement"),
                ),
                ambient: false,
            }],
            remove: arcs(&["Itcl"]),
        },
        origin: ConfigurationOrigin {
            provenance: Provenance::WorkspaceTrusted,
            content_hash: 0xDEAD_BEEF,
        },
    };
    let (derived, identity) = registry.apply_overlay(&overlay).expect("overlay applies");
    assert_eq!(identity.id.as_str(), "tcl8.6");
    assert_eq!(identity.generation, 0);
    assert_eq!(identity.overlay, Some(0xDEAD_BEEF));
    assert!(
        derived
            .targets
            .contains(&Version::parse("9.0").expect("version"))
    );
    assert!(
        derived
            .expected_packages
            .iter()
            .all(|p| *p.package != *"Itcl")
    );
    assert!(
        derived
            .expected_packages
            .iter()
            .any(|p| *p.package == *"json")
    );
    assert_eq!(derived.provenance, Provenance::WorkspaceTrusted);
    // The base is untouched: same value as before the overlay.
    let base_after = registry.resolve("tcl8.6").expect("tcl8.6");
    assert_eq!(*base_before, *base_after);
    assert_eq!(registry.identity_of(&base_after).overlay, None);

    // Overlay errors are typed.
    let unknown = EnvironmentOverlay {
        base: EnvironmentId::new("no-such-env"),
        target_changes: TargetChanges::default(),
        package_changes: PackageChanges::default(),
        origin: overlay.origin,
    };
    assert!(matches!(
        registry.apply_overlay(&unknown),
        Err(EnvironmentOverlayError::UnknownBase(_))
    ));
    let wrong_axis = EnvironmentOverlay {
        base: EnvironmentId::new("tcl8.6"),
        target_changes: TargetChanges {
            targets: Some(
                VersionSet::from_requirements(VersionAxisId::package("Tk"), &["8.6"])
                    .expect("targets"),
            ),
        },
        package_changes: PackageChanges::default(),
        origin: overlay.origin,
    };
    assert!(matches!(
        registry.apply_overlay(&wrong_axis),
        Err(EnvironmentOverlayError::Targets(
            VersionSetError::AxisMismatch { .. }
        ))
    ));
}
