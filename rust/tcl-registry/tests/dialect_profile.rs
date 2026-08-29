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

//! Contract tests binding the `DialectProfile` catalog (tcl-dialect) to the
//! registry's spec data — the two sides of the availability axis
//! (`docs/design/dialect-profile-model.md` §5/§9).
//!
//! iRules availability is fully explicit in the spec data: the profile is a
//! bare `IRULES` mask and every command carries an explicit `dialects` group
//! (universal `surface: None` was eliminated registry-wide), with the
//! `IRULES` bit present iff iRules enables the command. A sandbox-banned
//! command such as `exec` is `ALL_TCL` and simply never intersects the mask —
//! there is no subtractive disable list any more; the math-operator heads are
//! the one remaining profile-level exclusion (`OPERATOR_COMMAND` +
//! `operators_as_commands`). These tests pin that banned surface stays banned
//! and the retired `NON_IRULES_OPERATORS` union never creeps back as a gate.

use tcl_dialect::model::{SurfaceLayer, Family};
use tcl_dialect::{DialectProfile, DialectSet, NumberSyntax, TclVersion};
use tcl_registry::model::ingress::{
    static_context_for, static_document_context_for, static_document_context_for_profile as ctx_for,
};
use tcl_registry::traits::Traits;
use tcl_dialect::model::{SpecSurface};

/// Whether `name` is a `tcl::mathop` operator-command spelling (bare `+`,
/// `eq`, or a qualified `tcl::mathop::+` form). Data-driven: a bare name is
/// an operator head iff the registry also carries its `tcl::mathop::`-
/// qualified spelling — no hardcoded operator list.
fn is_mathop_spelling(name: &str) -> bool {
    let reg = static_context_for("tcl9.0").commands();
    name.strip_prefix("::tcl::mathop::").is_some()
        || name.strip_prefix("tcl::mathop::").is_some()
        || !reg.specs(&format!("tcl::mathop::{name}")).is_empty()
}

/// Pins the invariant that no spec gate at any level (command, subcommand,
/// option, form option, subcommand option) in any profile's registry is the
/// retired `NON_IRULES_OPERATORS` union — "every dialect except
/// iRules/Tk/BPF", reconstructed here because the constant itself was
/// deleted from `DialectSet`. Exclusion from iRules is modelled on the
/// profile (spec `dialects` group / operator trait), never by enumerating
/// the complement of the excluded dialects.
#[test]
fn retired_non_irules_operators_union_never_reappears_as_a_gate() {
    // Reconstructed from the non-iRules/Tk/BPF dialect bits that still exist;
    // the 5 EDA vendor bits that were also part of this union were retired by
    // the EDA-as-packages migration (eda-library-packages.md).
    let retired = SpecSurface::ALL_TCL | SpecSurface::IAPPS | SpecSurface::EXPECT;
    let check = |gate: Option<&'static [SpecSurface]>, what: &str| {
        assert_ne!(
            gate,
            Some(retired),
            "{what}: the retired NON_IRULES_OPERATORS union must not be a \
             spec gate — model iRules exclusion via each spec's explicit \
             non-IRULES `dialects` group / Traits::OPERATOR_COMMAND instead"
        );
    };
    for profile in DialectProfile::all() {
        let reg = static_context_for(profile.name).commands();
        for name in reg.command_names() {
            for spec in reg.specs(name) {
                check(spec.surface, spec.name);
                for opt in spec.options {
                    check(opt.surface, &format!("{} {}", spec.name, opt.name));
                }
                for form in spec.command_forms {
                    for opt in form.options {
                        check(opt.surface, &format!("{} {}", spec.name, opt.name));
                    }
                }
                for sub in spec.subcommands {
                    check(sub.surface, &format!("{} {}", spec.name, sub.name));
                    for opt in sub.options {
                        check(
                            opt.surface,
                            &format!("{} {} {}", spec.name, sub.name, opt.name),
                        );
                    }
                }
            }
        }
    }
}

