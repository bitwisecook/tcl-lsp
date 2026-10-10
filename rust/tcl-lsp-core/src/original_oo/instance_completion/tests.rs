// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use tcl_compiler::analyser::Analyser;

fn cursor(source: &str, marker: &str) -> u32 {
    u32::try_from(source.rfind(marker).unwrap() + marker.len()).unwrap()
}

fn clear_reports(analysis: &mut AnalysisResult) {
    analysis.all_classes.clear();
    analysis.superseded_classes.clear();
    analysis.global_scope.classes.clear();
    analysis.instance_classes.clear();
    analysis.created_instance_commands.clear();
}

#[test]
fn original_instance_candidates_join_genuine_construction_without_reporting_maps() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            "oo::class create C {method longMethod {} {}}; set o [C new]; $o longMet",
            "oo::class create C {method longMethod {} {}}; C create object; rename object moved; interp alias {} call {} moved; call longMet",
            "oo::class create C {method longMethod {} {}}; interp alias {} make {} C new; rename set store; store o [make]; $o longMet",
        ] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            clear_reports(&mut analysis);
            let at = u32::try_from(source.len()).unwrap();
            let receiver = instance_completion_source(&analysis, source, at).expect(source);
            let InstanceCompletionWords::Conditional(receipt) = &receiver.words else {
                panic!("genuine conditional constructor receipt");
            };
            assert_eq!(receiver.class.name_input().bytes(), b"C");
            assert_eq!(receipt.original_words(), receiver.original_words());
            assert_eq!(
                receipt
                    .instance()
                    .class_declaration()
                    .source_class(&analysis)
                    .unwrap(),
                receiver.class
            );
            assert!(!receipt.obligations().is_empty());
            assert_eq!(
                instance_method_completion_record(&analysis, source, at).unwrap(),
                receiver.class
            );
        }
    }
}

#[test]
fn original_instance_method_geometry_keeps_captured_selectors_and_later_arguments_separate() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create C {method longMethod args {}}; C create object; interp alias {} fixed {} object longMethod; fixed argument; object longMet argument; object \\\n\tlongMet";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let captured = cursor(source, "fixed argument");
    let receiver = instance_completion_source(&analysis, source, captured).unwrap();
    assert_eq!(receiver.argument_input(0).unwrap().bytes(), b"longMethod");
    assert!(!receiver.is_written_argument(0));
    assert!(instance_method_completion_record(&analysis, source, captured).is_none());
    assert!(
        instance_method_completion_record(
            &analysis,
            source,
            cursor(source, "object longMet argument")
        )
        .is_none()
    );
    assert!(
        instance_method_completion_record(&analysis, source, cursor(source, "object longMet"))
            .is_some()
    );
    let gap = cursor(source, "object \\\n\t");
    assert!(instance_method_completion_record(&analysis, source, gap).is_some());
    for ending in ["object\n", "object; ", "object # comment"] {
        let source = format!("oo::class create C {{}}; C create object; {ending}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        assert!(
            instance_method_completion_record(
                &analysis,
                &source,
                u32::try_from(source.len()).unwrap()
            )
            .is_none(),
            "{ending}"
        );
    }
}

#[test]
fn original_instance_selector_edits_keep_whole_grouped_words_without_identity_grants() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create C {method longMethod {} {}}; set o [C new]; $o {longMet}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    clear_reports(&mut analysis);
    let at = u32::try_from(source.len()).unwrap();
    let receiver = instance_completion_source(&analysis, source, at).unwrap();
    let selector = receiver.argument_word(0).unwrap();
    let (span, replacement) = receiver
        .selector_replacement(
            source,
            at,
            b"longMethod",
            receiver.class.name_input().policy(),
        )
        .unwrap();
    assert_eq!(span, selector.word_span());
    assert_eq!(source.get(span.as_range()), Some("{longMet}"));
    let mut edited = source.to_owned();
    edited.replace_range(span.as_range(), &replacement);
    assert_eq!(
        &edited[..span.start() as usize],
        &source[..span.start() as usize]
    );
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(&edited),
        Span::new(0, u32::try_from(edited.len()).unwrap()),
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    let words = &plan.commands.last().unwrap().words;
    assert_eq!(words.len(), 2);
    let key = SignatureSourceNameKey::from_original_native_word(
        &words[1],
        tcl_syntax::word_rules::WordValueRules::from_config(&words[1].config()),
        receiver.class.name_input().policy(),
    )
    .unwrap();
    assert_eq!(key.bytes(), b"longMethod");
}

