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
#[test]
fn command_offset_index_retains_the_first_complete_matching_segment() {
    let source = "set auto_index(a) [list source a.tcl]\nset auto_index(b) [list source b.tcl]\n";
    let parsed = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        LexerConfig::default(),
    );
    assert_eq!(parsed.len(), 2);
    let first = parsed[0].argv[0].span.start();
    let second = parsed[1].argv[0].span.start();
    let mut duplicate = parsed[0].clone();
    duplicate.texts[1] = "different duplicate".to_owned();
    let mut empty = parsed[0].clone();
    empty.argv.clear();
    let commands = [empty, parsed[1].clone(), parsed[0].clone(), duplicate];
    let index = index_commands_by_offset(&commands);
    assert_eq!(index.len(), 2);
    assert!(index.len() <= commands.len());
    assert!(std::ptr::eq(index[&first], &raw const commands[2]));
    assert!(std::ptr::eq(index[&second], &raw const commands[1]));
    for offset in [first, second, first + 1, u32::MAX] {
        let old = commands.iter().find(|command| {
            command
                .argv
                .first()
                .is_some_and(|word| word.span.start() == offset)
        });
        let new = index.get(&offset).copied();
        match (old, new) {
            (Some(old), Some(new)) => assert!(std::ptr::eq(old, new)),
            (None, None) => {}
            _ => panic!("head-offset join changed: {offset}"),
        }
    }
    let empty: [tcl_compiler::segmenter::SegmentedCommand; 0] = [];
    assert!(index_commands_by_offset(&empty).is_empty());
}

#[test]
fn invalid_filename_layout_matches_full_registration_vectors() {
    let source = "set auto_index(a) [list source -encoding utf-8 [file join $dir a.tcl]]\nset auto_index(b) [list source -encoding utf-8 [file join $dir b.tcl]]\n";
    let directory = Path::new("/library");
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let config = LexerConfig::from_grammar(profile.grammar);
        let policy = NamePolicyProtocol::authored_tcl(version);
        let full = survey_index_entries_with(
            source,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        let generation = crate::context_for_dialect_profile(profile);
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let context = tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext::capture_standalone_structure(
                source, tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile,
                    std::sync::Arc::new(generation.with_command_store(
                        registry.snapshot().shared_registry())), config));
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(plan.commands.len(), 2);
        let first = &plan.commands[0].words[2];
        let second = &plan.commands[1].words[2];
        let query =
            index_loader::survey_original_index_filename(source, &context, first, directory);
        if version == tcl_dialect::TclVersion::V8_4 {
            let index_loader::OriginalIndexFilenameSurvey::InvalidLayout(receipt) = query else {
                panic!("Invalid layout prerequisite")
            };
            assert!(receipt.matches_current_query(first, directory));
            assert!(
                full.is_empty(),
                "the unchanged full consumer contributes zero"
            );
            // In auto_load_index's frame the `[list source -encoding ...]`
            // builder is reached; the authenticated invalid-layout proof
            // exists only for an unreached builder, so the fast path abstains.
            assert!(survey_index_entries_from_source(source, directory, config, policy).is_none());
            assert!(
                survey_index_entries(source, directory, config, policy)
                    .iter()
                    .map(|entry| (&entry.input, &entry.file))
                    .eq(full.iter().map(|entry| (&entry.input, &entry.file)))
            );
        } else {
            let fresh = survey_index_entries_from_source(source, directory, config, policy)
                .expect("a valid layout keeps the fast path");
            assert!(
                fresh
                    .iter()
                    .map(|entry| (&entry.input, &entry.file))
                    .eq(full.iter().map(|entry| (&entry.input, &entry.file))),
                "{version:?}"
            );
            let index_loader::OriginalIndexFilenameSurvey::Filename(file) = query else {
                panic!("Selected encoding filename prerequisite: {version:?}")
            };
            // Both rows register: `$dir` is set in auto_load_index's frame.
            assert_eq!(full.len(), 2, "{version:?}");
            assert_eq!(file, full[0].file, "{version:?}");
            assert_eq!(full[0].input.bytes(), b"a");
            assert_eq!(full[1].input.bytes(), b"b");
        }
        let index_loader::OriginalIndexFilenameSurvey::NoContribution(receipt) =
            index_loader::survey_original_index_filename(source, &context, second, directory)
        else {
            panic!("Independent later schema abstention: {version:?}")
        };
        assert!(receipt.matches_current_query(second, directory));
    }
}