/// The commands F5's TMM interpreter removes from iRules (the K36322151
/// sandbox bans plus the project-modelled iRules-excluded internals). This
/// used to be a subtractive `DialectProfile::disabled_commands` list; it is
/// now encoded directly in each spec's explicit `dialects` group (a banned
/// command carries `ALL_TCL`, never the `IRULES` bit), so the list lives
/// here only as the test oracle for the contract below.
const IRULES_BANNED: &[&str] = &[
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_mkindex",
    "auto_mkindex_old",
    "auto_qualify",
    "auto_reset",
    "bgerror",
    "cd",
    "eof",
    "exec",
    "exit",
    "fblocked",
    "fconfigure",
    "fcopy",
    "file",
    "fileevent",
    "filename",
    "flush",
    "gets",
    "glob",
    "http",
    "interp",
    "load",
    "memory",
    "namespace",
    "open",
    "package",
    "pid",
    "pkg_mkindex",
    "pwd",
    "re_quote",
    "regex::quote",
    "regex_quote",
    "regexp::quote",
    "rename",
    "seek",
    "socket",
    "source",
    "tcl::OptKeyDelete",
    "tcl::OptKeyError",
    "tcl::OptKeyParse",
    "tcl::OptKeyRegister",
    "tcl::OptParse",
    "tcl::OptProc",
    "tcl::OptProcArgGiven",
    "tcl_findLibrary",
    "tell",
    "time",
    "unknown",
    "unload",
    "update",
    "vwait",
];

/// The banned-command exclusion is now encoded in the specs themselves:
/// every banned name still exists as registered spec data (so the LSP can
/// tell "exists, but not in iRules" from "unknown"), but each carries an
/// explicit `dialects` group WITHOUT the `IRULES` bit — so the bare `IRULES`
/// mask excludes it by plain intersection, with no subtractive ban list.
#[test]
fn irules_banned_commands_lack_the_irules_bit() {
    let reg = static_context_for("f5-irules").commands();
    for banned in IRULES_BANNED {
        let specs = reg.specs(banned);
        assert!(!specs.is_empty(), "{banned}: names no registered spec");
        assert!(
            !specs.iter().any(|s| s.supports_dialect(SpecSurface::IRULES)),
            "{banned}: must NOT carry the IRULES bit — a sandbox-banned \
             command is excluded from iRules by its explicit non-IRULES \
             dialect group, not by a ban list"
        );
    }
}

/// The math-operator heads are excluded from iRules by dialect *shape*
/// (`operators_as_commands == false` + `Traits::OPERATOR_COMMAND`), not by
/// the disable list. Both directions of the trait marking are data-bound:
/// a spec carries the trait iff it is a `tcl::mathop` spelling.
#[test]
fn operator_heads_carry_the_trait_and_follow_the_profile_shape() {
    let reg = static_context_for("tcl9.0").commands();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            assert_eq!(
                spec.traits.contains(Traits::OPERATOR_COMMAND),
                is_mathop_spelling(spec.name),
                "{}: OPERATOR_COMMAND must mark exactly the tcl::mathop \
                 spellings",
                spec.name
            );
        }
    }
    // TP: operator heads resolve where operators are command heads…
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    for op in ["+", "eq", "tcl::mathop::+"] {
        assert!(
            ctx_for(tcl90).resolve_spec(reg, op).is_some(),
            "{op} resolves under tcl9.0"
        );
    }
    // …TN: and never under iRules (operators live only inside expr there) —
    // through the profile query, a bare mask query on the stamped registry
    // (§9.2), and the snapshot's independent most-specific resolver alike.
    let ireg = static_context_for("f5-irules").commands();
    let irules = DialectProfile::irules();
    for op in ["+", "eq", "tcl::mathop::+"] {
        assert!(
            ctx_for(irules).resolve_spec(ireg, op).is_none(),
            "{op} must not resolve under f5-irules"
        );
        assert!(
            ireg.get_for_surface(op, SpecSurface::IRULES).is_none(),
            "{op} must not resolve via a bare mask query either (§9.2)"
        );
    }
    // …and never under plain tcl8.4 either: `::tcl::mathop` is TIP 174,
    // added in Tcl 8.5 — a real tclsh 8.4 has no `::tcl` namespace at all,
    // so 8.4 shares iRules' reasoning here even though it carries no vendor
    // bit to key a disable-list entry off. Same three angles as iRules.
    let reg84 = static_context_for("tcl8.4").commands();
    let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    for op in ["+", "eq", "tcl::mathop::+"] {
        assert!(
            ctx_for(tcl84).resolve_spec(reg84, op).is_none(),
            "{op} must not resolve under tcl8.4"
        );
        assert!(
            reg84.get_for_surface(op, SpecSurface::TCL84).is_none(),
            "{op} must not resolve via a bare mask query either under tcl8.4"
        );
    }
}

