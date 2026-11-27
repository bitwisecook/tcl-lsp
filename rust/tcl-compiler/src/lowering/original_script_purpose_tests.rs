// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn first_invocation(script: &Script) -> &CommandTokens {
    script
        .statements
        .iter()
        .find_map(Statement::tokens)
        .expect("retained original invocation")
}

#[test]
fn original_whole_and_zero_base_body_parsers_keep_distinct_bom_purposes() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Parser purpose and retained source ancestry; no Native entry or Normal store.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let context = input.context_registry();
    let registry = context.commands();
    let source = "\u{feff}unknown argument";
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut config = input.lexer_config();
        config.leading_bom = bom;
        config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let supplied = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let config = supplied.lexer_config();
        let mut whole =
            Lowerer::with_config(registry, config).with_resolved_analysis_input(supplied.clone());
        let module = whole.lower(source);
        let command = first_invocation(&module.top_level);
        assert_eq!(
            command.argv_texts[0],
            if bom == tcl_lexer::LeadingBom::Skip {
                "unknown"
            } else {
                "\u{feff}unknown"
            }
        );
        assert_eq!(
            command.argv[0].start(),
            if bom == tcl_lexer::LeadingBom::Skip {
                3
            } else {
                0
            }
        );
        assert_eq!(module.lexer_config, config);
        assert_eq!(module.source_metadata_input.as_ref(), Some(&supplied));

        let mut body =
            Lowerer::with_config(registry, config).with_resolved_analysis_input(supplied.clone());
        let script = body.lower_into_script(source, "::");
        let command = first_invocation(&script);
        assert_eq!(command.argv_texts[0], "\u{feff}unknown");
        assert_eq!(command.argv[0].start(), 0);
        assert_eq!(body.config, config);
        assert_eq!(body.metadata_input.as_ref(), Some(&supplied));
        if bom == tcl_lexer::LeadingBom::Skip {
            // This fresh local span space does not own a projected input.
            // Keep the actual parent input; mismatched metadata is terminal.
            assert!(
                body.source_metadata_input()
                    .retain()
                    .metadata_context_for_source(
                        registry,
                        config.nested(),
                        Some(supplied.unit_profile())
                    )
                    .is_none()
            );
        }
        assert!(crate::registry_invocation::proved_native_inline_operation(command).is_none());

        // An explicit non-BOM grammar override is retained for body parsing.
        let script = body.lower_into_script("puts \"${a{}\"", "::");
        let command = first_invocation(&script);
        assert_eq!(command.argv_texts, ["puts", "${a{}"]);
        assert!(
            body.eval_subst_nocommands_body("\u{feff}subst -nocommands {literal}")
                .is_none()
        );
        assert!(eval_list_literal_body("\u{feff}list item", config).is_none());
    }
}

#[test]
fn original_selected_nested_body_keeps_parent_metadata_for_known_command_advice() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // Existing original body inventory and metadata; no entered/Normal handler donation.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut config = input.lexer_config();
        config.leading_bom = bom;
        config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let supplied = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let context = supplied.context_registry();
        let registry = context.commands();
        let source = "eval {puts hi}";
        let identities =
            crate::realm::document_realm_bindings_with_resolved_input(source, &supplied, None);
        let parent = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            supplied.lexer_config(),
        )
        .pop()
        .unwrap();
        let original = identities
            .executed_script_for_word(parent.argv[1].span)
            .unwrap();
        let bindings = identities
            .source_bindings_ref()
            .selected_source(original)
            .unwrap();
        let mut lowerer = Lowerer::with_config(registry, supplied.lexer_config())
            .with_resolved_analysis_input(supplied.clone());
        lowerer.set_source_image_channel(&original.text);
        let script = lowerer.lower_into_script_with_bindings(
            original.try_text().unwrap(),
            original.base(),
            "::",
            bindings,
        );
        let command = first_invocation(&script);
        let site = command
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        assert_eq!(&site.source, &original.origin);
        assert_eq!(site.offset, original.base());
        assert_eq!(command.argv_texts, ["puts", "hi"]);
        let advice = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                registry, lowerer.invocation_metadata_context(), command).unwrap();
        assert_eq!(advice.unanimous_command_words().unwrap().command(), "puts");
        assert_eq!(lowerer.metadata_input.as_ref(), Some(&supplied));
        let mut changed_config = supplied.lexer_config();
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let changed = crate::analyser::ResolvedAnalysisInput::new(
            supplied.analyser_profile(),
            supplied.unit_profile(),
            supplied.context_registry(),
            changed_config,
        );
        assert!(
            crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                &changed,
                supplied.lexer_config(),
                Some(supplied.unit_profile())
            )
            .is_none()
        );
    }
}

