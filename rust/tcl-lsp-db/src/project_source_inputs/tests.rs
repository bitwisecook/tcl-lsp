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

fn procedure_pack(defines: bool) -> tcl_spectcl::pack::PackSet {
    let hook = if defines {
        "analyser_hook -native Proc\ncommand_table_effect DefinesProcedure"
    } else {
        ""
    };
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/project-source356.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        format!(
            "speclib project_source356 1.0 {{\ncommand project_source356::define {{\narity 3\narg 0 -role Name\narg 1 -role ParamList\narg 2 -role Body\n{hook}\n}}\n}}\n"
        ),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

fn has_procedure(tree: &ItemTree, name: &str) -> bool {
    tree.items.iter().any(|item| {
        item.sig.id.kind == tcl_compiler::analyser::ItemKind::Proc && item.sig.id.key == name
    })
}

#[test]
fn configured_project_headers_keep_pack_definers_and_withdraw_changed_source_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A selected authored definer supplies source cards, not Native execution.
    let mut db = TclDatabase::default();
    let packs = procedure_pack(true);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let library_config = configuration(&db, packs.key);
    let caller_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "project_source356::define provided {one} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let caller = SourceFile::new(&db, "provided value".to_owned(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, library_config), (caller, caller_config)]),
    );
    assert!(has_procedure(
        &item_tree_for_config(&db, library, library_config),
        "::provided"
    ));
    assert!(!has_procedure(&item_tree(&db, library), "::provided"));
    let headers = item_sigs_for_config(&db, library, library_config);
    let coordinate = headers
        .iter()
        .find_map(|header| header.logical_source_coordinate.clone())
        .expect("actual selected source header coordinate");
    assert_eq!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        ),
        Some(Arc::new(vec![(1, 1)]))
    );

    // A different folder's configuration is not a provider for this file.
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, caller_config),
        (caller, caller_config),
    ]));
    assert!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        )
        .is_none()
    );
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, library_config),
        (caller, caller_config),
    ]));
    let changed = procedure_pack(false);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &changed);
    library_config.set_spec_pack_key(&mut db).to(changed.key);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    assert!(!has_procedure(
        &item_tree_for_config(&db, library, library_config),
        "::provided"
    ));
    assert!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        )
        .is_none()
    );
    library_config.set_spec_pack_key(&mut db).to(u64::MAX - 356);
    assert!(document_analysis_input(&db, library, library_config).is_err());
    assert!(item_sigs_for_config(&db, library, library_config).is_empty());
    assert!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        )
        .is_none()
    );
}

#[test]
fn configured_proc_source_headers_defer_body_effects_and_keep_unknown_hook_effects() {
    let mut db = TclDatabase::default();
    let packs = procedure_pack(true);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let config = configuration(&db, packs.key);
    let source = "project_source356::define provided {one} {rename puts {}}; puts VALUE";
    let file = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let headers = item_sigs_for_config(&db, file, config);
    assert!(
        headers
            .iter()
            .any(|header| header.logical_source_coordinate.is_some())
    );
    let analysis = file_analysis(&db, file, config);
    let call = tcl_compiler::registry_invocation::source_structure::source_registry_words_at(
        source,
        &analysis,
        u32::try_from(source.rfind("puts VALUE").unwrap()).unwrap(),
    )
    .unwrap();
    let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = call.source() else {
        panic!("actual retained source transition owner");
    };
    assert!(advice.uncertain_operations().iter().any(|operation| {
        operation
            .first()
            .is_some_and(|head| head.span().start() == 0)
    }));
    let body_operation = u32::try_from(source.find("rename puts").unwrap()).unwrap();
    assert!(advice.uncertain_operations().iter().all(|operation| {
        operation
            .first()
            .is_none_or(|head| head.span().start() != body_operation)
    }));
}