/// The retag's false-negative fix: iRules subcommands whose *names* collide
/// with a banned command or an operator spelling (`DNS::header cd` — the
/// DNS Checking-Disabled flag; `IP::stats in` — inbound stats) were bulk
/// mis-tagged `NON_IRULES_OPERATORS` and thus wrongly unavailable under
/// f5-irules. Name-keyed exclusion must never leak into unrelated
/// subcommands again.
#[test]
fn irules_subcommands_named_like_banned_commands_stay_available() {
    let reg = static_context_for("f5-irules").commands();
    let irules = DialectProfile::irules();
    for (cmd, sub_name) in [("DNS::header", "cd"), ("IP::stats", "in")] {
        let spec = ctx_for(irules)
            .resolve_spec(reg, cmd)
            .unwrap_or_else(|| panic!("{cmd} resolves under f5-irules"));
        assert!(
            ctx_for(irules)
                .available_subcommands(spec)
                .iter()
                .any(|s| s.name == sub_name),
            "{cmd} {sub_name}: a real iRules subcommand must not be hidden \
             by its name colliding with a banned command / operator"
        );
    }
}

/// The user-facing contract: the banned commands never resolve under the
/// iRules profile, while the F5 surface and the universal 8.4 core still
/// do. The ban is carried by each spec's explicit `dialects` group, which
/// simply omits the `IRULES` bit.
#[test]
fn irules_banned_commands_never_resolve() {
    let reg = static_context_for("f5-irules").commands();
    let irules = DialectProfile::irules();

    // TP: genuinely banned commands do not resolve (excluded by their
    // explicit non-IRULES `dialects` group, not a ban list).
    for banned in IRULES_BANNED {
        assert!(
            ctx_for(irules).resolve_spec(reg, banned).is_none(),
            "{banned}: banned command must not resolve under f5-irules"
        );
    }
    // TN: the iRules-enabled core and the F5 surface still resolve.
    for allowed in ["set", "if", "string", "foreach", "pool", "when", "log"] {
        assert!(
            ctx_for(irules).resolve_spec(reg, allowed).is_some(),
            "{allowed}: must resolve under f5-irules"
        );
    }
    // Version-gated core (8.5+/8.6) is *unavailable* under the pinned-8.4
    // iRules runtime — via the version tags (which lack the IRULES bit).
    for versioned in ["dict", "lassign", "apply", "lmap", "coroutine"] {
        assert!(
            ctx_for(irules).resolve_spec(reg, versioned).is_none(),
            "{versioned}: 8.5+/8.6 core is never present in iRules (D3)"
        );
    }
}

