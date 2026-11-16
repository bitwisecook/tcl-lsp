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

#[test]
fn native_minifier_uses_separate_authentic_body_syntax_without_native_word_admission() {
    let source = "proc fact {n} {\n# base case\nif {$n <= 1} { return 1 }\nreturn [expr {$n * [fact [expr {$n - 1}]]}]\n}\nputs [fact 5]\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default();
    let analysis = super::analysis_for_profile(source, profile, &registry);
    assert!(!analysis.allows_lexical_declaration_advice());
    let config = analysis.body_lexer_config.unwrap();
    let structure = crate::source_structure::SourceStructure::capture_for_syntax_compaction(
        source, &analysis, config,
    )
    .unwrap();
    let command = structure
        .commands
        .iter()
        .find(|command| {
            command
                .argv
                .first()
                .is_some_and(|word| word.span.start() == 28)
        })
        .expect("authentic factorial if command");
    let inventory = structure.compiler_body_syntax.as_ref().unwrap();
    assert!(
        inventory.words(source, &analysis, command).is_none(),
        "source syntax cannot mint a Native Selected carrier"
    );
    let operands = inventory
        .expression_operands(source, &analysis, command)
        .expect("independent compiler-source Expr proof");
    assert_ne!(operands, [] as [(usize, tcl_lexer::NativeWord); 0]);
    assert!(
        inventory
            .script_bodies(source, &analysis, command)
            .is_some_and(|bodies| !bodies.is_empty())
    );
    let result = super::minify_with_analysis(source, &analysis, super::MinifyOptions::default());
    assert_eq!(
        result.source,
        "proc fact {n} {if {$n<=1} {return 1};return [expr {$n*[fact [expr {$n-1}]]}]};puts [fact 5]"
    );
    let foreign = super::analysis_for_profile(source, profile, &registry);
    assert!(
        inventory
            .expression_operands(source, &foreign, command)
            .is_none()
    );
    let stale = source.replace("return 1", "return 9");
    assert!(
        inventory
            .expression_operands(&stale, &analysis, command)
            .is_none()
    );
    let mut changed_config = analysis.clone();
    changed_config.body_lexer_config = None;
    assert!(
        inventory
            .expression_operands(source, &changed_config, command)
            .is_none()
    );
}

#[test]
fn native_static_quote_edits_keep_whole_script_word_boundaries() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default();
    for (source, expected) in [
        ("proc p {} \"return\"", "proc p {} return"),
        ("puts \"\\u0042\"", "puts \\u0042"),
        ("puts \"café\"", "puts café"),
        ("puts \"﻿42\"", "puts \"﻿42\""),
        ("puts \"42\"", "puts 42"),
        ("puts \"\"", "puts \"\""),
        ("puts \"a;b\"", "puts \"a;b\""),
        ("puts \"two words\"", "puts \"two words\""),
        ("puts \"#comment\"", "puts \"#comment\""),
        ("puts \"{data}\"", "puts \"{data}\""),
        ("puts \"$dynamic\"", "puts \"$dynamic\""),
        ("puts \"[list 42]\"", "puts \"[list 42]\""),
        ("puts {*}\"42\"", "puts {*}\"42\""),
        ("puts \"42", "puts \"42"),
    ] {
        let analysis = if source == "puts \"\u{feff}42\"" {
            // The refusal control must actually select whole-input BOM skipping.
            // Content is exercised independently by the positive control below.
            let config = tcl_lexer::LexerConfig {
                leading_bom: tcl_lexer::LeadingBom::Skip,
                ..tcl_lexer::LexerConfig::for_profile(Some(profile))
            };
            let generation = crate::context_for_dialect_profile(profile);
            let context = std::sync::Arc::new(
                generation.with_command_store(registry.snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            super::analysis_with_input(source, &input)
        } else {
            super::analysis_for_profile(source, profile, &registry)
        };
        assert!(!analysis.allows_lexical_declaration_advice());
        let result =
            super::minify_with_analysis(source, &analysis, super::MinifyOptions::default());
        assert_eq!(result.source, expected, "{source}");
    }
}

#[test]
fn native_static_quote_edits_retain_the_actual_bom_configuration() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Content,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let source = "puts \"\u{feff}42\"";
    let analysis = super::analysis_with_input(source, &input);
    assert!(!analysis.allows_lexical_declaration_advice());
    let result = super::minify_with_analysis(source, &analysis, super::MinifyOptions::default());
    assert_eq!(result.source, "puts \u{feff}42");
}

#[test]
fn native_syntax_compaction_uses_selected_original_expression_gaps() {
    // Decision 2026-10-22T13:17Z: syntax gaps need no pure-value rewrite.
    let registry = tcl_registry::CommandRegistry::build_default();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for (source, expected) in [
        ("expr { 1 + 2 * 3 }\n", "expr {1+2*3}"),
        ("expr {(1 + 2) * 3}\n", "expr {(1+2)*3}"),
        ("expr {1 / 0}\n", "expr {1/0}"),
        ("expr {1 +}\n", "expr {1 +}"),
        ("expr {$a + 1}\n", "expr {$a+1}"),
        ("expr {abs( 1 )}\n", "expr {abs(1)}"),
        (
            "trace add execution expr enter callback; expr {1 + 2}\n",
            "trace add execution expr enter callback;expr {1 + 2}",
        ),
    ] {
        let analysis = super::analysis_for_profile(source, profile, &registry);
        let result =
            super::minify_with_analysis(source, &analysis, super::MinifyOptions::default());
        assert_eq!(result.source, expected, "{source}");
    }
}

#[test]
fn native_compaction_uses_original_local_store_and_complete_edit_coverage() {
    let source = "proc p {} {set longVariable VALUE; return $longVariable}";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default();
    let analysis = super::analysis_for_profile(source, profile, &registry);
    assert!(!analysis.allows_lexical_declaration_advice());
    let result = super::minify_with_analysis(
        source,
        &analysis,
        super::MinifyOptions {
            tier: super::MinifyTier::Compact,
            ..super::MinifyOptions::default()
        },
    );
    assert_eq!(result.source, "proc p {} {set a VALUE;return $a}");
}

