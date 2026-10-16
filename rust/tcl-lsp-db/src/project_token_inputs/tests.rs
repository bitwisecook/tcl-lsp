// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use salsa::Setter as _;

fn configuration(db: &TclDatabase, overlay: u64) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        overlay,
        Vec::new(),
        Vec::new(),
    )
}

fn read_pack(reads: bool) -> tcl_spectcl::pack::PackSet {
    let role = if reads { "arg 0 -role VarRead" } else { "" };
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/project-input354.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        format!(
            "speclib project_input354 1.0 {{\ncommand project_input354::read {{\narity 1\n{role}\n}}\n}}\n"
        ),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

fn is_variable(tokens: &SemanticTokens, source: &str, value: &str) -> bool {
    let offset = u32::try_from(source.rfind(value).unwrap()).unwrap();
    let target = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    let mut line = 0;
    let mut column = 0;
    tokens.data.chunks_exact(5).any(|token| {
        line += token[0];
        column = if token[0] == 0 {
            column + token[1]
        } else {
            token[1]
        };
        line == target.line
            && column <= target.character.get()
            && target.character.get() < column + token[2]
            && token[3] == 2
    })
}

#[test]
fn project_tokens_keep_each_files_pack_roles_and_withdraw_changed_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source-only parameter hints; no Native invocation or variable binding.
    let mut db = TclDatabase::default();
    let packs = read_pack(true);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let library_config = configuration(&db, packs.key);
    let caller_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc ::reader {name} {project_input354::read $name}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "reader retained";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, library_config), (caller, caller_config)]),
    );
    let actual = file_token_facts_for_config(&db, library, library_config);
    assert!(
        !actual.proc_roles.is_empty(),
        "the actual library role is retained"
    );
    let index = project_proc_var_index_for_inputs(&db, project);
    let library_input = document_analysis_input(&db, library, library_config).unwrap();
    let caller_input = document_analysis_input(&db, caller, caller_config).unwrap();
    let library_analysis = document_analyser(&db, library, library_config)
        .with_resolved_input((*library_input).clone())
        .structure_only()
        .analyse(library.text(&db), library.dialect(&db));
    let caller_analysis = document_analyser(&db, caller, caller_config)
        .with_resolved_input((*caller_input).clone())
        .structure_only()
        .analyse(caller.text(&db), caller.dialect(&db));
    // Representation changed from joined report maps to genuine per-file
    // declaration/formal owners. Compare that exact transport, then project
    // the actual selected call back to the original library declaration.
    use tcl_compiler::registry_invocation::source_structure::{
        OriginalProjectProcedureRoles, original_project_procedure_arguments,
    };
    assert_eq!(
        index.project_source_roles(),
        &[
            OriginalProjectProcedureRoles::capture(&library_analysis).unwrap(),
            OriginalProjectProcedureRoles::capture(&caller_analysis).unwrap(),
        ]
    );
    let command = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        caller_input.lexer_config(),
    )
    .pop()
    .unwrap();
    let selected = original_project_procedure_arguments(
        source,
        &caller_analysis,
        &command,
        index.project_source_roles(),
    )
    .unwrap();
    let expected = library_analysis
        .original_procedure_declarations()
        .next()
        .unwrap();
    assert_eq!(selected.declaration(), expected);
    assert_eq!(
        selected.bindings(),
        &[
            tcl_syntax::formal_params::FormalByteArgumentBinding::Value {
                parameter: 0,
                argument: 0
            }
        ]
    );
    assert_eq!(
        selected.arguments(),
        &[tcl_compiler::registry_invocation::EffectiveInvocationWord::from_bytes(b"retained")]
    );
    assert!(selected.matches_source(&tcl_lexer::SourceImage::document(source), &caller_input));
    assert_eq!(
        selected.declaration().metadata().param_traits,
        expected.metadata().param_traits
    );
    assert!(
        expected.metadata().param_traits.get("name").is_some_and(
            |traits| traits.contains(&tcl_compiler::analyser::types::ProcArgTrait::VarRead)
        )
    );
    assert!(is_variable(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "retained"
    ));
    assert!(!is_variable(
        &semantic_tokens(&db, caller, caller_config),
        source,
        "retained"
    ));

    // The caller's configuration is not donated to the library.
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, caller_config),
        (caller, caller_config),
    ]));
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(!is_variable(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "retained"
    ));
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, library_config),
        (caller, caller_config),
    ]));
    assert!(is_variable(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "retained"
    ));

    let changed = read_pack(false);
    let _changed_registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &changed);
    library_config.set_spec_pack_key(&mut db).to(changed.key);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(!is_variable(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "retained"
    ));
    library_config.set_spec_pack_key(&mut db).to(u64::MAX - 354);
    assert!(document_analysis_input(&db, library, library_config).is_err());
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(!is_variable(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "retained"
    ));
}

