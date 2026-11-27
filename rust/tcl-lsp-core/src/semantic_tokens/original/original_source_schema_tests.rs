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
use crate::semantic_tokens::tests::decode_semantic;
use crate::semantic_tokens::{MOD_DECLARATION, full_with_cu_and_analysis};
use tcl_compiler::analyser::{Analyser, AnalysisResult};

fn colour_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: usize,
    kind: TokenKind,
    modifier: u32,
) -> bool {
    let profile = analysis.resolved_profile().unwrap();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        profile,
        analysis.resolved_registry().unwrap(),
        None,
        Some(analysis),
    ));
    let position =
        tcl_lexer::LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
    tokens.iter().any(|&(line, column, _, actual, mods)| {
        line == position.line
            && column == position.character.get()
            && actual == kind as u32
            && mods == modifier
    })
}

fn logical_role_input(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-source-role-colours";
    let profile = profile.intern();
    tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    )
}

#[test]
fn logical_definer_class_context_uses_the_selected_installation_name_operand() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut spec = registry.get("itcl::class").unwrap().clone();
    spec.name = "selected_class";
    spec.required_package = None;
    // No name role: the class name must come from the definer's own
    // installation operand.
    spec.arg_roles = &[(1, tcl_registry::ArgRole::Body)];
    registry.insert(spec);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.1")
            .with_command_store(std::sync::Arc::new(registry)),
    );
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let source = "selected_class C {\nmethod m {} {}\nmethod run {} {\n${this} m\n}\n}";
    let analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "report");
    let config = analysis.body_lexer_config.unwrap();
    let command =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words =
        crate::original_invocation::source_registry_words(source, &analysis, &command).unwrap();
    let selected = words
        .with_source_schema(&input.context_registry(), source_class_ordinal)
        .flatten();
    assert_eq!(selected, Some((0, true)));
    let method = source.rfind(" m").unwrap() + 1;
    assert!(colour_at(source, &analysis, method, TokenKind::Method, 0));
    let dynamic = source.replace("selected_class C", "selected_class $name");
    let dynamic_analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(&dynamic, "report");
    assert!(!colour_at(
        &dynamic,
        &dynamic_analysis,
        dynamic.rfind(" m").unwrap() + 1,
        TokenKind::Method,
        0
    ));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        full_with_cu_and_analysis(
            source,
            profile,
            analysis.resolved_registry().unwrap(),
            None,
            Some(&missing),
        )
        .data,
        [] as [u32; 0]
    );
}

#[test]
fn produced_prefix_colours_keep_owned_deferred_roles_and_static_target_words() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    for logical in [false, true] {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = if logical {
            tcl_dialect::DialectProfile::plain_tcl()
        } else {
            crate::profile_for_dialect("tcl9.1")
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        for (source, needle, kind, modifier) in [
            (
                "uplevel #0 [list upvar #0 originalName localName]",
                "localName",
                TokenKind::Variable,
                MOD_DECLARATION,
            ),
            (
                "after idle [list apply {{longname} {puts $longname}} 5]",
                "longname",
                TokenKind::Parameter,
                MOD_DECLARATION,
            ),
            // The body's own commands are unrecorded; an unrebound builtin
            // spelling there is coloured as it is at top level.
            (
                "after idle [list ::apply {{x} {puts $x}} 5]",
                "puts",
                TokenKind::Function,
                super::super::MOD_DEFAULT_LIBRARY,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            assert!(
                colour_at(
                    source,
                    &analysis,
                    source.find(needle).unwrap(),
                    kind,
                    modifier
                ),
                "logical={logical}: {source}"
            );
        }
        for source in [
            "set data [list apply {{longname} {puts $longname}} 5]",
            "proc list args {}; after idle [list apply {{longname} {puts $longname}} 5]",
            "proc apply args {}; after idle [list apply {{longname} {puts $longname}} 5]",
            "after idle [list $head {{longname} {puts $longname}} 5]",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            assert!(
                !colour_at(
                    source,
                    &analysis,
                    source.find("longname").unwrap(),
                    TokenKind::Parameter,
                    MOD_DECLARATION
                ),
                "logical={logical}: {source}"
            );
        }
    }
}

#[test]
fn logical_source_role_colours_keep_actual_option_context_and_reserved_data() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for (environment, expected) in [("tcl8.4", false), ("tcl9.1", true)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let input = logical_role_input(std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        ));
        let source = "lsearch -stride 2 -regexp {a b} a+";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "reporting-only");
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find("-stride").unwrap(),
                TokenKind::Decorator,
                0
            ),
            expected,
            "{environment}"
        );
    }
    let input = logical_role_input(driver);
    for source in [
        "string equal -nocase -nocase -nocase",
        "string equal -length 3 -nocase -nocase",
        "interp alias {} compare {} string equal -nocase; compare -nocase -nocase",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        let tails = source
            .match_indices("-nocase")
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>();
        for &offset in tails.iter().rev().take(2) {
            assert!(
                !colour_at(source, &analysis, offset, TokenKind::Decorator, 0),
                "{source}: {offset}"
            );
        }
        if source.starts_with("string equal -nocase") {
            assert!(colour_at(
                source,
                &analysis,
                tails[0],
                TokenKind::Decorator,
                0
            ));
        }
    }
}

