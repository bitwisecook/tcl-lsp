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
use clap::CommandFactory as _;

#[test]
fn cli_definition_is_well_formed() {
    Cli::command().debug_assert();
}

fn test_pair(reference: Engine, subject: Engine) -> ResolvedPair {
    ResolvedPair {
        reference,
        subject,
        tcl_version: None,
        reference_bin: PathBuf::from("/reference"),
        reference_args: reference.args(),
        subject_bin: PathBuf::from("/subject"),
        subject_args: subject.args(),
    }
}

fn pair_args(reference: Engine, subject: Engine, tcl_version: Option<TclVersion>) -> PairArgs {
    PairArgs {
        reference,
        subject,
        tcl_version,
    }
}

fn finding(seed: u64, tcl_version: TclVersion) -> Finding {
    let outcome = Outcome::Ran {
        stdout: "reference output\n".to_owned(),
        stderr: String::new(),
        errored: false,
    };
    Finding::new(
        seed,
        findings::Category::StdoutMismatch,
        "puts replayable",
        &findings::FindingContext {
            reference: &outcome,
            subject: &outcome,
            reference_engine: Engine::RuntimeRust.label(),
            subject_engine: Engine::Tclvm.label(),
            versions: &version::PairVersions {
                tcl_version: Some(tcl_version),
                reference: Some(version::EngineVersion::parse(tcl_version.patchlevel())),
                subject: Some(version::EngineVersion::parse(tcl_version.patchlevel())),
            },
            generator_rates: findings::GeneratorRates::of(&generator::GenConfig::default()),
        },
    )
}

fn replay_tmp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "tcl-fuzz-replay-test-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn replay_refuses_an_unreadable_existing_record() {
    let base = replay_tmp("unreadable");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 37;
    let dir = pair_findings_dir(&base, args.reference, args.subject, None);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("finding-{seed}.json")), b"{ truncated").unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("loading finding 37"));
    assert!(error.contains("finding-37.json"));
}

