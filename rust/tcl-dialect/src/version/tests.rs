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

#[test]
fn package_version_bytes_follow_counted_native_grammar() {
    use super::{compare_versions_bytes_for, validate_version_bytes_for};
    for release in TclVersion::ALL {
        assert_eq!(
            validate_version_bytes_for(b"1+\xff", release),
            release.allows_package_version_suffix()
        );
        assert!(!validate_version_bytes_for(b"1\0junk", release));
        assert!(!validate_version_bytes_for(b"\xff", release));
        if release.allows_package_version_suffix() {
            assert_eq!(
                compare_versions_bytes_for(b"1+\xff", b"1", release),
                std::cmp::Ordering::Equal
            );
            assert_eq!(
                compare_versions_bytes_for(
                    b"9223372036854775808+\xff",
                    b"9223372036854775807",
                    release
                ),
                std::cmp::Ordering::Greater
            );
        }
    }
}

#[test]
fn package_byte_selection_preserves_provider_and_endpoint_identity() {
    use super::{
        RequirementBytesValidationError, select_package_version_bytes_for,
        validate_requirement_bytes_for,
    };
    let providers: &[&[u8]] = &[b"1.2a1+\xff", b"1.2+\xff", b"1.3a1+\xff"];
    assert_eq!(
        select_package_version_bytes_for(
            providers,
            &[b"1-"],
            super::PackagePrefer::Stable,
            TclVersion::V9_0
        ),
        Some(1)
    );
    assert_eq!(
        select_package_version_bytes_for(
            providers,
            &[b"1-"],
            super::PackagePrefer::Latest,
            TclVersion::V9_0
        ),
        Some(2)
    );
    assert_eq!(
        validate_requirement_bytes_for(b"1-\xff", TclVersion::V8_6),
        Err(RequirementBytesValidationError::InvalidVersion(b"\xff"))
    );
    assert_eq!(
        validate_requirement_bytes_for(b"1+\xff-tail", TclVersion::V9_0),
        Ok(())
    );
    assert_eq!(
        validate_requirement_bytes_for(b"1-2+\xff", TclVersion::V9_0),
        Err(RequirementBytesValidationError::InvalidVersion(b"1-2+\xff"))
    );
}

#[test]
fn package_search_and_completion_keep_native_name_bytes() {
    use super::{DirectPackageFileKind, PackageProtocol};

    let files = PackageProtocol::Jim.direct_files_bytes(b".\0ignored", b"p\xff\0suffix", true);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, b"./p\xff.so");
    assert_eq!(files[0].kind, DirectPackageFileKind::Native);
    assert_eq!(files[1].path, b"p\xff.tcl");
    assert_eq!(files[1].kind, DirectPackageFileKind::Script);
    assert_eq!(
        PackageProtocol::Tcl.direct_files_bytes(b".", b"p", true),
        [] as [super::DirectPackageFileBytes; 0]
    );
    assert_eq!(
        PackageProtocol::Tcl
            .ifneeded_completion_error_bytes(b"p\xff", b"1.0", 2)
            .unwrap(),
        b"attempt to provide package p\xff 1.0 failed: bad return code: 2"
    );
    assert!(
        PackageProtocol::Tcl
            .ifneeded_completion_error_bytes(b"p", b"1.0", 1)
            .is_none()
    );
    assert!(
        PackageProtocol::Jim
            .ifneeded_completion_error_bytes(b"p", b"1.0", 2)
            .is_none()
    );
}