#[test]
fn logical_source_role_colours_keep_effective_alias_operands_and_shadow_refusal() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let input = logical_role_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    for (source, expected) in [
        ("string equal -nocase A a", true),
        (
            "interp alias {} compare {} string equal; compare -nocase A a",
            true,
        ),
        ("proc string args {}; string equal -nocase A a", false),
        ("string $operation -nocase A a", false),
    ] {
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.rfind("-nocase").unwrap(),
                TokenKind::Decorator,
                0
            ),
            expected,
            "{source}"
        );
    }
    let expanded_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        tcl_lexer::LexerConfig {
            expand_syntax: true,
            ..input.lexer_config()
        },
    );
    let source = "string equal {*}$arguments -nocase A a";
    let analysis = Analyser::new()
        .with_resolved_input(expanded_input)
        .analyse(source, "reporting-only");
    assert!(!colour_at(
        source,
        &analysis,
        source.find("-nocase").unwrap(),
        TokenKind::Decorator,
        0
    ));
}

#[test]
fn logical_source_role_colours_keep_nested_selectors_values_and_oo_members() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let input = logical_role_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    for (source, needle, kind, modifier) in [
        (
            "namespace ensemble configure ::E -namespace",
            "configure",
            TokenKind::Keyword,
            super::super::MOD_DEFAULT_LIBRARY,
        ),
        (
            "namespace ensemble configure ::E -namespace",
            "-namespace",
            TokenKind::Decorator,
            0,
        ),
        ("string is alnum abc", "alnum", TokenKind::EnumMember, 0),
        (
            "oo::define C method pick {arg} {return $arg}",
            "pick",
            TokenKind::Method,
            super::super::legend::MOD_DEFINITION,
        ),
        (
            "oo::class create C {method pick {arg} {return $arg}}",
            "pick",
            TokenKind::Method,
            super::super::legend::MOD_DEFINITION,
        ),
        (
            "switch $subject one {set local 1} default {set local 2}",
            "local",
            TokenKind::Variable,
            MOD_DECLARATION,
        ),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(
            colour_at(
                source,
                &analysis,
                source.find(needle).unwrap(),
                kind,
                modifier
            ),
            "{source}: {needle}"
        );
    }
}

#[test]
fn logical_format_colours_keep_actual_context_and_original_alias_geometry() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-format-colours";
    let profile = profile.intern();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let source = "clock scan 2020 -format {%Y}";
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find('Y').unwrap(),
                TokenKind::ClockSpec,
                0
            ),
            expected,
            "{environment}"
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            full_with_cu_and_analysis(source, profile, context.commands(), None, Some(&analysis))
                .data,
            [] as [u32; 0]
        );
    }
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        driver,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for (source, expected) in [
        ("proc p {} {format {%d} 7}", true),
        ("interp alias {} fmt {} format; fmt {%d} 7", true),
        ("interp alias {} fmt {} format {%d}; fmt 7", false),
        ("proc format args {}; format {%d} 7", false),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find("%d").unwrap() + 1,
                TokenKind::FormatSpec,
                0
            ),
            expected,
            "{source}"
        );
    }
}

