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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

fn supplied(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let (context, seed) = crate::interprocedural::logical_completion_unit("", "tcl9.1");
    let original = seed.ir_module.source_metadata_input.as_ref().unwrap();
    let config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..original.lexer_config()
    };
    let input = ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        std::sync::Arc::clone(&context),
        config,
    );
    assert!(input.has_logical_source_name_context());
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (context, unit)
}

fn assert_original_summary_scripts_keep_bom(
    driver: &LatticeDriver<'_>,
    originals: [&crate::interprocedural::OriginalSummaryScript; 2],
    script: &str,
    lattice: Lattice<
        '_,
        std::collections::hash_map::RandomState,
        std::collections::hash_map::RandomState,
    >,
) {
    for original in originals {
        assert_eq!(original.text(), script);
        assert!(original.base() > 0);
        let command = original.command_at(original.base()).unwrap();
        assert_eq!(command.argv_texts[0], "\u{feff}emit");
        let ran = driver
            .run_script(
                script,
                lattice,
                (Vec::new(), NestedPolicy::EffectFreeOnly),
                Some(original),
            )
            .unwrap();
        assert_eq!(ran.head, "\u{feff}emit");
        assert_eq!(ran.binding.identity, "list");
        assert!(
            driver
                .run_script(
                    "list VALUE",
                    lattice,
                    (Vec::new(), NestedPolicy::EffectFreeOnly),
                    Some(original)
                )
                .is_none()
        );
    }
    assert!(
        driver
            .run_script(
                script,
                lattice,
                (Vec::new(), NestedPolicy::EffectFreeOnly),
                None
            )
            .is_none()
    );
}

fn assert_missing_summary_owner(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    parents: &[crate::ir::CommandTokens],
    script: &str,
) {
    let mut changed = (*unit).clone();
    changed.ir_module.source_metadata_input = None;
    let missing = crate::interprocedural::ModuleProcedures::of_unit(&changed, registry);
    assert!(
        parents
            .iter()
            .all(|parent| missing.original_body(parent, 0, script).is_none())
    );
}

fn assert_protected_summary_keeps_original_source(
    driver: &LatticeDriver<'_>,
    script: &str,
    state: &mut EvaluationState,
    inputs: &LatticeInputs<
        '_,
        std::collections::hash_map::RandomState,
        std::collections::hash_map::RandomState,
    >,
) {
    assert!(matches!(
        driver.protected_script(script, state, inputs),
        EvalAnswer::Evaluated(_)
    ));
    assert!(matches!(
        driver.protected_script("list VALUE", state, inputs),
        EvalAnswer::Declined(_)
    ));
}

fn assert_summary_config_keeps_original_grammar(
    driver: &LatticeDriver<'_>,
    unit: &CompilationUnit,
    config: LexerConfig,
) {
    assert_eq!(driver.lexer_config, config);
    assert_eq!(
        unit.procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
            .source_lexer_config(),
        config
    );
}

