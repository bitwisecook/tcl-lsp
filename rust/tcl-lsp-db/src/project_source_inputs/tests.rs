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
            "speclib project_source356 1.0 {{\ncommand project_source356::define {{\narity 3\narg 0 -role Name\narg 1 -role Params\narg 2 -role Body\n{hook}\n}}\n}}\n"
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
    assert_eq!(
        command_arity_for_inputs(&db, project, CommandTail::new(&db, "provided".to_owned())),
        Some(Arc::new(vec![(1, 1)]))
    );

    // A different folder's configuration is not a provider for this file.
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, caller_config),
        (caller, caller_config),
    ]));
    assert!(
        command_arity_for_inputs(&db, project, CommandTail::new(&db, "provided".to_owned()))
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
        command_arity_for_inputs(&db, project, CommandTail::new(&db, "provided".to_owned()))
            .is_none()
    );
    library_config.set_spec_pack_key(&mut db).to(u64::MAX - 356);
    assert!(document_analysis_input(&db, library, library_config).is_err());
    assert!(item_sigs_for_config(&db, library, library_config).is_empty());
    assert!(
        command_arity_for_inputs(&db, project, CommandTail::new(&db, "provided".to_owned()))
            .is_none()
    );
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