#[test]
fn legacy_package_grammar_rejects_modern_versions_and_requirement_lists() {
    use super::{PackagePrefer, TclVersion};
    for word in ["1.0a1", "1.0b2", "1.0+tag"] {
        assert!(
            !super::validate_version_for(word, TclVersion::V8_4),
            "{word}"
        );
    }
    for requirement in ["1.0-", "1.0-2.0", "1.0a1"] {
        assert!(super::validate_requirement_for(requirement, TclVersion::V8_4).is_err());
    }
    let candidates = ["1.0", "2.0b1"];
    assert_eq!(
        super::select_package_version_for(
            &candidates,
            &[],
            PackagePrefer::Latest,
            TclVersion::V8_4
        ),
        Some(0)
    );
    assert_eq!(
        super::select_package_version_for(
            &candidates,
            &[],
            PackagePrefer::Latest,
            TclVersion::V8_5
        ),
        Some(1)
    );
    assert_eq!(
        super::select_package_version_for(
            &candidates,
            &["1.0", "2.0"],
            PackagePrefer::Latest,
            TclVersion::V8_4
        ),
        None
    );
}

use super::{
    RequirementValidationError, StringCharacterModel, TclVersion, Ternary, exact_requirement,
    validate_requirement, validate_version,
};

#[test]
fn lset_appends_at_the_length_from_8_6() {
    for v in [TclVersion::V8_4, TclVersion::V8_5] {
        assert!(!v.lset_appends_at_length(), "{v:?}");
        assert_eq!(TclVersion::lset_appends_at_length_for(Some(v)), Some(false));
    }
    for v in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert!(v.lset_appends_at_length(), "{v:?}");
        assert_eq!(TclVersion::lset_appends_at_length_for(Some(v)), Some(true));
    }
}

#[test]
fn lset_append_boundary_has_no_answer_without_a_release() {
    assert_eq!(TclVersion::lset_appends_at_length_for(None), None);
}

#[test]
fn regex_compile_error_prefix_is_reworded_in_9_0() {
    for v in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert_eq!(
            v.regex_compile_error_prefix(),
            "couldn't compile regular expression pattern: "
        );
    }
    for v in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            v.regex_compile_error_prefix(),
            "cannot compile regular expression pattern: "
        );
    }
}

#[test]
fn package_validation_reuses_the_version_parser() {
    assert!(validate_version("2.3a1"));
    assert!(!validate_version("2.a1"));
    assert!(!validate_version("2.3+platform"));
    assert_eq!(validate_requirement("2.1-3.2"), Ok(()));
    assert_eq!(validate_requirement("2.1-"), Ok(()));
    assert_eq!(
        validate_requirement("2.1+platform"),
        Err(RequirementValidationError::InvalidVersion("2.1+platform"))
    );
    assert_eq!(
        validate_requirement("2.1+platform-3"),
        Err(RequirementValidationError::InvalidVersion("2.1+platform"))
    );
    assert_eq!(
        validate_requirement("2.1-3.2-4.5"),
        Err(RequirementValidationError::InvalidRange("2.1-3.2-4.5"))
    );
    assert_eq!(
        validate_requirement("3.2-x.y"),
        Err(RequirementValidationError::InvalidVersion("x.y"))
    );
    assert!(!super::version_satisfies("2.1", "2.1+platform"));
    assert!(!super::version_satisfies("1.3", "1.2+-1.25"));
}

#[test]
fn package_plus_suffix_policy_matches_tcl_release_lines() {
    for release in [TclVersion::V8_4, TclVersion::V8_6] {
        assert!(!release.allows_package_version_suffix());
        assert!(!super::validate_version_for("1.2+platform", release));
        assert!(super::validate_requirement_for("1.2+platform", release).is_err());
        // 8.4 has no ranges and reports the whole word (tclsh 8.4.20).
        let reported = if release == TclVersion::V8_4 {
            "1-2+platform"
        } else {
            "2+platform"
        };
        assert_eq!(
            super::validate_requirement_for("1-2+platform", release),
            Err(RequirementValidationError::InvalidVersion(reported))
        );
        assert!(!super::version_satisfies_for(
            "1.2",
            "1.2+platform",
            release
        ));
    }
    for release in [TclVersion::V9_0, TclVersion::V9_1] {
        assert!(release.allows_package_version_suffix());
        assert!(super::validate_version_for("1.2+platform", release));
        assert_eq!(
            super::validate_requirement_for("1.2+platform-3", release),
            Ok(())
        );
        assert_eq!(
            super::validate_requirement_for("1-2+platform", release),
            Err(RequirementValidationError::InvalidVersion("1-2+platform"))
        );
        assert!(super::version_satisfies_for("1.2", "1.2+-1.25", release));
        assert!(!super::version_satisfies_for("1.30", "1.2+-1.25", release));
        assert!(super::version_satisfies_for("1.3", "1.2+-2+x-y", release));
        assert!(!super::version_satisfies_for("1.3", "1.2+x-1.2+x", release));
        assert!(!super::version_matches_exact_for("1.3", "1.2+x", release));
        assert!(super::version_matches_exact_for("1.3", "1.3+x", release));
        assert_eq!(
            super::compare_versions_for("1.2+platform", "1.2", release),
            core::cmp::Ordering::Equal
        );
    }
}

