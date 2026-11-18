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

//! W149: the arity window that covers the resolved floor.

use super::*;

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
        arity_diags(&mut a, "package require Probe\nprobe::grew a\n").len(),
        0,
        "the plain arity is exactly 1"
    );

    let mut a = analyser_with_windows(0x1627_1007, None);
    let diags = arity_diags(&mut a, "package require Probe\nprobe::grew a b\n");
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