#[test]
fn filename_layout_activation_preserves_mixed_row_order_and_fallback() {
    let version = tcl_dialect::TclVersion::V8_4;
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = NamePolicyProtocol::authored_tcl(version);
    let directory = Path::new("/library");
    let valid = "set auto_index(valid) [list source valid.tcl]\n";
    let invalid = "set auto_index(invalid) [list source -encoding utf-8 invalid.tcl]\n";
    let unknown = "set auto_index(unknown) [list source $missing]\n";
    let records = |entries: &[OriginalAutoIndexRegistration]| {
        entries
            .iter()
            .map(|entry| (entry.input.clone(), entry.file.clone()))
            .collect::<Vec<_>>()
    };
    for source in [format!("{valid}{invalid}"), format!("{invalid}{valid}")] {
        let full = survey_index_entries_with(
            &source,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        assert_eq!(full.len(), 1, "{source}");
        assert_eq!(full[0].input.bytes(), b"valid");
        assert_eq!(full[0].file, directory.join("valid.tcl"));
        assert!(
            survey_index_entries_from_source(&source, directory, config, policy).is_none(),
            "literal invalid rows retain the full fallback: {source}"
        );
        assert_eq!(
            records(&survey_index_entries(&source, directory, config, policy)),
            records(&full),
            "{source}"
        );
        let mixed = format!("{source}{unknown}");
        assert!(
            survey_index_entries_from_source(&mixed, directory, config, policy).is_none(),
            "a later unknown filename must retain full fallback: {mixed}"
        );
        let full = survey_index_entries_with(
            &mixed,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        assert_eq!(full.len(), 1, "{mixed}");
        assert_eq!(full[0].input.bytes(), b"valid");
        assert_eq!(full[0].file, directory.join("valid.tcl"));
        assert_eq!(
            records(&survey_index_entries(&mixed, directory, config, policy)),
            records(&full),
            "{mixed}"
        );
    }
}

#[test]
fn filename_layout_activation_accepts_conditional_mixed_rows() {
    let version = tcl_dialect::TclVersion::V8_4;
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = NamePolicyProtocol::authored_tcl(version);
    let directory = Path::new("/library");
    let valid = "set auto_index(valid) [list source valid.tcl]\n";
    let invalid =
        "set auto_index(invalid) [list source -encoding utf-8 [file join $dir invalid.tcl]]\n";
    let records = |entries: &[OriginalAutoIndexRegistration]| {
        entries
            .iter()
            .map(|entry| (entry.input.clone(), entry.file.clone()))
            .collect::<Vec<_>>()
    };
    // In auto_load_index's frame `$dir` is set, so every row is reached and
    // the invalid row contributes nothing wherever it stands. Its
    // `[list source -encoding ...]` builder is reached, and the
    // authenticated invalid-layout proof exists only for an unreached
    // builder, so the fast path abstains on a source holding it; a source
    // of valid rows keeps the fast path.
    for (source, expected, fast) in [
        (valid.to_owned(), 1, true),
        (format!("{valid}{invalid}"), 1, false),
        (format!("{invalid}{valid}"), 1, false),
    ] {
        let full = survey_index_entries_with(
            &source,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        assert_eq!(full.len(), expected, "{source}");
        if let Some(entry) = full.first() {
            assert_eq!(entry.input.bytes(), b"valid");
            assert_eq!(entry.file, directory.join("valid.tcl"));
        }
        let fresh = survey_index_entries_from_source(&source, directory, config, policy);
        assert_eq!(fresh.is_some(), fast, "{source}");
        if let Some(fresh) = fresh {
            assert_eq!(records(&fresh), records(&full), "{source}");
        }
        assert_eq!(
            records(&survey_index_entries(&source, directory, config, policy)),
            records(&full),
            "{source}"
        );
    }
}

/// `auto_load_index` evaluates the index inside its own procedure, so a row
/// that reads a variable nothing set raises (`can't read "missing": no such
/// variable`) before it registers anything: the fast path answers zero
/// entries. A row whose filename or builder the survey cannot name keeps the
/// unavailable fallback to the full analysis. Either way the current survey
/// agrees with the full registration consumer. Tcl 8.5.
#[test]
fn filename_layout_activation_retains_unavailable_fallbacks() {
    let version = tcl_dialect::TclVersion::V8_5;
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = NamePolicyProtocol::authored_tcl(version);
    let directory = Path::new("/library");
    for (source, fast) in [
        ("set auto_index(a) [list source $missing]", Some(0)),
        (
            "set auto_index(a) [list source $option utf-8 a.tcl]",
            Some(0),
        ),
        (
            "set auto_index(a) [list source -encoding utf-8 $missing]",
            Some(0),
        ),
        ("set auto_index(a) [list source {*}$unknown]", Some(0)),
        ("set auto_index(a) [list source [file dirname $dir]]", None),
        (
            "rename list saved; set auto_index(a) [list source a.tcl]",
            None,
        ),
    ] {
        assert_eq!(
            survey_index_entries_from_source(source, directory, config, policy)
                .map(|entries| entries.len()),
            fast,
            "{source}"
        );
        let full = survey_index_entries_with(
            source,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        let current = survey_index_entries(source, directory, config, policy);
        assert!(
            current
                .iter()
                .map(|entry| (&entry.input, &entry.file))
                .eq(full.iter().map(|entry| (&entry.input, &entry.file))),
            "{source}"
        );
    }
}

#[test]
fn filename_layout_activation_separates_positive_filenames_and_negative_proofs() {
    let directory = Path::new("/library");
    let records = |entries: &[OriginalAutoIndexRegistration]| {
        entries
            .iter()
            .map(|entry| (entry.input.clone(), entry.file.clone()))
            .collect::<Vec<_>>()
    };
    let version = tcl_dialect::TclVersion::V8_5;
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = NamePolicyProtocol::authored_tcl(version);
    for source in [
        "unavailable; set auto_index(a) [list source a.tcl]",
        "interp alias {} make {} list; set auto_index(a) [make source a.tcl]",
    ] {
        let full = survey_index_entries_with(
            source,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        let fresh = survey_index_entries_from_source(source, directory, config, policy)
            .expect("unchanged ordinary positive filename extraction");
        assert_eq!(full.len(), 1, "{source}");
        assert_eq!(full[0].input.bytes(), b"a");
        assert_eq!(full[0].file, directory.join("a.tcl"));
        assert_eq!(records(&fresh), records(&full), "{source}");
        assert_eq!(
            records(&survey_index_entries(source, directory, config, policy)),
            records(&full),
            "{source}"
        );
    }
    let version = tcl_dialect::TclVersion::V8_4;
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = NamePolicyProtocol::authored_tcl(version);
    let source = "unavailable; set auto_index(a) [list source -encoding utf-8 a.tcl]";
    assert!(
        survey_index_entries_from_source(source, directory, config, policy).is_none(),
        "the unknown command withdraws the negative proof"
    );
    let full = survey_index_entries_with(
        source,
        directory,
        config,
        policy,
        IndexAnalysis::Autoload {
            structure_only: true,
        },
    );
    assert!(full.is_empty());
    assert_eq!(
        records(&survey_index_entries(source, directory, config, policy)),
        records(&full)
    );
}

#[test]
fn autoload_multirow_source_abstention_matches_full_registration_consumer() {
    let two_rows = "set auto_index(a) [list source [file join $dir a.tcl]]\nset auto_index(b) [list source [file join $dir b.tcl]]\n";
    let with_tail = format!(
        "{two_rows}set auto_index(c) [list source [file join $dir c.tcl]]\nif {{[namespace exists ::tcl::unsupported]}} {{\n    set auto_index(timerate) {{namespace import ::tcl::unsupported::timerate}}\n}}\n"
    );
    let encoded = with_tail.replace("list source [", "list source -encoding utf-8 [");
    let blt = "set auto_index(a) [list source $dir/graph.tcl]\nset auto_index(b) [list source $dir/graph.tcl]\nset auto_index(c) [list source $dir/graph.tcl]\n";
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
        let policy = NamePolicyProtocol::authored_tcl(version);
        let directory = Path::new("/library");
        let records = |entries: &[OriginalAutoIndexRegistration]| {
            entries
                .iter()
                .map(|entry| (entry.input.clone(), entry.file.clone()))
                .collect::<Vec<_>>()
        };
        for source in [
            two_rows,
            with_tail.as_str(),
            blt,
            "set auto_index(a) [list source a.tcl]\nset auto_index(b) [list source b.tcl]\n",
        ] {
            let fresh = survey_index_entries_from_source(source, directory, config, policy)
                .expect("multirow source prerequisite must not enter the full analyser");
            let full = survey_index_entries_with(
                source,
                directory,
                config,
                policy,
                IndexAnalysis::Autoload {
                    structure_only: true,
                },
            );
            assert_eq!(records(&fresh), records(&full), "{version:?}: {source}");
        }
        let fresh = survey_index_entries_from_source(two_rows, directory, config, policy).unwrap();
        // `auto_load_index` evaluates the index inside a procedure with `dir`
        // set, so both rows register: with this index and `a.tcl` / `b.tcl`
        // defining `a` and `b` in an auto_path directory, tclsh 8.4.20,
        // 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print `A B` for `[a] [b]`.
        assert_eq!(fresh.len(), 2, "both rows register: {version:?}");
        let full = survey_index_entries_with(
            &encoded,
            directory,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        );
        let fresh = survey_index_entries_from_source(&encoded, directory, config, policy);
        if version == tcl_dialect::TclVersion::V8_4 {
            // The reached `[list source -encoding ...]` builder has no
            // authenticated invalid-layout proof, so the fast path abstains.
            assert!(fresh.is_none(), "the fast path abstains: {version:?}");
            assert!(full.is_empty(), "the full filename layout is invalid");
        } else {
            let fresh = fresh.expect("encoded multirow source prerequisite");
            assert_eq!(records(&fresh), records(&full), "{version:?}");
        }
        assert_eq!(
            records(&survey_index_entries(&encoded, directory, config, policy)),
            records(&full)
        );
    }
}

#[test]
fn autoload_source_context_matches_full_registration_consumer() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
        let policy = NamePolicyProtocol::authored_tcl(version);
        let directory = Path::new("/library");
        let positive = "set auto_index(p) [list source [file join $dir p.tcl]]";
        let fresh = survey_index_entries_from_source(positive, directory, config, policy)
            .expect("positive source-only prerequisite");
        assert_eq!(fresh.len(), 1, "{version:?}");
        let records = |entries: &[OriginalAutoIndexRegistration]| {
            entries
                .iter()
                .map(|entry| (entry.input.clone(), entry.file.clone()))
                .collect::<Vec<_>>()
        };
        for source in [
            positive,
            "set auto_index(p) [list source -encoding utf-8 [file join $dir p.tcl]]",
            "set {auto_index(café name)} [list source [file join $dir { café.tcl }]]",
            "proc set args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
            "proc list args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
            "proc file args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
            "rename file saved; set auto_index(p) [list source [file join $dir p.tcl]]",
            "unavailable; set auto_index(p) [list source [file join $dir p.tcl]]",
            "interp alias {} builder {} list source; set auto_index(p) [builder [file join $dir p.tcl]]",
            "set auto_index(p) [list source {*}$unknown]",
            "set auto_index(p) [list source $unknown]",
            "set auto_index(p) [list source [file join $dir p.tcl]",
        ] {
            let old = survey_index_entries_with(
                source,
                directory,
                config,
                policy,
                IndexAnalysis::Autoload {
                    structure_only: true,
                },
            );
            let current = survey_index_entries(source, directory, config, policy);
            assert_eq!(records(&current), records(&old), "{version:?}: {source}");
        }
        for directive in [
            "# tcl-lsp: supports tcl 8.6-9.0\n",
            "# tcl-lsp: package custom provides Tk\n",
        ] {
            let source = format!("{directive}{positive}");
            assert_eq!(source.lines().last(), Some(positive));
            assert_eq!(
                tcl_compiler::analyser::utils::parse_supports_directives(&source).len()
                    + tcl_compiler::analyser::utils::parse_provides_directives(&source).len(),
                1,
                "{version:?}: directive must alter the source context: {source}"
            );
            assert!(
                survey_index_entries_from_source(&source, directory, config, policy).is_none(),
                "{version:?}: recognized directive retains full analysis: {source}"
            );
            let old = survey_index_entries_with(
                &source,
                directory,
                config,
                policy,
                IndexAnalysis::Autoload {
                    structure_only: true,
                },
            );
            assert_eq!(
                records(&survey_index_entries(&source, directory, config, policy)),
                records(&old)
            );
        }
    }
}

#[test]
fn autoload_registration_structure_matches_full_source_records() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
        let policy = NamePolicyProtocol::authored_tcl(version);
        let directory = Path::new("/library");
        for (source, expected) in [
            (
                "set auto_index(p) [list source [file join $dir p.tcl]]",
                Some(1),
            ),
            (
                "proc set args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
                Some(0),
            ),
            (
                "proc list args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
                Some(0),
            ),
            (
                "proc file args {return ignored}; set auto_index(p) [list source [file join $dir p.tcl]]",
                Some(0),
            ),
            ("set auto_index(p) [list source $unknown]", Some(0)),
            ("set auto_index(p) [list source {*}$unknown]", None),
            (
                "set {auto_index(p\0tail)} [list source [file join $dir p.tcl]]",
                None,
            ),
            (
                "set {auto_index( spaced )} [list source [file join $dir { café.tcl }]]",
                None,
            ),
            (
                "interp alias {} builder {} list source; set auto_index(p) [builder [file join $dir p.tcl]]",
                None,
            ),
            (
                "set auto_index(p) [list source [file join $dir p.tcl]",
                None,
            ),
        ] {
            let full = survey_index_entries_with(
                source,
                directory,
                config,
                policy,
                IndexAnalysis::Autoload {
                    structure_only: false,
                },
            );
            let structural = survey_index_entries_with(
                source,
                directory,
                config,
                policy,
                IndexAnalysis::Autoload {
                    structure_only: true,
                },
            );
            if let Some(expected) = expected {
                assert_eq!(full.len(), expected, "{version:?}: {source}");
            }
            let records = |entries: &[OriginalAutoIndexRegistration]| {
                entries
                    .iter()
                    .map(|entry| (entry.input.clone(), entry.file.clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                records(&structural),
                records(&full),
                "{version:?}: {source}"
            );
        }
    }
}

#[test]
fn autoload_structure_retains_current_source_and_input_refusals() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = LexerConfig::for_file_grammar(profile.grammar);
    let source = "set auto_index(p) [list source [file join $dir p.tcl]]";
    let analysis = crate::source_structure::analyse_loader_index(
        source,
        profile,
        registry,
        config,
        Some(tcl_index_entry_frame(Path::new("/library"))),
        true,
    );
    assert!(index_loader::IndexLoader::new(source, &analysis).is_some());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(index_loader::IndexLoader::new(source, &missing).is_none());
    let mut changed = analysis.clone();
    changed.body_lexer_config = Some(LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    });
    assert!(index_loader::IndexLoader::new(source, &changed).is_none());
    assert!(index_loader::IndexLoader::new("set auto_index(q) {}", &analysis).is_none());
}

