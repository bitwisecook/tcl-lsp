// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::{TokenKind, full_with_cu_and_analysis, tests::decode_semantic};
use std::sync::Arc;
use tcl_compiler::{
    analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput},
    compilation_unit::{CompilationUnit, UnitBuildOptions},
};

fn analyse(source: &str, environment: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let actual =
        tcl_registry::model::ingress::resolve_environment(environment).default_context_registry();
    let context =
        Arc::new(actual.with_command_store(driver.commands().snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "presentation-only")
}

fn method_at(
    source: &str,
    analysis: &AnalysisResult,
    cu: Option<&CompilationUnit>,
    needle: &str,
) -> bool {
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        analysis.resolved_registry().unwrap(),
        cu,
        Some(analysis),
    ));
    let offset = u32::try_from(source.rfind(needle).unwrap()).unwrap();
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    tokens.iter().any(|&(line, column, _, kind, _)| {
        line == position.line
            && column == position.character.get()
            && kind == TokenKind::Method as u32
    })
}

#[test]
fn original_receiver_heads_keep_canonical_class_and_actual_availability() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    for source in [
        "oo::class create C {method ping {} {}}; [::C new] ping",
        "oo::class create C {method ping {} {}}; interp alias {} make {} C new; [make] ping",
        "oo::class create C {method ping {} {}}; rename C Held; [Held new] ping",
    ] {
        let current = analyse(source, "tcl8.6");
        assert!(method_at(source, &current, None, "ping"), "{source}");
        let unavailable = analyse(source, "tcl8.4");
        assert!(!method_at(source, &unavailable, None, "ping"), "{source}");
    }
    for source in [
        "oo::class create C {method ping {} {}}; proc C args {}; [C new] ping",
        "oo::class create C {method ping {} {}}; [C new; C new] ping",
        "oo::class create C {method ping {} {}}; [C new]suffix ping",
        "oo::class create C {}; [C new] new",
    ] {
        let selector = source.rsplit_once(' ').unwrap().1;
        assert!(
            !method_at(source, &analyse(source, "tcl8.6"), None, selector),
            "{source}"
        );
    }
}

#[test]
fn original_registry_and_collection_receivers_keep_selected_child_operands() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "# tcl-lsp: requires Tk\n[listbox .l] curselection";
    let current = analyse(source, "tcl8.6");
    assert!(method_at(source, &current, None, "curselection"));
    let unknown = "# tcl-lsp: requires Tk\nproc listbox args {}; [listbox .l] curselection";
    assert!(!method_at(
        unknown,
        &analyse(unknown, "tcl8.6"),
        None,
        "curselection"
    ));

    let collision =
        "# tcl-lsp: requires Tk\noo::class create listbox {}; [listbox new] curselection";
    assert!(!method_at(
        collision,
        &analyse(collision, "tcl8.6"),
        None,
        "curselection"
    ));

    let source = "oo::class create C {method ping {} {}}; dict set items key [C new]; [dict get $items key] ping";
    let analysis = analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    assert!(tcl_compiler::object_types::object_collection_classes(&unit).contains_key("items"));
    assert!(method_at(source, &analysis, Some(&unit), "ping"));
    let nested = source.replace("get $items key]", "get $items key inner]");
    assert!(!method_at(
        &nested,
        &analyse(&nested, "tcl8.6"),
        None,
        "ping"
    ));
}

#[test]
fn original_receiver_class_receipt_cannot_be_replaced_by_foreign_hierarchy_metadata() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "oo::class create C {method ping {} {}}; [C new] ping";
    let current = analyse(source, "tcl8.6");
    assert!(method_at(source, &current, None, "ping"));
    let foreign = analyse(
        "oo::class create C {method ping {} {return FOREIGN}}",
        "tcl8.6",
    );
    assert!(
        foreign
            .class_hierarchy()
            .method_target("::C", "ping")
            .is_some()
    );
    let tokens = decode_semantic(&super::super::full_with_cu_and_facts(
        source,
        current.resolved_profile().unwrap(),
        current.resolved_registry().unwrap(),
        None,
        super::super::WorkspaceTokenFacts {
            classes: Some(foreign.class_hierarchy()),
            proc_roles: None,
            named_instances: None,
            analysis: Some(&current),
        },
    ));
    let offset = u32::try_from(source.rfind("ping").unwrap()).unwrap();
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    assert!(!tokens.iter().any(|&(line, column, _, kind, _)| {
        line == position.line
            && column == position.character.get()
            && kind == TokenKind::Method as u32
    }));
}

#[test]
fn original_receiver_heads_keep_actual_leading_bom_policy() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "\u{feff}oo::class create C {method ping {} {}}; [C new] ping";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (leading_bom, expected) in [
        (tcl_lexer::LeadingBom::Skip, true),
        (tcl_lexer::LeadingBom::Content, false),
    ] {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "presentation-only");
        assert_eq!(method_at(source, &analysis, None, "ping"), expected);
    }
}

#[test]
fn original_receiver_public_tokens_refuse_populated_unavailable_and_stale_source() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "oo::class create C {method ping {} {}}; [C new] ping";
    let analysis = analyse(source, "tcl8.6");
    assert!(method_at(source, &analysis, None, "ping"));
    for withdrawn in [
        {
            let mut next = analysis.clone();
            next.resolved_input = None;
            next
        },
        {
            let mut next = analysis.clone();
            next.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".into(),
                overlay: 999,
            });
            next
        },
        {
            let mut next = analysis.clone();
            next.body_lexer_config = Some(tcl_lexer::LexerConfig::for_profile(Some(
                crate::profile_for_dialect("jim"),
            )));
            next
        },
    ] {
        assert!(
            full_with_cu_and_analysis(
                source,
                tcl_dialect::DialectProfile::plain_tcl(),
                analysis.resolved_registry().unwrap(),
                None,
                Some(&withdrawn)
            )
            .data
            .is_empty()
        );
    }
    assert!(
        full_with_cu_and_analysis(
            &source.replace("new]", "create other]"),
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis)
        )
        .data
        .is_empty()
    );
}
