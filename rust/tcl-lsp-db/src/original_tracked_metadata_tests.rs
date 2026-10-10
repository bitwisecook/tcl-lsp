// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The tracked document producers share one checked source input.

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
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/tracked-input349.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        format!(
            "speclib tracked_input349 1.0 {{\ncommand tracked_input349::read {{\narity 1\n{role}\n}}\n}}\n"
        ),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    packs
}

fn has_variable(tokens: &SemanticTokens, source: &str, value: &str) -> bool {
    let offset = u32::try_from(source.find(value).unwrap()).unwrap();
    let mut line = 0;
    let mut column = 0;
    tokens.data.chunks_exact(5).any(|token| {
        line += token[0];
        column = if token[0] == 0 {
            column + token[1]
        } else {
            token[1]
        };
        line == 0 && column == offset && token[3] == 2
    })
}

#[test]
fn tracked_source_units_share_checked_input_and_memoized_procedure_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A real Logical document supplies conditional source advice, no Native entry.
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let file = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let input = document_analysis_input(&db, file, config).unwrap();
    let unit = document_compilation_unit_for(&db, file, config).unwrap();
    let analysis = file_analysis_incremental(&db, file, config);
    assert_eq!(analysis.resolved_input.as_ref(), Some(input.as_ref()));
    assert_eq!(
        unit.ir_module.source_metadata_input.as_ref(),
        Some(input.as_ref())
    );
    assert_eq!(unit.ir_module.lexer_config, input.lexer_config());
    assert_eq!(
        unit.ir_module
            .dialect_profile
            .map(tcl_dialect::DialectProfile::cache_key),
        Some(input.unit_profile().cache_key()),
    );
    let function = unit
        .function("::subject")
        .expect("genuine Logical source procedure");
    let function_input = function.source_metadata_input().unwrap();
    assert!(Arc::ptr_eq(
        &function_input.context_registry(),
        &input.context_registry()
    ));
    assert_eq!(
        function.source_lexer_config().normalized(),
        function_input.lexer_config().normalized()
    );
    assert_eq!(*analysis, *file_analysis(&db, file, config));
    let diagnostics = compiler_check_diagnostics(&db, file, config);
    let branch = diagnostics
        .checks
        .iter()
        .find(|row| row.code == tcl_core_types::DiagCode::O100)
        .expect("the memoized source branch supplies conditional advice");
    assert!(branch.source_context.is_some());

    file.set_text(&mut db)
        .to(format!("# shifted declaration\n{source}"));
    let shifted = document_compilation_unit_for(&db, file, config).unwrap();
    assert_eq!(document_analysis_input(&db, file, config).unwrap(), input);
    let shifted_function = shifted.function("::subject").unwrap();
    assert_eq!(
        shifted_function.source_metadata_input(),
        function.source_metadata_input()
    );
    assert_eq!(
        shifted_function.source_lexer_config(),
        function.source_lexer_config()
    );
    assert!(
        compiler_check_diagnostics(&db, file, config)
            .checks
            .iter()
            .any(|row| row.code == tcl_core_types::DiagCode::O100)
    );
}

#[test]
fn tracked_document_input_reloads_actual_pack_roles_and_refuses_stale_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut db = TclDatabase::default();
    let packs = read_pack(true);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let config = configuration(&db, packs.key);
    let source = "tracked_input349::read retained";
    let file = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let input = document_analysis_input(&db, file, config).unwrap();
    let unit = document_compilation_unit_for(&db, file, config).unwrap();
    assert_eq!(
        unit.ir_module.source_metadata_input.as_ref(),
        Some(input.as_ref())
    );
    assert_eq!(
        input
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(has_variable(
        &semantic_tokens(&db, file, config),
        source,
        "retained"
    ));
    let stale = lexer_cfg_key(&db, "tcl8.4");
    assert!(compilation_unit_for_config(&db, file, stale, config).is_none());
    assert!(proc_taint_solve_for_config(&db, file, stale, config).is_none());

    let changed_packs = read_pack(false);
    let changed_registry =
        tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &changed_packs);
    config.set_spec_pack_key(&mut db).to(changed_packs.key);
    set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
    let changed_input = document_analysis_input(&db, file, config).unwrap();
    assert_ne!(changed_input, input);
    assert_eq!(
        changed_input
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        changed_registry.snapshot().semantic_key()
    );
    assert!(!has_variable(
        &semantic_tokens(&db, file, config),
        source,
        "retained"
    ));
    assert_eq!(
        file_analysis_incremental(&db, file, config)
            .resolved_input
            .as_ref(),
        Some(changed_input.as_ref())
    );
}