#[test]
fn package_provider_selection_uses_the_release_suffix_policy() {
    use super::{PackagePrefer, select_package_version_for};

    let providers = ["1.1", "1.2+platform"];
    assert_eq!(
        select_package_version_for(&providers, &[], PackagePrefer::Latest, TclVersion::V8_6),
        Some(0)
    );
    assert_eq!(
        select_package_version_for(&providers, &[], PackagePrefer::Latest, TclVersion::V9_0),
        Some(1)
    );
    assert_eq!(
        super::select_package_version_exact_for(&["1.2+x", "1.3+x"], "1.2+x", TclVersion::V9_0),
        Some(0)
    );
}

/// The core `Tcl` provide and `[info patchlevel]` are the same build
/// fact, so the literal in [`TclVersion::core_provided_packages`] must
/// stay equal to [`TclVersion::patchlevel`] — except at 8.4, which
/// provides `TCL_VERSION` (`tclsh8.4`: `package provide Tcl` → `8.4`
/// while `info patchlevel` → `8.4.20`).
#[test]
fn core_package_tracks_the_patch_level() {
    for version in TclVersion::ALL {
        let core = version.core_provided_packages()[0];
        assert_eq!(core.name, "Tcl", "{version:?} must provide Tcl first");
        let expected = if version == TclVersion::V8_4 {
            version.version_string()
        } else {
            version.patchlevel()
        };
        assert_eq!(core.version, expected, "{version:?} core provide");
        assert!(
            !core.ifneeded_stub,
            "{version:?}: `package versions Tcl` is empty on every release"
        );
    }
}

/// The lowercase `tcl` and `tcl::oo` spellings are Tcl 9 only, and
/// `TclOO` does not exist before 8.6 — measured on the reference
/// interpreters (8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 / 9.1b0).
#[test]
fn lowercase_core_spellings_are_tcl9_only() {
    let names = |v: TclVersion| -> Vec<&'static str> {
        v.core_provided_packages().iter().map(|p| p.name).collect()
    };
    // TN: 8.4/8.5 pre-provide the core alone.
    assert_eq!(names(TclVersion::V8_4), ["Tcl"]);
    assert_eq!(names(TclVersion::V8_5), ["Tcl"]);
    // TP: 8.6 gains TclOO, but neither lowercase spelling.
    assert_eq!(names(TclVersion::V8_6), ["Tcl", "TclOO"]);
    // TP: 9.x co-provides both lowercase names.
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(names(version), ["Tcl", "tcl", "TclOO", "tcl::oo"]);
    }
}