#[test]
fn project_token_hints_withdraw_when_another_members_source_owner_is_unavailable() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let other_config = configuration(&db, 0);
    let file = SourceFile::new(&db,
        "oo::class create ::ProjectThing {method kept {} {}}\n::ProjectThing create specimen\nproc bind {name} {upvar 1 $name cell; set cell 1}".to_owned(),
        "tcl".to_owned(), None);
    let other = SourceFile::new(&db, String::new(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![file, other],
        Some(vec![(file, config), (other, other_config)]),
    );
    assert!(checked_file_token_facts_for_config(&db, other, other_config).is_some());
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .contains_key("::ProjectThing")
    );
    assert!(!project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(!project_named_instance_index_for_inputs(&db, project).is_empty());
    other_config.set_spec_pack_key(&mut db).to(u64::MAX - 2354);
    assert!(checked_file_token_facts_for_config(&db, other, other_config).is_none());
    assert!(
        file_token_facts_for_config(&db, file, config)
            .classes
            .contains_key("::ProjectThing")
    );
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .is_empty()
    );
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(project_named_instance_index_for_inputs(&db, project).is_empty());
    other_config.set_spec_pack_key(&mut db).to(0);
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(file, config)]));
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .is_empty()
    );
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
    assert!(project_named_instance_index_for_inputs(&db, project).is_empty());
}

#[test]
fn project_token_hints_deduplicate_actual_file_owners_without_merging_equal_labels() {
    let db = TclDatabase::default();
    let config = configuration(&db, 0);
    let source = "oo::class create ::ProjectThing {method kept {} {}}";
    let first = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let second = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let single = Project::with_token_configurations(&db, vec![first], Some(vec![(first, config)]));
    let repeated =
        Project::with_token_configurations(&db, vec![first, first], Some(vec![(first, config)]));
    assert!(
        project_class_index_for_inputs(&db, single)
            .classes
            .contains_key("::ProjectThing")
    );
    assert_eq!(
        project_class_index_for_inputs(&db, repeated),
        project_class_index_for_inputs(&db, single)
    );
    assert_eq!(
        project_class_index(&db, repeated),
        project_class_index(&db, single)
    );
    let distinct = Project::with_token_configurations(
        &db,
        vec![first, second],
        Some(vec![(first, config), (second, config)]),
    );
    assert!(
        project_class_index_for_inputs(&db, distinct)
            .classes
            .is_empty()
    );
    assert!(project_class_index(&db, distinct).classes.is_empty());
}

#[test]
fn supplied_project_cards_require_complete_unambiguous_current_file_mapping() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut db = TclDatabase::default();
    let current = configuration(&db, 0);
    let unavailable = configuration(&db, u64::MAX - 1354);
    let file = SourceFile::new(&db,
        "oo::class create ::ProjectThing {method kept {} {}}\n::ProjectThing create specimen\nproc bind {name} {upvar 1 $name cell; set cell 1}".to_owned(),
        "tcl".to_owned(), None);
    let project = Project::with_token_configurations(&db, vec![file], Some(vec![(file, current)]));
    let facts = file_token_facts_for_config(&db, file, current);
    assert!(facts.classes.contains_key("::ProjectThing"));
    assert!(!facts.named_instances.is_empty());
    assert_eq!(
        facts.named_instances.get("::specimen"),
        Some("::ProjectThing")
    );
    assert_eq!(
        facts.named_instances.report_rows().collect::<Vec<_>>(),
        vec![("::specimen", "::ProjectThing")]
    );
    assert!(!facts.proc_roles.is_empty());
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .contains_key("::ProjectThing")
    );
    assert_eq!(
        *project_named_instance_index_for_inputs(&db, project),
        facts.named_instances
    );
    assert_eq!(
        *project_proc_var_index_for_inputs(&db, project),
        facts.proc_roles
    );
    assert!(
        !semantic_tokens_project_for_inputs(&db, file, project)
            .data
            .is_empty()
    );
    for mapping in [
        None,
        Some(Vec::new()),
        Some(vec![(file, current), (file, unavailable)]),
        Some(vec![(file, unavailable)]),
    ] {
        project.set_token_configurations(&mut db).to(mapping);
        assert!(
            project_class_index_for_inputs(&db, project)
                .classes
                .is_empty()
        );
        assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
        assert!(project_named_instance_index_for_inputs(&db, project).is_empty());
        assert!(
            semantic_tokens_project_for_inputs(&db, file, project)
                .data
                .is_empty()
        );
    }
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(file, current)]));
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .contains_key("::ProjectThing")
    );
    file.set_text(&mut db).to("proc unrelated {} {}".to_owned());
    assert!(
        project_class_index_for_inputs(&db, project)
            .classes
            .is_empty()
    );
    assert!(project_named_instance_index_for_inputs(&db, project).is_empty());
    assert!(project_proc_var_index_for_inputs(&db, project).is_empty());
}