#[test]
fn supplied_callback_headers_use_the_librarys_checked_input_and_body_free_signature() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Header/count advice retains the original callback subject and no entry claim.
    let mut db = TclDatabase::default();
    let library_config = configuration(&db, 0);
    let caller_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc external {one} {return FIRST}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let source = "lsort -command ::external {2 1}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, library_config), (caller, caller_config)]),
    );
    let analysis = file_analysis_incremental(&db, caller, caller_config);
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|row| row.original_callback_signature_lookup.is_some())
    );
    let first = project_original_command_signatures_for_inputs(&db, project);
    assert_eq!(first.len(), 1);
    assert_eq!(
        first.values().next().unwrap()[0]
            .formal_count_projection()
            .arity(),
        tcl_registry::Arity::exact(1)
    );
    let diagnostics = project_callback_diagnostics_for_analysis_with_inputs(
        &db,
        project,
        source,
        &analysis,
        |_| false,
    );
    let subject = diagnostics
        .iter()
        .find_map(|row| row.callback_source_arity())
        .expect("current original library header");
    assert_eq!(
        subject.argument_counts(),
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2])
    );
    assert!(
        diagnostics
            .iter()
            .any(|row| row.code == DiagCode::E003 && row.callback_source_arity().is_some())
    );
    library
        .set_text(&mut db)
        .to("proc external {one} {return LATER}".to_owned());
    assert_eq!(
        project_original_command_signatures_for_inputs(&db, project),
        first,
        "body-free signature equality"
    );
    library
        .set_text(&mut db)
        .to("proc external {one two} {return LATER}".to_owned());
    assert!(
        project_callback_diagnostics_for_analysis_with_inputs(
            &db,
            project,
            source,
            &analysis,
            |_| false
        )
        .iter()
        .all(|row| row.callback_source_arity().is_none())
    );

    library_config
        .set_spec_pack_key(&mut db)
        .to(u64::MAX - 1356);
    assert!(document_analysis_input(&db, library, library_config).is_err());
    assert!(document_analysis_input(&db, caller, caller_config).is_ok());
    assert!(project_original_command_signatures_for_inputs(&db, project).is_empty());
    assert!(
        project_callback_diagnostics_for_analysis_with_inputs(
            &db,
            project,
            source,
            &analysis,
            |_| false
        )
        .iter()
        .all(|row| row.callback_source_arity().is_none())
    );
    library_config.set_spec_pack_key(&mut db).to(0);
    for mapping in [
        None,
        Some(vec![(caller, caller_config)]),
        Some(vec![
            (library, library_config),
            (library, caller_config),
            (caller, caller_config),
        ]),
    ] {
        project.set_token_configurations(&mut db).to(mapping);
        assert!(project_original_command_signatures_for_inputs(&db, project).is_empty());
    }
}

#[test]
fn supplied_project_headers_keep_unavailable_members_as_lookup_barriers() {
    let mut db = TclDatabase::default();
    let library_config = configuration(&db, 0);
    let other_config = configuration(&db, 0);
    let caller_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc external {one} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let other = SourceFile::new(&db, String::new(), "tcl9.0".to_owned(), None);
    let source = "lsort -command ::external {2 1}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, other, caller],
        Some(vec![
            (library, library_config),
            (other, other_config),
            (caller, caller_config),
        ]),
    );
    let analysis = file_analysis_incremental(&db, caller, caller_config);
    assert!(
        project_callback_diagnostics_for_analysis_with_inputs(
            &db,
            project,
            source,
            &analysis,
            |_| false,
        )
        .iter()
        .any(|row| row.code == DiagCode::E003 && row.callback_source_arity().is_some())
    );
    other_config.set_spec_pack_key(&mut db).to(u64::MAX - 4356);
    assert!(document_analysis_input(&db, other, other_config).is_err());
    assert!(project_original_command_signatures_for_inputs(&db, project).is_empty());
    assert!(
        project_callback_diagnostics_for_analysis_with_inputs(
            &db,
            project,
            source,
            &analysis,
            |_| false,
        )
        .iter()
        .all(|row| row.callback_source_arity().is_none())
    );
    other_config.set_spec_pack_key(&mut db).to(0);
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, library_config),
        (caller, caller_config),
    ]));
    assert!(project_original_command_signatures_for_inputs(&db, project).is_empty());
    assert!(
        project_callback_diagnostics_for_analysis_with_inputs(
            &db,
            project,
            source,
            &analysis,
            |_| false,
        )
        .iter()
        .all(|row| row.callback_source_arity().is_none())
    );
}