#[test]
fn native_compaction_uses_fixed_formal_owner_and_original_edit_coverage() {
    let source = "proc p {unused longVariable} {return $longVariable}";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default();
    let analysis = super::analysis_for_profile(source, profile, &registry);
    assert!(!analysis.allows_lexical_declaration_advice());
    let result = super::minify_with_analysis(
        source,
        &analysis,
        super::MinifyOptions {
            tier: super::MinifyTier::Compact,
            ..super::MinifyOptions::default()
        },
    );
    assert_eq!(result.source, "proc p {unused a} {return $a}");
    let changed = format!("{source}\ninfo args p");
    let analysis = super::analysis_for_profile(&changed, profile, &registry);
    let result = super::minify_with_analysis(
        &changed,
        &analysis,
        super::MinifyOptions {
            tier: super::MinifyTier::Compact,
            ..super::MinifyOptions::default()
        },
    );
    assert!(result.source.contains("longVariable"));
    assert!(
        result
            .refusals
            .contains(&super::MinifyRefusal::MissingAlphaRenameContract)
    );
}
use super::*;
use crate::minify::SymbolMap;

fn analyse(source: &str, dialect: &str) -> AnalysisResult {
    Analyser::new().analyse(source, dialect)
}

#[test]
fn original_syntax_compaction_keeps_opaque_words_nested_scripts_and_expansion() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let source = "# comment\n  proc p\\uD800 {} { set x  1; return $x }\n  puts  ${x}  [list {# data} café]\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let analysis = analyse(source, dialect);
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        assert!(
            result.refusals.is_empty(),
            "{dialect}: {:?}",
            result.refusals
        );
        assert_eq!(
            result.source,
            "proc p\\uD800 {} {set x 1;return $x};puts ${x} [list {# data} café]"
        );
        let again = minify_with_analysis(
            &result.source,
            &analyse(&result.source, dialect),
            MinifyOptions::default(),
        );
        assert_eq!(again.source, result.source);
    }
    let expansion = "  {*}{puts p\\uD800}  café\n";
    let result = minify_with_analysis(
        expansion,
        &analyse(expansion, "tcl8.6"),
        MinifyOptions::default(),
    );
    assert_eq!(
        result.refusals,
        [] as [crate::minify::original::MinifyRefusal; 0]
    );
    assert_eq!(result.source, "{*}{puts p\\uD800} café");
}

#[test]
fn original_recursive_minification_uses_only_current_authenticated_script_regions() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let source = "proc p {argument} {\n    set literal {  # data  }\n    if {1} {\n        return [list  $argument  $literal]\n    }\n}\np VALUE\n";
        let mut analysis = analyse(source, dialect);
        analysis.all_procs.clear();
        analysis.command_invocations.clear();
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        assert!(
            result.refusals.is_empty(),
            "{dialect}: {:?}",
            result.refusals
        );
        assert_eq!(
            result.source,
            "proc p {argument} {set literal {  # data  };if {1} {return [list $argument $literal]}};p VALUE",
            "{dialect}"
        );
        let shadowed = "proc if {args} {}\nif {1} {\n  set opaque  1\n}\n";
        let result = minify_with_analysis(
            shadowed,
            &analyse(shadowed, dialect),
            MinifyOptions::default(),
        );
        assert!(
            result.refusals.is_empty(),
            "{dialect}: {:?}",
            result.refusals
        );
        assert_eq!(
            result.source, "proc if {args} {};if {1} {\n  set opaque  1\n}",
            "{dialect}"
        );
        let quoted = r#"proc p {} "set local \u0031""#;
        let result =
            minify_with_analysis(quoted, &analyse(quoted, dialect), MinifyOptions::default());
        assert_eq!(
            result.source, quoted,
            "noncontiguous cooked body remains unchanged for {dialect}"
        );
    }
}

#[test]
fn original_hosted_syntax_compaction_preserves_zero_gap_word_and_bracket_boundaries() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let source = "  {list}{A}{}[list {C}{D}{}]\n";
    let analysis = analyse(source, "f5-irules");
    let config = analysis.body_lexer_config.unwrap();
    assert!(config.irules_brace_separator);
    assert!(analysis.has_original_vendor_source_names());
    assert!(!analysis.allows_lexical_declaration_advice());
    let original = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    assert_eq!(original.commands[0].words.len(), 4);
    assert_eq!(
        original.commands[0].words[0].span().end(),
        original.commands[0].words[1].span().start()
    );
    assert_eq!(
        original.commands[0].words[2].span().end(),
        original.commands[0].words[3].span().start()
    );
    let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
    assert!(result.refusals.is_empty(), "{:?}", result.refusals);
    assert_eq!(result.source, "{list} {A} {} [list {C} {D} {}]");
    let again = minify_with_analysis(
        &result.source,
        &analyse(&result.source, "f5-irules"),
        MinifyOptions::default(),
    );
    assert!(again.refusals.is_empty(), "{:?}", again.refusals);
    assert_eq!(again.source, result.source);
}

#[test]
fn original_minifier_refuses_naming_insertion_and_keywords_without_promoting_isolation() {
    // Implementation contract: naming.minifier.original-naming-permission
    // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
    let source = "proc longname {longvar} {return $longvar}\nset existing 1\ntrace add variable existing write observe\nlongname 7\n";
    let mut analysis = analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.global_scope.variables.clear();
    analysis.dialect = "f5-irules".to_owned();
    for isolated in [false, true] {
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Aggressive,
                isolated,
                abbreviations: true,
            },
        );
        assert_eq!(result.symbol_map, SymbolMap::default());
        assert_eq!(result.optimisations_applied, 0);
        assert_eq!(
            result.refusals,
            vec![
                MinifyRefusal::MissingAlphaRenameContract,
                MinifyRefusal::MissingAliasInsertionContract,
                MinifyRefusal::MissingSemanticRewriteContract,
                MinifyRefusal::MissingKeywordSelectionContract
            ]
        );
        assert!(
            result
                .source
                .starts_with("proc longname {longvar} {return $longvar};set existing 1;")
        );
        assert!(!result.source.starts_with("set a "));
    }
}

