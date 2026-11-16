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

use super::super::super::state::Analyser;

/// `(code, message)` pairs for the version-gate codes only.
fn version_diags(source: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

fn fires(source: &str, code: &str) -> bool {
    version_diags(source).iter().any(|(c, _)| c == code)
}

fn count(source: &str, code: &str) -> usize {
    version_diags(source)
        .iter()
        .filter(|(c, _)| c == code)
        .count()
}

#[test]
fn original_lifecycle_gates_keep_alias_operands_and_typed_source_subjects() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // These assertions cover source selection and diagnostic coordinates;
    // they do not establish runtime alias installation or widget creation.
    for source in [
        "package require Tk 8.6\ninterp alias {} createEntry {} entry .e\ncreateEntry -placeholder text\n",
        "package require Tk 8.6\nrename entry createEntry\ncreateEntry .e -placeholder text\n",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let found = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W136)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        let subject = found[0]
            .registry_source()
            .expect("typed Registry lifecycle ownership");
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::Lifecycle
        );
        assert_eq!(subject.argument(), Some(1));
        assert_eq!(&source[subject.span().as_range()], "-placeholder");
    }
    for source in [
        "package require Tk 8.6\nproc entry args {}; entry .e -placeholder text\n",
        "package require Tk 8.6\ninterp alias {} createEntry {} entry .e -placeholder text\ncreateEntry\n",
        "package require Tk 8.6\nentry .e $flag text -placeholder text\n",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W136),
            "{source}"
        );
    }
}

#[test]
fn option_value_that_looks_like_a_flag_is_not_retested() {
    // `-placeholder`'s value is itself `-placeholder`. The
    // value word must be skipped, so exactly ONE W136 fires (the option),
    // not two (the option plus its value re-tested as an option).
    let src = "package require Tk 8.6\nentry .e -placeholder -placeholder\n";
    assert_eq!(count(src, "W136"), 1, "{:?}", version_diags(src));
}

#[test]
fn value_word_of_ungated_option_draws_no_spurious_diagnostic() {
    // `-textvariable` consumes the next word `-placeholder` as its VALUE, so
    // `-placeholder` must not be tested as a (gated) option — no W136.
    let src = "package require Tk 8.6\nentry .e -textvariable -placeholder\n";
    assert_eq!(count(src, "W136"), 0, "{:?}", version_diags(src));
}

#[test]
fn option_below_floor_fires_w136() {
    // `-placeholder` needs Tk 8.7; the require guarantees only 8.6.
    let src = "package require Tk 8.6\nentry .e -placeholder hi\n";
    assert!(fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn option_met_by_floor_is_silent() {
    let src = "package require Tk 8.7\nentry .e -placeholder hi\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

/// `(code, message)` version-gate pairs for an arbitrary dialect with
/// optional keyed library-version pins (§7.1 axis C).
fn version_diags_for(
    source: &str,
    dialect: &str,
    bigip_version: Option<&str>,
) -> Vec<(String, String)> {
    let mut a = Analyser::new();
    a.library_versions.bigip_version = bigip_version.map(str::to_owned);
    a.analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// Build a registry overlay declaring `package` ambient at `version`,
/// and return an analyser bound to it.
///
/// Goes straight at the registry cache rather than through `tcl-spectcl`:
/// the pack loader depends on this crate, so a test here cannot load a
/// `.tclspec`. What it *can* do is install the same thing the loader
/// installs, which is what the analyser actually reads.
fn analyser_with_ambient(
    dialect: &str,
    key: u64,
    rows: &[(&'static str, &'static str)],
) -> Analyser {
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let _registry = tcl_registry::registry_for_profile_with_overlay(profile, key, |registry| {
        for (package, version) in rows {
            registry.insert_ambient_package(package, version);
        }
    });
    Analyser::new().with_pack_overlay(key)
}

fn gate_messages(analyser: &mut Analyser, source: &str, dialect: &str) -> Vec<(String, String)> {
    analyser
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// A pack declaring a package ambient floors it without any `package
/// require` in the file — which is the whole point: a package that comes
/// *with* the dialect is never required, so nothing else could floor it.
#[test]
fn a_pack_ambient_declaration_floors_a_package_with_no_require() {
    // Tk is hosted (not ambient) on `tcl8.6`, so with no require and no
    // pack there is no floor at all and the 8.7 option is not gated.
    let src = "entry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(src, "tcl8.6", None).len(),
        0,
        "baseline: no floor, no gate"
    );

    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0001, &[("Tk", "8.6")]);
    let diags = gate_messages(&mut analyser, src, "tcl8.6");
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("a spec pack declares Tk 8.6 ambient here")),
        "the pack's declaration is the guarantor: {diags:?}"
    );
}

/// A pack's `ambient_package` row must silence **W120** too, not just
/// feed the version floor.
///
/// Registering the floor while the same package still draws "needs a
/// `package require`" is the worst of both worlds: the analyser tells the
/// author to add a require for something the pack has just declared the
/// runtime already provides, and offers an insert fix that would be
/// wrong.  Wiring the floor is not enough on its own: the three consumers
/// that ask "is this ambient?" have to honour the declaration too.
#[test]
fn a_pack_ambient_package_is_not_reported_as_a_missing_require() {
    // `entry` belongs to Tk, which is *hosted* on tcl8.6 — so with no
    // pack the missing require is real and W120 fires. That is the
    // baseline the pack has to change.
    let src = "entry .e\n";
    let bare = Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "W120");
    assert!(bare, "baseline: Tk is hosted on tcl8.6, so W120 is right");

    let mut analyser = analyser_with_ambient("tcl8.6", 0x1642_0001, &[("Tk", "8.6")]);
    let diags = analyser.analyse(src, "tcl8.6").diagnostics;
    let w120: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_str() == "W120")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        w120.len(),
        0,
        "a pack declaring Tk ambient means there is nothing to require: {w120:?}"
    );
}

/// The three floors compose by taking the greatest, exactly as two
/// `package require` lines already do — a floor is a lower bound.
#[test]
fn the_highest_of_the_three_floors_wins() {
    // The pack claims 8.7; the profile pin claims 8.6. The higher claim
    // holds, so the 8.7-introduced option is no longer gated.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0002, &[("Tk", "8.7")]);
    assert_eq!(
        gate_messages(
            &mut analyser,
            "package require Tk\nentry .e -placeholder hi\n",
            "tcl8.6"
        )
        .len(),
        0,
        "the pack's 8.7 outranks the profile's 8.6"
    );

    // The require claims 8.7; the pack claims only 8.6. Same outcome from
    // the other direction.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0003, &[("Tk", "8.6")]);
    assert_eq!(
        gate_messages(
            &mut analyser,
            "package require Tk 8.7\nentry .e -placeholder hi\n",
            "tcl8.6"
        )
        .len(),
        0,
        "the require's 8.7 outranks the pack's 8.6"
    );
}

/// At equal versions the *message* names the source closest to the
/// author's control, because that is the one they can act on: the require
/// in this file before the pack in this workspace before the profile
/// compiled into the server.
#[test]
fn equal_floors_are_reported_in_precedence_order() {
    // Pack and profile pin both say 8.6: the pack wins the report.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0004, &[("Tk", "8.6")]);
    let diags = gate_messages(
        &mut analyser,
        "package require Tk\nentry .e -placeholder hi\n",
        "tcl8.6",
    );
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("a spec pack declares Tk 8.6 ambient here")),
        "pack outranks the profile pin at an equal version: {diags:?}"
    );

    // Require and pack both say 8.6: the require wins the report.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0005, &[("Tk", "8.6")]);
    let diags = gate_messages(
        &mut analyser,
        "package require Tk 8.6\nentry .e -placeholder hi\n",
        "tcl8.6",
    );
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("`package require` guarantees only 8.6")),
        "the require outranks the pack at an equal version: {diags:?}"
    );
}

/// FN guard for the short-circuit: a session with no packs must answer
/// exactly as it did before this axis existed.
#[test]
fn no_pack_overlay_leaves_every_floor_where_it_was() {
    let src = "package require Tk\nentry .e -placeholder hi\n";
    let before = version_diags_for(src, "tcl8.6", None);
    let mut analyser = Analyser::new();
    assert_eq!(gate_messages(&mut analyser, src, "tcl8.6"), before);
    assert!(
        before
            .iter()
            .any(|(_, m)| m.contains("tcl8.6 ships Tk 8.6")),
        "still the profile pin: {before:?}"
    );
}

// Versioned arity, W149.