#[test]
fn original_package_advice_keeps_unowned_reports_and_metadata_distinct() {
    let unknown = PackageRequirementAdviceKey::from_original_name(None);
    assert_eq!(unknown, PackageRequirementAdviceKey::Unknown);
    assert!(!unknown.matches_ascii_metadata("Tk"));
    let metadata = PackageRequirementAdviceKey::authored_metadata("Tk");
    assert!(metadata.matches_ascii_metadata("Tk"));
    assert!(metadata.original().is_none());
    assert_eq!(
        PackageRequirementAdviceKey::authored_metadata("p\u{d7ff}"),
        PackageRequirementAdviceKey::Unknown
    );
    assert_eq!(
        PackageRequirementAdviceKey::authored_metadata("p\0tail"),
        PackageRequirementAdviceKey::Unknown
    );
    let resolver = PackageResolver::new();
    let available = resolver.transitive_package_advice(
        &[unknown, metadata.clone()],
        Some(tcl_dialect::TclVersion::V9_1),
        &|_| panic!("missing providers cannot supply dependency source"),
    );
    assert_eq!(
        available,
        HashSet::from([metadata, PackageRequirementAdviceKey::Unknown])
    );
}

fn original_requirement_key(
    entries: &[OriginalPackageInfo],
    name: &str,
) -> PackageRequirementAdviceKey {
    PackageRequirementAdviceKey::Original(
        entries
            .iter()
            .find(|entry| entry.name().matches_ascii(name))
            .expect("authentic original registration")
            .name()
            .clone(),
    )
}