#[test]
fn original_scalar_minification_requires_equivalence_and_complete_edit_geometry() {
    // Implementation contract: naming.minifier.original-scalar-body-alpha
    // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for name in ["longargument", "longé😀"] {
            let read = if name.is_ascii() {
                format!("${name}")
            } else {
                format!("${{{name}}}")
            };
            let source = format!("proc publicname {{{name}}} {{return {read}}}\n");
            let mut analysis = analyse(&source, dialect);
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.dialect = "f5-irules".to_owned();
            let declaration = analysis.original_procedure_declarations().next().unwrap();
            let receipt = analysis
                .original_scalar_body_alpha_rename(
                    declaration,
                    b"a",
                    analysis.resolved_registry().unwrap(),
                )
                .unwrap_or_else(|| {
                    panic!("independent scalar equivalence missing: {dialect}: {name}")
                });
            assert!(receipt.matches_source(
                &SourceImage::document(&source),
                analysis.body_lexer_config.unwrap()
            ));
            let options = MinifyOptions {
                tier: MinifyTier::Compact,
                ..MinifyOptions::default()
            };
            let result = minify_with_analysis(&source, &analysis, options);
            assert!(
                result.refusals.is_empty(),
                "{dialect}: {name}: {:?}",
                result.refusals
            );
            let expected = if read.starts_with("${") {
                "proc publicname {a} {return ${a}}"
            } else {
                "proc publicname {a} {return $a}"
            };
            assert_eq!(result.source, expected);
            assert_eq!(result.optimisations_applied, 1);
            assert_eq!(result.symbol_map.variables["::publicname"][name], "a");
            assert!(result.symbol_map.procs.is_empty());
            let mut missing_geometry = analysis.clone();
            missing_geometry.original_variable_symbols.clear();
            let unavailable = minify_with_analysis(&source, &missing_geometry, options);
            assert!(
                unavailable
                    .refusals
                    .contains(&MinifyRefusal::MissingAlphaRenameContract)
            );
            assert_eq!(unavailable.symbol_map, SymbolMap::default());
        }
    }
}

#[test]
fn original_scalar_minification_keeps_isolation_and_other_pass_permissions_separate() {
    // Implementation contract: naming.minifier.original-scalar-body-alpha
    // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
    // Implementation contract: naming.minifier.original-naming-permission
    // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
    let source = "proc publicname {longargument} {return $longargument}\n";
    let analysis = analyse(source, "tcl8.6");
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Aggressive,
            isolated: true,
            abbreviations: true,
        },
    );
    assert_eq!(result.source, "proc publicname {a} {return $a}");
    assert_eq!(result.optimisations_applied, 1);
    assert!(result.symbol_map.procs.is_empty());
    assert_eq!(
        result.refusals,
        vec![
            MinifyRefusal::MissingAlphaRenameContract,
            MinifyRefusal::MissingAliasInsertionContract,
            MinifyRefusal::MissingSemanticRewriteContract,
            MinifyRefusal::MissingKeywordSelectionContract
        ]
    );
}

#[test]
fn original_scalar_minification_declines_observers_stores_and_open_source() {
    // Implementation contract: naming.minifier.original-scalar-body-alpha
    // docs/design/analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md
    for source in [
        "proc p {longargument} {return [info locals]}\n",
        "proc p {longargument} {set local $longargument; return $local}\n",
        "proc p {longargument} {upvar 0 longargument alias; return $alias}\n",
        "proc p {longargument} {trace add variable longargument read callback; return $longargument}\n",
        "proc p {longargument} {return x$longargument}\n",
        "proc p {{longargument DEFAULT}} {return $longargument}\n",
        "proc p {args} {return $args}\n",
        "proc p {longargument} {return $longargument}\ninfo args p\n",
        "proc p {longargument} {return $longargument}\nproc return {args} {}\n",
    ] {
        let analysis = analyse(source, "tcl8.6");
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                ..MinifyOptions::default()
            },
        );
        assert!(
            result
                .refusals
                .contains(&MinifyRefusal::MissingAlphaRenameContract),
            "{source}"
        );
        assert_eq!(result.symbol_map, SymbolMap::default(), "{source}");
        assert_eq!(result.optimisations_applied, 0, "{source}");
    }
}

#[test]
fn original_minifier_declines_stale_images_full_configuration_changes_and_malformed_tail() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let source = "set café 1\nputs $café\n";
    let mut analysis = analyse(source, "tcl8.6");
    let stale = format!("# changed\n{source}");
    let result = minify_with_analysis(&stale, &analysis, MinifyOptions::default());
    assert_eq!(result.source, stale);
    assert_eq!(result.refusals, vec![MinifyRefusal::StaleSource]);
    let mut config = analysis.body_lexer_config.unwrap();
    config.strict_quoting = !config.strict_quoting;
    analysis.body_lexer_config = Some(config);
    assert_eq!(
        minify_with_analysis(source, &analysis, MinifyOptions::default()).refusals,
        vec![MinifyRefusal::StaleSource]
    );
    let malformed = "puts first\nset x {\n";
    let result = minify_with_analysis(
        malformed,
        &analyse(malformed, "tcl8.6"),
        MinifyOptions::default(),
    );
    assert_eq!(result.source, malformed);
    assert!(
        result
            .refusals
            .contains(&MinifyRefusal::IncompleteOriginalSyntax)
    );
}