#[test]
fn original_materialised_zero_base_body_keeps_its_selected_point_owner() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Actual original escaped body and selected derived source; no compiler/frame grant.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let context = input.context_registry();
    let registry = context.commands();
    let source = "eval \\uFEFFunknown";
    let identities =
        crate::realm::document_realm_bindings_with_resolved_input(source, &input, None);
    let parent = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    )
    .pop()
    .unwrap();
    let original = identities
        .executed_script_for_word(parent.argv[1].span)
        .expect("original evaluated script");
    assert_eq!(original.base(), 0);
    assert_eq!(
        original.mapping,
        tcl_names::origin::ExecutedScriptMapping::Materialised
    );
    let body = original.try_text().unwrap();
    assert_eq!(body, "\u{feff}unknown");
    let bindings = identities
        .source_bindings_ref()
        .selected_source(original)
        .unwrap();
    assert_eq!(bindings.source_origin(), Some(&original.origin));
    let mut lowerer = Lowerer::with_config(registry, input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    lowerer.set_source_image_channel(&original.text);
    let script = lowerer.lower_into_script_with_bindings(body, 0, "::", bindings);
    let command = first_invocation(&script);
    assert_eq!(command.argv_texts, ["\u{feff}unknown"]);
    let site = command
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .expect("actual derived point");
    assert_eq!(site.offset, 0);
    assert_eq!(&site.source, &original.origin);
    assert_eq!(site.source.source_image().bytes(), body.as_bytes());
    assert_eq!(
        script.executed_source.as_ref().unwrap().origin,
        original.origin
    );
    assert!(crate::registry_invocation::proved_native_inline_operation(command).is_none());
    assert_eq!(lowerer.config, input.lexer_config());
    assert_eq!(lowerer.metadata_input.as_ref(), Some(&input));

    // Equal derived text under another original parent has no retained point.
    let foreign_parent = tcl_names::command::CommandAllocationSite {
        source: std::sync::Arc::new(tcl_names::origin::SourceOriginId::authored(
            &std::sync::Arc::from("eval $other"),
        )),
        offset: 0,
    };
    let foreign =
        tcl_names::origin::ExecutedScriptSource::materialised(foreign_parent, vec![0], body);
    let foreign_bindings = identities
        .source_bindings_ref()
        .selected_source(&foreign)
        .unwrap();
    let refused = lowerer.lower_into_script_with_bindings(body, 0, "::", foreign_bindings);
    assert!(
        first_invocation(&refused)
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .is_none()
    );
}
#[test]
fn original_definer_targets_keep_literal_data_distinct_from_scalar_references() {
    for (target, expected) in [
        ("{$obj}", "::@objdefine@::$obj"),
        ("$obj", "::@objdefine@::obj"),
        ("${two words}", "::@objdefine@::two words"),
        ("{two words}", "::@objdefine@::two words"),
        ("{}", "::@objdefine@::"),
        ("{[obj]}", "::@objdefine@::[obj]"),
    ] {
        let source = format!("oo::objdefine {target} {{method m {{}} {{return VALUE}}}}");
        let analysis = crate::analyser::Analyser::new().analyse(&source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let module = lower_to_ir_with(
            Lowerer::with_config(
                input.borrowed_context_registry().commands(),
                input.lexer_config(),
            )
            .with_resolved_analysis_input(input.clone()),
            &source,
        );
        let method = module
            .methods
            .values()
            .find(|method| method.method_name == "m")
            .expect("actual conditional original definition body");
        assert_eq!(method.class_name, expected);
        assert!(method.definition_scope.is_some());
    }
    for target in [
        "$obj(key)",
        "${obj(key)}",
        "[pick]",
        "x$obj",
        "{nul\0receiver}",
    ] {
        let source = format!("oo::objdefine {target} {{method m {{}} {{return VALUE}}}}");
        let analysis = crate::analyser::Analyser::new().analyse(&source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let module = lower_to_ir_with(
            Lowerer::with_config(
                input.borrowed_context_registry().commands(),
                input.lexer_config(),
            )
            .with_resolved_analysis_input(input.clone()),
            &source,
        );
        assert!(module.methods.is_empty());
        assert!(module.oo_evidence.unretained_executable_roots);
    }
}
