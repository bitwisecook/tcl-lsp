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

//! The compile entry's policy: logical engine, lexical evidence, live policy and the namespace a runtime entry selects.

use super::*;

#[test]
fn native_entry_keeps_logical_engine_and_lexical_evidence_independent() {
    use tcl_dialect::model::{DialectPoint, Release};
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let actual = DialectPoint::canonical(Release::JIM_0_84);
    let mut entry = policy_entry(profile, actual);
    entry.lexer_grammar = Some(tcl_registry::InvocationDialect::of_point(actual).lexer_grammar);
    entry.invocation_policy = Some(
        tcl_registry::model::ingress::resolve_environment("jim")
            .analyser_profile()
            .cache_key(),
    );
    assert_eq!(
        BytecodeCompileService::native_entry_options(&entry, Some(profile))
            .invocation_dialect
            .expect("actual runtime")
            .core_point,
        Some(actual),
    );
    assert_eq!(
        BytecodeCompileService::native_entry_options(&entry, None)
            .invocation_dialect
            .expect("runtime without catalogue")
            .core_point,
        Some(actual),
    );
    let catalogue_config = LexerConfig::from_grammar(profile.grammar);
    assert_eq!(
        BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
        LexerConfig::from_grammar(tcl_registry::InvocationDialect::of_point(actual).lexer_grammar),
    );
    let mut grammar = entry.lexer_grammar.expect("measured grammar");
    grammar.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Folds;
    entry.lexer_grammar = Some(grammar);
    let selected = BytecodeCompileService::native_entry_options(&entry, Some(profile))
        .invocation_dialect
        .expect("independent actual axes");
    assert_eq!(selected.core_point, Some(actual));
    assert_eq!(selected.lexer_grammar, grammar);
    assert_eq!(
        selected.word_values,
        tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar)
    );
    assert_eq!(
        BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
        LexerConfig::from_grammar(grammar)
    );
    entry.lexer_grammar = None;
    assert_eq!(
        BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
        catalogue_config
    );
    entry.execution_point = None;
    entry.invocation_policy = Some(profile.cache_key());
    entry.lexer_grammar = None;
    assert_eq!(
        BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
        catalogue_config
    );
    assert_eq!(
        BytecodeCompileService::native_entry_options(&entry, Some(profile))
            .invocation_dialect
            .expect("explicit profile compatibility")
            .core_point,
        tcl_registry::InvocationDialect::of_profile(profile).core_point,
    );
    assert!(
        BytecodeCompileService::native_entry_options(&entry, Some(profile))
            .native_compiler_dialect()
            .is_none()
    );
    entry.execution_point = Some(actual);
    entry.invocation_policy = None;
    let unknown_policy = BytecodeCompileService::native_entry_options(&entry, Some(profile));
    assert!(unknown_policy.invocation_dialect.is_none());
    assert_eq!(
        unknown_policy.native_compiler_dialect().unwrap().core_point,
        Some(actual)
    );
}

#[test]
fn physical_c9_compiler_cannot_replace_f5_logical_value_or_frame_policy() {
    use tcl_dialect::model::{DialectPoint, Release};
    use tcl_registry::InvocationDialect;
    let logical_profile = DialectProfile::find("f5-irules").unwrap();
    let assistance = DialectProfile::find("tcl9.1").unwrap();
    let physical_point = DialectPoint::canonical(Release::TCL_9_1);
    let mut entry = policy_entry(logical_profile, physical_point);
    let logical = InvocationDialect::of_profile(logical_profile);
    let physical = InvocationDialect::of_point(physical_point);
    assert_ne!(logical.numbers, physical.numbers);
    let options = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
    assert_eq!(options.invocation_dialect, Some(logical));
    assert_eq!(options.native_compiler_dialect(), Some(physical));
    assert_eq!(entry.profile, logical_profile.cache_key());

    // A host can independently select a logical compatibility policy.
    let compatibility = DialectProfile::find("tcl8.4").unwrap();
    entry.invocation_policy = Some(compatibility.cache_key());
    let compatible = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
    assert_eq!(
        compatible.invocation_dialect,
        Some(InvocationDialect::of_profile(compatibility))
    );
    assert_eq!(compatible.native_compiler_dialect(), Some(physical));
    assert_eq!(entry.profile, logical_profile.cache_key());
    let compatible_policy = compatible.invocation_dialect;

    entry.execution_point = None;
    let unknown_engine = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
    assert_eq!(unknown_engine.invocation_dialect, compatible_policy);
    assert!(unknown_engine.native_compiler_dialect().is_none());
    entry.execution_point = Some(physical_point);
    entry.invocation_policy = None;
    let unknown_policy = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
    assert!(unknown_policy.invocation_dialect.is_none());
    assert_eq!(unknown_policy.native_compiler_dialect(), Some(physical));
}