#[test]
fn original_minifier_refuses_unavailable_context_with_retained_source_and_input() {
    let source = "expr { 3 + 4 }\n";
    let analysis = analyse(source, "tcl8.6");
    let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
    assert_ne!(result.source, source);

    let mut unavailable = analysis.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".into(),
        overlay: u64::MAX - 405,
    });
    let result = minify_with_analysis(source, &unavailable, MinifyOptions::default());
    assert_eq!(result.source, source);
    assert_eq!(result.refusals, vec![MinifyRefusal::MissingAnalysisContext]);
}

#[test]
fn original_minifier_holds_custom_profile_and_registry_instead_of_resolving_its_label() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let mut profile = crate::profile_for_dialect("tcl8.4").clone();
    profile.name = "f5-irules";
    let profile = profile.intern();
    let registry = CommandRegistry::build_default();
    let source = "puts { spaced  data }\n";
    let analysis = analysis_for_profile(source, profile, &registry);
    assert_eq!(
        analysis.resolved_profile().unwrap().cache_key(),
        profile.cache_key()
    );
    assert_eq!(
        analysis.resolved_registry().unwrap().snapshot(),
        registry.snapshot()
    );
    assert!(!analysis.allows_lexical_declaration_advice());
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Compact,
            ..MinifyOptions::default()
        },
    );
    assert_eq!(result.source, "puts { spaced  data }");
    assert_eq!(
        result.refusals,
        vec![MinifyRefusal::MissingAlphaRenameContract]
    );
}

#[test]
fn original_hosted_minifier_keeps_source_syntax_without_native_alpha_permission() {
    // Implementation contract: naming.minifier.original-naming-permission
    // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
    let source = "proc longname {longvar} {return $longvar}\n";
    let analysis = analyse(source, "f5-irules");
    assert!(analysis.has_original_vendor_source_names());
    assert!(!analysis.allows_lexical_declaration_advice());
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated: true,
            ..MinifyOptions::default()
        },
    );
    assert_eq!(result.source, "proc longname {longvar} {return $longvar}");
    assert_eq!(
        result.refusals,
        vec![MinifyRefusal::MissingAlphaRenameContract]
    );
    assert_eq!(result.symbol_map, SymbolMap::default());
}

#[test]
fn lexical_minifier_keeps_explicit_authoring_compaction_separate() {
    // Implementation contract: naming.minifier.lexical-authoring-passes
    // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
    let source = "proc longname {longvar} {return $longvar}\n";
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-lexical-authoring";
    let registry = CommandRegistry::build_default();
    let analysis = analysis_for_profile(source, profile.intern(), &registry);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(!analysis.has_original_vendor_source_names());
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated: true,
            ..MinifyOptions::default()
        },
    );
    assert!(result.refusals.is_empty(), "{:?}", result.refusals);
    assert!(!result.symbol_map.variables.is_empty());
    assert!(!result.source.contains("longvar"));
}
#[test]
fn isolated_logical_formal_alpha_uses_general_bindings_and_preserves_defaults() {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    let source = "proc general {longleft {longright longleft}} {list $longleft $longright $longleft; return $longright}\n";
    let analysis =
        analysis_for_profile(source, lexical_profile(), &CommandRegistry::build_default());
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated: true,
            ..MinifyOptions::default()
        },
    );
    assert!(result.refusals.is_empty(), "{:?}", result.refusals);
    let mapping = result
        .symbol_map
        .variables
        .values()
        .next()
        .expect("general formal binding map");
    assert_eq!(mapping.len(), 2);
    assert!(mapping.contains_key("longleft"));
    assert!(mapping.contains_key("longright"));
    assert!(!result.source.contains("$longleft"), "{}", result.source);
    assert!(!result.source.contains("$longright"), "{}", result.source);
    assert!(
        result.source.contains("longleft}"),
        "literal default is unchanged: {}",
        result.source
    );
}

#[test]
fn logical_formal_alpha_requires_isolation_and_closed_binding_observability() {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    let source = "proc general {longleft longright} {list $longleft $longright; return $longleft}";
    let analysis =
        analysis_for_profile(source, lexical_profile(), &CommandRegistry::build_default());
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated: false,
            ..MinifyOptions::default()
        },
    );
    assert!(result.symbol_map.variables.is_empty());
    for body in [
        "mystery $longleft",
        "info locals; return $longleft",
        "global longleft; return $longleft",
        "trace add variable longleft read hook; return $longleft",
        "set longleft changed; return $longleft",
    ] {
        let source = format!("proc general {{longleft longright}} {{{body}}}");
        let analysis = analysis_for_profile(
            &source,
            lexical_profile(),
            &CommandRegistry::build_default(),
        );
        let result = minify_with_analysis(
            &source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert!(
            result.symbol_map.variables.is_empty(),
            "{body}: {}",
            result.source
        );
        assert!(result.source.contains("longleft"));
    }
}

fn lexical_profile() -> &'static tcl_dialect::DialectProfile {
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-lexical-authoring";
    profile.intern()
}

#[test]
fn lexical_roles_use_retained_context_instead_of_heterogeneous_last_spec() {
    // Implementation contract: naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    let source = "expr { 1 + 2 }\n";
    let analysis = analysis_for_profile(source, profile, &registry);
    assert!(analysis.allows_lexical_declaration_advice());
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let env = super::super::MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities: analysis.retained_command_realm().unwrap(),
    };
    let arguments = [tcl_registry::InvocationWord::Literal("1 + 2")];
    let (traits, roles, complete) =
        super::super::lexical_source_schema(env, "expr", &arguments, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            (schema.semantics.traits, roles, complete)
        })
        .unwrap();
    assert!(traits.contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS));
    assert!(complete);
    assert!(roles.contains(&(0, tcl_registry::ArgRole::Expr)));
    assert!(
        !registry
            .get("expr")
            .unwrap()
            .traits
            .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
    );
    let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
    assert!(result.refusals.is_empty(), "{:?}", result.refusals);
    assert_eq!(result.source, "expr {1+2}");
}

