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
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