#[test]
fn from_dialect_maps_every_versioned_tcl() {
    // Tcl9.1 must resolve to a version, not `None` (which
    // silently degrades versioned folds to the dialect-invariant subset).
    assert_eq!(
        TclVersion::from_dialect(Some("tcl8.4")),
        Some(TclVersion::V8_4)
    );
    assert_eq!(
        TclVersion::from_dialect(Some("tcl8.5")),
        Some(TclVersion::V8_5)
    );
    assert_eq!(
        TclVersion::from_dialect(Some("tcl8.6")),
        Some(TclVersion::V8_6)
    );
    assert_eq!(
        TclVersion::from_dialect(Some("tcl9.0")),
        Some(TclVersion::V9_0)
    );
    assert_eq!(
        TclVersion::from_dialect(Some("tcl9.1")),
        Some(TclVersion::V9_1)
    );
    // A vendor fork whose release was measured has its own point.
    assert_eq!(
        TclVersion::from_dialect(Some("f5-irules")),
        Some(TclVersion::V8_4)
    );
    // Unversioned, unknown, or a base nothing measured → None.
    assert_eq!(TclVersion::from_dialect(Some("tcl")), None);
    assert_eq!(TclVersion::from_dialect(Some("expect")), None);
    assert_eq!(TclVersion::from_dialect(None), None);
}

#[test]
fn v9_1_orders_at_or_above_v9_0() {
    // A `>= V9_0` gate must include 9.1.
    assert!(TclVersion::V9_1 >= TclVersion::V9_0);
    assert!(TclVersion::V9_1 > TclVersion::V8_6);
}

#[test]
fn ternary_maps_inert_to_none() {
    assert_eq!(Ternary::Yes.as_bool(), Some(true));
    assert_eq!(Ternary::No.as_bool(), Some(false));
    assert_eq!(Ternary::Inert.as_bool(), None);
}

/// Every row pinned against a live `package vsatisfies [package provide
/// Tcl] REQ` on `tclsh8.6` (8.6.14) and `tclsh9.0` (9.0.4).
#[test]
fn version_satisfies_matches_the_interpreter() {
    use super::version_satisfies;
    // Bare `X.Y` is bounded by the next *major*, not the next minor.
    assert!(version_satisfies("8.6", "8.4"));
    assert!(version_satisfies("8.6", "8.5"));
    assert!(version_satisfies("8.6", "8.6"));
    assert!(!version_satisfies("8.6", "9"));
    assert!(!version_satisfies("9.0", "8.6"));
    assert!(version_satisfies("9.0", "9"));
    assert!(version_satisfies("9.0", "9.0"));
    // Open-ended.
    assert!(version_satisfies("8.6", "8.5-"));
    assert!(version_satisfies("9.0", "8.5-"));
    assert!(!version_satisfies("8.6", "9-"));
    assert!(version_satisfies("9.0", "9-"));
    // Explicit, half-open range.
    assert!(version_satisfies("8.6", "8.5-9.0"));
    assert!(!version_satisfies("9.0", "8.5-9.0"));
    // Patch levels compare component-wise.
    assert!(version_satisfies("8.5.2", "8.5-9.0"));
}

/// The tcllib `pkgIndex.tcl` head guard, both ways round.
#[test]
fn satisfies_any_is_the_multi_requirement_or() {
    // `package vsatisfies [package provide Tcl] 8.5 9` — true on both.
    assert!(TclVersion::V8_6.satisfies_any(&["8.5", "9"]));
    assert!(TclVersion::V9_0.satisfies_any(&["8.5", "9"]));
    // …but 8.4 satisfies neither requirement.
    assert!(!TclVersion::V8_4.satisfies_any(&["8.5", "9"]));
    // An empty requirement list is never satisfied.
    assert!(!TclVersion::V9_0.satisfies_any::<&str>(&[]));
}

/// TP — a requirement the members of a release line genuinely disagree
/// about is undecidable, and only then.
///
/// Oracle (`tclsh9.0`, `[package provide Tcl]` = 9.0.4):
/// `vsatisfies 9.0.4 9.0.1` = 1, `vsatisfies 9.0.4 9.0.1-` = 1,
/// `vsatisfies 9.0.4 9.0.9-` = 0 — two shipped 9.0 releases disagree, so
/// the honest answer for the enum's `9.0` is neither.
#[test]
fn a_patch_level_requirement_the_line_disagrees_about_is_undecidable() {
    for requirement in ["9.0.1", "9.0.1-", "9.0-9.0.2"] {
        assert_eq!(
            TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
            Ternary::Inert,
            "{requirement}"
        );
    }
}