#[test]
fn supplied_project_indexes_keep_the_structure_only_firewall_across_source_edits() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let mut db = TclDatabase::with_event_logger(move |event| sink.lock().unwrap().push(event));
    let config = configuration(&db, 0);
    let file = SourceFile::new(&db,
        "oo::class create ::Light {method kept {} {return one}}\nproc writer {name} {upvar 1 $name cell; set cell 1}".to_owned(),
        "tcl".to_owned(), None);
    let project = Project::with_token_configurations(&db, vec![file], Some(vec![(file, config)]));
    let _ = project_class_index_for_inputs(&db, project);
    let _ = project_proc_var_index_for_inputs(&db, project);
    let _ = project_named_instance_index_for_inputs(&db, project);
    let first = events.lock().unwrap().clone();
    assert!(
        first
            .iter()
            .any(|event| event.contains("file_token_facts_for_config")),
        "{first:?}"
    );
    assert!(
        first.iter().all(|event| !event.contains("file_analysis")
            && !event.contains("compilation_unit")
            && !event.contains("function_lattice")),
        "{first:?}"
    );
    events.lock().unwrap().clear();
    file.set_text(&mut db).to("oo::class create ::Changed {method kept {} {return two}}\nproc writer {name} {upvar 1 $name cell; set cell 1}".to_owned());
    let classes = project_class_index_for_inputs(&db, project);
    assert!(classes.classes.contains_key("::Changed"));
    assert!(!classes.classes.contains_key("::Light"));
    let _ = project_proc_var_index_for_inputs(&db, project);
    let _ = project_named_instance_index_for_inputs(&db, project);
    let after = events.lock().unwrap().clone();
    assert!(
        after
            .iter()
            .any(|event| event.contains("file_token_facts_for_config")),
        "{after:?}"
    );
    assert!(
        after.iter().all(|event| !event.contains("file_analysis")
            && !event.contains("compilation_unit")
            && !event.contains("function_lattice")),
        "{after:?}"
    );
}

#[test]
fn project_source_roles_select_current_call_declaration_and_formals() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    for dialect in ["tcl", "tcl8.6", "jim"] {
        let library = SourceFile::new(&db, "proc reader {name} {upvar 1 $name cell; set cell 1}; proc puts {name} {upvar 1 $name cell; set cell 1}".to_owned(), dialect.to_owned(), None);
        let caller = SourceFile::new(&db, "reader retained".to_owned(), dialect.to_owned(), None);
        let project = Project::with_token_configurations(
            &db,
            vec![library, caller],
            Some(vec![(library, config), (caller, config)]),
        );
        for (source, expected) in [
            ("reader retained", true),
            ("puts retained", false),
            ("proc reader {value} {}; reader retained", false),
            ("interp alias {} reader {} puts; reader retained", false),
            (
                "proc reader {value} {}; rename reader {}; reader retained",
                false,
            ),
            ("unknown_mutation; reader retained", false),
            ("reader [unknown_operand] retained", false),
            ("reader {*}{retained}", false),
        ] {
            caller.set_text(&mut db).to(source.to_owned());
            let tokens = semantic_tokens_project_for_inputs(&db, caller, project);
            assert_eq!(
                is_variable(&tokens, source, "retained"),
                expected,
                "{dialect}: {source}"
            );
        }
        caller.set_text(&mut db).to("reader retained".to_owned());
        let duplicate = SourceFile::new(&db, library.text(&db).clone(), dialect.to_owned(), None);
        project
            .set_files(&mut db)
            .to(vec![library, caller, duplicate]);
        project.set_token_configurations(&mut db).to(Some(vec![
            (library, config),
            (caller, config),
            (duplicate, config),
        ]));
        assert!(!is_variable(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "reader retained",
            "retained"
        ));
    }
}