#[test]
fn lexical_context_refusal_retains_source_for_every_tier() {
    // Implementation contract: naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    let context = Arc::new(
        crate::context_for_dialect("tcl8.6")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        LexerConfig::for_profile(Some(profile)),
    );
    let source = "proc longname {longvar} {return $longvar}\n";
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    for tier in [
        MinifyTier::Default,
        MinifyTier::Compact,
        MinifyTier::Aggressive,
    ] {
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert_eq!(result.source, source);
        assert_eq!(result.original_length, source.len());
        assert_eq!(result.symbol_map, SymbolMap::default());
        assert_eq!(result.optimisations_applied, 0);
        assert_eq!(
            result.refusals,
            vec![MinifyRefusal::UnsupportedLexicalContext]
        );
    }
}

#[test]
fn lexical_custom_configuration_refuses_every_tier_without_fallback() {
    // Implementation contract: naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    let context = Arc::new(
        crate::context_for_dialect_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let mut config = LexerConfig::for_profile(Some(profile));
    config.strict_quoting = !config.strict_quoting;
    let input = ResolvedAnalysisInput::new(profile, profile, context, config);
    let source = "set longname  1\n";
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(analysis.matches_original_source_image(&SourceImage::document(source), config));
    for tier in [
        MinifyTier::Default,
        MinifyTier::Compact,
        MinifyTier::Aggressive,
    ] {
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier,
                isolated: true,
                ..MinifyOptions::default()
            },
        );
        assert_eq!(result.source, source);
        assert_eq!(result.symbol_map, SymbolMap::default());
        assert_eq!(result.optimisations_applied, 0);
        assert_eq!(
            result.refusals,
            vec![MinifyRefusal::UnsupportedLexicalConfiguration]
        );
    }
}

#[test]
fn lexical_intermediate_source_retains_actual_input_generation() {
    // Implementation contract: naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    let source = "set longname  1\n";
    let analysis = analysis_for_profile(source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let rewritten = "set a  1\n";
    let current = analysis_with_input(rewritten, input);
    let retained = current.resolved_input.as_ref().unwrap();
    assert_eq!(retained, input);
    assert!(Arc::ptr_eq(
        &retained.context_registry(),
        &input.context_registry()
    ));
    assert!(current.matches_original_source_image(
            &SourceImage::document(rewritten),
            input.lexer_config(),
        ));
    assert!(
        !current
            .matches_original_source_image(&SourceImage::document(source), input.lexer_config(),)
    );
    assert_eq!(
        super::super::lexical_default(rewritten, &current).unwrap(),
        "set a 1"
    );
}
#[test]
fn lexical_case_shape_retains_dynamic_subject_and_expansion_uncertainty() {
    // Implementation contract: naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    let source = "switch -exact $value {a {puts A}}\n";
    let analysis = analysis_for_profile(source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let env = super::super::MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities: analysis.retained_command_realm().unwrap(),
    };
    let mut arguments = [
        tcl_registry::InvocationWord::Literal("-exact"),
        tcl_registry::InvocationWord::Dynamic,
        tcl_registry::InvocationWord::Literal("a {puts A}"),
    ];
    let selected = super::super::lexical_source_schema(env, "switch", &arguments, |schema| {
        schema.authored_source_case_invocation()
    })
    .flatten()
    .unwrap();
    assert_eq!(selected.1.clause_list_index, Some(2));
    arguments[1] = tcl_registry::InvocationWord::Expanded;
    assert!(
        super::super::lexical_source_schema(
            env,
            "switch",
            &arguments,
            tcl_registry::ResolvedInvocation::authored_source_case_invocation,
        )
        .flatten()
        .is_none()
    );
}

#[test]
fn lexical_source_headers_keep_runtime_and_metadata_separate() {
    // naming.minifier.logical-source-header
    // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
    let registry = CommandRegistry::build_default();
    let source = "if {1} {  puts  payload  }\n";
    let analysis = analysis_for_profile(source, lexical_profile(), &registry);
    let realm = analysis.retained_command_realm().unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = SourceImage::document(source);
    assert!(realm.invocation_at_source("if", 0).unknown);
    assert_eq!(
        realm
            .lexical_source_header_unpositioned(&image, input, "if")
            .as_deref(),
        Some("if"),
    );
    let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
    assert_eq!(
        result.refusals,
        [] as [crate::minify::original::MinifyRefusal; 0]
    );
    assert_eq!(result.source, "if {1} {puts payload}");
    // This readonly projection did not replace the retained execution fact.
    assert!(realm.invocation_at_source("if", 0).unknown);
}

#[test]
fn lexical_source_headers_refuse_dynamic_argv_and_prior_mutations() {
    // naming.minifier.logical-source-header
    // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
    let registry = CommandRegistry::build_default();
    for source in [
        "if $condition {  puts  payload  }\n",
        "mystery; if {1} {  puts  payload  }\n",
        "proc if {c b} {return $b}; if {1} {  puts  payload  }\n",
        "rename if old; if {1} {  puts  payload  }\n",
    ] {
        let analysis = analysis_for_profile(source, lexical_profile(), &registry);
        let realm = analysis.retained_command_realm().unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        assert!(
            realm
                .lexical_source_header_unpositioned(&SourceImage::document(source), input, "if",)
                .is_none(),
            "{source}"
        );
    }
    let native = crate::profile_for_dialect("tcl8.6");
    let source = "if {1} {puts payload}\n";
    let analysis = analysis_for_profile(source, native, &registry);
    assert!(
        analysis
            .retained_command_realm()
            .unwrap()
            .lexical_source_header_unpositioned(
                &SourceImage::document(source),
                analysis.resolved_input.as_ref().unwrap(),
                "if",
            )
            .is_none()
    );
}

#[test]
fn lexical_source_header_checks_retained_input_source_and_configuration() {
    // naming.minifier.logical-source-header
    // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
    let registry = CommandRegistry::build_default();
    let source = "expr {1 + 2}\n";
    let analysis = analysis_for_profile(source, lexical_profile(), &registry);
    let realm = analysis.retained_command_realm().unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = SourceImage::document(source);
    assert!(
        realm
            .lexical_source_header_unpositioned(&image, &input.clone(), "expr")
            .is_some()
    );
    assert!(
        realm
            .lexical_source_header_unpositioned(
                &SourceImage::document("expr {2 + 3}\n"),
                input,
                "expr",
            )
            .is_none()
    );
    let mut wrong_config = input.lexer_config();
    wrong_config.strict_quoting = !wrong_config.strict_quoting;
    let wrong = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        wrong_config,
    );
    assert!(
        realm
            .lexical_source_header_unpositioned(&image, &wrong, "expr")
            .is_none()
    );
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::new(
            crate::context_for_dialect_profile(input.analyser_profile())
                .with_command_store(registry.snapshot().shared_registry()),
        ),
        input.lexer_config(),
    );
    assert!(
        realm
            .lexical_source_header_unpositioned(&image, &foreign, "expr")
            .is_none()
    );
}

