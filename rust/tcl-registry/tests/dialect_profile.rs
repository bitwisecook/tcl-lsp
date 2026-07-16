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
//! The iRules profile is SUBTRACTIVE: bare `IRULES` mask + an explicit
//! disable list. Before the Milestone 5 retag the per-spec
//! `NON_IRULES_OPERATORS` tags encoded the same exclusion and these tests
//! proved the two encodings agreed *by data*; since the retag the profile
//! side is load-bearing (specs are `dialects: None`, the disable list and
//! the `OPERATOR_COMMAND` trait do the excluding) and these tests prove the
//! retired encoding never creeps back while the banned surface stays
//! banned.

use tcl_dialect::{DialectProfile, DialectSet};
use tcl_registry::traits::Traits;
use tcl_registry::{ProfileQueries, registry_for_dialect};

/// Whether `name` is a `tcl::mathop` operator-command spelling (bare `+`,
/// `eq`, or a qualified `tcl::mathop::+` form). Data-driven: a bare name is
/// an operator head iff the registry also carries its `tcl::mathop::`-
/// qualified spelling — no hardcoded operator list.
fn is_mathop_spelling(name: &str) -> bool {
    let reg = registry_for_dialect("tcl9.0");
    name.strip_prefix("::tcl::mathop::").is_some()
        || name.strip_prefix("tcl::mathop::").is_some()
        || !reg.specs(&format!("tcl::mathop::{name}")).is_empty()
}

/// The Milestone 5 retag is complete and stays complete: no spec gate at
/// any level (command, subcommand, option, form option, subcommand option)
/// in any profile's registry is the retired `NON_IRULES_OPERATORS` union —
/// "every dialect except iRules/Tk/BPF", reconstructed here because the
/// constant itself was deleted from `DialectSet`. Exclusion from iRules is
/// modelled on the profile (disable list / operator trait), never by
/// enumerating the complement of the excluded dialects.
#[test]
fn retired_non_irules_operators_union_never_reappears_as_a_gate() {
    let retired = DialectSet::ALL_TCL
        | DialectSet::IAPPS
        | DialectSet::EXPECT
        | DialectSet::SYNOPSYS
        | DialectSet::CADENCE
        | DialectSet::XILINX
        | DialectSet::QUARTUS
        | DialectSet::MENTOR;
    let check = |gate: Option<DialectSet>, what: &str| {
        assert_ne!(
            gate,
            Some(retired),
            "{what}: the retired NON_IRULES_OPERATORS union must not be a \
             spec gate — model iRules exclusion on the profile instead \
             (disabled_commands / Traits::OPERATOR_COMMAND)"
        );
    };
    for profile in DialectProfile::all() {
        let reg = registry_for_dialect(profile.name);
        for name in reg.command_names() {
            for spec in reg.specs(name) {
                check(spec.dialects, spec.name);
                for opt in spec.options {
                    check(opt.dialects, &format!("{} {}", spec.name, opt.name));
                }
                for form in spec.command_forms {
                    for opt in form.options {
                        check(opt.dialects, &format!("{} {}", spec.name, opt.name));
                    }
                }
                for sub in spec.subcommands {
                    check(sub.dialects, &format!("{} {}", spec.name, sub.name));
                    for opt in sub.options {
                        check(
                            opt.dialects,
                            &format!("{} {} {}", spec.name, sub.name, opt.name),
                        );
                    }
                }
            }
        }
    }
}

/// Post-retag, the profile disable list is **load-bearing**: every banned
/// name still exists in the f5-irules registry as spec data that the bare
/// `IRULES` mask alone would admit (universal `dialects: None` after the
/// retag), so the §9 disable filter — not a version tag, not a mask bit —
/// is the only thing keeping it out. A list entry the mask already
/// excludes would be dead weight hiding a modelling error on the wrong
/// axis.
#[test]
fn irules_disable_list_is_load_bearing() {
    let reg = registry_for_dialect("f5-irules");
    for banned in DialectProfile::irules().disabled_commands {
        let specs = reg.specs(banned);
        assert!(
            !specs.is_empty(),
            "{banned}: disable-list entry names no registered spec"
        );
        assert!(
            specs.iter().any(|s| s.supports_dialect(DialectSet::IRULES)),
            "{banned}: the bare IRULES mask already excludes this spec, so \
             the disable-list entry is not load-bearing — if it is version \
             gating, it belongs on the version axis, not in the ban list"
        );
    }
}