#[test]
fn retained_head_colours_keep_the_selected_full_context_descriptor_traits() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let modern = crate::profile_for_dialect("tcl9.1");
    let mut registry = tcl_registry::CommandRegistry::build_default().project_for_profile(modern);
    for (surface, traits) in [
        (
            tcl_dialect::model::SpecSurface::TCL84,
            tcl_registry::Traits::LANGUAGE_KEYWORD,
        ),
        (
            tcl_dialect::model::SpecSurface::TCL91,
            tcl_registry::Traits::OPERATOR_COMMAND,
        ),
    ] {
        registry.insert(tcl_registry::CommandSpec {
            name: "source-head-fixture",
            surface: Some(surface),
            traits,
            arity: tcl_registry::Arity::exact(0),
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    assert!(
        registry
            .get("source-head-fixture")
            .unwrap()
            .traits
            .contains(tcl_registry::Traits::OPERATOR_COMMAND)
    );
    let registry = std::sync::Arc::new(registry);
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-head-traits";
    let profile = profile.intern();
    for (environment, kind) in [
        ("tcl8.4", TokenKind::Keyword),
        ("tcl9.1", TokenKind::Operator),
    ] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(actual.with_command_store(registry.clone()));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let source = "source-head-fixture";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "presentation-only");
        let words = crate::original_invocation::source_registry_words(
            source,
            &analysis,
            &tcl_source::segment::segment_commands_with_offset_and_config(
                source,
                0,
                analysis.body_lexer_config.unwrap(),
            )[0],
        )
        .unwrap();
        assert!(
            words
                .with_source_schema(
                    &analysis.resolved_input.as_ref().unwrap().context_registry(),
                    |schema| schema.authored_source_descriptors().command.traits
                )
                .unwrap()
                .contains(if kind == TokenKind::Keyword {
                    tcl_registry::Traits::LANGUAGE_KEYWORD
                } else {
                    tcl_registry::Traits::OPERATOR_COMMAND
                })
        );
        assert!(colour_at(source, &analysis, 0, kind, 0), "{environment}");
    }
}

fn original_head_evidence(source: &str, analysis: &AnalysisResult) -> String {
    let config = analysis.body_lexer_config.unwrap();
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words = crate::original_invocation::source_registry_words(source, analysis, &segment);
    let owned = words.as_ref().and_then(|words| words.head_source());
    let word = owned.and_then(|owned| owned.word());
    let value = word.and_then(tcl_syntax::word_rules::original_static_word_ascii_presentation);
    let boundaries = word.map(|word| {
        [0, 2, 8].map(|offset| {
            tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, offset)
        })
    });
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        analysis.resolved_registry().unwrap(),
        None,
        Some(analysis),
    ));
    format!(
        "schema={} original={word:?} value={value:?} boundaries={boundaries:?} tokens={tokens:?}",
        words.is_some()
    )
}

#[test]
fn logical_qualified_head_colours_keep_original_decoded_boundaries() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-head-qualifiers";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for (source, tail) in [("::format {%d} 7", 2), (r"\u003a\u003aformat {%d} 7", 12)] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert!(
            colour_at(
                source,
                &analysis,
                0,
                TokenKind::Namespace,
                super::super::MOD_DEFAULT_LIBRARY
            ),
            "{source}: {:?}",
            original_head_evidence(source, &analysis)
        );
        assert!(
            colour_at(
                source,
                &analysis,
                tail,
                TokenKind::Function,
                super::super::MOD_DEFAULT_LIBRARY
            ),
            "{source}: {:?}",
            original_head_evidence(source, &analysis)
        );
    }
}

#[test]
fn original_format_colours_keep_selected_availability_and_generation() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "clock scan 2020 -format {%Y}";
    let profile = crate::profile_for_dialect("tcl9.1");
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let offset = source.find("%Y").unwrap() + 1;
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(source, &analysis, offset, TokenKind::ClockSpec, 0),
            expected,
            "{environment}"
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            full_with_cu_and_analysis(source, profile, context.commands(), None, Some(&analysis))
                .data,
            [] as [u32; 0]
        );
    }
}