fn original_requirement_advice(
    entries: &[OriginalPackageInfo],
    name: &str,
    alternatives: &[&str],
    exact: bool,
    prefer: super::super::PackagePrefer,
) -> PackageRequirementAdvice {
    PackageRequirementAdvice::new(
        original_requirement_key(entries, name),
        alternatives
            .iter()
            .map(|version| (*version).to_owned())
            .collect(),
        exact,
        prefer,
    )
}

fn assert_package_selection(
    resolver: &PackageResolver,
    version: tcl_dialect::TclVersion,
    [exact, stable, latest, alternatives]: [&PackageRequirementAdvice; 4],
) {
    assert_eq!(
        resolver.resolve_requirement_advice(exact, Some(version)),
        vec![PathBuf::from("/library/stable.tcl")]
    );
    assert_eq!(
        resolver.resolve_requirement_advice(stable, Some(version)),
        vec![PathBuf::from("/library/stable.tcl")]
    );
    assert_eq!(
        resolver.resolve_requirement_advice(latest, Some(version)),
        vec![PathBuf::from("/library/beta.tcl")]
    );
    assert_eq!(
        resolver.resolve_requirement_advice(alternatives, Some(version)),
        vec![PathBuf::from("/library/major.tcl")]
    );
}

#[test]
fn original_package_requirement_closure_keeps_versions_and_preference() {
    // The shared candidate owner also serves exact_require_selects_that_release_or_nothing
    // and prefer_latest_selects_the_prerelease in package_resolver::tests;
    // those tests retain their concrete C Tcl version-selection controls.
    let index = "package ifneeded w 1.2 {source stable.tcl}\npackage ifneeded w 1.3b1 {source beta.tcl}\npackage ifneeded w 2.0 {source major.tcl}\npackage ifneeded dep 1.0 {source dep-old.tcl}\npackage ifneeded dep 2.0 {source dep-new.tcl}";
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut resolver = PackageResolver::new();
        resolver.add_original_pkg_index(
            index,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
        );
        let entries = original_package_entries(
            index,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
            version,
        );
        let require = |name: &str, alternatives: &[&str], exact, prefer| {
            original_requirement_advice(&entries, name, alternatives, exact, prefer)
        };
        let exact = require("w", &["1.2"], true, super::super::PackagePrefer::Latest);
        let stable = require("w", &["1.0"], false, super::super::PackagePrefer::Stable);
        let latest = require("w", &["1.0"], false, super::super::PackagePrefer::Latest);
        let alternatives = require(
            "w",
            &["0.9", "2.0"],
            false,
            super::super::PackagePrefer::Stable,
        );
        assert_package_selection(
            &resolver,
            version,
            [&exact, &stable, &latest, &alternatives],
        );
        let dependency = require("dep", &["1.0"], true, super::super::PackagePrefer::Stable);
        let available = resolver.transitive_requirement_advice(
            std::slice::from_ref(&latest),
            Some(version),
            &|file| {
                if file == Path::new("/library/beta.tcl") {
                    vec![dependency.clone()]
                } else if file == Path::new("/library/dep-old.tcl") {
                    vec![exact.clone()]
                } else if file == Path::new("/library/stable.tcl") {
                    vec![latest.clone()]
                } else {
                    panic!("constraints must not visit {file:?}")
                }
            },
        );
        assert_eq!(available, HashSet::from([latest, dependency, exact]));
        let foreign_key = tcl_registry::native_package::NativePackageNameKey::from_native_units(
            b"dep",
            NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        );
        let foreign = PackageRequirementAdvice::new(
            PackageRequirementAdviceKey::Original(foreign_key),
            vec!["1.0".to_owned()],
            true,
            super::super::PackagePrefer::Latest,
        );
        let unknown = foreign.clone().unknown();
        let available = resolver.transitive_requirement_advice(
            std::slice::from_ref(&stable),
            Some(version),
            &|file| {
                assert_eq!(file, Path::new("/library/stable.tcl"));
                vec![foreign.clone()]
            },
        );
        assert_eq!(available, HashSet::from([stable, unknown]));

        let root = require("w", &["1.0"], false, super::super::PackagePrefer::Stable);
        let raised = require("dep", &["1.0"], true, super::super::PackagePrefer::Latest);
        let after_raise = require("w", &["1.0"], false, super::super::PackagePrefer::Latest);
        let available = resolver.transitive_requirement_advice_with_context(
            std::slice::from_ref(&root),
            Some(version),
            &|file, parent| {
                if file == Path::new("/library/stable.tcl") {
                    assert_eq!(parent.prefer(), super::super::PackagePrefer::Stable);
                    vec![raised.clone()]
                } else if file == Path::new("/library/dep-old.tcl") {
                    assert_eq!(parent.prefer(), super::super::PackagePrefer::Latest);
                    vec![after_raise.clone()]
                } else {
                    assert_eq!(file, Path::new("/library/beta.tcl"));
                    assert_eq!(parent.prefer(), super::super::PackagePrefer::Latest);
                    Vec::new()
                }
            },
        );
        assert_eq!(available, HashSet::from([root, raised, after_raise]));
    }
}