/// TN — a patch-level requirement the line's *major/minor* already settles
/// is decided, not abstained on.
///
/// Oracle (`tclsh8.6`, `[package provide Tcl]` = 8.6.14): `vsatisfies
/// 8.6.14 9.0.1` = 0, `vsatisfies 8.6.14 8.6.0` = 1 — and no 8.6.x
/// disagrees with either, because the bound sits outside the line in one
/// case and below all of it in the other.
#[test]
fn a_patch_level_requirement_outside_the_line_still_decides() {
    for requirement in ["9.0.1", "9.0.1-", "9.0.0-9.0.2", "8.7.0"] {
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
            Ternary::No,
            "{requirement}"
        );
    }
    for requirement in ["8.6.0", "8.6.0-", "8.5.1-", "8.6.0-8.7"] {
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
            Ternary::Yes,
            "{requirement}"
        );
    }
}

/// FP guard — the degenerate `V-V` range `package require -exact V`
/// builds is *not* satisfied by the whole line.
///
/// Oracle: `vsatisfies 8.6 8.6-8.6` = 1 but `vsatisfies 8.6.14 8.6-8.6` =
/// 0 (the exact form is compared unpadded), so which answer a build gives
/// depends on its patch level and the line as a whole abstains — where the
/// old `major.minor`-only comparison answered a flat `Yes`.
#[test]
fn an_exact_requirement_on_the_line_abstains() {
    assert_eq!(
        TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("8.6")]),
        Ternary::Inert,
    );
    // …while an exact requirement naming a *different* line is still a
    // decided `No`.
    assert_eq!(
        TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("9.0")]),
        Ternary::No,
    );
    // TP — an exact requirement naming one patch release of this line is
    // undecidable: that build satisfies it, its siblings do not.
    assert_eq!(
        TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("8.6.14")]),
        Ternary::Inert,
    );
}

/// TN — a malformed requirement is refused rather than abstained on, the
/// same conservative reading [`super::version_satisfies`] takes.
#[test]
fn a_malformed_requirement_is_not_satisfied() {
    for requirement in ["", "x", "1-2-3", " 8.6", "8.6.", "8.6c1"] {
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
            Ternary::No,
            "{requirement}"
        );
    }
}

/// TN — every two-component requirement form still decides, and the OR
/// short-circuits on a satisfied one beside an undecidable one.
#[test]
fn two_component_requirements_still_decide() {
    for requirement in ["8.5", "8.5-", "8.5-9.0", "9-", "9"] {
        assert_ne!(
            TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
            Ternary::Inert,
            "{requirement}"
        );
    }
    assert_eq!(
        TclVersion::V9_0.satisfies_any_ternary(&["9.0.1", "9"]),
        Ternary::Yes,
    );
    // Both requirements name the 9 line, which no 8.6.x reaches — so the
    // OR is a decided `No`, not an abstention.
    assert_eq!(
        TclVersion::V8_6.satisfies_any_ternary(&["9.0.1", "9"]),
        Ternary::No,
    );
    assert_eq!(TclVersion::V8_6.satisfies_any_ternary(&["9"]), Ternary::No);
    // …and an undecidable requirement beside a refused one still carries
    // the whole test to `Inert`.
    assert_eq!(
        TclVersion::V8_6.satisfies_any_ternary(&["8.6.14-", "9"]),
        Ternary::Inert,
    );
}