#[test]
fn original_pattern_colours_keep_selected_availability_and_generation() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let source = "lsearch -regexp -stride 2 {a b} {a+}";
    let profile = crate::profile_for_dialect("tcl9.1");
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let offset = source.find("a+").unwrap();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(source, &analysis, offset, TokenKind::Regexp, 0),
            expected,
            "{environment}"
        );
        assert_eq!(
            full_with_cu_and_analysis(
                &source.replace("a+", "b+"),
                profile,
                context.commands(),
                None,
                Some(&analysis)
            )
            .data,
            [] as [u32; 0]
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            full_with_cu_and_analysis(source, profile, context.commands(), None, Some(&analysis))
                .data,
            [] as [u32; 0]
        );
    }
}

#[test]
fn original_pattern_colours_keep_alias_written_operands_and_captured_refusal() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    for (source, expected) in [
        (
            "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
            true,
        ),
        (
            "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
            false,
        ),
        ("lsearch $mode {a b} {a+}", false),
        ("lsearch {*}$arguments {a+}", false),
        ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl9.1");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find("a+").unwrap(),
                TokenKind::Regexp,
                0
            ),
            expected,
            "{source}"
        );
    }
}

#[test]
fn logical_pattern_colours_keep_actual_context_over_a_profiled_store() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let source = "lsearch -regexp -stride 2 {a b} {a+}";
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-pattern-colours";
    let profile = profile.intern();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(analysis.allows_lexical_declaration_advice());
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find("a+").unwrap(),
                TokenKind::Regexp,
                0
            ),
            expected,
            "{environment}"
        );
        assert_eq!(
            full_with_cu_and_analysis(
                &source.replace("a+", "b+"),
                profile,
                context.commands(),
                None,
                Some(&analysis)
            )
            .data,
            [] as [u32; 0]
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            full_with_cu_and_analysis(source, profile, context.commands(), None, Some(&analysis))
                .data,
            [] as [u32; 0]
        );
    }
}

#[test]
fn logical_pattern_colours_keep_nested_and_alias_original_operand_anchors() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-pattern-anchors";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for (source, expected) in [
        ("proc p {} {regexp {a+} $value}", true),
        (
            "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
            true,
        ),
        (
            "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
            false,
        ),
        ("lsearch $mode {a b} {a+}", false),
        ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
    ] {
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(analysis.allows_lexical_declaration_advice());
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        assert_eq!(
            colour_at(
                source,
                &analysis,
                source.find("a+").unwrap(),
                TokenKind::Regexp,
                0
            ),
            expected,
            "{source}"
        );
    }
}

#[test]
fn original_semantic_command_qualifiers_use_decoded_source_geometry() {
    // naming.core.original-command-head-colours
    // docs/design/analysis/name-resolution-proofs/original-command-head-colours.md
    for source in [r"::set marker 1", r"\u003a\u003aset marker 1"] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.command_invocations.clear();
        analysis.global_scope.variables.clear();
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis),
        ));
        let tail = u32::try_from(source.find("set").unwrap()).unwrap();
        assert!(
            tokens.iter().any(|&(line, column, length, kind, _)| {
                line == 0 && column == 0 && length == tail && kind == TokenKind::Namespace as u32
            }),
            "{source}: {tokens:?}"
        );
        assert!(
            tokens.iter().any(|&(line, column, length, kind, _)| {
                line == 0 && column == tail && length == 3 && kind == TokenKind::Function as u32
            }),
            "{source}: {tokens:?}"
        );
    }
    let source = r"set\u0000::other marker 1";
    let mut analysis = Analyser::new().analyse(source, "tcl9.0");
    analysis.command_invocations.clear();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        analysis.resolved_registry().unwrap(),
        None,
        Some(&analysis),
    ));
    assert!(
        !tokens
            .iter()
            .any(|&(_, _, _, kind, _)| kind == TokenKind::Namespace as u32),
        "{tokens:?}"
    );
    assert!(tokens.iter().any(|&(line, column, _, kind, _)| {
        line == 0 && column == 0 && kind == TokenKind::Function as u32
    }));
}