#[test]
fn project_source_roles_bind_original_optional_formals_and_keep_foreign_class_barriers() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    for dialect in ["tcl", "tcl8.6"] {
        let library = SourceFile::new(
            &db,
            "proc reader {{unused DEFAULT} name} {upvar 1 $name cell; set cell 1}".to_owned(),
            dialect.to_owned(),
            None,
        );
        let caller = SourceFile::new(
            &db,
            "reader FIRST retained".to_owned(),
            dialect.to_owned(),
            None,
        );
        let project = Project::with_token_configurations(
            &db,
            vec![library, caller],
            Some(vec![(library, config), (caller, config)]),
        );
        assert!(is_variable(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "reader FIRST retained",
            "retained"
        ));
        caller.set_text(&mut db).to("reader retained".to_owned());
        assert!(!is_variable(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "reader retained",
            "retained"
        ));
        caller
            .set_text(&mut db)
            .to("reader FIRST retained".to_owned());
        let class_file = SourceFile::new(
            &db,
            "oo::class create reader {}".to_owned(),
            dialect.to_owned(),
            None,
        );
        project
            .set_files(&mut db)
            .to(vec![library, caller, class_file]);
        project.set_token_configurations(&mut db).to(Some(vec![
            (library, config),
            (caller, config),
            (class_file, config),
        ]));
        assert!(!is_variable(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "reader FIRST retained",
            "retained"
        ));
    }
}

fn is_method(tokens: &SemanticTokens, source: &str, word: &str) -> bool {
    let offset = u32::try_from(source.rfind(word).unwrap()).unwrap();
    let target = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    let kind = u32::try_from(
        tcl_lsp_core::semantic_tokens::legend_token_types()
            .iter()
            .position(|kind| *kind == "method")
            .unwrap(),
    )
    .unwrap();
    let (mut line, mut column) = (0, 0);
    tokens.data.chunks_exact(5).any(|token| {
        line += token[0];
        column = if token[0] == 0 {
            column + token[1]
        } else {
            token[1]
        };
        line == target.line && column == target.character.get() && token[3] == kind
    })
}

#[test]
fn project_class_display_keeps_original_factory_and_named_instance_cards() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        for source in ["[::C new] ping", "::obj ping"] {
            let db = TclDatabase::default();
            let config = configuration(&db, 0);
            let library = SourceFile::new(
                &db,
                "oo::class create ::C {method ping {} {}}; ::C create ::obj".to_owned(),
                dialect.to_owned(),
                None,
            );
            let caller = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
            let project = Project::with_token_configurations(
                &db,
                vec![library, caller],
                Some(vec![(library, config), (caller, config)]),
            );
            assert!(
                file_token_facts_for_config(&db, library, config)
                    .class_cards
                    .is_some()
            );
            assert!(
                is_method(
                    &semantic_tokens_project_for_inputs(&db, caller, project),
                    source,
                    "ping"
                ),
                "{dialect}: {source}"
            );
            assert!(!is_method(
                &semantic_tokens(&db, caller, config),
                source,
                "ping"
            ));
        }
    }
}