#[test]
fn lexical_complete_heads_preserve_computed_body_data() {
    // Implementation contract: naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    for head in [
        "$if",
        "if${suffix}",
        "[list if]",
        "\"$if\"",
        "\"if${suffix}\"",
    ] {
        let source = format!("{head} {{1}} {{  puts  payload  }}\n");
        let analysis = analysis_for_profile(&source, profile, &registry);
        assert!(analysis.allows_lexical_declaration_advice());
        let result = minify_with_analysis(&source, &analysis, MinifyOptions::default());
        assert!(result.refusals.is_empty(), "{head}: {:?}", result.refusals);
        assert!(
            result.source.ends_with("{  puts  payload  }"),
            "{head}: {}",
            result.source
        );
    }
}

#[test]
fn lexical_complete_static_heads_keep_selected_body_roles() {
    // Implementation contract: naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let profile = lexical_profile();
    let registry = CommandRegistry::build_default();
    for head in ["if", "{if}", "\"if\"", r"i\x66"] {
        let source = format!("{head} {{1}} {{  puts  payload  }}\n");
        let analysis = analysis_for_profile(&source, profile, &registry);
        let result = minify_with_analysis(&source, &analysis, MinifyOptions::default());
        assert!(result.refusals.is_empty(), "{head}: {:?}", result.refusals);
        assert!(
            result.source.ends_with("{puts payload}"),
            "{head}: {}",
            result.source
        );
    }
}

#[test]
fn lexical_head_value_checks_whole_image_config_and_tokens() {
    // Implementation contract: naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let source = "if {1} {puts payload}\n";
    let config = LexerConfig::for_profile(Some(lexical_profile()));
    let plan = super::super::lexical_script_words(source, config).unwrap();
    let tokens = tcl_lexer::Lexer::with_config(source, config)
        .tokenise_all()
        .unwrap();
    let commands = super::super::parse_commands(source, &tokens, config);
    let head = &commands[0][0];
    let word = &plan.commands[0].words[0];
    let sm = tcl_lexer::SourceMap::new(source);
    assert_eq!(
        super::super::lexical_head_value(&sm, head, word, config).as_deref(),
        Some("if")
    );
    let foreign_source = "if {1} {puts another}\n";
    let foreign = super::super::lexical_script_words(foreign_source, config).unwrap();
    assert!(
        super::super::lexical_head_value(&sm, head, &foreign.commands[0].words[0], config)
            .is_none()
    );
    let mut foreign_config = config;
    foreign_config.strict_quoting = !config.strict_quoting;
    assert!(super::super::lexical_head_value(&sm, head, word, foreign_config).is_none());
    assert!(super::super::lexical_head_value(&sm, &commands[0][1], word, config).is_none());
}

#[test]
fn lexical_head_value_keeps_expansion_and_opaque_units_unavailable() {
    // Implementation contract: naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let mut config = LexerConfig::for_profile(Some(lexical_profile()));
    config.expand_syntax = true;
    for source in [
        "{*}{if} {1} {puts payload}\n",
        r"i\uD800 {1} {puts payload}",
        "é {1} {puts payload}\n",
    ] {
        let plan = super::super::lexical_script_words(source, config).unwrap();
        let tokens = tcl_lexer::Lexer::with_config(source, config)
            .tokenise_all()
            .unwrap();
        let commands = super::super::parse_commands(source, &tokens, config);
        let sm = tcl_lexer::SourceMap::new(source);
        assert!(
            super::super::lexical_head_value(
                &sm,
                &commands[0][0],
                &plan.commands[0].words[0],
                config
            )
            .is_none(),
            "{source}"
        );
    }
    let source = "{*} {1} {puts payload}\n";
    config.expand_syntax = false;
    let plan = super::super::lexical_script_words(source, config).unwrap();
    let tokens = tcl_lexer::Lexer::with_config(source, config)
        .tokenise_all()
        .unwrap();
    let commands = super::super::parse_commands(source, &tokens, config);
    let sm = tcl_lexer::SourceMap::new(source);
    assert_eq!(
        super::super::lexical_head_value(&sm, &commands[0][0], &plan.commands[0].words[0], config)
            .as_deref(),
        Some("*")
    );
}