/// Per-rule TP/FP/TN/FN for the four requirement forms.
///
/// The whole grid lives in the pinned corpus
/// (`tests/data/package_version_oracle.txt`); these are the rows that name
/// the rule they exercise, so a regression reads as "the min-bound rule
/// broke" rather than "142 corpus pairs disagree".
#[test]
fn each_requirement_form_decides_its_own_way() {
    use super::version_satisfies as sat;
    // Min bound within the major: `[1.2, 2)`.
    assert!(sat("1.2", "1.2"), "TP — the bound itself");
    assert!(sat("1.10", "1.2"), "TP — 10 is a number, not a string tail");
    assert!(sat("1.2.3", "1.2"), "TP — a patch release of the bound");
    assert!(!sat("1.1", "1.2"), "TN — below the bound");
    assert!(!sat("2.0", "1.2"), "FP guard — the next major is excluded");
    assert!(!sat("0.9", "1.2"), "TN — a lower major");
    // …and the alpha of the bound *is* accepted (the `a0` pad), which is
    // the rule a naive `>=` comparison gets wrong in the FN direction.
    assert!(sat("1.2a1", "1.2"), "TP — `vsatisfies 1.2a1 1.2` is 1");
    assert!(!sat("1.2a1", "1.2.0"), "TN — but not of a longer bound");

    // Open-ended `min-`: no upper bound at all, so the next major counts.
    assert!(sat("2.0", "1.2-"), "TP — no major cap");
    assert!(sat("1.2a1", "1.2-"), "TP — the alpha pad applies here too");
    assert!(!sat("1.1", "1.2-"), "TN — still bounded below");

    // Half-open range `min-max`.
    assert!(sat("1.9", "1.2-2.0"), "TP — inside");
    assert!(!sat("2.0", "1.2-2.0"), "FP guard — the max is excluded");
    assert!(!sat("1.1", "1.2-2.0"), "TN — below the min");
    assert!(!sat("1.3", "1.2a1-1.3"), "FP guard — max excluded, padded");

    // Degenerate `v-v` — what `-exact` builds.
    assert!(sat("2.0", "2.0-2.0"), "TP");
    assert!(
        sat("2.0.0", "2.0-2.0"),
        "TP — trailing zeros are not a digit"
    );
    assert!(!sat("2.0a1", "2.0-2.0"), "FP guard — no alpha pad on exact");
    assert!(!sat("2.3", "2.0-2.0"), "TN — a later release");

    // Malformed: real Tcl raises, so nothing satisfies.
    for bad in ["1.2-1.3-1.4", "-1.2", "", "a-b", "1.2-x"] {
        assert!(!sat("1.2", bad), "malformed requirement {bad:?}");
    }
    for bad in ["", "1.", ".1", "1..2", "1a", "1a1b2", " 1.2"] {
        assert!(!sat(bad, "1.2"), "malformed version {bad:?}");
    }
}

/// `select_package_version`'s two selection axes: highest
/// acceptable version, and `package prefer`'s stable-first tie-break.
#[test]
fn selection_takes_the_highest_acceptable_preferring_stable() {
    use super::{PackagePrefer, select_package_version as pick};
    let avail = ["1.5", "2.3", "2.0"];
    // TP — unconstrained picks the best, not index 0.
    assert_eq!(pick(&avail, &[], PackagePrefer::Stable), Some(1));
    // TP — a constraint narrows, then the best of what is left wins.
    assert_eq!(pick(&avail, &["2.0"], PackagePrefer::Stable), Some(1));
    assert_eq!(pick(&avail, &["1.2"], PackagePrefer::Stable), Some(0));
    // TP — the requirement list is an OR.
    assert_eq!(
        pick(&avail, &["1.2", "2.0"], PackagePrefer::Stable),
        Some(1)
    );
    // TN — nothing acceptable, and an empty candidate list.
    assert_eq!(pick(&avail, &["3.0"], PackagePrefer::Stable), None);
    assert_eq!(pick::<&str>(&[], &[], PackagePrefer::Stable), None);
    // TN — a candidate whose version does not parse is skipped, not
    // treated as version 0.
    assert_eq!(
        pick(&["not-a-version", "1.0"], &[], PackagePrefer::Stable),
        Some(1)
    );

    // Prefer: the prerelease is the higher version but loses by default.
    let mixed = ["1.2", "1.3b1"];
    assert_eq!(pick(&mixed, &[], PackagePrefer::Stable), Some(0));
    assert_eq!(pick(&mixed, &[], PackagePrefer::Latest), Some(1));
    // FN guard — with nothing stable to prefer, the prerelease still wins
    // under `stable`; abstaining there would resolve nothing at all.
    assert_eq!(pick(&["1.3b1"], &[], PackagePrefer::Stable), Some(0));
    // Ties keep the earlier candidate, so discovery order breaks them.
    assert_eq!(pick(&["1.0", "1.0.0"], &[], PackagePrefer::Stable), Some(0));
}