#[test]
fn original_future_instance_candidates_do_not_borrow_frames_cells_or_navigation() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source =
        "proc p {} {set o [C new]; $o longMethod}; oo::class create C {method longMethod {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let at = cursor(source, "$o longMethod");
    let receiver = instance_completion_source(&analysis, source, at).unwrap();
    let InstanceCompletionWords::Conditional(receipt) = &receiver.words else {
        panic!("future source carrier");
    };
    assert!(receipt.source_body().is_some());
    assert!(receipt.obligations().contains(&tcl_compiler::command_binding::SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability));
    let command = crate::source_structure::SourceStructure::capture(
        source,
        Some(&analysis),
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap()
    .commands
    .into_iter()
    .find(|command| {
        command
            .argv
            .first()
            .is_some_and(|head| head.span.start() == receipt.site().offset)
    })
    .unwrap();
    assert!(crate::receiver_identity::class_at_command_head(&analysis, source, &command).is_none());
    assert!(crate::receiver_identity::method_at_command(&analysis, source, &command).is_none());
    for source in [
        "oo::class create C {}; set o [C new]; proc p {} {$o longMet}",
        "oo::class create C {}; proc p {} {set o [C new]}; $o longMet",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            instance_completion_source(&analysis, source, cursor(source, "$o longMet")).is_none()
        );
    }
}

#[test]
fn original_instance_candidates_refuse_stale_input_terminal_mutations_and_detached_reports() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create C {method longMethod {} {}}; set o [C new]; $o longMet";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let at = u32::try_from(source.len()).unwrap();
    assert!(instance_completion_source(&analysis, source, at).is_some());
    assert!(instance_completion_source(&analysis, &format!("#{source}"), at).is_none());
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    assert!(instance_completion_source(&stale, source, at).is_none());
    stale = analysis.clone();
    stale.original_class_metadata.clear();
    assert!(instance_completion_source(&stale, source, at).is_none());
    let foreign = crate::profile_for_dialect("jim");
    stale = analysis;
    stale.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        foreign,
        foreign,
        tcl_registry::model::ingress::context_for_profile(foreign),
        stale.body_lexer_config.unwrap(),
    ));
    assert!(instance_completion_source(&stale, source, at).is_none());
    for source in [
        "oo::class create C {}; set o prefix[C new]; $o longMet",
        "oo::class create C {}; set o [C new]; set o replacement; $o longMet",
        "oo::class create C {}; C create object; rename object {}; object longMet",
        "oo::class create C {}; C create object; proc object args {}; object longMet",
        "unknown longMet",
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis
            .instance_classes
            .insert("unknown".to_owned(), "::C".to_owned());
        analysis
            .created_instance_commands
            .insert("unknown".to_owned());
        assert!(
            instance_completion_source(&analysis, source, u32::try_from(source.len()).unwrap())
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_instance_candidates_keep_opaque_canonical_class_inputs_distinct() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = r"oo::class create C\uD800 {method left {} {}}; oo::class create C\uD801 {method right {} {}}; set o [C\uD800 new]; $o left";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    clear_reports(&mut analysis);
    let receiver =
        instance_completion_source(&analysis, source, u32::try_from(source.len()).unwrap())
            .unwrap();
    assert_eq!(receiver.class.name_input().bytes(), b"C\xed\xa0\x80");
    assert!(
        analysis
            .original_class_declarations()
            .any(|record| record.name_input().bytes() == b"C\xed\xa0\x81"
                && record.declaration_site() != receiver.class.declaration_site())
    );
}

#[test]
fn original_instance_operand_projection_never_borrows_native_purpose_from_a_class() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create C {}; proc p {} {set o [C new]; $o ping}";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        LexerConfig::for_file_grammar(profile.grammar),
    );
    let logical = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let offset = u32::try_from(source.find("$o ping").unwrap()).unwrap();
    let at = cursor(source, "$o ping");
    assert!(
        tcl_compiler::registry_invocation::source_structure::source_class_instance_words_at(
            source, &logical, offset
        )
        .is_none()
    );
    assert!(instance_completion_source(&logical, source, at).is_none());
    let native = Analyser::new().analyse(source, "tcl9.0");
    let receiver = instance_completion_source(&native, source, at).unwrap();
    let InstanceCompletionWords::Conditional(receipt) = &receiver.words else {
        panic!("independently selected Native source receipt");
    };
    assert_eq!(
        receipt
            .argument_input(0)
            .unwrap()
            .native_input()
            .unwrap()
            .bytes(),
        b"ping"
    );
    assert_eq!(receiver.argument_input(0).unwrap().bytes(), b"ping");
    // Equal source bytes do not let the Logical realm borrow this Native input.
    let mut foreign = logical;
    foreign.resolved_input = native.resolved_input.clone();
    assert!(instance_completion_source(&foreign, source, at).is_none());
}