/// Install a command owned by `Probe` whose signature changed: two
/// arguments from 3.0 until 5.0, three from 5.0. The plain `arity` is the
/// pre-3.0 shape and the fallback whenever no window covers the floor.
fn analyser_with_windows(key: u64, ambient: Option<&'static str>) -> Analyser {
    use tcl_registry::arity::{Arity, ArityWindow};
    use tcl_registry::lifecycle::Lifecycle;

    const WINDOWS: &[ArityWindow] = &[
        ArityWindow {
            lifecycle: Lifecycle {
                introduced: Some("3.0"),
                deprecated: None,
                retired: Some("5.0"),
                deprecation_fix: None,
            },
            arity: Arity::exact(2),
        },
        ArityWindow {
            lifecycle: Lifecycle {
                introduced: Some("5.0"),
                deprecated: None,
                retired: None,
                deprecation_fix: None,
            },
            arity: Arity::exact(3),
        },
    ];

    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let _registry = tcl_registry::registry_for_profile_with_overlay(profile, key, |registry| {
        registry.insert(tcl_registry::CommandSpec {
            name: "probe::grew",
            arity: Arity::exact(1),
            arity_windows: WINDOWS,
            required_package: Some("Probe"),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        if let Some(version) = ambient {
            registry.insert_ambient_package("Probe", version);
        }
    });
    Analyser::new().with_pack_overlay(key)
}

/// A spec whose window is **narrower** than its fallback — the shape the
/// surplus-run bug needs, since it only shows when the selected `max` is
/// below the `max` the walk anchored to.
fn analyser_with_narrowing_window(key: u64) -> Analyser {
    use tcl_registry::arity::{Arity, ArityWindow};
    use tcl_registry::lifecycle::Lifecycle;

    const WINDOWS: &[ArityWindow] = &[ArityWindow {
        lifecycle: Lifecycle {
            introduced: Some("3.0"),
            deprecated: None,
            retired: None,
            deprecation_fix: None,
        },
        arity: Arity::exact(2),
    }];

    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let _registry = tcl_registry::registry_for_profile_with_overlay(profile, key, |registry| {
        registry.insert(tcl_registry::CommandSpec {
            name: "probe::shrank",
            // Fallback takes three; from 3.0 it takes two.
            arity: Arity::exact(3),
            arity_windows: WINDOWS,
            required_package: Some("Probe"),
            ..tcl_registry::CommandSpec::DEFAULT
        });
    });
    Analyser::new().with_pack_overlay(key)
}

/// The surplus run is anchored to the shape actually reported against.
///
/// The walk computes the excess from the *fallback* `max`, because the
/// window is not chosen until the floor is known. Reporting a narrower
/// window's verdict with that stale anchor makes the "remove surplus
/// arguments" fix delete only the tail of the surplus and leave a call
/// that is still wrong.
#[test]
fn the_surplus_run_follows_the_window_not_the_fallback() {
    let mut analyser = analyser_with_narrowing_window(0x1642_0002);
    let src = "package require Probe 3.0\nprobe::shrank a b c d\n";
    let result = analyser.analyse(src, "tcl8.6");
    let e003: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "E003")
        .collect();
    assert_eq!(e003.len(), 1, "{:?}", result.diagnostics);

    // Floor 3.0 selects the two-argument shape, so `c` and `d` are both
    // surplus and the anchor starts at `c`. Anchoring to the fallback's
    // three would have started at `d`.
    let expected = u32::try_from(src.find("c d").expect("the surplus run")).expect("offset");
    assert_eq!(
        e003[0].span.start(),
        expected,
        "the anchor must cover every surplus word, not just the last: {:?}",
        e003[0]
    );
}

/// An ensemble that *gained* a mandatory subcommand word at 3.0: bare
/// calls are legal below that floor and an E001 at or above it.
fn analyser_with_versioned_ensemble(key: u64) -> Analyser {
    use tcl_registry::arity::{Arity, ArityWindow};
    use tcl_registry::lifecycle::Lifecycle;

    const WINDOWS: &[ArityWindow] = &[ArityWindow {
        lifecycle: Lifecycle {
            introduced: Some("3.0"),
            deprecated: None,
            retired: None,
            deprecation_fix: None,
        },
        arity: Arity::at_least(1),
    }];
    const SUBS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: "go",
        arity: Arity::exact(0),
        ..tcl_registry::SubCommand::DEFAULT
    }];

    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let _registry = tcl_registry::registry_for_profile_with_overlay(profile, key, |registry| {
        registry.insert(tcl_registry::CommandSpec {
            name: "probe::ens",
            // Below 3.0 a bare call has a defined default.
            arity: Arity::at_least(0),
            arity_windows: WINDOWS,
            subcommands: SUBS,
            required_package: Some("Probe"),
            ..tcl_registry::CommandSpec::DEFAULT
        });
    });
    Analyser::new().with_pack_overlay(key)
}

/// Whether an ensemble requires a subcommand at all can change across
/// releases, so the bare-call verdict has to be taken at the resolved
/// floor like every other arity fact.
///
/// `SubcommandSig::subcommand_required` is derived from the *fallback*
/// arity, so without consulting the parent's windows a versioned
/// ensemble's E001 would ignore the floor entirely.
#[test]
fn a_versioned_ensembles_bare_call_verdict_follows_the_floor() {
    let e001 = |analyser: &mut Analyser, src: &str| {
        analyser
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "E001")
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    };

    // Below the window: the fallback's zero minimum stands, so a bare
    // call is a defined default and not an error.
    let mut low = analyser_with_versioned_ensemble(0x1642_0003);
    assert_eq!(
        e001(&mut low, "package require Probe 1.0\nprobe::ens\n").len(),
        0,
        "below 3.0 a bare call is legal"
    );

    // At the window: the subcommand word became mandatory.
    let mut high = analyser_with_versioned_ensemble(0x1642_0004);
    assert_eq!(
        e001(&mut high, "package require Probe 3.0\nprobe::ens\n").len(),
        1,
        "from 3.0 a bare call is an E001"
    );

    // ...and naming the subcommand is fine at that floor.
    let mut named = analyser_with_versioned_ensemble(0x1642_0005);
    assert_eq!(
        e001(&mut named, "package require Probe 3.0\nprobe::ens go\n").len(),
        0,
        "a named subcommand satisfies the requirement"
    );
}