#[test]
fn version_string_round_trips_through_from_package_version() {
    for v in TclVersion::ALL {
        assert_eq!(
            TclVersion::from_package_version(v.version_string()),
            Some(v)
        );
    }
}

#[test]
fn release_profile_names_cover_every_runtime_choice() {
    for (version, profile) in [
        (TclVersion::V8_4, "tcl8.4"),
        (TclVersion::V8_5, "tcl8.5"),
        (TclVersion::V8_6, "tcl8.6"),
        (TclVersion::V9_0, "tcl9.0"),
        (TclVersion::V9_1, "tcl9.1"),
    ] {
        assert_eq!(version.dialect_profile_name(), profile);
    }
}

#[test]
fn string_character_model_changes_at_tcl_nine() {
    assert_eq!(
        TclVersion::V8_6.string_character_model(),
        StringCharacterModel::Utf16CodeUnits
    );
    assert_eq!(
        TclVersion::V9_0.string_character_model(),
        StringCharacterModel::UnicodeScalars
    );
}

/// A UTF-8 file holding U+1D11E or `é`, sourced after `encoding system
/// iso8859-1`: tclsh 8.6 counts 4 and 2, under utf-8 2 and 1; 8.4 and 8.5
/// count 4 and 1 under utf-8; 9.0 counts 1 and 1 under both. Authored text
/// counts only where the release reads source as UTF-8.
#[test]
fn authored_text_counts_only_where_the_release_reads_source_as_utf8() {
    use super::StringCharacterModel as M;
    for text in ["\u{1D11E}", "\u{e9}"] {
        assert_eq!(M::count_authored(Some(M::UnicodeScalars), text), Some(1));
        for model in [
            None,
            Some(M::Utf16CodeUnits),
            Some(M::BmpCharsElseUtf8Bytes),
        ] {
            assert_eq!(M::count_authored(model, text), None, "{model:?} {text}");
        }
    }
    assert_eq!(M::count_authored(Some(M::Utf16CodeUnits), "abc"), Some(3));
    assert_eq!(M::count_authored(None, "abc"), Some(3));
}

/// Issue #2128: the counting model is three-valued, not two. Each row is
/// a witness measured with
/// `string length [encoding convertfrom utf-8 …]` on the real tclsh of
/// that release, under `LANG=C.UTF-8`.
///
/// The supplementary character is the discriminator; `é` and `€` are here
/// to pin that 8.4/8.5 are *not* byte counting, which is the reading that
/// would otherwise seem to fit the 4.
#[test]
fn string_character_model_is_three_valued_issue_2128() {
    let two_byte = "\u{00E9}"; // é
    let three_byte = "\u{20AC}"; // €
    let supplementary = "\u{1F600}"; // 😀

    for (version, expected_supplementary) in [
        (TclVersion::V8_4, 4),
        (TclVersion::V8_5, 4),
        (TclVersion::V8_6, 2),
        (TclVersion::V9_0, 1),
        (TclVersion::V9_1, 1),
    ] {
        let model = version.string_character_model();
        assert_eq!(
            model.count(supplementary),
            expected_supplementary,
            "{version:?} counts a supplementary character",
        );
        // Every release agrees on everything inside the BMP.
        assert_eq!(model.count(two_byte), 1, "{version:?} counts é");
        assert_eq!(model.count(three_byte), 1, "{version:?} counts €");
    }

    // 8.4/8.5 must be their own variant, not the 8.6 one.
    assert_eq!(
        TclVersion::V8_4.string_character_model(),
        StringCharacterModel::BmpCharsElseUtf8Bytes,
    );
    assert_ne!(
        TclVersion::V8_4.string_character_model(),
        TclVersion::V8_6.string_character_model(),
    );

    // Unanimity is over all three models. A BMP string is still folded
    // without a stated release; a supplementary one is not, where the old
    // 8.6/9.0-pair rule would have answered 2 and been wrong for 8.4.
    assert_eq!(StringCharacterModel::count_for(None, three_byte), Some(1));
    assert_eq!(StringCharacterModel::count_for(None, supplementary), None);
}