#[test]
fn original_package_keys_keep_opaque_names_and_nested_source_owners() {
    let source = r"if {1} {package ifneeded p\uD800 1.0 {source a.tcl}}\npackage ifneeded p\uD801 2.0 {source b.tcl}".replace("\\n", "\n");
    for version in tcl_dialect::TclVersion::ALL {
        let entries = original_package_entries(
            &source,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
            version,
        );
        assert_eq!(entries.len(), 2, "{version:?}");
        assert_eq!(entries[0].name.bytes(), b"p\xed\xa0\x80");
        assert_eq!(entries[1].name.bytes(), b"p\xed\xa0\x81");
        assert!(entries[0].name_input().display().is_none());
        assert_eq!(
            entries[0].name_input().source_image(),
            &SourceImage::document(&source)
        );
        assert!(entries[0].name_input().span().start() > 10);
        let mut resolver = PackageResolver::new();
        resolver.add_original_pkg_index(
            &source,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
        );
        assert_eq!(
            resolver.resolve_original_require(
                entries[0].name(),
                &[],
                false,
                super::super::PackagePrefer::Stable
            ),
            vec![PathBuf::from("/library/a.tcl")]
        );
        assert_eq!(
            resolver.resolve_original_require(
                entries[1].name(),
                &[],
                false,
                super::super::PackagePrefer::Stable
            ),
            vec![PathBuf::from("/library/b.tcl")]
        );
    }
}