/// The math-operator heads are excluded from iRules by dialect *shape*
/// (`operators_as_commands == false` + `Traits::OPERATOR_COMMAND`), not by
/// the disable list. Both directions of the trait marking are data-bound:
/// a spec carries the trait iff it is a `tcl::mathop` spelling.
#[test]
fn operator_heads_carry_the_trait_and_follow_the_profile_shape() {
    let reg = registry_for_dialect("tcl9.0");
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
    let tcl90 = DialectProfile::by_name("tcl9.0");
    for op in ["+", "eq", "tcl::mathop::+"] {
        assert!(
            tcl90.resolve_command(reg, op).is_some(),
            "{op} resolves under tcl9.0"
        );
    }
    // …TN: and never under iRules (operators live only inside expr there) —
    // through the profile query, a bare mask query on the stamped registry
    // (§9.2), and the snapshot's independent most-specific resolver alike.
    let ireg = registry_for_dialect("f5-irules");
    let irules = DialectProfile::irules();
    for op in ["+", "eq", "tcl::mathop::+"] {
        assert!(
            irules.resolve_command(ireg, op).is_none(),
            "{op} must not resolve under f5-irules"
        );
        assert!(
            ireg.get_for_dialect(op, DialectSet::IRULES).is_none(),
            "{op} must not resolve via a bare mask query either (§9.2)"
        );
        assert!(
            !irules.is_command_disabled(op),
            "{op}: operator heads are excluded by shape, not the ban list — \
             keep the mechanisms separate"
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
    let reg = registry_for_dialect("f5-irules");
    let irules = DialectProfile::irules();
    for (cmd, sub_name) in [("DNS::header", "cd"), ("IP::stats", "in")] {
        let spec = irules
            .resolve_command(reg, cmd)
            .unwrap_or_else(|| panic!("{cmd} resolves under f5-irules"));
        assert!(
            irules
                .available_subcommands(spec)
                .iter()
                .any(|s| s.name == sub_name),
            "{cmd} {sub_name}: a real iRules subcommand must not be hidden \
             by its name colliding with a banned command / operator"
        );
    }
}

/// The user-facing contract the disable list exists for: the banned
/// commands never resolve under the iRules profile, while the F5 surface
/// and the universal 8.4 core still do. This is the test that must KEEP
/// passing across the Milestone 5 retag (when the per-spec tags flip to
/// `None` and only the disable list bans them).
#[test]
fn irules_banned_commands_never_resolve() {
    let reg = registry_for_dialect("f5-irules");
    let irules = DialectProfile::irules();

    // TP: genuinely banned commands do not resolve.
    for banned in irules.disabled_commands {
        assert!(
            irules.resolve_command(reg, banned).is_none(),
            "{banned}: banned command must not resolve under f5-irules"
        );
    }
    // TN: the universal core and the F5 surface still resolve.
    for allowed in ["set", "if", "string", "foreach", "pool", "when", "log"] {
        assert!(
            irules.resolve_command(reg, allowed).is_some(),
            "{allowed}: must resolve under f5-irules"
        );
    }
    // Version-gated core (8.5+/8.6) is *unavailable* under the pinned-8.4
    // iRules runtime — via the version tags, not the disable list.
    for versioned in ["dict", "lassign", "apply", "lmap", "coroutine"] {
        assert!(
            irules.resolve_command(reg, versioned).is_none(),
            "{versioned}: 8.5+/8.6 core is never present in iRules (D3)"
        );
        assert!(
            !irules.is_command_disabled(versioned),
            "{versioned}: version-gated, not banned — keep the axes separate"
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
        // iApps: Tcl 8.5.13 host — dict/lassign/apply are real; lmap (8.6)
        // and zipfs (9.0) are not. exec/file/socket are ALLOWED (host
        // interpreter, not the TMM sandbox).
        (
            "f5-iapps",
            &["dict", "lassign", "apply", "exec", "file", "socket", "set"],
            &["lmap", "coroutine", "zipfs"],
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
        let profile = DialectProfile::by_name(dialect);
        let reg = registry_for_dialect(dialect);
        for name in resolves {
            assert!(
                profile.resolve_command(reg, name).is_some(),
                "{dialect}: {name} must resolve (embedded-core fix)"
            );
        }
        for name in unavailable {
            assert!(
                profile.resolve_command(reg, name).is_none(),
                "{dialect}: {name} must stay unavailable"
            );
        }
    }
}

/// Alias canonicalisation is load-bearing (§2.4): the legacy `irules`
/// spelling must behave exactly like `f5-irules` through the profile path.
#[test]
fn irules_alias_resolves_like_the_canonical_profile() {
    let via_alias = DialectProfile::by_name("irules");
    let reg = registry_for_dialect("f5-irules");
    assert!(via_alias.resolve_command(reg, "exec").is_none());
    assert!(via_alias.resolve_command(reg, "pool").is_some());
}

/// §5.2 option gating: `intersects` membership plus the version ceiling.
/// The confirmed defect: `expect_after`'s options carry no own gate and
/// inherit the command's `EXPECT` — under the old `contains` rule a
/// composed (version|vendor) active mask silently dropped every inherited
/// option on every vendor command.
#[test]
fn option_gating_resolves_inherited_vendor_options() {
    let reg = registry_for_dialect("expect");
    let expect = DialectProfile::by_name("expect");
    let spec = expect
        .resolve_command(reg, "expect_after")
        .expect("expect_after resolves under expect");
    // TP: inherited (gate = parent EXPECT) options resolve under expect.
    let names = expect.available_option_names(spec);
    for opt in ["-re", "-ex", "-gl", "-nocase", "-i", "-info"] {
        assert!(names.contains(&opt), "{opt} must resolve under expect");
    }
    // TN: the same options do NOT resolve under plain tcl8.6 (the command
    // itself is expect-only, so its gate never intersects TCL86).
    let tcl86 = DialectProfile::by_name("tcl8.6");
    for opt in spec.options {
        assert!(
            !tcl86.is_option_available(opt, spec.dialects),
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
    let reg = registry_for_dialect("tcl9.0");
    let switch_spec = reg.get("switch").expect("switch spec");
    let nocase = switch_spec
        .options
        .iter()
        .find(|o| o.name == "-nocase")
        .expect("switch -nocase is a declared option");
    // switch -nocase is TCL85_PLUS (verified data anchor, §13).
    assert_eq!(nocase.dialects, Some(DialectSet::TCL85_PLUS));

    // TP: resolves at/above 8.5 — including the composed vendor profiles
    // whose embedded core is 8.5+ (the fix).
    for d in ["tcl8.5", "tcl8.6", "tcl9.0", "f5-iapps", "expect"] {
        assert!(
            DialectProfile::by_name(d).is_option_available(nocase, switch_spec.dialects),
            "{d}: switch -nocase is 8.5+ core"
        );
    }
    // TN/FN-guard: hidden below 8.5 — tcl8.4 (ceiling V8_4) and iRules
    // (embedded 8.4; its bare mask never intersects a pure version gate).
    for d in ["tcl8.4", "f5-irules"] {
        assert!(
            !DialectProfile::by_name(d).is_option_available(nocase, switch_spec.dialects),
            "{d}: switch -nocase must stay hidden on an 8.4 base"
        );
    }
}

/// The ceiling prevents the §5.2 leak: an option introduced *above* the
/// profile's embedded base never resolves there even though the composed
/// mask intersects its gate's other bits.
#[test]
fn option_gating_blocks_later_version_leaks_into_supersets() {
    let reg = registry_for_dialect("f5-iapps");
    let regsub = reg.get("regsub").expect("regsub spec");
    let command_opt = regsub
        .options
        .iter()
        .find(|o| o.name == "-command")
        .expect("regsub -command is declared (9.0+)");
    let gate = command_opt.dialects.expect("-command is version-gated");
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
            !DialectProfile::by_name(d).is_option_available(command_opt, regsub.dialects),
            "{d}: regsub -command is 9.0-only"
        );
    }
    // TP: resolves at 9.0/9.1.
    for d in ["tcl9.0", "tcl9.1"] {
        assert!(
            DialectProfile::by_name(d).is_option_available(command_opt, regsub.dialects),
            "{d}"
        );
    }
}

/// §5.1 `available_subcommands` — the completion gap: version-gated
/// subcommands follow the profile mask.
#[test]
fn available_subcommands_follow_the_profile_mask() {
    let reg = registry_for_dialect("tcl8.6");
    let dict = reg.get("dict").expect("dict spec");
    let subs_86: Vec<&str> = DialectProfile::by_name("tcl8.6")
        .available_subcommands(dict)
        .iter()
        .map(|s| s.name)
        .collect();
    let subs_90: Vec<&str> = DialectProfile::by_name("tcl9.0")
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

/// §9.2 pre-retag gate, enforced BY CONSTRUCTION: a bare `IRULES` mask
/// query on the profile-built f5-irules registry can never re-admit a
/// banned command — the registry applies its stamped profile's subtractive
/// rules inside `get_for_dialect` itself, so every low-level consumer
/// (`defines_symbol` / `resolve_call` / `resolve_terminator` / the CLI
/// snapshot's `command_names`) is covered without per-caller audits. This
/// is the test that must hold BEFORE and AFTER the Milestone 5 data retag.
#[test]
fn bare_irules_mask_queries_apply_the_disable_filter() {
    let reg = registry_for_dialect("f5-irules");
    for banned in DialectProfile::irules().disabled_commands {
        assert!(
            reg.get_for_dialect(banned, DialectSet::IRULES).is_none(),
            "{banned}: a bare-mask query on the f5-irules registry must not \
             re-admit a banned command (§9.2)"
        );
    }
    // The F5 surface and universal core still resolve through the same path.
    for ok in ["set", "pool", "when", "HTTP::header"] {
        assert!(
            reg.get_for_dialect(ok, DialectSet::IRULES).is_some(),
            "{ok} must resolve under the bare IRULES mask"
        );
    }
    // A hand-assembled registry (no profile stamp) keeps raw tag semantics —
    // the subtractive rules belong to profile-built registries only.
    let mut raw = tcl_registry::CommandRegistry::build_default();
    raw.load_dialect(DialectSet::IRULES);
    assert!(
        raw.get_for_dialect("set", DialectSet::IRULES).is_some(),
        "raw registries keep working"
    );
}

/// §5.4 out-of-registry vendor knowledge: the vendor surface summary is
/// registry-derived (never hand-listed prose), honours the subtractive
/// rules, and abstains for profiles without a vendor surface.
#[test]
fn vendor_surface_summarises_the_registry_truth() {
    let reg = registry_for_dialect("f5-irules");
    let surface = DialectProfile::irules()
        .vendor_surface(reg)
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
        DialectProfile::by_name("tcl8.6")
            .vendor_surface(registry_for_dialect("tcl8.6"))
            .is_none()
    );
    assert!(
        DialectProfile::by_name("f5-bigip")
            .vendor_surface(registry_for_dialect("f5-bigip"))
            .is_none()
    );
}

/// The iRules event/command cross-product never lists banned commands —
/// `commands_for_event` routes through the same subtractive filter.
#[test]
fn commands_for_event_excludes_banned_commands() {
    let reg = registry_for_dialect("f5-irules");
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
