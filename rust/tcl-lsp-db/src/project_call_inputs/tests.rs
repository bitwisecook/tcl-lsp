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

fn body_pack(body: bool) -> tcl_spectcl::pack::PackSet {
    let role = if body {
        "arg 0 -role Body"
    } else {
        "arg 0 -role Value"
    };
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/project-call357.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        format!(
            "speclib project_call357 1.0 {{\ncommand project_call357::run {{\narity 1\n{role}\n}}\n}}\n"
        ),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

#[test]
fn supplied_project_callers_keep_per_file_body_roles_and_withdraw_unavailable_contributors() {

    // This records conditional caller advice, without Native execution or entry.
    let mut db = TclDatabase::default();
    let packs = body_pack(true);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let library_config = configuration(&db, 0);
    let caller_config = configuration(&db, packs.key);
    let library = SourceFile::new(
        &db,
        "proc helper {mode} {return $mode}".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/library.tcl".to_owned()),
    );
    let caller = SourceFile::new(
        &db,
        "project_call357::run {$command kept}\nsource library.tcl".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/main.tcl".to_owned()),
    );
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, library_config), (caller, caller_config)]),
    );
    let current = file_external_call_sites_for_inputs(&db, library, project)
        .expect("all actual contributors are available");
    assert!(
        current.get("::helper").is_some(),
        "the selected pack body can dispatch into its source component"
    );
    assert!(current.get("::helper").unwrap().opaque_caller);
    assert_eq!(current.get("::helper").unwrap().uniform_literal_at(0), None);
    assert!(
        file_source_targets_for_inputs(&db, caller, project)
            .unwrap()
            .iter()
            .any(|site| site.is_literal && site.raw_path == "library.tcl")
    );
    assert!(
        file_call_site_evidence_for_inputs(&db, caller, project)
            .unwrap()
            .get("::helper")
            .is_some()
    );

    let changed = body_pack(false);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &changed);
    caller_config.set_spec_pack_key(&mut db).to(changed.key);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let value_only = file_external_call_sites_for_inputs(&db, library, project).unwrap();
    assert!(
        value_only.get("::helper").is_none(),
        "literal payload has no selected child body"
    );
    caller_config.set_spec_pack_key(&mut db).to(u64::MAX - 357);
    assert!(document_analysis_input(&db, caller, caller_config).is_err());
    assert!(document_analysis_input(&db, library, library_config).is_ok());
    assert!(file_external_call_sites_for_inputs(&db, library, project).is_none());
    assert!(file_source_targets_for_inputs(&db, caller, project).is_none());
    project
        .set_token_configurations(&mut db)
        .to(Some(vec![(library, library_config)]));
    assert!(file_external_call_sites_for_inputs(&db, library, project).is_none());
}

#[test]
fn supplied_source_components_keep_selected_paths_and_dynamic_dispatch_withdrawals() {

    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc helper {mode} {return $mode}".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/library.tcl".to_owned()),
    );
    let caller = SourceFile::new(
        &db,
        "source library.tcl\n::helper kept\nset command $unknown; $command changed".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/main.tcl".to_owned()),
    );
    let isolated = SourceFile::new(
        &db,
        "proc isolated {} {}".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/isolated.tcl".to_owned()),
    );
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller, isolated],
        Some(vec![
            (library, config),
            (caller, config),
            (isolated, config),
        ]),
    );
    let components = project_dispatch_components_for_inputs(&db, project).unwrap();
    assert!(components.reach(1).unwrap().contains("::helper"));
    assert!(!components.reach(1).unwrap().contains("::isolated"));
    let evidence = file_external_call_sites_for_inputs(&db, library, project).unwrap();
    assert!(evidence.get("::helper").unwrap().opaque_caller);
    assert_eq!(
        evidence.get("::helper").unwrap().uniform_literal_at(0),
        None
    );
    assert!(
        file_external_call_sites_for_inputs(&db, isolated, project)
            .unwrap()
            .is_empty()
    );
    caller
        .set_text(&mut db)
        .to("source $path\n::helper kept".to_owned());
    let targets = file_source_targets_for_inputs(&db, caller, project).unwrap();
    assert!(targets.iter().any(|site| !site.is_literal));
    let config_without_generation = configuration(&db, u64::MAX - 1357);
    project.set_token_configurations(&mut db).to(Some(vec![
        (library, config),
        (caller, config),
        (isolated, config_without_generation),
    ]));
    assert!(
        file_external_call_sites_for_inputs(&db, library, project).is_none(),
        "an unavailable contributor cannot mean zero callers"
    );
}