#[test]
fn original_summary_child_keeps_bom_alias_and_complete_parent_correspondence() {
    // naming.interprocedural.original-transfer-summary-source-context
    // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
    // Conditional source/value modelling, not Native Normal completion or store permission.
    let script = "\u{feff}emit VALUE";
    let source = "\u{feff}interp alias {} \u{feff}emit {} list\nproc p {} {catch {\u{feff}emit VALUE}; return [\u{feff}emit VALUE]}";
    let (context, unit) = supplied(source);
    let module = crate::interprocedural::ModuleProcedures::of_unit(&unit, context.commands());
    let procedure = unit
        .ir_module
        .procedures
        .get(&fixture_id(&unit.ir_module.procedures, "::p"))
        .expect("authored fixture body");
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let config = input.lexer_config();
    let parents: Vec<_> = procedure
        .body
        .statements
        .iter()
        .filter_map(|statement| {
            module.original_statement_tokens(
                Some(fixture_id(&unit.ir_module.procedures, "::p")),
                statement,
            )
        })
        .collect();
    let body = parents
        .iter()
        .find_map(|parent| module.original_body(parent, 0, script))
        .expect("genuine retained catch body");
    let child = parents
        .iter()
        .find_map(|parent| {
            let (_, site) = parent
                .words()
                .iter()
                .find_map(crate::ir::WordExpr::sole_command_substitution)?;
            module.original_substitution(parent, site, script)
        })
        .expect("genuine retained bracket child");
    let driver = LatticeDriver::new(
        TraceInputs {
            source_metadata_input: Some(input),
            registry: context.commands(),
            traced_variables: &EMPTY_NAMES,
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
        None,
        FoldPolicy::for_profile(None, Some(input.unit_profile())),
        &HashSet::new(),
    )
    .with_cfg_metadata(
        &unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
            .cfg
            .metadata_context,
    )
    .in_module(
        &unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .unwrap()
            .cfg,
        crate::sccp::ModuleRun {
            analysis_body: Some(crate::sccp::AnalysisBody::Procedure(fixture_id(
                &unit.ir_module.procedures,
                "::p",
            ))),
            procedures: Some(&module),
            owned: None,
            level: crate::sccp::ModuleLevel::Places,
            reads_exits: false,
        },
    );
    let uses = HashMap::<Symbol, Version>::new();
    let values = HashMap::<ValueKey, LatticeValue>::new();
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    assert_original_summary_scripts_keep_bom(
        &driver,
        [&body, &child],
        script,
        (&uses, &values, ssa),
    );
    let inputs = driver.expression_inputs((&uses, &values, ssa));
    *inputs.nested_source.borrow_mut() = Some(body.clone());
    let mut state = EvaluationState::new(NestedPolicy::Protected);
    assert_protected_summary_keeps_original_source(&driver, script, &mut state, &inputs);
    assert_summary_config_keeps_original_grammar(&driver, &unit, config);
    assert_missing_summary_owner(&unit, context.commands(), &parents, script);
}

#[test]
fn original_return_region_keeps_bom_alias_first_close_and_whole_word_config() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Actual reached CFG return source geometry; Native value/read/observer gates remain independent.
    for argument in ["VALUE", "${a{b}}"] {
        let source = format!(
            "\u{feff}rename return {{\u{feff}back}}\nproc p {{}} {{\u{feff}back {argument}}}"
        );
        let (context, unit) = supplied(&source);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body");
        let config = function.source_lexer_config();
        let (&block, _) = function
            .cfg
            .blocks
            .iter()
            .find(|(_, block)| {
                matches!(
                    &block.terminator,
                    Some(crate::cfg::Terminator::Return { span: Some(_), .. })
                )
            })
            .expect("original return terminator");
        let tokens = return_command_tokens(function, block, &source, config).unwrap();
        assert_eq!(tokens.argv_texts, ["\u{feff}back", argument]);
        assert_eq!(&source[tokens.argv[0].as_range()], "\u{feff}back");
        assert_eq!(
            &source[tokens.words()[1].source().span.as_range()],
            argument
        );
        assert_eq!(
            function.source_metadata_input().unwrap().lexer_config(),
            config
        );
        assert!(
            function
                .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
                .is_some()
        );
        let mut unreached = function.clone();
        unreached.sccp.executable_blocks.remove(&block);
        assert!(return_command_tokens(&unreached, block, &source, config).is_none());
    }
}

#[test]
fn standalone_loop_child_keeps_bom_and_registered_utility_purpose() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Explicit independent registry utility; no original Module, frame or Native entry is issued.
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let original = registry.get("list").unwrap().clone();
    registry.insert(tcl_registry::CommandSpec {
        name: "\u{feff}list",
        ..original
    });
    let mut driver = LatticeDriver::over_registry(&registry, FoldPolicy::default());
    driver.lexer_config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        strict_quoting: true,
        ..driver.lexer_config
    };
    let parent = driver.lexer_config;
    let state = crate::static_loops::LoopState::default();
    let input = StateInputs {
        driver: &driver,
        state: &state,
        view: ResolvedInvocationView {
            canonical_command: "list",
            subcommand: None,
            form: None,
            layout: InvocationLayout::Source,
            operands: Vec::new(),
            argument_offset: 0,
            arity: None,
        },
    };
    assert!(matches!(
        input.run_nested("\u{feff}list VALUE"),
        EvalAnswer::Evaluated(_)
    ));
    assert_eq!(driver.lexer_config, parent);
    assert!(matches!(
        input.run_nested("unknown VALUE"),
        EvalAnswer::Declined(_)
    ));
}