#[test]
fn original_semantic_method_roles_keep_canonical_worker_words() {
    // naming.core.original-member-body-colours
    // docs/design/analysis/name-resolution-proofs/original-member-body-colours.md
    let source = "oo::class create C {method p {argument {optional 7}} {set local $argument; return $local}}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let class = analysis.original_class_declarations().next().unwrap();
    let member = class
        .metadata()
        .original_members
        .declarations()
        .next()
        .unwrap();
    assert!(member.parameters_word().is_some() && member.body_word().is_some());
    assert!(member.source_dialect().is_some());
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    analysis.command_invocations.clear();
    for name in ["argument", "optional"] {
        assert!(
            colour_at(
                source,
                &analysis,
                source.find(name).unwrap(),
                TokenKind::Parameter,
                MOD_DECLARATION
            ),
            "{name}"
        );
    }
    assert!(colour_at(
        source,
        &analysis,
        source.find("local").unwrap(),
        TokenKind::Variable,
        MOD_DECLARATION
    ));
    assert_eq!(
        full_with_cu_and_analysis(
            &format!("#{source}"),
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis)
        )
        .data,
        [] as [u32; 0]
    );
}

#[test]
fn original_semantic_source_roles_survive_reporting_map_erasure() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "set marker 1\nif {1} {set inside 2}\nproc p {a {b 5}} {return $a}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.command_invocations.clear();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    analysis.global_scope.variables.clear();
    for name in ["marker", "inside"] {
        assert!(
            colour_at(
                source,
                &analysis,
                source.find(name).unwrap(),
                TokenKind::Variable,
                MOD_DECLARATION
            ),
            "{name}"
        );
    }
    let params = source.find("{a {b 5}}").unwrap();
    assert!(colour_at(
        source,
        &analysis,
        params + 1,
        TokenKind::Parameter,
        MOD_DECLARATION
    ));
    assert!(colour_at(
        source,
        &analysis,
        params + 4,
        TokenKind::Parameter,
        MOD_DECLARATION
    ));
    assert_eq!(
        full_with_cu_and_analysis(
            &source.replace("marker", "absent"),
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis)
        )
        .data,
        [] as [u32; 0]
    );
}

#[test]
fn original_semantic_roles_decline_custom_and_deleted_command_cells() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    for source in [
        "proc set {name value} {return value}\nset marker 1\n",
        "rename set saved\nset marker 1\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(!colour_at(
            source,
            &analysis,
            source.find("marker").unwrap(),
            TokenKind::Variable,
            MOD_DECLARATION
        ));
    }
    let source = "set {*}{marker 1}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(colour_at(
        source,
        &analysis,
        source.find("marker").unwrap(),
        TokenKind::Variable,
        MOD_DECLARATION
    ));
}