/// The additive-profile fix this milestone ships: real 8.5/8.6 core resolves
/// under the composed (version|vendor) masks that the old bare-bit view
/// wrongly excluded (the confirmed W123/W002 defect), while later-version
/// core stays correctly unavailable.
#[test]
fn additive_profiles_resolve_their_embedded_tcl_core() {
    // (profile, resolves, still_unavailable)
    let cases: &[(&str, &[&str], &[&str])] = &[
        // iApps ride the `f5-tcl` trunk — a fork of Tcl at 8.4.6, NOT an
        // 8.5 host (measured: bigip-irule-parser-measurements.md §4a —
        // `IAppImplementation` fails every 8.5 discriminator, dict/
        // lassign/apply included). exec/file/socket are ALLOWED (host
        // interpreter, not the TMM sandbox).
        (
            "f5-iapps",
            &["exec", "file", "socket", "set"],
            &["dict", "lassign", "apply", "lmap", "coroutine", "zipfs"],
        ),
        // expect: Tcl 8.6 base — coroutine/lmap/dict are real; zipfs (9.0)
        // is not; the expect surface resolves.
        (
            "expect",
            &["dict", "lassign", "lmap", "coroutine", "spawn", "send"],
            &["zipfs"],
        ),
        // EDA on an 8.5 base: dict yes, lmap (8.6) no.
        ("xilinx-eda-tcl", &["dict", "lassign"], &["lmap", "zipfs"]),
        // EDA on an 8.6 base: lmap yes, zipfs no.
        ("synopsys-eda-tcl", &["dict", "lmap"], &["zipfs"]),
    ];
    for &(dialect, resolves, unavailable) in cases {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let reg = static_context_for(dialect).commands();
        for name in resolves {
            assert!(
                ctx_for(profile).resolve_spec(reg, name).is_some(),
                "{dialect}: {name} must resolve (embedded-core fix)"
            );
        }
        for name in unavailable {
            assert!(
                ctx_for(profile).resolve_spec(reg, name).is_none(),
                "{dialect}: {name} must stay unavailable"
            );
        }
    }
}

/// BPF is a genuine Tcl 9.0 embedding, not merely a BPF command pack. The
/// cache must install the pack and stamp that one profile so registry-owning
/// consumers receive the full release/dialect fact set (issue #1466).
#[test]
fn bpf_registry_is_stamped_with_its_tcl90_embedding() {
    let registry = static_context_for("bpf").commands();
    let profile = tcl_registry::model::ingress::resolve_environment("bpf").analyser_profile();

    assert_eq!(registry.profile(), Some(profile));
    assert_eq!(
        profile.surface_query(),
        SpecSurface::TCL90 | SpecSurface::BPF
    );
    assert_eq!(registry.runtime_version(), Some(TclVersion::V9_0));
    assert_eq!(registry.numbers(), NumberSyntax::Tcl90);
    assert_eq!(registry.octal_fold_policy(), Some(false));
    assert!(ctx_for(profile).resolve_spec(registry, "zipfs").is_some());
    assert!(ctx_for(profile).resolve_spec(registry, "setint").is_some());
}

/// Alias canonicalisation is load-bearing (§2.4): the legacy `irules`
/// spelling must behave exactly like `f5-irules` through the profile path.
#[test]
fn irules_alias_resolves_like_the_canonical_profile() {
    let via_alias = DialectProfile::irules();
    let reg = static_context_for("f5-irules").commands();
    assert!(ctx_for(via_alias).resolve_spec(reg, "exec").is_none());
    assert!(ctx_for(via_alias).resolve_spec(reg, "pool").is_some());
}

/// §5.2 option gating: `intersects` membership plus the version ceiling.
/// The confirmed defect: `expect_after`'s options carry no own gate and
/// inherit the command's `EXPECT` — under the old `contains` rule a
/// composed (version|vendor) active mask silently dropped every inherited
/// option on every vendor command.
#[test]
fn option_gating_resolves_inherited_vendor_options() {
    let reg = static_context_for("expect").commands();
    let expect = tcl_registry::model::ingress::resolve_environment("expect").analyser_profile();
    let spec = ctx_for(expect)
        .resolve_spec(reg, "expect_after")
        .expect("expect_after resolves under expect");
    // TP: inherited (gate = parent EXPECT) options resolve under expect.
    let names = ctx_for(expect).available_option_names(spec);
    for opt in ["-re", "-ex", "-gl", "-nocase", "-i", "-info"] {
        assert!(names.contains(&opt), "{opt} must resolve under expect");
    }
    // TN: the same options do NOT resolve under plain tcl8.6 (the command
    // itself is expect-only, so its gate never intersects TCL86).
    let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for opt in spec.options {
        assert!(
            !ctx_for(tcl86).option_available(opt, spec.surface),
            "{}: expect-gated option must not resolve under plain tcl8.6",
            opt.name
        );
    }
}