#[test]
fn tracked_document_input_refuses_unavailable_generation_before_any_source_walk() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut db = TclDatabase::default();
    let config = configuration(&db, u64::MAX - 349);
    let file = SourceFile::new(
        &db,
        "proc subject {} {puts retained}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let miss = document_analysis_input(&db, file, config).unwrap_err();
    assert_eq!(miss.overlay, config.spec_pack_key(&db));
    assert!(document_compilation_unit_for(&db, file, config).is_none());
    let cfg = lexer_cfg_key(&db, file.dialect(&db));
    assert!(proc_taint_solve_for_config(&db, file, cfg, config).is_none());
    let incremental = file_analysis_incremental(&db, file, config);
    assert_eq!(
        incremental.analysis_context_unavailable.as_ref(),
        Some(&miss)
    );
    assert!(incremental.resolved_input.is_none());
    assert!(incremental.command_invocations.is_empty());
    assert_eq!(*incremental, *file_analysis(&db, file, config));
    let diagnostics = compiler_check_diagnostics(&db, file, config);
    assert!(diagnostics.checks.is_empty() && diagnostics.optimisations.is_empty());

    config.set_spec_pack_key(&mut db).to(0);
    let restored = file_analysis_incremental(&db, file, config);
    assert!(restored.analysis_context_unavailable.is_none());
    assert!(restored.resolved_input.is_some());
    assert!(
        document_compilation_unit_for(&db, file, config)
            .unwrap()
            .function("::subject")
            .is_some()
    );
}

#[test]
fn tracked_document_input_keeps_keyed_availability_on_the_same_command_store() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A source lifecycle query supplies no appliance or TMM execution receipt.
    let mut db = TclDatabase::default();
    let config = configuration(&db, 0);
    let file = SourceFile::new(
        &db,
        "when CLIENTSSL_HANDSHAKE {SSL::c3d cert_lifespan 24}".to_owned(),
        "f5-irules".to_owned(),
        None,
    );
    let older = document_analysis_input(&db, file, config).unwrap();
    let older_unit = document_compilation_unit_for(&db, file, config).unwrap();
    assert_eq!(
        older_unit.ir_module.source_metadata_input.as_ref(),
        Some(older.as_ref())
    );
    assert_eq!(
        older
            .availability_context()
            .placement_floor("f5-irules-cmds")
            .map(|version| version.as_str()),
        Some("15.0.0")
    );
    assert!(
        file_analysis_incremental(&db, file, config)
            .diagnostics
            .iter()
            .any(|row| row.code == tcl_core_types::DiagCode::W150)
    );

    config
        .set_bigip_version(&mut db)
        .to(Some("21.1.0".to_owned()));
    let current = document_analysis_input(&db, file, config).unwrap();
    assert_ne!(current, older);
    assert!(Arc::ptr_eq(
        current.borrowed_context_registry().commands(),
        older.borrowed_context_registry().commands()
    ));
    assert_eq!(
        current
            .availability_context()
            .placement_floor("f5-irules-cmds")
            .map(|version| version.as_str()),
        Some("21.1.0")
    );
    assert_eq!(
        document_compilation_unit_for(&db, file, config)
            .unwrap()
            .ir_module
            .source_metadata_input
            .as_ref(),
        Some(current.as_ref())
    );
    let analysis = file_analysis_incremental(&db, file, config);
    assert_eq!(analysis.resolved_input.as_ref(), Some(current.as_ref()));
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|row| row.code == tcl_core_types::DiagCode::W150)
    );
}
