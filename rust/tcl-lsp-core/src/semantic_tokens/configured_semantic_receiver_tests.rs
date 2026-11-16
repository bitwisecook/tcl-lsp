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
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

fn analyse(source: &str, config: tcl_lexer::LexerConfig, custom_class: bool) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = CommandRegistry::build_default();
    if custom_class {
        let mut spec = registry.get("itcl::class").unwrap().clone();
        spec.name = "source_class";
        spec.required_package = None;
        registry.insert(spec);
    }
    let context = tcl_registry::model::ingress::static_context_for("tcl9.1")
        .with_command_store(Arc::new(registry));
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(context),
            config,
        ))
        .analyse(source, profile.name)
}

fn method_lines(source: &str, analysis: &AnalysisResult) -> Vec<u32> {
    let registry = analysis.resolved_registry().unwrap();
    let unit =
        crate::original_invocation::source_compilation_unit(source, analysis, registry).unwrap();
    super::tests::decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        registry,
        Some(&unit),
        Some(analysis),
    ))
    .into_iter()
    .filter_map(|(line, _, _, kind, _)| (kind == TokenKind::Method as u32).then_some(line))
    .collect()
}

#[test]
fn actual_method_colours_do_not_promote_compound_or_escaped_variable_heads() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = concat!(
        "oo::class create C {method longMethod {} {}}\n",
        "set p [C new]\n",
        "${p} longMethod\n",
        "${p}suffix longMethod\n",
        "\\$p longMethod\n",
        "${p}[set suffix] longMethod\n",
        "\"$p\" longMethod\n",
        "set pé [C new]\n",
        "${pé} longMethod\n",
        "${pé}suffix longMethod\n",
    );
    let current = analyse(source, tcl_lexer::LexerConfig::default(), false);
    let lines = method_lines(source, &current);
    assert!(lines.contains(&2), "{lines:?}");
    assert!(lines.contains(&6), "{lines:?}");
    assert!(lines.contains(&8), "{lines:?}");
    for line in [3, 4, 5, 9] {
        assert!(!lines.contains(&line), "{line}: {lines:?}");
    }
}

#[test]
fn logical_method_colours_use_the_current_source_object_candidate_producer() {
    let source = "oo::class create C {method m {} {}}\nset object [C new]\n$object m";
    let analysis = analyse(source, tcl_lexer::LexerConfig::default(), false);
    let registry = analysis.resolved_registry().unwrap();
    let unit =
        crate::original_invocation::source_compilation_unit(source, &analysis, registry).unwrap();
    let colours = |unit: &CompilationUnit| {
        super::tests::decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            registry,
            Some(unit),
            Some(&analysis),
        ))
        .into_iter()
        .any(|(line, _, _, kind, _)| line == 2 && kind == TokenKind::Method as u32)
    };
    assert!(colours(&unit));
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert!(!colours(&missing));
    let mut changed = unit.clone();
    changed.source.push(' ');
    assert!(!colours(&changed));
}

#[test]
fn actual_implicit_self_colours_require_the_sole_original_variable_word() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = concat!(
        "source_class C {\n",
        "method m {} {}\n",
        "method run {} {\n",
        "${this} m\n",
        "${this}suffix m\n",
        "\\$this m\n",
        "}\n",
        "}\n",
    );
    let current = analyse(source, tcl_lexer::LexerConfig::default(), true);
    let lines = method_lines(source, &current);
    assert!(lines.contains(&3), "{lines:?}");
    assert!(!lines.contains(&4), "{lines:?}");
    assert!(!lines.contains(&5), "{lines:?}");
}

#[test]
fn implicit_receiver_colours_use_the_selected_explicit_variable_role() {
    use tcl_registry::definer::{DefinitionBodyGrammar, ITCL_GRAMMAR};
    const GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
        implicit_vars: &["receiver", "options"],
        implicit_receiver_vars: &["receiver", "hidden"],
        ..ITCL_GRAMMAR
    };
    let source = concat!(
        "source_class C {\n",
        "method m {} {}\n",
        "method run {} {\n",
        "${receiver} m\n",
        "${options} m\n",
        "${hidden} m\n",
        "${this} m\n",
        "proc helper {} {${receiver} m}\n",
        "apply {{} {${receiver} m}}\n",
        "}\n",
        "}\n",
    );
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = CommandRegistry::build_default();
    let mut spec = registry.get("itcl::class").unwrap().clone();
    spec.name = "source_class";
    spec.required_package = None;
    spec.definition_body = Some(&GRAMMAR);
    registry.insert(spec);
    let context = tcl_registry::model::ingress::static_context_for("tcl9.1")
        .with_command_store(Arc::new(registry));
    let analysis = Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(context),
            tcl_lexer::LexerConfig::default(),
        ))
        .analyse(source, profile.name);
    let lines = method_lines(source, &analysis);
    assert!(lines.contains(&3), "{lines:?}");
    for line in [4, 5, 6, 7, 8] {
        assert!(!lines.contains(&line), "{line}: {lines:?}");
    }
}

#[test]
fn implicit_self_receiver_vocabulary_does_not_leak_into_fresh_bodies() {
    let source = concat!(
        "source_class C {\n",
        "method m {} {}\n",
        "method run {} {\n",
        "${this} m\n",
        "if {1} {${this} m}\n",
        "proc helper {} {${this} m}\n",
        "apply {{} {${this} m}}\n",
        "{${this}} m\n",
        "}\n",
        "}\n",
    );
    let current = analyse(source, tcl_lexer::LexerConfig::default(), true);
    let lines = method_lines(source, &current);
    assert!(lines.contains(&3), "{lines:?}");
    assert!(lines.contains(&4), "{lines:?}");
    for line in [5, 6, 7] {
        assert!(!lines.contains(&line), "{line}: {lines:?}");
    }
}

#[test]
fn actual_loop_collection_names_follow_the_retained_brace_configuration() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "foreach out ${a{b}c} {}";
    let collections = ObjectClassMap::from([(
        "a{b}c".to_owned(),
        std::collections::HashSet::from(["::Element".to_owned()]),
    )]);
    let mut config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::default()
    };
    let current = analyse(source, config, false);
    let mut handles = ObjectClassMap::default();
    augment_loop_var_handles(source, &current, &collections, &mut handles);
    assert_eq!(handles.get("out"), collections.get("a{b}c"));
    config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
    let first_close = analyse(source, config, false);
    handles.clear();
    augment_loop_var_handles(source, &first_close, &collections, &mut handles);
    assert!(!handles.contains_key("out"));
    let stale = "foreach out ${a{b}c}Suffix {}";
    handles.clear();
    augment_loop_var_handles(stale, &current, &collections, &mut handles);
    assert!(!handles.contains_key("out"));
}