fn arity_diags(analyser: &mut Analyser, source: &str) -> Vec<(String, String)> {
    analyser
        .analyse(source, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "E002" | "E003" | "W149"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// The window covering the resolved floor is the shape the call is
/// judged against — not the plain `arity`, which is only the fallback.
#[test]
fn the_window_covering_the_resolved_floor_decides_the_arity() {
    // Floor 3.0 selects the two-argument window, so two words are right
    // and one is too few even though the plain `arity` is exactly 1.
    let mut a = analyser_with_windows(0x1627_1001, None);
    assert_eq!(
        arity_diags(&mut a, "package require Probe 3.0\nprobe::grew a b\n").len(),
        0,
        "two arguments fit the 3.0 window"
    );

    // Floor 5.0 selects the three-argument window.
    let mut a = analyser_with_windows(0x1627_1002, None);
    assert_eq!(
        arity_diags(&mut a, "package require Probe 5.0\nprobe::grew a b c\n").len(),
        0,
        "three arguments fit the 5.0 window"
    );
}

/// The case W149 exists for: the call is not malformed, it is written
/// for a different release. Reporting "too many arguments" would send the
/// author to fix a call that is already correct somewhere.
#[test]
fn a_count_fitting_another_window_is_w149_not_a_plain_arity_error() {
    // Floor 3.0 wants two; the call passes three, which is the 5.0 shape.
    let mut a = analyser_with_windows(0x1627_1003, None);
    let diags = arity_diags(&mut a, "package require Probe 3.0\nprobe::grew a b c\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].0, "W149", "not a bare E003: {diags:?}");
    assert!(
        diags[0].1.contains("matches Probe 5.0"),
        "names the release it fits: {diags:?}"
    );
    assert!(
        diags[0].1.contains("package require Probe 5.0"),
        "a later window points forward, at raising the floor: {diags:?}"
    );

    // The mirror: floor 5.0 wants three, the call passes two — the shape
    // that was valid until 5.0. Same code, opposite direction.
    let mut a = analyser_with_windows(0x1627_1004, None);
    let diags = arity_diags(&mut a, "package require Probe 5.0\nprobe::grew a b\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].0, "W149", "{diags:?}");
    assert!(
        diags[0].1.contains("was valid until 5.0"),
        "an earlier window points at the call, not the floor: {diags:?}"
    );
}

/// A count no window accepts is an ordinary arity error. There is no
/// version story to tell and inventing one would be noise.
#[test]
fn a_count_matching_no_window_stays_a_plain_arity_error() {
    let mut a = analyser_with_windows(0x1627_1005, None);
    let diags = arity_diags(&mut a, "package require Probe 3.0\nprobe::grew a b c d e\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].0, "E003", "five arguments fit nothing: {diags:?}");
}

/// FN guard: with no floor there is nothing to select against, so the
/// plain `arity` decides exactly as it did before windows existed — and
/// a sibling window fitting the count proves nothing about which release
/// the file targets, so it must not become a W149.
#[test]
fn no_resolvable_floor_falls_back_to_the_plain_arity() {
    let mut a = analyser_with_windows(0x1627_1006, None);
    assert_eq!(
        arity_diags(&mut a, "probe::grew a\n").len(),
        0,
        "the plain arity is exactly 1"
    );

    let mut a = analyser_with_windows(0x1627_1007, None);
    let diags = arity_diags(&mut a, "probe::grew a b\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0].0, "E003",
        "no floor ⇒ no version claim, however well two words fit the 3.0 window: {diags:?}"
    );
}

/// The two mechanisms this issue added meet here: a pack's
/// `ambient_package` floor selects the window with no `package require`
/// in the file at all.
#[test]
fn a_pack_ambient_floor_selects_the_window() {
    let mut a = analyser_with_windows(0x1627_1008, Some("5.0"));
    assert_eq!(
        arity_diags(&mut a, "probe::grew a b c\n").len(),
        0,
        "the ambient 5.0 selects the three-argument window"
    );

    let mut a = analyser_with_windows(0x1627_1009, Some("5.0"));
    let diags = arity_diags(&mut a, "probe::grew a b\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].0, "W149", "{diags:?}");
}

#[test]
fn tracksbase_tk_pin_floors_an_unversioned_require() {
    // §7.1: `tcl8.6` ships Tk 8.6, so `package require Tk` *without a
    // version* still guarantees only 8.6 — the 8.7-introduced
    // `-placeholder` draws W136 (the old rule was silent here: an
    // unversioned require yielded no floor at all).
    let src = "package require Tk\nentry .e -placeholder hi\n";
    let diags = version_diags_for(src, "tcl8.6", None);
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("tcl8.6 ships Tk 8.6")),
        "TracksBase floor names the runtime as the guarantor: {diags:?}"
    );
    // TN: Tk 9.0 (tracking the tcl9.0 base) carries the 8.7 additions.
    assert_eq!(
        version_diags_for(src, "tcl9.0", None).len(),
        0,
        "Tk 9.0 meets an 8.7 introduction"
    );
    // An explicit require can only RAISE the pin floor, never lower it:
    // requiring 8.7 on the 8.6 base is satisfied at 8.7.
    let raised = "package require Tk 8.7\nentry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(raised, "tcl8.6", None),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn keyed_bigip_floor_gates_the_f5_surface() {
    // HTTP2::header was introduced in BIG-IP 16.1.0 (the backfilled
    // datum); the iRules profile keys its surface on BigipVersion.
    let src = "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n";
    // TN at the oldest-supported default (16.1.0 meets 16.1.0)…
    assert!(
        version_diags_for(src, "f5-irules", None).is_empty(),
        "the default floor admits the 16.1.0 surface"
    );
    // …TP pinned below the introduction…
    let below = version_diags_for(src, "f5-irules", Some("15.1.0"));
    assert!(
        below.iter().any(|(c, m)| c == "W135"
            && m.contains("requires f5-irules-cmds 16.1.0")
            && m.contains("f5-irules ships f5-irules-cmds 15.1.0")),
        "a 15.1.0 pin exposes the 16.1.0 introduction: {below:?}"
    );
    // …TN pinned above.
    assert_eq!(
        version_diags_for(src, "f5-irules", Some("17.1.0")).len(),
        0,
        "a 17.1.0 pin satisfies a 16.1.0 introduction"
    );
}

#[test]
fn bigip_21_1_subcommands_follow_the_configured_floor() {
    for source in [
        "SSL::c3d cert_lifespan 5\n",
        "SSL::c3d cert_start_date override\n",
        "persist mcp persistence_name\n",
    ] {
        let below = version_diags_for(source, "f5-irules", Some("21.0.0"));
        assert!(
            below.iter().any(|(code, message)| code == "W135"
                && message.contains("requires f5-irules-cmds 21.1.0")),
            "21.0 must reject the 21.1 subcommand: {below:?}"
        );
        assert_eq!(
            version_diags_for(source, "f5-irules", Some("21.1.0")).len(),
            0,
            "21.1 must admit {source:?}"
        );
    }
}

#[test]
fn persist_table_mcp_mode_follows_the_configured_floor() {
    for operation in ["add", "lookup", "delete"] {
        let source = format!("persist {operation} mcp key\n");
        let below = version_diags_for(&source, "f5-irules", Some("21.0.0"));
        assert!(
            below.iter().any(|(code, message)| code == "W135"
                && message.contains("Argument value 'mcp'")
                && message.contains("requires f5-irules-cmds 21.1.0")),
            "21.0 must reject persist {operation} mcp: {below:?}"
        );
        assert_eq!(
            version_diags_for(&source, "f5-irules", Some("21.1.0")).len(),
            0,
            "21.1 must admit persist {operation} mcp"
        );
    }
}

/// Version-gate + argument-DSL codes for `source` under `dialect`.
fn dsl_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W137" | "W138" | "W200" | "W202"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

#[test]
fn original_format_gates_keep_alias_ordinals_and_written_template_anchors() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    // This checks readonly source metadata and diagnostic geometry, not
    // alias installation or successful execution in a native interpreter.
    for source in [
        "interp alias {} encode {} binary format\nencode cu 5\n",
        "rename binary encode\nencode format cu 5\n",
        "::binary format cu 5\n",
        "binary fo cu 5\n",
        "set output [binary format cu 5]\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.4");
        let found = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W200)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(&source[found[0].span.as_range()], "cu", "{source}");
    }
    // The effective template is a captured prefix. No invocation operand
    // can borrow its location from the alias declaration or a later value.
    assert_eq!(
        dsl_diags(
            "interp alias {} encode {} binary format cu\nencode 5\n",
            "tcl8.4",
        ),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn original_format_gates_read_literal_values_without_promoting_dynamic_words() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    // These source-consumer controls do not claim native runtime results.
    for (source, template) in [
        (r"format \x25\u0062 5", b"%b".as_slice()),
        (r"binary format c\u0075 5", b"cu".as_slice()),
    ] {
        let result = Analyser::new().analyse(source, "tcl8.4");
        let config = result.body_lexer_config.unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source, &result, &segment,
        )
        .unwrap_or_else(|| panic!("source schema missing: {source}"));
        let context = result.resolved_input.as_ref().unwrap().context_registry();
        let formats = words
            .with_source_schema(
                &context,
                crate::analyser::diagnostic_registry::source_format_arguments,
            )
            .flatten()
            .unwrap_or_else(|| panic!("format schema missing: {source}"));
        assert_eq!(formats.len(), 1, "format role missing: {source}");
        assert_eq!(
            words.arguments()[formats[0].index].literal_bytes(),
            Some(template),
            "shared literal bytes: {source}"
        );
        assert_eq!(dsl_diags(source, "tcl8.4").len(), 1, "{source}");
    }
    // naming.source.original-static-escape-values
    // docs/design/analysis/name-resolution-proofs/original-source-static-escape-values.md
    // SourceEscapes173 records pre-8.6 greedy hex as '[' for this exact
    // spelling. It therefore contains no percent conversion to version-gate.
    for dialect in ["tcl8.4", "tcl8.5"] {
        assert!(
            dsl_diags(r"format \x25b 5", dialect).is_empty(),
            "{dialect}"
        );
    }
    for source in [
        r"format {\x25b} 5",
        r#"format "%b$part" 5"#,
        "format $template 5",
        "format [makeTemplate] 5",
        "binary {*}$selector cu 5",
        "binary $selector cu 5",
        "proc format args {}; format %b 5",
        "proc binary args {}; binary format cu 5",
        "rename format {}; format %b 5",
        "namespace eval n {proc binary args {}; binary format cu 5}",
        "clock format 0 -format {%b}",
    ] {
        assert!(dsl_diags(source, "tcl8.4").is_empty(), "{source}");
    }
    for dialect in ["tcl8.5", "tcl8.6"] {
        assert!(
            dsl_diags("format {*}$arguments %llu 5", dialect).is_empty(),
            "{dialect}: unknown expansion supplies no template ordinal"
        );
    }
}

#[test]
fn w202_gated_field_letter_names_the_letter_and_the_floor() {
    let diags = dsl_diags("binary format q 1.0\n", "tcl8.4");
    let (code, msg) = diags
        .iter()
        .find(|(c, _)| c == "W202")
        .expect("q is bad field specifier on tclsh 8.4.20");
    assert_eq!(code, "W202");
    assert!(msg.contains("'q'"), "{msg}");
    assert!(msg.contains("8.5"), "{msg}");
    // Clean once the floor is met.
    assert_eq!(
        dsl_diags("binary format q 1.0\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

/// The argument-DSL gates run against the command a head *is*, not the
/// word it is spelled with (#2065).
///
/// Both reproductions shadow a built-in from inside a `namespace eval`
/// body, where C Tcl resolves the bare head in that namespace before the
/// global table — tclsh 8.6.18 / 9.0.4, byte-identical:
///
/// ```tcl
/// namespace eval n {proc format {args} {return F}; puts [format %b 5]}
/// namespace eval m {proc binary {args} {return B}
///                   puts [binary format q 1.0]}      ;# F, then B
/// ```
#[test]
fn a_shadowed_head_is_not_gated_as_the_builtin() {
    for (src, code) in [
        (
            "namespace eval n { proc format {args} {}\nformat %b 5 }\n",
            "W138",
        ),
        (
            "namespace eval n { proc binary {args} {}\nbinary format q 1.0 }\n",
            "W202",
        ),
        // The document-level shadow reads the same way.
        ("proc format {args} {}\nformat %b 5\n", "W138"),
        // As does a name the document renamed the built-in away from.
        ("rename format origfmt\nformat %b 5\n", "W138"),
    ] {
        assert_eq!(
            dsl_diags(src, "tcl8.4").len(),
            0,
            "{code} must not gate a head the document rebound: {src}"
        );
    }
    // The shadow is not a licence to stop gating everywhere: a call
    // outside the namespace still runs the built-in, and the rename's
    // target still *is* the built-in.
    assert!(
        dsl_diags(
            "namespace eval n { proc format {args} {} }\nformat %b 5\n",
            "tcl8.4"
        )
        .iter()
        .any(|(c, _)| c == "W138"),
        "a namespace-local shadow must not silence the global call"
    );
    assert!(
        dsl_diags("rename format origfmt\norigfmt %b 5\n", "tcl8.4")
            .iter()
            .any(|(c, m)| c == "W138" && m.contains("origfmt")),
        "the gate follows a proven rename onto its new name"
    );
}

#[test]
fn w138_format_binary_conversion_is_86_gated() {
    // Oracle-verified surface: `%b` was added in Tcl 8.6.
    let src = "format %b 5\n";
    // TP: 8.4/8.5-era runtimes (incl. iRules' embedded 8.4.6).
    for d in ["tcl8.4", "tcl8.5", "f5-irules", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
            "{d}: %b needs 8.6, got {diags:?}"
        );
    }
    // TN: 8.6+ runtimes.
    for d in ["tcl8.6", "tcl9.0", "expect", "bpf", "synopsys-eda-tcl"] {
        assert_eq!(dsl_diags(src, d).len(), 0, "{d}: %b is real on 8.6+");
    }
    // FP-guard: `%%b` is a literal percent + `b`, not the conversion;
    // a dynamic format string abstains.
    assert_eq!(
        dsl_diags("format %%b 5\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("format $fmt 5\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    // The permissive fallback abstains entirely (§8).
    assert_eq!(
        dsl_diags(src, "tcl"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w138_format_unsigned_bignum_is_90_gated() {
    // Oracle-verified: tclsh8.6 raises "unsigned bignum format is
    // invalid" for %llu; tclsh9.0.4 renders it.
    let src = "format %llu 5\n";
    let diags = dsl_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(c, m)| c == "W138" && m.contains("%llu")),
        "tcl8.6: %llu needs 9.0, got {diags:?}"
    );
    assert_eq!(dsl_diags(src, "tcl9.0").len(), 0, "9.0 renders %llu");
    // Plain %lld is fine everywhere the ladder models.
    assert_eq!(
        dsl_diags("format %lld 5\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w138_scan_binary_conversion_is_86_gated() {
    let src = "scan 101 %b x\n";
    let diags = dsl_diags(src, "tcl8.5");
    assert!(
        diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
        "tcl8.5: scan %b needs 8.6, got {diags:?}"
    );
    assert_eq!(
        dsl_diags(src, "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w137_string_is_class_follows_the_effective_version() {
    // `string is dict` — oracle-verified 9.0-only (tclsh8.6: bad
    // class; tclsh9.0: works).
    let src = "string is dict {a 1}\n";
    for d in ["tcl8.4", "tcl8.6", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags
                .iter()
                .any(|(c, m)| c == "W137" && m.contains("'dict'")),
            "{d}: string is dict needs 9.0, got {diags:?}"
        );
    }
    for d in ["tcl9.0", "tcl9.1", "bpf"] {
        assert_eq!(dsl_diags(src, d).len(), 0, "{d}: dict class is real");
    }
    // entier is 8.6+; wideinteger is 8.5+.
    assert!(
        dsl_diags("string is entier 5\n", "tcl8.5")
            .iter()
            .any(|(c, _)| c == "W137"),
        "entier needs 8.6"
    );
    assert_eq!(
        dsl_diags("string is entier 5\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert!(
        dsl_diags("string is wideinteger 5\n", "tcl8.4")
            .iter()
            .any(|(c, _)| c == "W137"),
        "wideinteger needs 8.5"
    );
    assert_eq!(
        dsl_diags("string is wideinteger 5\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
    // FP-guards: an always-available class, a dynamic class, and the
    // unique-prefix abbreviation of an ungated class stay silent.
    assert_eq!(
        dsl_diags("string is alpha abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("string is $cls abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("string is xd abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn dsl_gates_honour_a_package_require_tcl_floor() {
    // §6.1: `package require Tcl 9.0` raises the effective version
    // above the ambient tcl8.6 dialect — the file validates as 9.0.
    let src = "package require Tcl 9.0\nformat %llu 5\nstring is dict {a 1}\n";
    assert_eq!(
        dsl_diags(src, "tcl8.6").len(),
        0,
        "a 9.0 core floor admits 9.0 DSL features"
    );
}

#[test]
fn w200_binary_modifiers_follow_the_effective_version() {
    // TIP 275: binary format/scan u/s modifiers are 8.5+. The F5
    // trunk profiles all ride the fork of Tcl at 8.4.6 — measured,
    // bigip-irule-parser-measurements.md §4a: iApps/tmsh fail every
    // 8.5 discriminator, so the modifiers flag there too.
    let src = "binary format cu 5\n";
    for d in ["tcl8.4", "f5-irules", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags.iter().any(|(c, _)| c == "W200"),
            "{d}: binary u modifier needs 8.5, got {diags:?}"
        );
    }
    for d in ["tcl8.5", "tcl8.6"] {
        assert_eq!(
            dsl_diags(src, d).len(),
            0,
            "{d}: binary u modifier is real on 8.5+"
        );
    }
}

#[test]
fn baseline_floor_declares_the_f5_surface_15_0_plus() {
    // F5 specs with no explicit introduction inherit the declared
    // 15.0 baseline. TN at the 16.1 default and any 15.0+ pin…
    let src = "when HTTP_REQUEST {\n  pool p\n  HTTP::uri\n}\n";
    for pin in [None, Some("15.0.0"), Some("17.1.0")] {
        let mut a = Analyser::new();
        a.library_versions.bigip_version = pin.map(str::to_owned);
        let w135: Vec<String> = a
            .analyse(src, "f5-irules")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W135")
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(w135.len(), 0, "pin {pin:?}: baseline is met, got {w135:?}");
    }
    // …TP below the baseline: the whole modelled surface is declared
    // 15.0+, so a 14.x target flags it.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("14.1.0".to_owned());
    let diags = a.analyse(src, "f5-irules").diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "W135" && d.message.contains("15.0.0")),
        "a pre-baseline pin flags the declared floor: {:?}",
        diags
            .iter()
            .filter(|d| d.code.as_str() == "W135")
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
    // Explicit data still wins over the baseline: HTTP2::header is
    // 16.1.0-introduced, so a 15.1 pin flags it while `pool` is fine.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("15.1.0".to_owned());
    let diags = a
        .analyse(
            "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
            "f5-irules",
        )
        .diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "W135" && d.message.contains("16.1.0")),
        "explicit introduction data outranks the baseline"
    );
}

#[test]
fn event_version_clause_follows_the_declared_range() {
    // Known event at the default target: clean (baseline 15.0 ≤ 16.1).
    let mut a = Analyser::new();
    let diags = a
        .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
        .diagnostics;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("target release")),
        "events at the default target stay clean"
    );
    // Pinned below the baseline: the event's declared range excludes it.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("14.1.0".to_owned());
    let diags = a
        .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
        .diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("15.0.0")),
        "a pre-baseline target flags the event's declared range: {:?}",
        diags.iter().map(|d| d.code.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn ambient_f5_surface_never_draws_missing_require() {
    // HTTP2::header carries `required_package: f5-irules-cmds`, but the
    // profile ships that surface ambiently (§7.1) — no W120, and the
    // command stays resolved (no W123/W002 either).
    let mut a = Analyser::new();
    let result = a.analyse(
        "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
        "f5-irules",
    );
    let noisy: Vec<&str> = result
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .filter(|c| matches!(*c, "W120" | "W123" | "W002"))
        .collect();
    assert_eq!(
        noisy.len(),
        0,
        "ambient vendor surface must not draw require/unknown codes: {noisy:?}"
    );
}

#[test]
fn ungated_option_is_silent() {
    // `-width` carries no `min_version`, so no version diagnostic.
    let src = "package require Tk 8.6\nentry .e -width 10\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn require_without_version_is_permissive_for_unpinned_packages() {
    // No version floor to compare against ⇒ nothing flagged. This is
    // the contract for a package the profile does NOT pin — under the
    // permissive fallback profile (`"tcl"`, no library pins) an
    // unversioned require yields no floor. On a pinned host the
    // shipped version floors it instead (§7.1 — see
    // `tracksbase_tk_pin_floors_an_unversioned_require`).
    let src = "package require Tk\nentry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(src, "tcl", None).len(),
        0,
        "{:?}",
        version_diags_for(src, "tcl", None)
    );
}

#[test]
fn no_require_draws_no_version_diagnostic() {
    // Missing `package require` is W120's job, not a version diagnostic.
    let src = "entry .e -placeholder hi\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn command_below_floor_fires_w135() {
    // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
    // 8.4 (TracksBase, §7.1) and a `require Tk 8.4` cannot raise it — so
    // the floor is 8.4 either way and W135 fires.
    //
    // The floors are equal, so which one the message *names* is the
    // reporting tie-break, and it names the require: of the ways a floor
    // can arise, the line in this file is the one whose author is reading
    // the diagnostic.  The tie-break is explicit — see `FloorSource` — and
    // the file's own require leads it.
    let src = "package require Tk 8.4\nttk::button .b\n";
    let diags = version_diags_for(src, "tcl8.4", None);
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W135" && m.contains("`package require` guarantees only 8.4")),
        "{diags:?}"
    );
    // On a tcl8.6 host the same source is FINE: `package require Tk
    // 8.4` states a minimum, it does not downgrade the shipped Tk 8.6 —
    // the old require-only floor drew a false positive here.
    assert_eq!(
        version_diags_for(src, "tcl8.6", None).len(),
        0,
        "the shipped Tk 8.6 satisfies an 8.5 introduction"
    );
}

#[test]
fn command_met_by_floor_is_silent() {
    let src = "package require Tk 8.6\nttk::button .b\n";
    assert!(!fires(src, "W135"), "{:?}", version_diags(src));
}

#[test]
fn conditional_probe_does_not_raise_floor() {
    // An optional `catch {package require Tk 8.7}` does not guarantee 8.7;
    // the guaranteed floor is the unconditional 8.6, so W136 still fires.
    let src = "package require Tk 8.6\n\
                   catch {package require Tk 8.7}\n\
                   entry .e -placeholder hi\n";
    assert!(fires(src, "W136"), "{:?}", version_diags(src));
}

/// Every version-gate code for `source` under `dialect`, including the
/// retirement (W139) and deprecation (W144) rungs the narrower helpers
/// above filter out.
fn lifecycle_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139" | "W144"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

#[test]
fn command_level_argument_value_follows_the_core_floor() {
    // `close chan ?read|write?` — the half-close direction word is TIP
    // 332, Tcl 8.6.  It sits at a command-level `arg_values` position
    // that is deliberately *not* a closed value set, so W137 never sees
    // it; the command-level version gate is what reports it.
    let src = "close $s read\n";
    for dialect in ["tcl8.4", "tcl8.5"] {
        let diags = lifecycle_diags(src, dialect);
        assert!(
            diags.iter().any(|(code, message)| code == "W135"
                && message.contains("Argument value 'read' on 'close'")
                && message.contains("requires Tcl 8.6")),
            "{dialect}: half-close needs 8.6, got {diags:?}"
        );
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert_eq!(
            lifecycle_diags(src, dialect).len(),
            0,
            "{dialect}: half-close is real from 8.6"
        );
    }
    // FP-guards: the plain one-argument form has no direction word at
    // all, and a dynamic direction is not a literal value.
    assert_eq!(
        lifecycle_diags("close $s\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        lifecycle_diags("close $s $dir\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn sub_subcommand_follows_the_core_floor() {
    // `info object properties` / `info class definitionnamespace` are the
    // 9.0 TclOO introspection additions (TIP 558 / TIP 524).
    for src in [
        "info object properties $o\n",
        "info class definitionnamespace $c\n",
    ] {
        let diags = lifecycle_diags(src, "tcl8.6");
        assert!(
            diags
                .iter()
                .any(|(code, message)| code == "W135" && message.contains("requires Tcl 9.0")),
            "8.6 must reject {src:?}: {diags:?}"
        );
        assert_eq!(
            lifecycle_diags(src, "tcl9.0").len(),
            0,
            "9.0 must admit {src:?}"
        );
    }
    // The 8.6 operations of the same ensembles stay silent, and a unique
    // prefix of a gated operation is gated exactly as the full word is.
    assert_eq!(
        lifecycle_diags("info object class $o\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert!(
        lifecycle_diags("info class definitionn $c\n", "tcl8.6")
            .iter()
            .any(|(code, _)| code == "W135"),
        "an abbreviation resolves to the same gated operation"
    );
}

#[test]
fn deprecated_subcommand_reports_once_on_the_axis_that_still_has_it() {
    // `trace variable` is deprecated from 8.4 and removed in 9.0.
    let src = "trace variable v w handler\n";
    let diags = lifecycle_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(code, message)| code == "W144"
            && message.contains("Subcommand 'variable' on 'trace'")
            && message.contains("deprecated as of Tcl 8.4")),
        "8.6 still has the legacy form, deprecated: {diags:?}"
    );
    // On 9.0 the form is gone from the profile entirely — that is W002's
    // word, and the version gate must not flag it a second time.
    assert_eq!(
        lifecycle_diags(src, "tcl9.0").len(),
        0,
        "a dialect-absent subcommand stays W002's alone"
    );
}

#[test]
fn a_require_range_that_reaches_a_retirement_is_hedged() {
    // The floor (8.6) still has `trace variable`, but the range admits
    // every version up to 9.1 — including the 9.0 that removed it.
    let src = "package require Tcl 8.5-9.1\ntrace variable v w handler\n";
    let diags = lifecycle_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(code, message)| code == "W139"
            && message.contains("not available in every version satisfying requirement `8.5-9.1`")
            && message.contains("removed in Tcl 9.0")),
        "a straddling range is hedged, not asserted: {diags:?}"
    );
    // A range that stops below the retirement keeps the ordinary
    // floor verdict …
    let inside = "package require Tcl 8.5-8.7\ntrace variable v w handler\n";
    assert!(
        lifecycle_diags(inside, "tcl8.6")
            .iter()
            .all(|(code, _)| code == "W144"),
        "a range inside the window reports only the deprecation"
    );
    // … and so does an open-ended or bare requirement, which states no
    // ceiling at all.
    for req in ["8.5-", "8.5"] {
        let src = format!("package require Tcl {req}\ntrace variable v w handler\n");
        assert!(
            lifecycle_diags(&src, "tcl8.6")
                .iter()
                .all(|(code, _)| code == "W144"),
            "`{req}` states no ceiling"
        );
    }
    // A conditional probe guarantees nothing, so it cannot widen the
    // window either.
    let probed = "package require Tcl 8.5\n\
                      catch {package require Tcl 8.5-9.1}\n\
                      trace variable v w handler\n";
    assert!(
        lifecycle_diags(probed, "tcl8.6")
            .iter()
            .all(|(code, _)| code == "W144"),
        "a conditional require is not a guarantee"
    );
}

/// A synthetic package whose two options conflict **only from 2.0**.
///
/// Nothing shipped declares a lifecycle on an `option_conflict` yet — a
/// `.tclspec` pack is the first thing that will (`option_conflict {-a -b}
/// -introduced 2.0` is loader vocabulary 1.1) — so the gate is exercised
/// through an overlay registry, which is exactly the route a workspace's
/// packs reach the analyser by.
mod gated_option_conflict {
    use super::super::super::super::state::Analyser;

    /// Overlay key for the synthetic pack below. Any non-zero value works;
    /// the cache keys `(profile, overlay)` on it.
    const OVERLAY: u64 = 0x7e57_c0f1;

    const OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-alpha",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-beta",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];

    const CONSTRAINTS: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
        terms: &[
            tcl_registry::OptionTerm::Option("-alpha"),
            tcl_registry::OptionTerm::Option("-beta"),
        ],
        lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("2.0"),
        ..tcl_registry::OptionRelation::DEFAULT
    }];

    /// The same relationship with no lifecycle at all — the zero-change
    /// control for the inline path.
    const UNGATED_CONSTRAINTS: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
        terms: &[
            tcl_registry::OptionTerm::Option("-alpha"),
            tcl_registry::OptionTerm::Option("-beta"),
        ],
        ..tcl_registry::OptionRelation::DEFAULT
    }];

    fn spec(
        name: &'static str,
        constraints: &'static [tcl_registry::OptionRelation],
    ) -> tcl_registry::CommandSpec {
        tcl_registry::CommandSpec {
            name,
            required_package: Some("Fauxpkg"),
            arity: tcl_registry::Arity::any(),
            options: OPTIONS,
            option_relations: constraints,
            ..tcl_registry::CommandSpec::DEFAULT
        }
    }

    /// Every W147 message `source` draws under `tcl8.6`, with the
    /// synthetic pack installed.
    fn conflicts(source: &str) -> Vec<String> {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let _registry =
            tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |registry| {
                registry.insert(spec("fauxgated", CONSTRAINTS));
                registry.insert(spec("fauxplain", UNGATED_CONSTRAINTS));
            });
        Analyser::new()
            .with_pack_overlay(OVERLAY)
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W147")
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn a_conflict_introduced_later_is_silent_below_its_floor() {
        // The relationship does not exist in Fauxpkg 1.x, so enforcing it
        // would report a rule that release has not got.
        let src = "package require Fauxpkg 1.0\nfauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 0, "{:?}", conflicts(src));
    }

    #[test]
    fn a_conflict_introduced_later_fires_once_the_floor_reaches_it() {
        let src = "package require Fauxpkg 2.0\nfauxgated -alpha -beta\n";
        let diags = conflicts(src);
        assert!(
            diags
                .iter()
                .any(|m| m.contains("-alpha, -beta") && m.contains("fauxgated")),
            "{diags:?}"
        );
    }

    #[test]
    fn no_resolvable_floor_stays_permissive() {
        // An unversioned require on an unpinned package resolves no floor
        // at all, and "no version known ⇒ do not gate" is the standing
        // registry rule — so the conflict is reported.
        let src = "package require Fauxpkg\nfauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 1, "{:?}", conflicts(src));
    }

    #[test]
    fn an_unversioned_constraint_is_untouched_by_the_gate() {
        // The zero-behaviour-change control: a constraint with no
        // lifecycle keeps the inline path, floor or no floor.
        for src in [
            "package require Fauxpkg 1.0\nfauxplain -alpha -beta\n",
            "package require Fauxpkg 2.0\nfauxplain -alpha -beta\n",
            "package require Fauxpkg\nfauxplain -alpha -beta\n",
        ] {
            assert_eq!(conflicts(src).len(), 1, "{src:?}: {:?}", conflicts(src));
        }
    }

    #[test]
    fn a_retired_conflict_stops_being_enforced_but_a_deprecated_one_does_not() {
        // Retirement is exclusive: the relationship is gone at 3.0…
        const RETIRED: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
            terms: &[
                tcl_registry::OptionTerm::Option("-alpha"),
                tcl_registry::OptionTerm::Option("-beta"),
            ],
            lifecycle: tcl_registry::lifecycle::Lifecycle::UNSPECIFIED
                .retired_from("3.0")
                .deprecated_from("2.0"),
            ..tcl_registry::OptionRelation::DEFAULT
        }];
        const KEY: u64 = 0x7e57_c0f2;
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let _registry = tcl_registry::registry_for_profile_with_overlay(profile, KEY, |registry| {
            registry.insert(spec("fauxretired", RETIRED));
        });
        let count = |source: &str| {
            Analyser::new()
                .with_pack_overlay(KEY)
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .filter(|d| d.code.as_str() == "W147")
                .count()
        };
        assert_eq!(
            count("package require Fauxpkg 3.0\nfauxretired -alpha -beta\n"),
            0,
            "a retired relationship is not enforced"
        );
        // … but a *deprecated* one still exists, and deprecation is not
        // absence.
        assert_eq!(
            count("package require Fauxpkg 2.0\nfauxretired -alpha -beta\n"),
            1,
            "a deprecated relationship still holds"
        );
    }

    #[test]
    fn a_shadowing_user_proc_still_suppresses_a_gated_conflict() {
        // The gate decides whether the relationship exists; it must not
        // bypass the `pending_arity` shadowing suppression an ungated
        // conflict goes through.
        let src = "package require Fauxpkg 2.0\n\
                       proc fauxgated args {}\n\
                       fauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 0, "{:?}", conflicts(src));
    }

    #[test]
    fn a_require_inside_the_file_gates_a_conflict_in_a_proc_body() {
        // The floor is a whole-file fact and the `package require` may
        // follow the call: buffering is what makes this work.
        let below = "proc use {} { fauxgated -alpha -beta }\n\
                         package require Fauxpkg 1.0\n";
        assert_eq!(conflicts(below).len(), 0, "{:?}", conflicts(below));
        let met = "proc use {} { fauxgated -alpha -beta }\n\
                       package require Fauxpkg 2.0\n";
        assert_eq!(conflicts(met).len(), 1, "{:?}", conflicts(met));
    }
}

// §5.4 range targeting, W150 / W151.

mod range_targeting {
    use super::super::super::super::state::Analyser;

    /// Every range / version-gate / numeral code for `source` under
    /// `dialect`, on a fresh analyser.
    fn diags(source: &str, dialect: &str) -> Vec<(String, String)> {
        diags_with(Analyser::new(), source, dialect)
    }

    fn diags_with(mut analyser: Analyser, source: &str, dialect: &str) -> Vec<(String, String)> {
        analyser
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.code.as_str(),
                    "W002"
                        | "W135"
                        | "W136"
                        | "W137"
                        | "W138"
                        | "W139"
                        | "W144"
                        | "W148"
                        | "W150"
                        | "W151"
                )
            })
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn fires(diags: &[(String, String)], code: &str) -> bool {
        diags.iter().any(|(c, _)| c == code)
    }

    /// The owner's canonical case, both ways: a leading-zero literal
    /// under a range spanning 8.x and 9.x draws the meaning-change
    /// warning; a single-target 9.0 project — and an undeclared one —
    /// stays clean.
    #[test]
    fn the_octal_case_both_ways() {
        let spanning = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {010}\n";
        let diags = diags(spanning, "tcl8.6");
        assert!(
            diags.iter().any(|(c, m)| c == "W151"
                && m.contains("means 8")
                && m.contains("10")
                && m.contains("silent meaning change")),
            "8.5-9.0 spans the octal/decimal boundary: {diags:?}"
        );

        let single = "# tcl-lsp: supports tcl 9.0\nexpr {010}\n";
        assert!(
            !fires(&super::range_targeting::diags(single, "tcl9.0"), "W151"),
            "a single 9.0 target has one numeral grammar"
        );
        let undeclared = "expr {010}\n";
        for dialect in ["tcl8.6", "tcl9.0", "tcl"] {
            let diags = super::range_targeting::diags(undeclared, dialect);
            assert!(
                !fires(&diags, "W151") && !fires(&diags, "W150"),
                "{dialect}: no declaration, no range mode: {diags:?}"
            );
        }
    }

    /// Prefix and separator numerals draw the validity half of W151
    /// when a declared target predates them — and only then.
    #[test]
    fn prefix_numerals_follow_the_declared_range() {
        // `0o17` predates nothing in 8.5-9.0…
        let fine = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {0o17}\n";
        assert!(
            !fires(&diags(fine, "tcl8.6"), "W151"),
            "0o is 8.5+ and the range starts at 8.5"
        );
        // …but an 8.4 target rejects it.
        let with_84 = "# tcl-lsp: supports tcl 8.4-9.0\nexpr {0o17}\n";
        let d = diags(with_84, "tcl8.6");
        assert!(
            d.iter()
                .any(|(c, m)| c == "W151" && m.contains("not accepted") && m.contains("8.4")),
            "0o17 is a bareword under 8.4: {d:?}"
        );
        // 9.x-only spellings under a range reaching back to 8.x — the
        // primary (9.0) accepts them, the 8.x targets do not.
        for word in ["0d5", "1_000"] {
            let source = format!("# tcl-lsp: supports tcl 8.5-9.0\nset x {word}\n");
            let d = diags(&source, "tcl9.0");
            assert!(
                d.iter()
                    .any(|(c, m)| c == "W151" && m.contains("not accepted")),
                "{word} is 9.0-only: {d:?}"
            );
            // Under an 8.6 primary the same word is W148's
            // single-target fact, never double-reported as W151.
            let d = diags(&source, "tcl8.6");
            assert!(
                fires(&d, "W148") && !fires(&d, "W151"),
                "{word}: primary rejection stays W148's: {d:?}"
            );
        }
    }

    /// A command introduced at 8.6 under an 8.5–9.0 range: fine at the
    /// primary, missing at the declared 8.5 target — the availability
    /// half of the range family (mask-gated, no lifecycle involved).
    #[test]
    fn a_command_introduced_at_86_fails_the_85_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nlmap x {1 2} {set x}\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("'lmap' requires Tcl 8.6")
                && m.contains("include 8.5")),
            "lmap is 8.6+: {d:?}"
        );
        // A declared single target equal to the primary changes nothing.
        let pinned = "# tcl-lsp: supports tcl 8.6\nlmap x {1 2} {set x}\n";
        assert!(!fires(&diags(pinned, "tcl8.6"), "W150"));
    }

    /// The removal direction: `case` is gone from 9.0, so a range that
    /// includes 9.0 flags it even though the 8.6 primary still has it.
    #[test]
    fn a_command_removed_at_9_fails_the_90_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\ncase $x in {a {set y 1}}\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter()
                .any(|(c, m)| c == "W150" && m.contains("'case'") && m.contains("missing at 9.0")),
            "case does not exist at the 9.0 target: {d:?}"
        );
    }

    /// The assistance/semantic split: an item that fails at the
    /// **primary** keeps today's semantic-floor diagnostic (W002 here)
    /// and never draws the assistance range warning on top.
    #[test]
    fn a_primary_failure_stays_the_floor_diagnostic() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nlmap x {1 2} {set x}\n";
        let d = diags(source, "tcl8.5");
        assert!(
            !fires(&d, "W150"),
            "absent at the primary is W002's, not a range fact: {d:?}"
        );
        assert!(
            fires(&d, "W002"),
            "the semantic floor verdict stands: {d:?}"
        );
    }

    /// A library range (`supports Tk 8.5-8.6`) gates package items
    /// through the same placement/floor machinery: the require
    /// satisfies the floor, the declared 8.5–8.6 window does not reach
    /// the option's 8.7 introduction.
    #[test]
    fn a_package_item_under_a_library_range() {
        let source = "# tcl-lsp: supports Tk 8.5-8.6\n\
                          package require Tk 8.7\n\
                          entry .e -placeholder hi\n";
        let d = diags(source, "tcl8.6");
        assert!(
            !fires(&d, "W136"),
            "the 8.7 require satisfies the floor: {d:?}"
        );
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("'-placeholder'")
                && m.contains("requires Tk 8.7")
                && m.contains("8.5")),
            "the declared Tk range misses the 8.7 introduction: {d:?}"
        );
        // Without the declaration the same source is silent — the
        // library range is what changes the verdict.
        let undeclared = "package require Tk 8.7\nentry .e -placeholder hi\n";
        assert!(!fires(&diags(undeclared, "tcl8.6"), "W150"));
    }

    /// Settings-declared targets (`tclLsp.targets`) resolve exactly as
    /// the directive does, and the directive wins per provider.
    #[test]
    fn settings_and_directive_ingress() {
        let configured = || {
            Analyser::new().with_declared_targets(vec![("tcl".to_owned(), "8.5-9.0".to_owned())])
        };
        let source = "lmap x {1 2} {set x}\n";
        let d = diags_with(configured(), source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150" && m.contains("'lmap'")),
            "settings alone switch range mode on: {d:?}"
        );
        // A source directive narrows the same provider back to the
        // primary's own line — the more specific source wins.
        let overridden = "# tcl-lsp: supports tcl 8.6\nlmap x {1 2} {set x}\n";
        assert!(
            !fires(&diags_with(configured(), overridden, "tcl8.6"), "W150"),
            "the directive overrides the configured pair"
        );
        // A malformed declaration is dropped, never guessed at.
        let malformed = "# tcl-lsp: supports tcl not-a-version\nlmap x {1 2} {set x}\n";
        assert!(!fires(&diags(malformed, "tcl8.6"), "W150"));
    }

    /// The argument-DSL rung follows the range too: a conversion fine
    /// at the primary but newer than a declared target warns.
    #[test]
    fn a_dsl_feature_fails_an_older_declared_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nformat %b 5\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("%b")
                && m.contains("requires Tcl 8.6")
                && m.contains("8.5")),
            "%b is 8.6+ and 8.5 is declared: {d:?}"
        );
        assert!(!fires(&d, "W138"), "the primary itself is satisfied");
    }

    /// The no-range pin: a declared single target equal to the
    /// primary's own line leaves the diagnostic list byte-identical to
    /// an undeclared document.
    #[test]
    fn a_single_target_matching_the_primary_changes_nothing() {
        let body = "lmap x {1 2} {set x}\n\
                        expr {010}\n\
                        format %b 5\n\
                        entry .e -placeholder hi\n\
                        trace variable v w handler\n";
        let declared = format!("# tcl-lsp: supports tcl 8.6\n{body}");
        let base: Vec<String> = Analyser::new()
            .analyse(body, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code.as_str(), d.message))
            .collect();
        let pinned: Vec<String> = Analyser::new()
            .analyse(&declared, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code.as_str(), d.message))
            .collect();
        assert_eq!(base, pinned, "single target ⇒ today's behaviour");
    }

    // The Tk pilot rides the same range machinery.
    //
    // §5.4's "packages take range targets exactly like cores" (§3.2,
    // last bullet), proved on the acceptance case. `Tk` is a package
    // axis, so `supports Tk …` is the library half of ruling R6's
    // directive and nothing about it is Tcl-core-specific.

    /// A project declaring `Tk 8.5-8.6` gets warned about an item Tk
    /// only grew at 8.6 — a representative case. `tk busy`
    /// is `Lifecycle::introduced_in("8.6")` on the `tk` ensemble.
    #[test]
    fn a_tk_range_warns_on_an_item_the_older_tk_lacks() {
        let src = "# tcl-lsp: supports Tk 8.5-8.6\n\
                       package require Tk\n\
                       tk busy hold .\n";
        let d = diags(src, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("Tk 8.6")
                && m.contains("declared targets include")
                && m.contains("8.5")),
            "an 8.6-only Tk subcommand under Tk 8.5-8.6: {d:?}"
        );
    }

    /// The three controls that keep the Tk range honest: a range that
    /// starts at 8.6 is clean, a `tcl` declaration does not gate the
    /// `Tk` axis (invariant I2 — values from different axes are never
    /// compared), and an undeclared document is untouched.
    #[test]
    fn a_tk_range_is_scoped_to_the_tk_axis() {
        let body = "package require Tk\ntk busy hold .\n";
        let covered = format!("# tcl-lsp: supports Tk 8.6-9.0\n{body}");
        assert!(
            !fires(&diags(&covered, "tcl8.6"), "W150"),
            "Tk 8.6-9.0 covers an 8.6 introduction"
        );
        // A *core* range naming the same numerals says nothing about
        // the Tk axis.
        let core_only = format!("# tcl-lsp: supports tcl 8.5-8.6\n{body}");
        let d = diags(&core_only, "tcl8.6");
        assert!(
            !d.iter().any(|(c, m)| c == "W150" && m.contains("Tk 8.6")),
            "the core axis must not gate the Tk axis: {d:?}"
        );
        assert!(!fires(&diags(body, "tcl8.6"), "W150"), "undeclared");
    }

    // The tcllib adversarial modules.
    //
    // A tcllib module is an independently versioned package with its
    // own axis, so `supports struct::tree …` gates on the *module's*
    // train, not on tcllib-as-a-blob and not on the Tcl core. The
    // acceptance item is `::struct::tree::prune`: `tree1.tcl` (the
    // 1.2.3 train) has no such proc, `tree_tcl.tcl` (2.1.3) defines
    // it as `return -code 5`, so it carries
    // `introduced: "2.0"` on the `struct::tree` axis.

    /// A project declaring `struct::tree 1.2-2.2` — both trains — is
    /// told that `prune` does not exist across the whole range.
    #[test]
    fn a_tcllib_module_range_warns_on_an_item_the_older_release_lacks() {
        let src = "# tcl-lsp: supports struct::tree 1.2-2.2\n\
                       package require struct::tree\n\
                       ::struct::tree::prune\n";
        let d = diags(src, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("struct::tree 2.0")
                && m.contains("declared targets include")
                && m.contains("1.2")),
            "a 2.0-only command under struct::tree 1.2-2.2: {d:?}"
        );
        // The declaration alone is enough — no `package require`
        // needed, because a target set is a project fact, not a
        // document one.
        let bare = "# tcl-lsp: supports struct::tree 1.2-2.2\n::struct::tree::prune\n";
        assert!(fires(&diags(bare, "tcl8.6"), "W150"));
    }

    /// The module range lives on **that module's** axis (invariant
    /// I2), which a tcllib module can prove in a way `Tk` could not:
    /// a *different tcllib module's* range with overlapping numerals
    /// must say nothing about this one.
    #[test]
    fn a_tcllib_module_range_is_scoped_to_that_modules_axis() {
        let body = "package require struct::tree\n::struct::tree::prune\n";
        // Covered: a range that starts inside the 2.x train is clean.
        let covered = format!("# tcl-lsp: supports struct::tree 2.1-\n{body}");
        assert!(
            !fires(&diags(&covered, "tcl8.6"), "W150"),
            "2.1- covers 2.0"
        );
        // Another *module* with the same numerals says nothing here.
        let sibling = format!("# tcl-lsp: supports struct::list 1.2-2.2\n{body}");
        assert!(
            !fires(&diags(&sibling, "tcl8.6"), "W150"),
            "struct::list's axis must not gate struct::tree's items",
        );
        // Neither does the core axis.
        let core = format!("# tcl-lsp: supports tcl 8.5-9.0\n{body}");
        assert!(
            !fires(&diags(&core, "tcl8.6"), "W150"),
            "the core axis must not gate a tcllib module axis (I2)",
        );
        assert!(!fires(&diags(body, "tcl8.6"), "W150"), "undeclared");
    }

    /// The single-floor seed of the same family still works, and a
    /// module is **hosted**: the floor comes from the document's own
    /// `package require`, never from a placement pin.
    #[test]
    fn a_tcllib_module_floor_comes_from_the_package_require() {
        let old = "package require struct::tree 1.2\n::struct::tree::prune\n";
        let d = diags(old, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W135"
                && m.contains("struct::tree 2.0")
                && m.contains("guarantees only 1.2")),
            "the 1.2 require floors below the 2.0 introduction: {d:?}"
        );
        let new = "package require struct::tree 2.1\n::struct::tree::prune\n";
        assert!(!fires(&diags(new, "tcl8.6"), "W135"), "2.1 satisfies it");
        // No require at all is permissive on the version axis — W120
        // owns the missing-require nag for a hosted package.
        let none = "::struct::tree::prune\n";
        assert!(!fires(&diags(none, "tcl8.6"), "W135"));
    }

    // The jim ladder.
    //
    // Jim's releases are targets on the `jim` **core** axis, not nine
    // catalogue profiles, so a jim range is declared exactly as a Tcl
    // one is and the §5.4 machinery answers on the family's own
    // ladder. Invariant I2 is the load-bearing half: a jim
    // declaration must never reach the Tcl axis, and a Tcl one must
    // never reach jim's.

    /// The declared targets of a `jim` document land on the jim core
    /// axis, spelled off the jim ladder — and nowhere else.
    #[test]
    fn a_declared_jim_range_lands_on_the_jim_core_axis() {
        use tcl_dialect::model::{Family, Version, VersionAxisId};
        let mut analyser = Analyser::new();
        let _ = analyser.analyse("# tcl-lsp: supports jim 0.76-0.79\nset x 1\n", "jim");
        let context = analyser
            .range_context
            .as_ref()
            .expect("a jim declaration switches range mode on");
        let declared = context
            .declared_targets(&VersionAxisId::core(Family::Jim))
            .expect("targets on the jim axis");
        assert!(declared.contains(&Version::parse("0.78").expect("version")));
        assert!(!declared.contains(&Version::parse("0.84").expect("version")));
        // I2, both spellings of the leak: not the Tcl core axis, and
        // not a fictitious `package:jim` axis either, which is what an
        // ingress recognising only the name `tcl` as a family would mint.
        assert!(
            context
                .declared_targets(&VersionAxisId::core(Family::Tcl))
                .is_none(),
            "a jim range is not a Tcl range"
        );
        assert!(
            context
                .declared_targets(&VersionAxisId::package("jim"))
                .is_none(),
            "jim is a family, not a package"
        );
    }

    /// A family range declared against the wrong core is **dropped**,
    /// not coerced — in both directions. A `supports jim` line in a
    /// Tcl document and a `supports tcl` line in a jim document are
    /// each a statement about a ladder the document is not on.
    #[test]
    fn a_family_range_off_its_own_core_is_dropped() {
        use tcl_dialect::model::{Family, VersionAxisId};
        let mut in_tcl = Analyser::new();
        let _ = in_tcl.analyse("# tcl-lsp: supports jim 0.81-0.84\nset x 1\n", "tcl8.6");
        assert!(
            in_tcl.range_context.is_none(),
            "a jim range says nothing about a Tcl document"
        );

        let mut in_jim = Analyser::new();
        let _ = in_jim.analyse("# tcl-lsp: supports tcl 8.5-9.0\nset x 1\n", "jim");
        assert!(
            in_jim.range_context.is_none(),
            "a Tcl range says nothing about a jim document"
        );

        // …while the *package* axes still cross freely, because a
        // package is not a ladder: `supports Tk 8.5-8.6` is a real
        // declaration under either core.
        let mut package = Analyser::new();
        let _ = package.analyse("# tcl-lsp: supports Tk 8.5-8.6\nset x 1\n", "jim");
        assert!(
            package
                .range_context
                .as_ref()
                .expect("range mode")
                .declared_targets(&VersionAxisId::package("Tk"))
                .is_some()
        );
        assert!(
            package
                .range_context
                .as_ref()
                .expect("range mode")
                .declared_targets(&VersionAxisId::core(Family::Jim))
                .is_none()
        );
    }

    /// The W151 cross-target numeral check is a **Tcl-ladder** fact,
    /// so a jim range never switches it on — the octal case under
    /// `supports jim 0.76-0.84` stays silent where the same span of
    /// the Tcl ladder would warn.
    #[test]
    fn a_jim_range_does_not_drive_the_tcl_numeral_check() {
        let jim = "# tcl-lsp: supports jim 0.76-0.84\nexpr {010}\n";
        assert!(!fires(&diags(jim, "jim"), "W151"));
        // The control: the same shape on the Tcl axis does warn.
        let tcl = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {010}\n";
        assert!(fires(&diags(tcl, "tcl8.6"), "W151"));
    }

    /// Under `jim` the shared core surface resolves through the
    /// ancestry edge: no W002 for `lassign`, `lmap` or `dict`, which
    /// on the jim branch were three of 76 hand-re-authored specs.
    #[test]
    fn the_jim_environment_resolves_the_inherited_core_surface() {
        let source = "set x 1\nlassign {a b} p q\nlmap i {1 2} {set i}\ndict get {a 1} a\n";
        let d = diags(source, "jim");
        assert!(
            !d.iter().any(|(code, _)| code == "W002"),
            "inherited core commands must resolve: {d:?}"
        );
    }

    /// Under `wish` the same declaration works with **no** `package
    /// require`: the `tk` environment ships Tk ambient, so the range
    /// is declared against a package the environment already placed.
    #[test]
    fn a_tk_range_applies_under_the_ambient_wish_placement() {
        let src = "# tcl-lsp: supports Tk 8.5-8.6\ntk busy hold .\n";
        let d = diags(src, "tk");
        assert!(
            d.iter().any(|(c, m)| c == "W150" && m.contains("Tk 8.6")),
            "wish + Tk 8.5-8.6: {d:?}"
        );
        assert!(
            !d.iter().any(|(c, _)| c == "W120"),
            "and no missing-require nag under an ambient placement: {d:?}"
        );
    }
}