/// §5.2's version guard: a version-gated core option resolves when the
/// profile's ceiling covers it and stays hidden below — for both plain
/// versions and composed vendor profiles.
#[test]
fn option_gating_honours_the_version_ceiling() {
    let reg = static_context_for("tcl9.0").commands();
    let switch_spec = reg.get("switch").expect("switch spec");
    let nocase = switch_spec
        .options
        .iter()
        .find(|o| o.name == "-nocase")
        .expect("switch -nocase is a declared option");
    // switch -nocase is TCL85_PLUS (a verified data anchor).
    assert_eq!(nocase.surface, Some(SpecSurface::TCL85_PLUS));

    // TP: resolves at/above 8.5 — including the composed vendor profiles
    // whose embedded core is 8.5+ (the fix).
    for d in ["tcl8.5", "tcl8.6", "tcl9.0", "expect"] {
        assert!(
            static_document_context_for(d).option_available(nocase, switch_spec.surface),
            "{d}: switch -nocase is 8.5+ core"
        );
    }
    // TN/FN-guard: hidden below 8.5 — tcl8.4 (ceiling V8_4), iRules
    // (embedded 8.4; its bare mask never intersects a pure version gate),
    // and the trunk-riding iApps host (fork of Tcl at 8.4.6 — measured,
    // bigip-irule-parser-measurements.md §4a).
    for d in ["tcl8.4", "f5-irules", "f5-iapps"] {
        assert!(
            !static_document_context_for(d).option_available(nocase, switch_spec.surface),
            "{d}: switch -nocase must stay hidden on an 8.4 base"
        );
    }
}

/// The ceiling prevents the §5.2 leak: an option introduced *above* the
/// profile's embedded base never resolves there even though the composed
/// mask intersects its gate's other bits.
#[test]
fn option_gating_blocks_later_version_leaks_into_supersets() {
    let reg = static_context_for("f5-iapps").commands();
    let regsub = reg.get("regsub").expect("regsub spec");
    let command_opt = regsub
        .options
        .iter()
        .find(|o| o.name == "-command")
        .expect("regsub -command is declared (9.0+)");
    let gate = command_opt.surface.expect("-command is version-gated");
    assert_eq!(gate.min_version(), Some(tcl_dialect::TclVersion::V9_0));

    // TN: 9.0-only options stay hidden under every 8.x profile — plain and
    // composed vendor alike.
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "f5-iapps",
        "expect",
        "f5-irules",
    ] {
        assert!(
            !static_document_context_for(d).option_available(command_opt, regsub.surface),
            "{d}: regsub -command is 9.0-only"
        );
    }
    // TP: resolves at 9.0/9.1.
    for d in ["tcl9.0", "tcl9.1"] {
        assert!(
            static_document_context_for(d).option_available(command_opt, regsub.surface),
            "{d}"
        );
    }
}

/// §5.1 `available_subcommands` — the completion gap: version-gated
/// subcommands follow the profile mask.
#[test]
fn available_subcommands_follow_the_profile_mask() {
    let reg = static_context_for("tcl8.6").commands();
    let dict = reg.get("dict").expect("dict spec");
    let subs_86: Vec<&str> = static_document_context_for("tcl8.6")
        .available_subcommands(dict)
        .iter()
        .map(|s| s.name)
        .collect();
    let subs_90: Vec<&str> = static_document_context_for("tcl9.0")
        .available_subcommands(dict)
        .iter()
        .map(|s| s.name)
        .collect();
    // getwithdefault is the 9.0 addition (TIP 342 landed getdef/getwithdefault in 9.0).
    assert!(
        !subs_86.contains(&"getwithdefault"),
        "dict getwithdefault is 9.0+ and must be filtered under 8.6: {subs_86:?}"
    );
    assert!(subs_90.contains(&"getwithdefault"), "{subs_90:?}");
    // The universal core subcommands are present in both.
    for sub in ["get", "set", "keys"] {
        assert!(subs_86.contains(&sub) && subs_90.contains(&sub), "{sub}");
    }
}