#[test]
fn batched_index_surveys_match_each_release_and_recheck_existence() {
    let source = "set auto_index(a) [list source [file join $dir a.tcl]]\n\
                      set auto_index(b) [list source -encoding utf-8 [file join $dir b.tcl]]\n\
                      set auto_index(dynamic) [list source $unknown]\n";
    let requests = tcl_dialect::TclVersion::ALL.map(|version| {
        (
            LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
            ),
            NamePolicyProtocol::authored_tcl(version),
        )
    });
    let batch_dir = Path::new("/batch-survey");
    let batch = original_index_entries_batch(source, batch_dir, &|_| true, &requests);
    assert!(
        batch.iter().all(|rows| !rows.is_empty()),
        "each release must retain the valid row"
    );
    let facts = |rows: &[OriginalAutoIndexRegistration]| {
        rows.iter()
            .map(|entry| (entry.input.clone(), entry.file.clone()))
            .collect::<Vec<_>>()
    };
    for ((config, policy), rows) in requests.into_iter().zip(&batch) {
        let serial = survey_index_entries(source, batch_dir, config, policy);
        assert_eq!(facts(rows), facts(&serial), "{policy:?}");
    }
    let warm = original_index_entries_batch(source, batch_dir, &|_| true, &requests);
    assert_eq!(
        batch.iter().map(|rows| facts(rows)).collect::<Vec<_>>(),
        warm.iter().map(|rows| facts(rows)).collect::<Vec<_>>()
    );
    assert!(
        original_index_entries_batch(source, batch_dir, &|_| false, &requests)
            .iter()
            .all(Vec::is_empty),
        "cached rows must recheck existence"
    );
    let partial_dir = Path::new("/partially-cached-survey");
    original_index_entries(source, partial_dir, &|_| true, requests[0].0, requests[0].1);
    let partial = original_index_entries_batch(source, partial_dir, &|_| true, &requests);
    for ((config, policy), rows) in requests.into_iter().zip(&partial) {
        assert_eq!(
            facts(rows),
            facts(&survey_index_entries(source, partial_dir, config, policy))
        );
    }
    let changed = source.replace("a.tcl", "changed.tcl");
    let changed_rows = original_index_entries_batch(&changed, batch_dir, &|_| true, &requests);
    assert!(changed_rows.iter().all(|rows| {
        rows.iter()
            .any(|row| row.file == batch_dir.join("changed.tcl"))
    }));
}