#[test]
fn project_class_display_does_not_recover_refused_receipts_from_report_maps() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let mut db = TclDatabase::default();
        let config = configuration(&db, 0);
        let library = SourceFile::new(
            &db,
            "oo::class create C {method ping {} {}}; C create obj".to_owned(),
            dialect.to_owned(),
            None,
        );
        let caller = SourceFile::new(&db, "[C new] ping".to_owned(), dialect.to_owned(), None);
        let project = Project::with_token_configurations(
            &db,
            vec![library, caller],
            Some(vec![(library, config), (caller, config)]),
        );
        for source in [
            "proc C {args} {}; [C new] ping",
            "proc C {args} {}; rename C {}; [C new] ping",
            "interp alias {} C {} list; [C new] ping",
            "unknown_effect; [C new] ping",
            "proc obj {args} {}; obj ping",
            "interp alias {} obj {} list; obj ping",
        ] {
            caller.set_text(&mut db).to(source.to_owned());
            assert!(
                !is_method(
                    &semantic_tokens_project_for_inputs(&db, caller, project),
                    source,
                    "ping"
                ),
                "{dialect}: {source}"
            );
        }
        caller.set_text(&mut db).to("obj ping".to_owned());
        library
            .set_text(&mut db)
            .to("oo::class create C {method ping {} {}}; C create obj; rename obj {}".to_owned());
        assert!(!is_method(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "obj ping",
            "ping"
        ));
        // Retained class reports cannot restore a known deleted source slot.
        assert!(
            project_class_index_for_inputs(&db, project)
                .classes
                .contains_key("::C")
        );
        caller.set_text(&mut db).to("[C new] ping".to_owned());
        assert!(is_method(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "[C new] ping",
            "ping"
        ));
        project
            .set_token_configurations(&mut db)
            .to(Some(vec![(caller, config)]));
        assert!(!is_method(
            &semantic_tokens_project_for_inputs(&db, caller, project),
            "[C new] ping",
            "ping"
        ));
    }
}

#[test]
fn project_configurable_display_preserves_cross_file_factory_positive() {
    let db = TclDatabase::default();
    let config = configuration(&db, 0);
    let source = "[::Pin new] configure -node 5";
    let library = SourceFile::new(
        &db,
        "oo::configurable create ::Pin {property node}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, config), (caller, config)]),
    );
    assert!(is_method(
        &semantic_tokens_project_for_inputs(&db, caller, project),
        source,
        "configure"
    ));
}

#[test]
fn project_class_collection_display_keeps_original_container_and_receiver_paths() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let mut db = TclDatabase::default();
        let config = configuration(&db, 0);
        let caller = SourceFile::new(&db, String::new(), dialect.to_owned(), None);
        let project =
            Project::with_token_configurations(&db, vec![caller], Some(vec![(caller, config)]));
        for (storage, expected) in [
            (
                "dict set values key [C new]; [dict get $values key] ping",
                true,
            ),
            ("lappend values [C new]; [lindex $values 0] ping", true),
            (
                "dict set values key [C new]; rename C D; [dict get $values key] ping",
                true,
            ),
            (
                "dict set values key [C new]; set item [dict get $values key]; $item ping",
                true,
            ),
            (
                "lappend values [C new]; foreach item $values {$item ping}",
                true,
            ),
            (
                "dict set values key [C new]; dict for {key item} $values {$item ping}",
                true,
            ),
            (
                "dict set other key [C new]; [dict get $values key] ping",
                false,
            ),
            (
                "dict set values outer inner [C new]; [dict get $values outer] ping",
                false,
            ),
            (
                "dict lappend values key [C new]; [dict get $values key] ping",
                false,
            ),
            (
                "dict set values key UNKNOWN; [dict get $values key] ping",
                false,
            ),
            (
                "dict set values key [C new]; unknown_effect; [dict get $values key] ping",
                false,
            ),
            (
                "dict set values key [C new]; rename C {}; [dict get $values key] ping",
                false,
            ),
        ] {
            let source = format!("oo::class create C {{method ping {{}} {{}}}}; {storage}");
            caller.set_text(&mut db).to(source.clone());
            assert_eq!(
                is_method(
                    &semantic_tokens_project_for_inputs(&db, caller, project),
                    &source,
                    "ping"
                ),
                expected,
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn project_class_collection_display_retains_foreign_getter_unknown_barrier() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let db = TclDatabase::default();
        let config = configuration(&db, 0);
        let library = SourceFile::new(
            &db,
            "oo::class create C {method ping {} {}}".to_owned(),
            dialect.to_owned(),
            None,
        );
        let source = "dict set values key [C new]; [dict get $values key] ping";
        let caller = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
        let project = Project::with_token_configurations(
            &db,
            vec![library, caller],
            Some(vec![(library, config), (caller, config)]),
        );
        assert!(
            !is_method(
                &semantic_tokens_project_for_inputs(&db, caller, project),
                source,
                "ping"
            ),
            "{dialect}"
        );
    }
}