#[test]
fn original_semantic_source_grammar_and_hosted_object_roles_remain_independent() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "\u{feff}set marker 1\n";
    let profile = crate::environment_for_dialect("tcl9.0").analyser_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let analyse = |leading_bom| {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    assert!(colour_at(
        source,
        &analyse(tcl_lexer::LeadingBom::Skip),
        source.find("marker").unwrap(),
        TokenKind::Variable,
        MOD_DECLARATION
    ));
    assert!(!colour_at(
        source,
        &analyse(tcl_lexer::LeadingBom::Content),
        source.find("marker").unwrap(),
        TokenKind::Variable,
        MOD_DECLARATION
    ));
    let hosted = "when HTTP_REQUEST {pool safe; set target 1}\n";
    let analysis = Analyser::new().analyse(hosted, "f5-irules");
    assert!(colour_at(
        hosted,
        &analysis,
        hosted.find("safe").unwrap(),
        TokenKind::Object,
        0
    ));
    assert!(colour_at(
        hosted,
        &analysis,
        hosted.find("target").unwrap(),
        TokenKind::Variable,
        MOD_DECLARATION
    ));
    let shadow = "proc pool {name} {return $name}\nwhen HTTP_REQUEST {pool safe}\n";
    let custom = Analyser::new().analyse(shadow, "f5-irules");
    assert!(!colour_at(
        shadow,
        &custom,
        shadow.find("safe").unwrap(),
        TokenKind::Object,
        0
    ));
}
#[test]
fn original_expression_colours_require_actual_source_roles_and_lambda_parent_geometry() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let data = "set data {1 + 2}\n";
    let analysis = Analyser::new().analyse(data, "tcl8.6");
    assert!(!colour_at(
        data,
        &analysis,
        data.find('+').unwrap(),
        TokenKind::Operator,
        0
    ));
    let expression = "expr {1 + 2}\n";
    let analysis = Analyser::new().analyse(expression, "tcl8.6");
    assert!(colour_at(
        expression,
        &analysis,
        expression.find('+').unwrap(),
        TokenKind::Operator,
        0
    ));
    let shadow = "proc expr {value} {return $value}\nexpr {1 + 2}\n";
    let analysis = Analyser::new().analyse(shadow, "tcl8.6");
    assert!(!colour_at(
        shadow,
        &analysis,
        shadow.find('+').unwrap(),
        TokenKind::Operator,
        0
    ));
    let lambda = "apply {x {return $x}} 1\n";
    let analysis = Analyser::new().analyse(lambda, "tcl8.6");
    assert!(colour_at(
        lambda,
        &analysis,
        lambda.find("{x").unwrap() + 1,
        TokenKind::Parameter,
        MOD_DECLARATION
    ));
    assert!(colour_at(
        lambda,
        &analysis,
        lambda.find("$x").unwrap(),
        TokenKind::Variable,
        0
    ));
}
#[test]
fn refused_logical_head_schema_keeps_generic_colour_without_library_flags() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let input = logical_role_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    for (source, head) in [
        ("proc string args {}; string equal A a", "string"),
        ("rename if {}; if 1 {}", "if"),
        ("unprovidedCommand argument", "unprovidedCommand"),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        let offset = source.rfind(head).unwrap();
        assert!(
            colour_at(source, &analysis, offset, TokenKind::Function, 0),
            "{source}"
        );
        assert!(
            !colour_at(source, &analysis, offset, TokenKind::Keyword, 0),
            "{source}"
        );
        assert!(
            !colour_at(
                source,
                &analysis,
                offset,
                TokenKind::Function,
                super::super::MOD_DEFAULT_LIBRARY
            ),
            "{source}"
        );
    }
    let source = "unknown argument";
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "reporting-only");
    assert!(colour_at(
        source,
        &analysis,
        0,
        TokenKind::Function,
        super::super::MOD_DEFAULT_LIBRARY
    ));
}
#[test]
fn logical_list_roles_keep_complete_unicode_escaped_and_semicolon_fields() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let input = logical_role_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    for (source, field, kind) in [
        (
            "proc p {é {lo\\u006eng 5} semi;colon} {}",
            "é",
            TokenKind::Parameter,
        ),
        (
            "proc p {é {lo\\u006eng 5} semi;colon} {}",
            r"lo\u006eng",
            TokenKind::Parameter,
        ),
        ("proc p {semi;colon} {}", "semi;colon", TokenKind::Parameter),
        ("foreach {é long} {} {}", "é", TokenKind::Variable),
        (
            "apply {{é {lo\\u006eng 5}} {return $é $long}}",
            r"lo\u006eng",
            TokenKind::Parameter,
        ),
        (
            "after idle [list apply {{é {lo\\u006eng 5}} {return $é $long}} 5]",
            r"lo\u006eng",
            TokenKind::Parameter,
        ),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(
            colour_at(
                source,
                &analysis,
                source.find(field).unwrap(),
                kind,
                MOD_DECLARATION
            ),
            "{source}: {field}"
        );
    }
}

#[test]
fn logical_list_roles_decline_cooked_subranges_opaque_and_malformed_lambdas() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let input = logical_role_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    for (source, field) in [
        (r"proc p {escaped\ name} {}", "escaped"),
        (r"proc p {\uD800} {}", r"\uD800"),
        (
            "apply {{argument} {return $argument} extra fourth}",
            "argument",
        ),
        ("apply $lambda argument", "argument"),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(
            !colour_at(
                source,
                &analysis,
                source.find(field).unwrap(),
                TokenKind::Parameter,
                MOD_DECLARATION
            ),
            "{source}"
        );
    }
    let source = "proc p {argument} {}";
    let mut analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "reporting-only");
    analysis.body_lexer_config = Some(tcl_lexer::LexerConfig {
        list_parse: tcl_dialect::ListParse::Lenient,
        ..analysis.body_lexer_config.unwrap()
    });
    assert!(!colour_at(
        source,
        &analysis,
        source.find("argument").unwrap(),
        TokenKind::Parameter,
        MOD_DECLARATION
    ));
}