/// The rescan memo retains the survey, never the existence answer: the
/// same index re-checks its target files on every call, and changed bytes
/// are surveyed again.
#[test]
fn index_survey_memo_rechecks_existence_and_misses_changed_bytes() {
    let version = tcl_dialect::TclVersion::V8_6;
    let config = LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
    );
    let policy = NamePolicyProtocol::authored_tcl(version);
    let source = "set auto_index(memo_probe) [list source [file join $dir m.tcl]]\n";
    let dir = Path::new("/memo-library");
    let present = original_index_entries(source, dir, &|_| true, config, policy);
    assert_eq!(present.len(), 1);
    assert_eq!(present[0].file, dir.join("m.tcl"));
    assert!(original_index_entries(source, dir, &|_| false, config, policy).is_empty());
    let edited = original_index_entries(
        "set auto_index(memo_probe) [list source [file join $dir n.tcl]]\n",
        dir,
        &|_| true,
        config,
        policy,
    );
    assert_eq!(edited[0].file, dir.join("n.tcl"));
    let other_dir =
        original_index_entries(source, Path::new("/elsewhere"), &|_| true, config, policy);
    assert_eq!(other_dir[0].file, Path::new("/elsewhere").join("m.tcl"));
}

#[test]
fn original_index_grammar_requires_the_current_registry_implementation() {
    let version = tcl_dialect::TclVersion::V9_1;
    let config = LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
    );
    let policy = NamePolicyProtocol::authored_tcl(version);
    let source =
        "proc set args {return ignored}\nset auto_index(p) [list source [file join $dir p.tcl]]";
    assert!(
        original_index_entries(source, Path::new("/library"), &|_| true, config, policy).is_empty()
    );
    let source = "proc package args {return ignored}\npackage ifneeded p 1.0 {source p.tcl}";
    assert_eq!(
        original_package_entries(
            source,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
            version
        ),
        [] as [crate::package_resolver::original_names::OriginalPackageInfo; 0]
    );
}

