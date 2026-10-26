// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn original(source: &str) -> (AnalysisResult, NativeWord) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        config,
    );
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let words = source_registry_words(source, &analysis, commands.last().unwrap()).unwrap();
    let word = words.operands()[1]
        .as_ref()
        .unwrap()
        .word()
        .unwrap()
        .clone();
    (analysis, word)
}

#[test]
fn original_selected_source_path_values_keep_root_and_bound_prefix_arguments() {

    // Genuine retained Logical source operation and original argument vector;
    // no source evaluation, host path or Native filename is issued.
    for (source, expected) in [
        ("set p [file join ROOT child]", "ROOT/child"),
        ("set p [file join ROOT]", "ROOT"),
        ("set p [file dirname ROOT/child]", "ROOT"),
        (
            "interp alias {} build {} file join ROOT; set p [build child]",
            "ROOT/child",
        ),
        (
            "rename file moved; set p [moved join ROOT child]",
            "ROOT/child",
        ),
    ] {
        let (analysis, word) = original(source);
        let expression = capture_source_path_expression(source, &analysis, &word).unwrap();
        assert_eq!(
            expression.evaluate(None, &|_| None).as_deref(),
            Some(expected),
            "{source}"
        );
        assert_eq!(expression.source_span(), word.word_span());
        assert!(expression.matches_analysis(&analysis));
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(!expression.matches_analysis(&changed));
        assert!(capture_source_path_expression(source, &changed, &word).is_none());
        let independently_analysed = original(source).0;
        assert!(!expression.matches_analysis(&independently_analysed));
    }
    let (analysis, word) = original("set p [file join $root child]");
    let expression =
        capture_source_path_expression("set p [file join $root child]", &analysis, &word).unwrap();
    assert!(expression.evaluate(None, &|_| None).is_none());
    assert_eq!(
        expression
            .evaluate(None, &|_| Some("ROOT".to_owned()))
            .as_deref(),
        Some("ROOT/child")
    );
}