/// A bare `IRULES` mask query never admits a sandbox-banned command,
/// enforced BY CONSTRUCTION via the spec tags themselves: exclusion is pure
/// mask intersection inside `get_for_surface`, so every low-level consumer
/// (`defines_symbol` / `resolve_call` / `resolve_terminator` / the CLI
/// snapshot's `command_names`) is covered without per-caller audits. Because
/// the exclusion lives in each spec's `dialects` group (a banned command is
/// `ALL_TCL`, with no `IRULES` bit) rather than in a profile-side disable
/// list, it holds uniformly on a raw, un-stamped registry too.
#[test]
fn bare_irules_mask_queries_exclude_non_irules_commands() {
    let reg = static_context_for("f5-irules").commands();
    for banned in IRULES_BANNED {
        assert!(
            reg.get_for_surface(banned, SpecSurface::IRULES).is_none(),
            "{banned}: a bare-mask query on the f5-irules registry must not \
             admit a command that lacks the IRULES bit"
        );
    }
    // The F5 surface and iRules-enabled core still resolve through the same path.
    for ok in ["set", "pool", "when", "HTTP::header"] {
        assert!(
            reg.get_for_surface(ok, SpecSurface::IRULES).is_some(),
            "{ok} must resolve under the bare IRULES mask"
        );
    }
    // The exclusion is intrinsic to the tags, not to the profile stamp: a
    // raw, hand-assembled registry excludes the banned command by plain
    // intersection just the same, while the IRULES-tagged core still resolves.
    let mut raw = tcl_registry::CommandRegistry::build_default();
    raw.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    assert!(
        raw.get_for_surface("set", SpecSurface::IRULES).is_some(),
        "an IRULES-tagged command resolves on a raw registry"
    );
    assert!(
        raw.get_for_surface("exec", SpecSurface::IRULES).is_none(),
        "a non-IRULES (ALL_TCL) command is excluded even on a raw registry"
    );
}

/// §5.4 out-of-registry vendor knowledge: the vendor surface summary is
/// registry-derived (never hand-listed prose), honours each spec's explicit
/// dialect gate, and abstains for profiles without a vendor surface.
#[test]
fn vendor_surface_summarises_the_registry_truth() {
    let reg = static_context_for("f5-irules").commands();
    let surface = ctx_for(DialectProfile::irules())
        .vendor_command_surface(reg)
        .expect("iRules has a vendor surface");
    assert!(
        surface.command_count > 900,
        "the modelled F5 surface is large: {}",
        surface.command_count
    );
    let ns_names: Vec<&str> = surface
        .namespaces
        .iter()
        .map(|(ns, _)| ns.as_str())
        .collect();
    for expected in ["HTTP", "TCP", "SSL", "LB", "DNS"] {
        assert!(ns_names.contains(&expected), "{expected}:: in the surface");
    }
    // Sorted by descending size.
    assert!(
        surface.namespaces.windows(2).all(|w| w[0].1 >= w[1].1),
        "namespaces sort by descending count"
    );
    // No vendor bit → no surface (plain Tcl); a vendor bit with an
    // identity-only mask (f5-bigip has no command pack) also abstains.
    assert!(
        static_document_context_for("tcl8.6")
            .vendor_command_surface(static_context_for("tcl8.6").commands())
            .is_none()
    );
    assert!(
        static_document_context_for("f5-bigip")
            .vendor_command_surface(static_context_for("f5-bigip").commands())
            .is_none()
    );
}

/// The iRules event/command cross-product never lists banned commands —
/// `commands_for_event` resolves through the same explicit-tag intersection,
/// so a command that lacks the `IRULES` bit is excluded there too.
#[test]
fn commands_for_event_excludes_banned_commands() {
    let reg = static_context_for("f5-irules").commands();
    let events = tcl_registry::events::EventRegistry::build();
    let profiles = tcl_registry::profiles::ProfileRegistry::build();
    let cmds = reg.valid_irules_commands_for_event("HTTP_REQUEST", &events, &profiles, None);
    for banned in ["exec", "file", "socket", "exit"] {
        assert!(
            !cmds.contains(&banned),
            "{banned} must not appear in the HTTP_REQUEST command set"
        );
    }
    assert!(
        cmds.contains(&"pool"),
        "the F5 surface is present: {}",
        cmds.len()
    );
}