#[test]
fn direct_source_options_cannot_fill_missing_live_policy_from_authoring() {
    use tcl_compiler::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_dialect::model::{DialectPoint, Release};
    use tcl_registry::InvocationDialect;
    let logical = DialectProfile::find("f5-irules").unwrap();
    let physical = DialectPoint::canonical(Release::TCL_9_1);
    let mut entry = policy_entry(logical, physical);
    let config = LexerConfig {
        strict_quoting: true,
        base_line: 12,
        base_col: 4,
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..LexerConfig::from_grammar(DialectProfile::find("tcl8.6").unwrap().grammar)
    };
    let registry = CommandRegistry::build_default();
    let retained_policy = |entry: &tcl_runtime_api::NativeCompilationEntry| {
        let options = SourceAnalysisOptions {
            native_entry: Some(entry),
            invocation_dialect: Some(InvocationDialect::of_point(physical)),
            ..SourceAnalysisOptions::default()
        };
        assert_eq!(options.native_lexer_config(config), config);
        assert_eq!(
            options.native_compiler_dialect().unwrap().core_point,
            Some(physical)
        );
        SourceCommandBindings::analyse_with_options("set x 1", config, &registry, options)
            .invocation_at_source("set", 0)
            .variable_context
            .invocation_dialect
    };
    let policy = retained_policy(&entry).expect("retained logical policy");
    let expected = InvocationDialect::of_profile(logical);
    assert_eq!(policy.numbers, expected.numbers);
    assert_eq!(policy.characters, expected.characters);
    assert_eq!(policy.upvar_level_presence, expected.upvar_level_presence);
    assert_eq!(
        policy.lexer_grammar,
        config.grammar_over(expected.lexer_grammar)
    );
    entry.invocation_policy = None;
    assert!(retained_policy(&entry).is_none());
    let mut measured = config.grammar_over(expected.lexer_grammar);
    measured.var_syntax = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84))
        .lexer_grammar
        .var_syntax;
    entry.lexer_grammar = Some(measured);
    let measured_options = SourceAnalysisOptions {
        native_entry: Some(&entry),
        invocation_dialect: Some(expected),
        ..SourceAnalysisOptions::default()
    };
    assert!(measured_options.logical_invocation_dialect().is_none());
    assert_eq!(
        measured_options.native_lexer_config(config),
        config.with_grammar(measured)
    );
    let positioned = LexerConfig {
        base_offset: 37,
        ..config
    };
    assert_eq!(
        BytecodeCompileService::native_entry_config(positioned, Some(&entry)),
        positioned.with_grammar(measured)
    );
}

#[test]
fn runtime_entry_selects_actual_namespace_and_keeps_custom_shadow_generic() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let namespace = tcl_core_types::ByteNamespacePath::from_segments(["N"]);
    let entry = opaque_set_namespace_entry(profile, namespace);
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let ir = lower_script_module_for_bytecode_with_options(
        "set x 1",
        "",
        registry,
        LexerConfig::from_grammar(profile.grammar),
        Some(profile),
        false,
        Some(BytecodeCompileService::native_entry_options(
            &entry,
            Some(profile),
        )),
    );
    assert_eq!(ir.top_level_namespace, "::N");
    assert_eq!(ir.source_entry.native_entry.as_deref(), Some(&entry));
    let service = BytecodeCompileService::for_profile(profile);
    let compiled = service
        .compile_script_with_entry(
            ScriptCompileTarget {
                source: "set x 1",
                namespace: "",
                evaluation: CommandEvaluation::Compiled,
            },
            profile,
            &entry,
        )
        .unwrap();
    assert!(
        compiled
            .top_level
            .instructions
            .iter()
            .any(|instruction| matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            ))
    );
    assert_eq!(
        compiled.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
}