fn with_lexical_metadata<T>(
    source: &str,
    project: impl FnOnce(super::super::MinifyEnv<'_>) -> T,
) -> T {
    let registry = CommandRegistry::build_default();
    let analysis = analysis_for_profile(source, lexical_profile(), &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    project(super::super::MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities: analysis.retained_command_realm().unwrap(),
    })
}

#[test]
fn logical_render_roles_retain_effective_alias_prefix_and_original_body() {
    let source = "interp alias {} choose {} if 1; choose { puts   yes }";
    with_lexical_metadata(source, |env| {
        let output = super::super::minify_body(
            source,
            env,
            0,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        assert!(output.ends_with("choose {puts yes}"), "{output}");
    });
    let source = "interp alias {} choose {} switch --; choose $value { a { puts   yes } default { puts   no } }";
    with_lexical_metadata(source, |env| {
        let output = super::super::minify_body(
            source,
            env,
            0,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        assert!(output.contains("{puts yes}"), "{output}");
        assert!(output.contains("{puts no}"), "{output}");
    });
}

#[test]
fn logical_render_roles_refuse_detached_and_changed_original_regions() {
    let source = "if 1 { expr { 1 + 2 } }";
    with_lexical_metadata(source, |env| {
        let detached = source.to_owned();
        assert!(env.original_region(&detached).is_none());
        let output = super::super::minify_body(
            &detached,
            env,
            0,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        assert!(output.contains("{ expr { 1 + 2 } }"), "{output}");
        let body = &source[source.find("expr").unwrap()..source.len() - 2];
        assert!(env.original_region(body).is_some());
        let output = super::super::minify_body(
            source,
            env,
            0,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        assert!(output.contains("{expr {1+2}}"), "{output}");
    });
}

#[test]
fn logical_expression_lexing_retains_configured_names_and_contextual_boundaries() {
    let source = "expr { 1e-3 + sqrt( 4 ) }";
    with_lexical_metadata(source, |env| {
        for expression in [
            "1e-3 + sqrt( 4 )",
            "$array($key + 1) + 2",
            "$x eq \"word\"",
            "$x eq {a  b}",
            "1 + +2",
            "0x10 + 1",
        ] {
            let output = super::super::strip_expr_whitespace(expression, env, 0);
            assert!(
                super::super::expression_token_signature(expression, env, false).is_some(),
                "{expression}"
            );
            assert_eq!(
                super::super::expression_token_signature(expression, env, false),
                super::super::expression_token_signature(&output, env, false),
                "{expression} -> {output}"
            );
        }
        let output = super::super::strip_expr_whitespace("1e-3 + sqrt( 4 )", env, 0);
        assert!(output.contains("1e-3"), "{output}");
        assert!(output.contains("sqrt"), "{output}");
        let literal = super::super::strip_expr_whitespace("$x eq {a  b}", env, 0);
        assert!(literal.contains("{a  b}"), "{literal}");
        assert!(
            matches!(super::super::tokenise_expr("1e-3", env, 0).as_slice(), [super::super::ExprTok::Other(value)] if value == "1e-3")
        );
        for malformed in [
            "[unterminated + 1",
            "\"unterminated + 1",
            "$array(unclosed + 1",
            "1 + \\q",
            "café(1)",
            "$x   $y",
            "${a{b}c} + 1",
        ] {
            assert_eq!(
                super::super::strip_expr_whitespace(malformed, env, 0),
                malformed
            );
        }
    });
}

#[test]
fn logical_render_keeps_failed_original_regions_under_the_full_configuration() {
    let profile = lexical_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    for config in [
        LexerConfig {
            strict_quoting: false,
            ..LexerConfig::for_profile(Some(profile))
        },
        LexerConfig {
            strict_quoting: false,
            leading_bom: tcl_lexer::LeadingBom::Content,
            word_separators: tcl_dialect::WordSeparators::Jim,
            ..LexerConfig::for_profile(Some(profile))
        },
    ] {
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let source = "puts    before; puts \"unterminated";
        assert!(
            tcl_lexer::Lexer::with_config(source, config)
                .tokenise_all()
                .is_ok()
        );
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_some());
        let analysis = analysis_with_input(source, &input);
        assert_eq!(
            super::super::lexical_default(source, &analysis).unwrap(),
            source
        );
        assert_eq!(analysis.body_lexer_config, Some(config));
    }
    let source = "list {puts   before; puts \"unterminated}";
    with_lexical_metadata(source, |env| {
        let original = super::super::lexical_script_words(source, env.config).unwrap();
        let body_span = original.commands[0].words[1].content_span().unwrap();
        let body = &source[body_span.as_range()];
        assert_eq!(env.original_region(body), Some(body_span));
        assert_eq!(
            super::super::minify_body(
                body,
                env,
                1,
                tcl_lexer::SourceSegmentationPurpose::NestedScript
            ),
            body
        );
    });
}

#[test]
fn logical_keyword_metadata_edits_whole_selected_controls_only() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "string equal -nocase A a\n";
    let (output, count) = with_lexical_metadata(source, |env| {
        super::super::lexical_keywords::abbreviate(source, env)
    });
    assert!(count > 0, "{output}");
    assert!(output.contains("-n A a"), "{output}");
    for source in [
        "string length$tail value\n",
        "string [set selector length] value\n",
        "string equal -nocase $value other\n",
        "string equal -nocase[set suffix {}] A a\n",
        "string equal -nocase -nocase other\n",
        "string equal -- -nocase other\n",
        "string equal A -nocase\n",
        r"string equal -no\x63ase A a",
    ] {
        let (output, count) = with_lexical_metadata(source, |env| {
            super::super::lexical_keywords::abbreviate(source, env)
        });
        if source.ends_with(" -nocase other\n") || source.ends_with(" A -nocase\n") {
            assert!(
                output.ends_with("-nocase other\n") || output.ends_with("A -nocase\n"),
                "{source} -> {output}"
            );
        }
        if source.contains("length$") || source.contains("[set") || source.contains("$value") {
            assert_eq!((output.as_str(), count), (source, 0));
        }
    }
}

#[test]
fn logical_keyword_walk_distinguishes_data_from_selected_script_regions() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let data = "list {string equal -nocase A a}\n";
    with_lexical_metadata(data, |env| {
        let commands = super::super::lexical_metadata::commands(data, env).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            super::super::lexical_keywords::abbreviate(data, env),
            (data.to_owned(), 0)
        );
    });
    let body = "if {1} {string equal -nocase A a}\n";
    with_lexical_metadata(body, |env| {
        let commands = super::super::lexical_metadata::commands(body, env).unwrap();
        assert_eq!(commands.len(), 2);
        assert!(commands[1].span.start() > 0);
    });
    let bracket = "list [string equal -nocase A a]\n";
    with_lexical_metadata(bracket, |env| {
        assert_eq!(
            super::super::lexical_metadata::commands(bracket, env)
                .unwrap()
                .len(),
            2
        );
    });
}

#[test]
fn logical_positioned_metadata_retains_unknown_values_and_exact_owner() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "string equal -nocase $value other\n";
    with_lexical_metadata(source, |env| {
        let image = SourceImage::document(source);
        let plan = super::super::lexical_script_words(source, env.config).unwrap();
        let command = &plan.commands[0];
        assert_eq!(
            env.identities
                .lexical_source_header_for_words(&image, env.input, &command.words)
                .as_deref(),
            Some("string")
        );
        let dynamic = super::super::lexical_metadata::with_command_schema(env, command, |schema| {
            matches!(
                schema.words.arguments().get(2),
                Some(tcl_registry::InvocationWord::Dynamic)
            )
        });
        assert_eq!(dynamic, Some(true));
        assert!(
            env.identities
                .lexical_source_header_for_words(
                    &SourceImage::document("string equal -nocase changed other\n"),
                    env.input,
                    &command.words
                )
                .is_none()
        );
        let mut config = env.input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let changed = ResolvedAnalysisInput::new(
            env.input.analyser_profile(),
            env.input.unit_profile(),
            env.input.context_registry(),
            config,
        );
        assert!(
            env.identities
                .lexical_source_header_for_words(&image, &changed, &command.words)
                .is_none()
        );
    });
    let unknown = "mystery; string equal -nocase A a\n";
    with_lexical_metadata(unknown, |env| {
        let image = SourceImage::document(unknown);
        let plan = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(unknown.len()).unwrap()),
            env.config,
        )
        .unwrap();
        let command = &plan.commands[1];
        let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
                env.identities, env.input, command,
            ).expect("conditional source metadata remains available after an unknown operation");
        let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("missing positively selected Logical source purpose");
            };
        assert!(advice.obligations().contains(
                &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
            ));
        assert_ne!(advice.uncertain_operations(), []);
        assert!(
            super::super::lexical_metadata::with_command_schema(env, command, |_| ()).is_some()
        );
        assert!(
            super::super::lexical_metadata::with_editable_command_schema(env, command, |_| ())
                .is_none()
        );
    });
    for source in [
        "mystery; string equal -nocase A a\n",
        "proc string {args} {return value}; string equal -nocase A a\n",
        "rename string old; string equal -nocase A a\n",
        "string[set suffix {}] equal -nocase A a\n",
    ] {
        with_lexical_metadata(source, |env| {
            assert_eq!(
                super::super::lexical_keywords::abbreviate(source, env),
                (source.to_owned(), 0)
            );
        });
    }
}