/// Issue #2170: the model's units, not just their count. Measured with
/// `scan [string index $s $i] %c` over every index of a literal
/// `a<U+1D11E>b` in a UTF-8 script, `LANG=C.UTF-8`:
///
/// | release | code points |
/// |---|---|
/// | 8.4.20, 8.5.19 | 97 240 157 132 158 98 |
/// | 8.6.18 | 97 55348 56606 98 |
/// | 9.0.4, 9.1.0 | 97 119070 98 |
///
/// 8.6's surrogate halves cannot be held apart, so the pair is one run of
/// two units.
#[test]
fn units_are_the_releases_characters_issue_2170() {
    let runs = |version: TclVersion, value: &str| -> Vec<(u32, usize)> {
        version
            .string_character_model()
            .units(value)
            .map(|run| (u32::from(run.held), run.units))
            .collect()
    };
    let subject = "a\u{1D11E}b";
    for version in [TclVersion::V8_4, TclVersion::V8_5] {
        assert_eq!(
            runs(version, subject),
            [(97, 1), (240, 1), (157, 1), (132, 1), (158, 1), (98, 1)],
            "{version:?}"
        );
    }
    assert_eq!(
        runs(TclVersion::V8_6, subject),
        [(97, 1), (0x1D11E, 2), (98, 1)]
    );
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            runs(version, subject),
            [(97, 1), (0x1D11E, 1), (98, 1)],
            "{version:?}"
        );
    }
    // `count` is the units' sum under every model.
    for model in StringCharacterModel::ALL {
        for value in ["", "abc", "\u{00E9}\u{20AC}", subject, "\u{1F600}\u{1D11E}"] {
            assert_eq!(
                model.count(value),
                model.units(value).map(|run| run.units).sum::<usize>(),
                "{model:?} {value:?}"
            );
        }
    }
}

#[test]
fn package_error_codes_follow_the_measured_release_boundaries() {
    use super::PackageDiagnostic;
    for version in TclVersion::ALL {
        assert_eq!(
            version.package_error_code(PackageDiagnostic::BadResult),
            (version >= TclVersion::V8_6).then_some(&["TCL", "PACKAGE", "BADRESULT"][..]),
            "{version:?}"
        );
        assert_eq!(
            version.package_error_code(PackageDiagnostic::NotPresent),
            (version >= TclVersion::V8_5).then_some(&["TCL", "LOOKUP", "PACKAGE"][..]),
            "{version:?}"
        );
        assert_eq!(
            version.has_package_requirement_ranges(),
            version != TclVersion::V8_4,
            "{version:?}"
        );
    }
    assert_eq!(
        super::validate_requirement_for("1-2", TclVersion::V8_4),
        Err(super::RequirementValidationError::InvalidVersion("1-2"))
    );
    assert_eq!(
        super::validate_requirement_for("1-2", TclVersion::V8_5),
        Ok(())
    );
}

#[test]
fn structured_error_stacks_begin_at_tcl_eight_six() {
    assert!(!TclVersion::V8_4.has_error_stack());
    assert!(!TclVersion::V8_5.has_error_stack());
    assert!(TclVersion::V8_6.has_error_stack());
    assert!(TclVersion::V9_0.has_error_stack());
    assert!(TclVersion::V9_1.has_error_stack());
}