#[test]
fn supplied_logical_arities_require_every_members_source_owner() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let other_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc helper {one two} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let other = SourceFile::new(&db, String::new(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, other],
        Some(vec![(library, config), (other, other_config)]),
    );
    let coordinate = item_sigs_for_config(&db, library, config)
        .iter()
        .find_map(|header| header.logical_source_coordinate.clone())
        .unwrap();
    assert_eq!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        ),
        Some(Arc::new(vec![(2, 2)]))
    );
    other_config.set_spec_pack_key(&mut db).to(u64::MAX - 5356);
    assert!(document_analysis_input(&db, other, other_config).is_err());
    assert!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate.clone()),
            true,
        )
        .is_none()
    );
    other_config.set_spec_pack_key(&mut db).to(0);
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(library, config)]));
    assert!(
        logical_source_command_arity(
            &db,
            project,
            LogicalSourceCommandSlot::new(&db, coordinate),
            true,
        )
        .is_none()
    );
}

#[test]
fn supplied_factory_oracles_require_current_file_inputs_without_a_derived_query_cycle() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional class-manufacturer hints are independent of Native allocation.
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let unavailable = configuration(&db, u64::MAX - 2356);
    let library = SourceFile::new(&db,
        "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}".to_owned(), "tcl".to_owned(), None);
    let consumer = SourceFile::new(
        &db,
        "::Meta create ::Made {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let project = Project::with_token_configurations(
        &db,
        vec![library, consumer],
        Some(vec![(library, config), (consumer, config)]),
    );
    let first = project_class_factories_for_inputs(&db, project);
    assert!(first.contains_key("::Meta"));
    for file in [library, consumer] {
        file.set_workspace_class_factories(&mut db)
            .to(Some(Arc::clone(&first)));
    }
    assert_eq!(
        project_class_factories_for_inputs(&db, project),
        first,
        "the oracle is an input, not a recursive project query"
    );
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(library, unavailable), (consumer, config)]));
    assert!(
        project_class_factories_for_inputs(&db, project).is_empty(),
        "the old oracle cannot recreate a withdrawn declaration"
    );
    project.set_token_configurations(&mut db).to(None);
    assert!(project_class_factories_for_inputs(&db, project).is_empty());
}

#[test]
fn supplied_factory_hints_require_every_members_source_owner() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let other_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let other = SourceFile::new(&db, String::new(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, other],
        Some(vec![(library, config), (other, other_config)]),
    );
    assert!(project_class_factories_for_inputs(&db, project).contains_key("::Meta"));
    other_config.set_spec_pack_key(&mut db).to(u64::MAX - 6356);
    assert!(document_analysis_input(&db, other, other_config).is_err());
    assert!(project_class_factories_for_inputs(&db, project).is_empty());
    other_config.set_spec_pack_key(&mut db).to(0);
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(library, config)]));
    assert!(project_class_factories_for_inputs(&db, project).is_empty());
}

#[test]
fn project_factory_hints_preserve_duplicate_provider_ambiguity_in_every_order() {
    let db = TclDatabase::default();
    let config = configuration(&db, 0);
    let source = "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}";
    let first = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let second = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let third = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let distinct = SourceFile::new(
        &db,
        "oo::class create ::Other {superclass oo::class; self method create {name body} {next $name $body}}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let single = Project::with_token_configurations(&db, vec![first], Some(vec![(first, config)]));
    assert!(project_class_factories_for_inputs(&db, single).contains_key("::Meta"));
    let repeated_owner =
        Project::with_token_configurations(&db, vec![first, first], Some(vec![(first, config)]));
    assert_eq!(
        project_class_factories_for_inputs(&db, repeated_owner),
        project_class_factories_for_inputs(&db, single)
    );
    assert_eq!(
        project_class_factories(&db, repeated_owner),
        project_class_factories(&db, single)
    );
    for files in [
        vec![first, second, third, distinct],
        vec![distinct, third, second, first],
    ] {
        let mappings = files.iter().map(|&file| (file, config)).collect();
        let project = Project::with_token_configurations(&db, files, Some(mappings));
        let supplied = project_class_factories_for_inputs(&db, project);
        assert!(!supplied.contains_key("::Meta"));
        assert!(supplied.contains_key("::Other"));
        assert_eq!(project_class_factories(&db, project), supplied);
    }
}

#[test]
fn configured_project_headers_and_factories_preserve_the_structure_tier_firewall() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let mut db = TclDatabase::with_event_logger(move |event| sink.lock().unwrap().push(event));
    let config = configuration(&db, 0);
    let file = SourceFile::new(
        &db,
        "proc kept {one} {return FIRST}\noo::class create ::Meta {superclass oo::class}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let project = Project::with_token_configurations(&db, vec![file], Some(vec![(file, config)]));
    let signatures = item_sigs_for_config(&db, file, config);
    assert!(has_procedure(
        &item_tree_for_config(&db, file, config),
        "::kept"
    ));
    let _ = project_original_command_signatures_for_inputs(&db, project);
    let _ = project_class_factories_for_inputs(&db, project);
    events.lock().unwrap().clear();
    file.set_text(&mut db).to(
        "proc kept {one} {return LATER}\noo::class create ::Meta {superclass oo::class}".to_owned(),
    );
    assert_eq!(item_sigs_for_config(&db, file, config), signatures);
    let _ = project_original_command_signatures_for_inputs(&db, project);
    let _ = project_class_factories_for_inputs(&db, project);
    let events = events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.contains("item_tree_for_config")),
        "{events:?}"
    );
    assert!(
        events.iter().all(|event| !event.contains("file_analysis")
            && !event.contains("compilation_unit")
            && !event.contains("function_lattice")),
        "{events:?}"
    );
}