#[test]
fn original_old_index_uses_list_grammar_and_translated_line_boundaries() {
    for version in tcl_dialect::TclVersion::ALL {
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
        );
        for newline in ["\n", "\r\n", "\r"] {
            let source = format!(
                "# Tcl autoload index file: each line identifies a Tcl{newline}{{$literal}} {{file name.tcl}}{newline}"
            );
            let entries = original_index_entries(
                &source,
                Path::new("/library"),
                &|_| true,
                config,
                NamePolicyProtocol::authored_tcl(version),
            );
            assert_eq!(entries.len(), 1, "{version:?}: {newline:?}");
            assert_eq!(entries[0].input.bytes(), b"$literal");
            assert_eq!(entries[0].file, Path::new("/library/file name.tcl"));
        }
    }
}

#[test]
fn original_index_keeps_counted_literal_names_and_does_not_trim_indices() {
    let config = LexerConfig::from_grammar(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1).lexer_grammar,
    );
    let source = "set {auto_index(p\0tail)} [list source [file join $dir p.tcl]]\nset {auto_index( spaced )} [list source [file join $dir q.tcl]]";
    let entries = original_index_entries(
        source,
        Path::new("/library"),
        &|_| true,
        config,
        NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_1),
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.input.bytes(), entry.file.as_path()))
            .collect::<Vec<_>>(),
        vec![
            (b"p\xc0\x80tail".as_slice(), Path::new("/library/p.tcl")),
            (b" spaced ".as_slice(), Path::new("/library/q.tcl"))
        ]
    );
}