#[test]
fn project_class_collection_field_display_uses_original_declaration_and_receiver_side() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let mut db = TclDatabase::default();
        let config = configuration(&db, 0);
        let caller = SourceFile::new(&db, String::new(), dialect.to_owned(), None);
        let project =
            Project::with_token_configurations(&db, vec![caller], Some(vec![(caller, config)]));
        for (cache, expected) in [
            (
                "oo::class create Cache {variable values; constructor {} {dict set values key [C new]}; method read {} {[dict get $values key] ping}}",
                true,
            ),
            (
                "oo::class create Cache {variable values; method put {} {dict set values key [C new]}; method read {} {[dict get $values key] ping}}",
                true,
            ),
            (
                "oo::class create Cache {method put {} {dict set values key [C new]}; method read {} {[dict get $values key] ping}}",
                false,
            ),
            (
                "oo::class create Cache {variable values; method put {} {dict set values key [C new]}; method read {values} {[dict get $values key] ping}}",
                false,
            ),
            (
                "oo::class create Cache {variable values; method put {} {dict set values key [C new]}; self method read {} {[dict get $values key] ping}}",
                false,
            ),
            (
                "oo::class create Cache {variable values; method put {} {dict set values key [C new]}}; oo::class create Other {variable values; method read {} {[dict get $values key] ping}}",
                false,
            ),
        ] {
            let source = format!("oo::class create C {{method ping {{}} {{}}}}; {cache}");
            caller.set_text(&mut db).to(source.clone());
            assert_eq!(
                is_method(
                    &semantic_tokens_project_for_inputs(&db, caller, project),
                    &source,
                    "ping"
                ),
                expected,
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn project_class_inherited_display_does_not_collapse_distinct_equal_content_files() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let db = TclDatabase::default();
        let config = configuration(&db, 0);
        let library = "oo::class create Base {method ping {} {}}";
        let a = SourceFile::new(&db, library.to_owned(), dialect.to_owned(), None);
        let b = SourceFile::new(&db, library.to_owned(), dialect.to_owned(), None);
        assert_ne!(a, b);
        let source = "oo::class create C {superclass Base}; [C new] ping";
        let caller = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
        let one = Project::with_token_configurations(
            &db,
            vec![a, caller],
            Some(vec![(a, config), (caller, config)]),
        );
        assert!(
            is_method(
                &semantic_tokens_project_for_inputs(&db, caller, one),
                source,
                "ping"
            ),
            "{dialect}: one foreign Base"
        );
        assert!(
            is_method(
                &semantic_tokens_project(&db, caller, one, config),
                source,
                "ping"
            ),
            "{dialect}: compatibility one foreign Base"
        );
        let project = Project::with_token_configurations(
            &db,
            vec![a, b, caller],
            Some(vec![(a, config), (b, config), (caller, config)]),
        );
        assert!(
            !is_method(
                &semantic_tokens_project_for_inputs(&db, caller, project),
                source,
                "ping"
            ),
            "{dialect}"
        );
        assert!(
            !is_method(
                &semantic_tokens_project(&db, caller, project, config),
                source,
                "ping"
            ),
            "{dialect}: compatibility entrance"
        );
    }
}

#[test]
fn project_class_foreign_equal_content_unknown_inventory_stays_terminal() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let db = TclDatabase::default();
        let config = configuration(&db, 0);
        let library = SourceFile::new(
            &db,
            "oo::class create C {method ping {} {}}".to_owned(),
            dialect.to_owned(),
            None,
        );
        let source = "[C new] ping; unknown_effect";
        let caller = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
        let duplicate = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
        assert_ne!(caller, duplicate);
        let one = Project::with_token_configurations(
            &db,
            vec![library, caller],
            Some(vec![(library, config), (caller, config)]),
        );
        assert!(
            is_method(
                &semantic_tokens_project_for_inputs(&db, caller, one),
                source,
                "ping"
            ),
            "{dialect}: caller-only later unknown is not an earlier lookup mutation"
        );
        assert!(
            is_method(
                &semantic_tokens_project(&db, caller, one, config),
                source,
                "ping"
            ),
            "{dialect}: compatibility control"
        );
        let foreign_unknown = Project::with_token_configurations(
            &db,
            vec![library, caller, duplicate],
            Some(vec![
                (library, config),
                (caller, config),
                (duplicate, config),
            ]),
        );
        assert!(
            !is_method(
                &semantic_tokens_project_for_inputs(&db, caller, foreign_unknown),
                source,
                "ping"
            ),
            "{dialect}: distinct equal-content unknown provider"
        );
        assert!(
            !is_method(
                &semantic_tokens_project(&db, caller, foreign_unknown, config),
                source,
                "ping"
            ),
            "{dialect}: compatibility unknown provider"
        );
    }
}