#[test]
fn configured_project_diagnostics_use_actual_foreign_headers_for_direct_calls_and_callbacks() {
    let mut db = TclDatabase::default();
    let packs = procedure_pack(true);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let library_config = configuration(&db, packs.key);
    let caller_config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "project_source356::define provided {one} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let caller = SourceFile::new(&db, "provided ONE TWO".to_owned(), "tcl".to_owned(), None);
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, library_config), (caller, caller_config)]),
    );
    for source in ["provided ONE TWO", "lsort -command provided {a b}"] {
        caller.set_text(&mut db).to(source.to_owned());
        let supplied = project_diagnostics_for_inputs(&db, caller, caller_config, project);
        assert_eq!(
            supplied
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::E003)
                .count(),
            1,
            "{source}: {supplied:?}"
        );
        let standalone = project_diagnostics(&db, caller, caller_config, project);
        assert!(
            standalone
                .iter()
                .all(|diagnostic| diagnostic.code != DiagCode::E003),
            "{source}: {standalone:?}"
        );
        project
            .set_token_configurations(&mut db)
            .to(Some(vec![(caller, caller_config)]));
        let unavailable = project_diagnostics_for_inputs(&db, caller, caller_config, project);
        assert!(
            unavailable
                .iter()
                .all(|diagnostic| diagnostic.code != DiagCode::E003),
            "{source}: {unavailable:?}"
        );
        if source.starts_with("provided") {
            assert!(
                unavailable
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagCode::W123)
            );
            assert!(
                supplied
                    .iter()
                    .all(|diagnostic| diagnostic.code != DiagCode::W123)
            );
        }
        project.set_token_configurations(&mut db).to(Some(vec![
            (library, library_config),
            (caller, caller_config),
        ]));
    }
    library_config.set_spec_pack_key(&mut db).to(u64::MAX - 357);
    assert!(document_analysis_input(&db, library, library_config).is_err());
    assert!(
        project_diagnostics_for_inputs(&db, caller, caller_config, project)
            .iter()
            .all(|diagnostic| diagnostic.code != DiagCode::E003)
    );
}

#[test]
fn configured_project_diagnostics_do_not_default_missing_native_members() {
    let db = TclDatabase::default();
    let config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc external {one} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let caller = SourceFile::new(
        &db,
        "external ONE TWO".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    for (members, expected) in [
        (vec![(library, config), (caller, config)], true),
        (vec![(caller, config)], false),
    ] {
        let project = Project::with_token_configurations(&db, vec![library, caller], Some(members));
        let supplied = project_diagnostics_for_inputs(&db, caller, config, project);
        assert_eq!(
            supplied
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E003),
            expected,
            "{supplied:?}"
        );
        assert_eq!(
            supplied
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W123),
            !expected,
            "{supplied:?}"
        );
        assert!(
            project_diagnostics(&db, caller, config, project)
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E003)
        );
    }
}
