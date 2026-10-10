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
    assert_eq!(
        *project_proc_var_index_for_inputs(&db, project),
        actual.proc_roles
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