#[test]
fn replay_refuses_orphan_and_staged_record_companions() {
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 38;
    for (name, write, is_json) in [
        ("tcl-orphan", "finding-38.tcl", false),
        ("json-orphan", "finding-38.json", true),
        ("staged", "finding-38.tcl.stage", false),
    ] {
        let base = replay_tmp(name);
        let dir = pair_findings_dir(&base, args.reference, args.subject, None);
        std::fs::create_dir_all(&dir).unwrap();
        let contents = if is_json {
            serde_json::to_vec_pretty(&finding(seed, TclVersion::V8_6)).unwrap()
        } else {
            b"puts interrupted".to_vec()
        };
        std::fs::write(dir.join(write), contents).unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(
            error.contains("incomplete or interrupted paired record"),
            "{name}: {error}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn release_flag_accepts_owner_defined_aliases_and_rejects_unknown_values() {
    assert_eq!(parse_tcl_version("8.6"), Ok(TclVersion::V8_6));
    assert_eq!(parse_tcl_version("8.6.16"), Ok(TclVersion::V8_6));
    assert_eq!(parse_tcl_version("tcl8.6"), Ok(TclVersion::V8_6));
    assert!(parse_tcl_version("8.7").is_err());
    // A vendor fork has a measured release of its own, but is not a release
    // a campaign can name.
    assert!(parse_tcl_version("f5-irules").is_err());

    let modern = Cli::try_parse_from(["tcl-fuzz", "run", "--tcl-version", "8.6"])
        .expect("canonical release flag parses");
    let legacy = Cli::try_parse_from(["tcl-fuzz", "run", "--subject-tcl-version", "tcl8.6"])
        .expect("subject-only spelling remains a compatibility alias");
    let Cmd::Run(modern) = modern.command else {
        panic!("expected run command");
    };
    let Cmd::Run(legacy) = legacy.command else {
        panic!("expected run command");
    };
    assert_eq!(modern.pair.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(legacy.pair.tcl_version, Some(TclVersion::V8_6));
    assert!(Cli::try_parse_from(["tcl-fuzz", "run", "--tcl-version", "8.7"]).is_err());
}

#[test]
fn replay_discovers_a_recorded_release_and_propagates_it_to_both_engines() {
    let base = replay_tmp("propagation");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 19;
    let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
    Registry::open(&dir)
        .unwrap()
        .record(&finding(seed, TclVersion::V8_6))
        .unwrap();

    let selection = select_replay(&base, args, seed).unwrap();
    assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(
        selection.finding.as_ref().map(|finding| finding.seed),
        Some(seed)
    );
    let mut resolved = test_pair(args.reference, args.subject);
    resolved.apply_tcl_version(selection.tcl_version.unwrap());
    assert!(
        resolved
            .reference_args
            .windows(2)
            .any(|args| args == ["--tcl-version", "8.6"])
    );
    assert!(
        resolved
            .subject_args
            .windows(2)
            .any(|args| args == ["--tcl-version", "8.6"])
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn replay_accepts_a_markerless_pinned_finding_from_the_release_namespace() {
    let base = replay_tmp("markerless-pinned");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 22;
    let mut pinned = finding(seed, TclVersion::V8_6);
    pinned.replay_metadata_version = None;
    let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
    Registry::open(&dir).unwrap().record(&pinned).unwrap();

    let selection = select_replay(&base, args, seed).unwrap();
    assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(selection.finding.map(|finding| finding.seed), Some(seed));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn replay_refuses_legacy_findings_without_release_metadata() {
    let base = replay_tmp("legacy");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 23;
    let mut legacy = finding(seed, TclVersion::V9_0);
    legacy.tcl_version = None;
    legacy.replay_metadata_version = None;
    let dir = pair_findings_dir(&base, args.reference, args.subject, None);
    Registry::open(&dir).unwrap().record(&legacy).unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("predates replay-safe campaign metadata"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn replay_immediately_reuses_a_new_unpinned_campaign_finding() {
    let base = replay_tmp("new-unpinned");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 231;
    let mut unpinned = finding(seed, TclVersion::V9_0);
    unpinned.tcl_version = None;
    assert_eq!(unpinned.replay_metadata_version, Some(1));
    let dir = pair_findings_dir(&base, args.reference, args.subject, None);
    Registry::open(&dir).unwrap().record(&unpinned).unwrap();

    let selection = select_replay(&base, args, seed).unwrap();
    assert_eq!(selection.tcl_version, None);
    assert_eq!(selection.finding.map(|finding| finding.seed), Some(seed));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn marked_unpinned_and_pinned_records_are_ambiguous_without_a_release() {
    let base = replay_tmp("marked-unpinned-ambiguous");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 232;
    let mut unpinned = finding(seed, TclVersion::V9_0);
    unpinned.tcl_version = None;
    Registry::open(pair_findings_dir(&base, args.reference, args.subject, None))
        .unwrap()
        .record(&unpinned)
        .unwrap();
    Registry::open(pair_findings_dir(
        &base,
        args.reference,
        args.subject,
        Some(TclVersion::V8_6),
    ))
    .unwrap()
    .record(&finding(seed, TclVersion::V8_6))
    .unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("multiple Tcl configurations"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_release_selects_pinned_record_over_marked_unpinned_record() {
    let base = replay_tmp("marked-unpinned-explicit");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V8_6));
    let seed = 233;
    let mut unpinned = finding(seed, TclVersion::V9_0);
    unpinned.tcl_version = None;
    Registry::open(pair_findings_dir(&base, args.reference, args.subject, None))
        .unwrap()
        .record(&unpinned)
        .unwrap();
    Registry::open(pair_findings_dir(
        &base,
        args.reference,
        args.subject,
        Some(TclVersion::V8_6),
    ))
    .unwrap()
    .record(&finding(seed, TclVersion::V8_6))
    .unwrap();

    let selection = select_replay(&base, args, seed).unwrap();
    assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(
        selection
            .finding
            .and_then(|finding| finding.campaign_tcl_version().ok().flatten()),
        Some(TclVersion::V8_6),
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_replay_refuses_a_legacy_release_less_finding() {
    let base = replay_tmp("explicit-legacy");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
    let seed = 24;
    let mut legacy = finding(seed, TclVersion::V9_0);
    legacy.tcl_version = None;
    legacy.replay_metadata_version = None;
    let dir = pair_findings_dir(&base, args.reference, args.subject, None);
    Registry::open(&dir).unwrap().record(&legacy).unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("predates replay-safe campaign metadata"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_replay_refuses_a_finding_from_a_different_release() {
    let base = replay_tmp("explicit-mismatch");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
    let seed = 25;
    let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
    Registry::open(&dir)
        .unwrap()
        .record(&finding(seed, TclVersion::V8_6))
        .unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("records Tcl 8.6"));
    assert!(error.contains("requested Tcl 9.0"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_replay_selects_its_release_when_the_seed_exists_at_multiple_releases() {
    let base = replay_tmp("explicit-duplicate");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
    let seed = 27;
    for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
        Registry::open(&dir)
            .unwrap()
            .record(&finding(seed, tcl_version))
            .unwrap();
    }

    let selection = select_replay(&base, args, seed).unwrap();
    assert_eq!(selection.tcl_version, Some(TclVersion::V9_0));
    assert_eq!(
        selection
            .finding
            .as_ref()
            .and_then(|finding| finding.campaign_tcl_version().ok().flatten()),
        Some(TclVersion::V9_0),
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_replay_refuses_metadata_mismatched_to_its_release_namespace() {
    let base = replay_tmp("explicit-metadata-mismatch");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
    let seed = 28;
    let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V9_0));
    let mut mismatched = finding(seed, TclVersion::V8_6);
    mismatched.replay_metadata_version = None;
    Registry::open(&dir).unwrap().record(&mismatched).unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("stored under Tcl 9.0"));
    assert!(error.contains("records Tcl 8.6"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn replay_refuses_an_invalid_pinned_release_even_without_the_marker() {
    let base = replay_tmp("invalid-markerless-pinned");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 281;
    let mut invalid = finding(seed, TclVersion::V8_6);
    invalid.tcl_version = Some("8.7".to_owned());
    invalid.replay_metadata_version = None;
    let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
    Registry::open(&dir).unwrap().record(&invalid).unwrap();

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("unrecognised recorded Tcl release"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_replay_allows_an_arbitrary_seed_absent_from_every_registry() {
    let base = replay_tmp("arbitrary-seed");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));

    let selection = select_replay(&base, args, 26).unwrap();
    assert_eq!(selection.tcl_version, Some(TclVersion::V9_0));
    assert!(selection.finding.is_none());
    assert!(
        !base.exists(),
        "an arbitrary replay must not create registries"
    );
}

#[test]
fn replay_refuses_a_seed_which_exists_at_multiple_releases() {
    let base = replay_tmp("ambiguous");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let seed = 29;
    for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
        Registry::open(&dir)
            .unwrap()
            .record(&finding(seed, tcl_version))
            .unwrap();
    }

    let error = select_replay(&base, args, seed).unwrap_err();
    assert!(error.contains("multiple Tcl configurations"));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn default_summary_discovers_each_release_namespace_without_seed_collisions() {
    let base = replay_tmp("summary-all");
    let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    let shared_seed = 30;
    for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
        Registry::open(&dir)
            .unwrap()
            .record(&finding(shared_seed, tcl_version))
            .unwrap();
    }
    let mut unpinned = finding(31, TclVersion::V9_0);
    unpinned.tcl_version = None;
    let legacy_dir = pair_findings_dir(&base, args.reference, args.subject, None);
    Registry::open(&legacy_dir)
        .unwrap()
        .record(&unpinned)
        .unwrap();

    let selected = summary_registry_dirs(&base, args);
    assert_eq!(
        selected
            .iter()
            .map(|(release, _)| *release)
            .collect::<Vec<_>>(),
        vec![None, Some(TclVersion::V8_6), Some(TclVersion::V9_0)],
    );
    assert_eq!(
        selected
            .iter()
            .map(|(_, dir)| Registry::open(dir)
                .unwrap()
                .summary()
                .unwrap()
                .values()
                .sum::<usize>())
            .sum::<usize>(),
        3,
        "same seed is kept as two findings because the release namespace is part of identity",
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explicit_summary_selects_only_its_release_namespace() {
    let base = replay_tmp("summary-explicit");
    let unpinned = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
    for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
        let dir = pair_findings_dir(
            &base,
            unpinned.reference,
            unpinned.subject,
            Some(tcl_version),
        );
        Registry::open(&dir)
            .unwrap()
            .record(&finding(32, tcl_version))
            .unwrap();
    }
    let selected = summary_registry_dirs(
        &base,
        pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0)),
    );
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].0, Some(TclVersion::V9_0));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn default_pair_keeps_the_plain_findings_dir() {
    // The default `tclvm` subject / `tclsh` reference pair must keep
    // writing to `<findings>/` so existing registries need no migration
    // when other pairs are added.
    let base = Path::new("fuzz-findings");
    assert_eq!(
        pair_findings_dir(base, Engine::Tclsh, Engine::Tclvm, None),
        base.to_owned()
    );
}

#[test]
fn other_pairs_are_namespaced_so_seeds_never_collide() {
    let base = Path::new("fuzz-findings");
    assert_eq!(
        pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust, None),
        base.join("runtime-rust-vs-tclsh")
    );
    assert_eq!(
        pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclvm, None),
        base.join("tclvm-vs-runtime-rust")
    );
    // The same two engines swapped is a *different* registry — subject and
    // reference are not interchangeable.
    assert_ne!(
        pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust, None),
        pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclsh, None)
    );
}

#[test]
fn release_namespaces_keep_same_pair_seeds_separate() {
    let base = Path::new("fuzz-findings");
    let old = pair_findings_dir(
        base,
        Engine::RuntimeRust,
        Engine::Tclvm,
        Some(TclVersion::V8_6),
    );
    let new = pair_findings_dir(
        base,
        Engine::RuntimeRust,
        Engine::Tclvm,
        Some(TclVersion::V9_0),
    );
    assert_eq!(old, base.join("tclvm-vs-runtime-rust/tcl8.6"));
    assert_eq!(new, base.join("tclvm-vs-runtime-rust/tcl9.0"));
    assert_ne!(old, new, "a seed must not deduplicate across releases");
}

#[test]
fn every_engine_names_the_flag_that_overrides_it() {
    for engine in [Engine::Tclvm, Engine::Tclsh, Engine::RuntimeRust] {
        assert_eq!(cli_flag_for(engine), engine.label());
    }
}

#[test]
fn a_campaign_pin_reaches_both_sides_of_an_emulated_pair() {
    let mut pair = test_pair(Engine::RuntimeRust, Engine::Tclvm);
    pair.apply_tcl_version(TclVersion::V8_6);
    assert_eq!(pair.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(
        pair.reference_args,
        vec![
            "--quiet".to_owned(),
            "--tcl-version".to_owned(),
            "8.6".to_owned(),
        ],
        "reference must receive the same release pin",
    );
    assert_eq!(
        pair.subject_args,
        vec!["--tcl-version".to_owned(), "8.6".to_owned()],
        "subject must receive the same release pin",
    );
}

#[test]
fn a_fixed_release_engine_refuses_when_its_selected_binary_cannot_honour_the_pin() {
    let mut pair = test_pair(Engine::Tclsh, Engine::Tclvm);
    pair.apply_tcl_version(TclVersion::V8_6);
    assert_eq!(pair.tcl_version, Some(TclVersion::V8_6));
    assert_eq!(pair.reference_args, [] as [String; 0]);
    assert_eq!(
        pair.subject_args,
        vec!["--tcl-version".to_owned(), "8.6".to_owned()]
    );
    let versions = version::PairVersions {
        tcl_version: Some(TclVersion::V8_6),
        reference: Some(version::EngineVersion::parse("9.0.4")),
        subject: Some(version::EngineVersion::parse("8.6.16")),
    };
    let error = verify_requested_tcl_version(&versions).unwrap_err();
    assert!(error.contains("reference engine reports Tcl 9.0.4"));
}

#[test]
fn identical_backend_pairs_are_rejected() {
    let cli = Cli::parse_from(["tcl-fuzz", "run"]);
    let pair = PairArgs {
        reference: Engine::Tclvm,
        subject: Engine::Tclvm,
        tcl_version: None,
    };
    assert!(ResolvedPair::resolve(pair, &cli).is_err());
}

#[test]
fn an_explicit_override_resolves_that_engine_and_its_args() {
    let cli = Cli::parse_from([
        "tcl-fuzz",
        "--runtime-rust",
        "/opt/run_script",
        "run",
        "--subject",
        "runtime-rust",
    ]);
    let (bin, args) = resolve_engine(Engine::RuntimeRust, &cli).expect("explicit path resolves");
    assert_eq!(bin, PathBuf::from("/opt/run_script"));
    // `runtime/rust`'s runner needs `--quiet` to match `tclsh script.tcl`'s
    // stdout contract — see `engine::Engine::args`.
    assert_eq!(args, vec!["--quiet".to_string()]);
}

#[test]
fn characterise_resolution_keeps_each_backend_invocation_together() {
    let cli = Cli::parse_from([
        "tcl-fuzz",
        "--tclsh",
        "/opt/tclsh9.0",
        "--tclvm",
        "/opt/tclvm",
        "--runtime-rust",
        "/opt/run_script",
        "characterise",
    ]);
    let resolved = CharacteriseInvocations::resolve(&cli).expect("explicit paths resolve");
    assert_eq!(resolved.oracle.binary, PathBuf::from("/opt/tclsh9.0"));
    assert_eq!(resolved.oracle.args, [] as [String; 0]);
    assert_eq!(resolved.tclvm.binary, PathBuf::from("/opt/tclvm"));
    assert_eq!(resolved.tclvm.args, [] as [String; 0]);
    assert_eq!(resolved.runtime.binary, PathBuf::from("/opt/run_script"));
    assert_eq!(resolved.runtime.args, vec!["--quiet".to_string()]);
}

#[test]
fn requested_release_must_be_reported_by_both_backends() {
    let versions = version::PairVersions {
        tcl_version: Some(TclVersion::V8_6),
        reference: Some(version::EngineVersion::parse("8.6.16")),
        subject: Some(version::EngineVersion::parse("8.6.16")),
    };
    assert!(verify_requested_tcl_version(&versions).is_ok());
    let ignored = version::PairVersions {
        subject: Some(version::EngineVersion::parse("9.0.4")),
        ..versions
    };
    assert!(verify_requested_tcl_version(&ignored).is_err());
}

#[test]
fn replay_patch_drift_warning_compares_both_recorded_backends() {
    let versions = version::PairVersions {
        tcl_version: Some(TclVersion::V9_0),
        reference: Some(version::EngineVersion::parse("9.0.1")),
        subject: Some(version::EngineVersion::parse("9.0.4")),
    };
    assert!(!replay_versions_changed(
        Some("9.0.1"),
        Some("9.0.4"),
        &versions,
    ));
    assert!(replay_versions_changed(
        Some("9.0.1"),
        Some("9.0.5"),
        &versions,
    ));
    assert!(replay_versions_changed(None, Some("9.0.4"), &versions));
}

#[test]
fn characterise_finding_count_excludes_agreement_and_incomplete_runs() {
    let counts = std::collections::BTreeMap::from([
        (Classification::AllAgree, 11),
        (Classification::TclVmDiverges, 2),
        (Classification::SharedDivergence, 3),
        (Classification::Incomplete, 7),
    ]);
    assert_eq!(characterise_finding_count(&counts), 5);
}
