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
//! disable list. Until the Milestone 5 retag, the per-spec
//! `NON_IRULES_OPERATORS` tags encode the same exclusion, so these tests
//! prove the two encodings agree *by data*, not by convention — and they
//! keep protecting the banned surface after the retag flips which side is
//! load-bearing.

use std::collections::BTreeSet;

use tcl_dialect::{DialectProfile, DialectSet};
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

/// The disable list authored on the f5-irules profile must equal, exactly,
/// the set of command-level specs the registry data currently excludes from
/// iRules via a `NON_IRULES_OPERATORS` tag (minus the math-operator heads,
/// which are excluded because operators are not command heads in iRules —
/// modelled separately by the Milestone 5 `operators_as_commands` toggle,
/// not by the disable list).
///
/// Direction matters both ways: a command in the profile list that the data
/// doesn't exclude would make the profile *stricter* than shipped behaviour;
/// a tagged command missing from the profile list would be silently
/// re-admitted by profile-driven consumers after the Milestone 5 retag.
#[test]
fn irules_disabled_commands_match_the_spec_data() {
    let reg = registry_for_dialect("f5-irules");
    let derived: BTreeSet<&str> = reg
        .command_names()
        .flat_map(|name| reg.specs(name))
        .filter(|spec| spec.dialects == Some(DialectSet::NON_IRULES_OPERATORS))
        .map(|spec| spec.name)
        .filter(|name| !is_mathop_spelling(name))
        .collect();
    let authored: BTreeSet<&str> = DialectProfile::irules()
        .disabled_commands
        .iter()
        .copied()
        .collect();

    let missing: Vec<&&str> = derived.difference(&authored).collect();
    let extra: Vec<&&str> = authored.difference(&derived).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "profile disable list and spec data disagree.\n\
         tagged in data but missing from profile: {missing:?}\n\
         in profile but not tagged in data: {extra:?}\n\
         full derived set: {derived:?}"
    );
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