#[test]
fn logical_barriers_use_complete_selected_form_and_unknown_coverage() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    for source in ["info locals\n", "info locals$tail\n", "mystery\n"] {
        let registry = CommandRegistry::build_default();
        let analysis = analysis_for_profile(source, lexical_profile(), &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let env = super::super::MinifyEnv {
            original_source: source,
            analysis: &analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities: analysis.retained_command_realm().unwrap(),
        };
        let commands = super::super::lexical_metadata::commands(source, env).unwrap();
        let barriers = super::super::find_rename_barriers(&analysis, env, &commands, true);
        assert!(!barriers.allows_scope(&analysis.global_scope), "{source}");
        if source.contains('$') || source.contains("mystery") {
            assert!(barriers.procs);
        }
    }
    let source = "list {info locals}\n";
    with_lexical_metadata(source, |env| {
        let commands = super::super::lexical_metadata::commands(source, env).unwrap();
        assert_eq!(commands.len(), 1);
    });
}

#[test]
fn logical_metadata_never_supplies_native_or_hosted_keyword_permission() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "string equal -nocase A a\n";
    for name in ["tcl8.6", "f5-irules"] {
        let registry = CommandRegistry::build_default();
        let analysis = analysis_for_profile(source, crate::profile_for_dialect(name), &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        let plan = super::super::lexical_script_words(source, input.lexer_config()).unwrap();
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .lexical_source_header_for_words(&image, input, &plan.commands[0].words)
                .is_none()
        );
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Aggressive,
                abbreviations: true,
                ..MinifyOptions::default()
            },
        );
        assert!(result.source.contains("-nocase"));
        assert!(
            result
                .refusals
                .contains(&MinifyRefusal::MissingKeywordSelectionContract)
        );
    }
}
#[test]
fn body_syntax_compaction_retains_changed_handler_and_observer_refusals() {
    for source in [
        "proc if {args} {return [lindex $args 0]}\nproc example {} {if { 3 + 4 } { puts NEVER }}\nputs [example]\n",
        "proc example {x} {if {$x <= 1} { return 1 }}\nrename if native_if\nproc if {args} {return [lindex $args 0]}\nputs [example 0]\n",
        "proc witness {args} {}\ntrace add execution if enter witness\nproc example {x} {if {$x <= 1} { return 1 }}\nexample 0\n",
    ] {
        let analysis = analyse(source, "tcl8.6");
        let result = minify_with_analysis(source, &analysis, MinifyOptions::default());
        let expected = if source.contains("{ 3 + 4 }") {
            "{ 3 + 4 }"
        } else {
            "{$x <= 1}"
        };
        assert!(
            result.source.contains(expected),
            "{source} -> {}",
            result.source
        );
    }
}