#[test]
fn supplied_project_evidence_keeps_header_cutoffs_and_avoids_document_deep_queries() {

    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let mut db = TclDatabase::with_event_logger(move |event| sink.lock().unwrap().push(event));
    let config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc helper {mode} {return FIRST}".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/library.tcl".to_owned()),
    );
    let caller = SourceFile::new(
        &db,
        "source library.tcl; $command VALUE".to_owned(),
        "tcl".to_owned(),
        Some("/project-call357/main.tcl".to_owned()),
    );
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller],
        Some(vec![(library, config), (caller, config)]),
    );
    let first = file_external_call_sites_for_inputs(&db, library, project).unwrap();
    assert!(first.get("::helper").unwrap().opaque_caller);
    events.lock().unwrap().clear();
    library
        .set_text(&mut db)
        .to("proc helper {mode} {return LATER}".to_owned());
    assert_eq!(
        file_external_call_sites_for_inputs(&db, library, project).unwrap(),
        first
    );
    let events = events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.contains("file_source_structure")),
        "{events:?}"
    );
    assert!(
        events.iter().all(|event| !event.contains("file_analysis")
            && !event.contains("compilation_unit")
            && !event.contains("function_lattice")),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .all(|event| !event.contains("project_dispatch_components_for_inputs")),
        "a body-only edit keeps selected paths and declaration topology equal: {events:?}"
    );
}

#[test]
fn source_dispatch_bounds_use_original_paths_and_keep_dynamic_coverage_open() {
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let library = SourceFile::new(
        &db,
        "proc helper {value} {}".to_owned(),
        "tcl".to_owned(),
        Some("/source-bound/library.tcl".to_owned()),
    );
    let caller = SourceFile::new(
        &db,
        "source library.tcl".to_owned(),
        "tcl".to_owned(),
        Some("/source-bound/main.tcl".to_owned()),
    );
    let isolated = SourceFile::new(
        &db,
        "proc isolated {} {}".to_owned(),
        "tcl".to_owned(),
        Some("/source-bound/isolated.tcl".to_owned()),
    );
    let project = Project::with_token_configurations(
        &db,
        vec![library, caller, isolated],
        Some(vec![
            (library, config),
            (caller, config),
            (isolated, config),
        ]),
    );
    let finite = project_dispatch_components_for_inputs(&db, project).unwrap();
    assert!(finite.reach(1).unwrap().contains("::helper"));
    assert!(!finite.reach(1).unwrap().contains("::isolated"));
    let targets = file_source_targets_for_inputs(&db, caller, project).unwrap();
    let mut reporting = (*targets).clone();
    reporting[0].raw_path = "isolated.tcl".to_owned();
    reporting[0].is_literal = false;
    assert_eq!(
        conditional_source_links(caller.path(&db).as_deref(), &reporting),
        Some(BTreeSet::from(["/source-bound/library.tcl".to_owned()]))
    );
    reporting[0].original_source_load = None;
    assert!(conditional_source_links(caller.path(&db).as_deref(), &reporting).is_none());
    for source in [
        "source $path",
        "set path library.tcl; source $path",
        "source {*}$arguments",
        "source ~user/library.tcl",
        "source {}",
        "source external-unrepresented.tcl",
    ] {
        caller.set_text(&mut db).to(source.to_owned());
        assert!(
            project_dispatch_components_for_inputs(&db, project).is_none(),
            "{source}"
        );
        assert!(
            project_dispatch_components(&db, project).is_none(),
            "standalone {source}"
        );
        assert!(
            file_external_call_sites_for_inputs(&db, library, project).is_none(),
            "{source}"
        );
        let unknown = file_call_site_evidence(&db, caller, project);
        let helper = unknown
            .get("::helper")
            .expect("unknown external code may call any represented procedure");
        assert!(helper.opaque_caller);
        assert_eq!(helper.uniform_literal_at(0), None);
    }
    caller.set_text(&mut db).to("source library.tcl".to_owned());
    caller.set_path(&mut db).to(None);
    assert!(project_dispatch_components_for_inputs(&db, project).is_none());
}

#[test]
fn checked_project_projections_share_one_interpretation_and_recheck_changed_inputs() {
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let mut db = TclDatabase::with_event_logger(move |event| sink.lock().unwrap().push(event));
    let config = configuration(&db, 0);
    let file = SourceFile::new(
        &db,
        "proc helper {value} {return $value}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let project = Project::with_token_configurations(&db, vec![file], Some(vec![(file, config)]));
    let read = |db: &TclDatabase| {
        assert!(file_source_structure(db, file, config).is_some());
        let _ = project_token_inputs::file_token_facts_for_config(db, file, config);
        let _ = project_token_inputs::project_proc_var_index_for_inputs(db, project);
    };
    let count = || {
        std::mem::take(&mut *events.lock().unwrap())
            .iter()
            .filter(|event| event.contains("file_project_source_facts"))
            .count()
    };
    read(&db);
    assert_eq!(count(), 1, "all three projections share one source walk");
    read(&db);
    assert_eq!(count(), 0, "unchanged owners reuse the actual capture");
    file.set_text(&mut db)
        .to("proc helper {value} {return CHANGED}".to_owned());
    read(&db);
    assert_eq!(count(), 1, "a changed image gets one fresh interpretation");
    file.set_sidecar_stubs_epoch(&mut db).to(1);
    read(&db);
    assert_eq!(
        count(),
        1,
        "a changed sidecar epoch rechecks the shared input"
    );
    config.set_spec_pack_key(&mut db).to(u64::MAX - 359);
    assert!(file_source_structure(&db, file, config).is_none());
    assert!(
        project_token_inputs::file_token_facts_for_config(&db, file, config)
            .classes
            .is_empty()
    );
    assert!(project_token_inputs::project_proc_var_index_for_inputs(&db, project).is_empty());
    assert_eq!(count(), 1, "unavailable input withdraws every projection");
}
