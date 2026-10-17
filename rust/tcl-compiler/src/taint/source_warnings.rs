// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional warnings retain source metadata independently of execution.

use super::{TaintColour, TaintLattice, TaintSourceContext, TaintWarning};
use crate::compilation_unit::FunctionUnit;
use crate::ir::{CommandBindingSites, CommandTokens, Module, SourceSite, WordExpr, WordPart};
use crate::registry_invocation::{EffectiveCommandWords, EffectiveInvocationWord};
use crate::ssa::{SsaSourceView, ValueKey};
use std::collections::HashMap;
use tcl_registry::{CommandRegistry, resolved_invocation::AuthoredSourceDestructivePathArguments};

struct SourceWarningInvocation {
    command: String,
    effective: EffectiveCommandWords,
    constraints: Vec<tcl_registry::taint::SetterConstraint>,
    paths: Option<AuthoredSourceDestructivePathArguments>,
    config: tcl_lexer::LexerConfig,
}

fn source_warning_invocation(
    registry: &CommandRegistry,
    module: &Module,
    tokens: &CommandTokens,
) -> Option<SourceWarningInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    metadata.source_analysis_input()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let schema = |selected: &tcl_registry::ResolvedInvocation<'_, '_>| {
        Some((
            selected
                .authored_source_descriptors()
                .command
                .setter_constraints
                .to_vec(),
            selected.authored_source_destructive_path_arguments(),
        ))
    };
    if let Some(assistance) =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )
        && assistance.unanimous_command_words().is_some()
        && let Some(shape) = assistance.candidates.first()
        && let Some((constraints, paths)) =
            shape.with_source_schema(registry, metadata, tokens, schema)
    {
        return Some(SourceWarningInvocation {
            command: shape.command.clone(),
            effective: shape.effective.clone(),
            constraints,
            paths,
            config,
        });
    }
    // Hosted names retain their own original vector, rather than borrowing a
    // Native handler or a Logical candidate from the written command label.
    let hosted = crate::registry_invocation::hosted_source_taint_invocation(registry, tokens)?;
    let shape = hosted.metadata.shape();
    let (constraints, paths) = shape.with_source_schema(registry, metadata, schema)?;
    Some(SourceWarningInvocation {
        command: shape.command().to_owned(),
        effective: hosted.effective,
        constraints,
        paths,
        config,
    })
}

impl SourceWarningInvocation {
    fn span(
        &self,
        tokens: &CommandTokens,
        argument: usize,
        fallback: tcl_lexer::Span,
    ) -> tcl_lexer::Span {
        self.effective
            .written_argument(argument)
            .and_then(|written| tokens.argv.get(written + 1).copied())
            .unwrap_or(fallback)
    }

    fn literal(&self, word: &WordExpr) -> EffectiveInvocationWord {
        crate::registry_invocation::effective_invocation_word(
            word,
            self.config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
    }

    fn variable(&self, spelling: &str) -> String {
        tcl_syntax::naming::var_reference_for_style(spelling, self.config.braced_var).to_owned()
    }
}

fn original_tokens<'a>(
    function: &'a FunctionUnit,
    view: SsaSourceView<'a>,
    statement: &'a crate::ir::Statement,
) -> Option<&'a CommandTokens> {
    let retained = CommandBindingSites::unanimous_statement_source_tokens(
        function.cfg.command_binding_sites.iter(),
        statement,
    )?;
    if view.source_tokens().is_some_and(|point| point != retained) {
        return None;
    }
    Some(retained)
}

fn variable_reads(word: &WordExpr) -> Vec<(&str, &SourceSite)> {
    match word {
        WordExpr::Variable { spelling, source } => vec![(spelling, source)],
        WordExpr::Template { parts, .. } => parts
            .iter()
            .filter_map(|part| match part {
                WordPart::Variable { spelling, source } => Some((spelling.as_str(), source)),
                WordPart::Text { .. }
                | WordPart::CommandSubstitution { .. }
                | WordPart::Opaque { .. } => None,
            })
            .collect(),
        WordExpr::Literal { .. }
        | WordExpr::BracedLiteral { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Expand { .. }
        | WordExpr::Opaque { .. } => Vec::new(),
    }
}

fn read_taint<S: std::hash::BuildHasher>(
    view: SsaSourceView<'_>,
    source: &SourceSite,
    spelling: &str,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> Option<TaintLattice> {
    let read = view.read_reference(source, spelling)?;
    taints.get(&(read.symbol, read.version?)).copied()
}

/// Path-concatenation advice with the same Module/FU currency and full lexer.
/// Source reachability is diagnostic knowledge, not Native execution authority.
#[must_use]
pub fn find_path_concat_warnings_for_module_function<S: std::hash::BuildHasher>(
    function: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> Vec<crate::path_concat::PathConcatWarning> {
    if TaintSourceContext::for_module_function(registry, module, function)
        .metadata
        .is_none()
    {
        return Vec::new();
    }
    crate::path_concat::find_path_concat_warnings(
        &function.cfg,
        &function.ssa,
        &function.rendered_props,
        taints,
        function.diagnostic_value_facts().executable_blocks(),
        function.source_lexer_config(),
    )
}

/// Setter warnings from the owning Module/FU, original argv and diagnostic values.
/// Missing ownership is terminal. Conditional knowledge supplies no Native
/// setter, completed read, sanitisation or executable replacement permission.
#[must_use]
pub fn find_setter_constraint_warnings_for_module_function<S: std::hash::BuildHasher>(
    function: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> Vec<TaintWarning> {
    let source = TaintSourceContext::for_module_function(registry, module, function);
    if source.metadata.is_none() || !super::is_irules_dialect(source.dialect) {
        return Vec::new();
    }
    let values = function.diagnostic_value_facts();
    let mut warnings = Vec::new();
    let safe_colours =
        TaintColour::PATH_PREFIXED | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED;
    for block in super::cfg_order(&function.cfg) {
        if !values.executable_blocks().contains(&block) {
            continue;
        }
        let Some(statements) = function.ssa.blocks.get(&block) else {
            continue;
        };
        for (index, statement) in statements.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            let Some(tokens) = original_tokens(function, view, &statement.statement) else {
                continue;
            };
            let Some(selected) = source_warning_invocation(registry, module, tokens) else {
                continue;
            };
            for constraint in &selected.constraints {
                let argument = usize::from(constraint.arg_index);
                let Some(word) = selected.effective.words.get(argument + 1) else {
                    continue;
                };
                let mut variable = String::new();
                let prefix_known = match selected.literal(word) {
                    EffectiveInvocationWord::Literal(value) => {
                        Some(value.starts_with(constraint.required_prefix))
                    }
                    EffectiveInvocationWord::ByteLiteral(value) => {
                        Some(value.starts_with(constraint.required_prefix.as_bytes()))
                    }
                    _ => None,
                };
                let violates = if let Some(prefix_known) = prefix_known {
                    !prefix_known
                } else if let Some((spelling, read_source)) = word.sole_variable_substitution() {
                    variable = selected.variable(spelling);
                    let known = view
                        .read_occurrence_reference(read_source, spelling)
                        .and_then(|key| values.values().get(&key))
                        .and_then(|value| match value {
                            crate::analyses::LatticeValue::Const(value) => {
                                crate::value_transfer::const_text(value)
                            }
                            _ => None,
                        });
                    if let Some(value) = known {
                        !value.starts_with(constraint.required_prefix)
                    } else {
                        !read_taint(view, read_source, spelling, taints).is_some_and(|taint| {
                            taint.is_tainted() && taint.colours.intersects(safe_colours)
                        })
                    }
                } else {
                    true
                };
                if violates {
                    warnings.push(TaintWarning {
                        source_sink: None,
                        span: selected.span(tokens, argument, statement.statement.span()),
                        variable,
                        sink_command: selected.command.clone(),
                        code: constraint.code,
                        message: constraint.message.to_owned(),
                        replacement: None,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }
    warnings
}

/// Destructive-path warnings from the original source schema and exact read sites.
/// Only represented colours suppress. Written guard/normalise labels cannot
/// establish path safety; dynamic option selection remains MAY metadata.
#[must_use]
pub fn find_destructive_file_warnings_for_module_function<S: std::hash::BuildHasher>(
    function: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> Vec<TaintWarning> {
    if TaintSourceContext::for_module_function(registry, module, function)
        .metadata
        .is_none()
    {
        return Vec::new();
    }
    let values = function.diagnostic_value_facts();
    let mut warnings = Vec::new();
    for block in super::cfg_order(&function.cfg) {
        if !values.executable_blocks().contains(&block) {
            continue;
        }
        let Some(statements) = function.ssa.blocks.get(&block) else {
            continue;
        };
        for (index, statement) in statements.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            let Some(tokens) = original_tokens(function, view, &statement.statement) else {
                continue;
            };
            let Some(selected) = source_warning_invocation(registry, module, tokens) else {
                continue;
            };
            let Some(paths) = selected.paths.as_ref() else {
                continue;
            };
            let mut warning = None;
            for argument in paths.arguments.clone() {
                let Some(word) = selected.effective.words.get(argument + 1) else {
                    continue;
                };
                for (spelling, read_source) in variable_reads(word) {
                    let taint = read_taint(view, read_source, spelling, taints);
                    let normalised = taint.is_some_and(|taint| {
                        taint.is_tainted() && taint.colours.contains(TaintColour::PATH_NORMALISED)
                    });
                    let bounded = taint.is_some_and(|taint| {
                        taint.is_tainted() && taint.colours.contains(TaintColour::PATH_BOUNDED)
                    });
                    if bounded && !paths.selection_unknown {
                        continue;
                    }
                    let variable = selected.variable(spelling);
                    let selector = paths.subcommands.join(" ");
                    let sink = if selector.is_empty() {
                        selected.command.clone()
                    } else {
                        format!("{} {selector}", selected.command)
                    };
                    let message = if normalised {
                        format!(
                            "{sink} with normalised path (${variable}) — verify it stays within the intended directory."
                        )
                    } else {
                        format!(
                            "{sink} with a variable path (${variable}) risks path-traversal. Normalise the path and verify it stays within the intended directory."
                        )
                    };
                    warning = Some(TaintWarning {
                        source_sink: None,
                        span: selected.span(tokens, argument, statement.statement.span()),
                        variable,
                        sink_command: sink,
                        code: crate::compiler_checks::DiagCode::W313,
                        message,
                        replacement: None,
                        fixes: Vec::new(),
                    });
                    break;
                }
                if warning.is_some() {
                    break;
                }
            }
            warnings.extend(warning);
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;

    fn unit(source: &str, input: &ResolvedAnalysisInput) -> CompilationUnit {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }

    fn logical_input(
        context: Arc<tcl_registry::model::ContextRegistry>,
        config: tcl_lexer::LexerConfig,
    ) -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        ResolvedAnalysisInput::new(profile, profile, context, config)
    }

    fn destructive(unit: &CompilationUnit, registry: &CommandRegistry) -> Vec<TaintWarning> {
        find_destructive_file_warnings_for_module_function(
            &unit.top_level,
            &unit.ir_module,
            registry,
            &HashMap::<ValueKey, TaintLattice>::new(),
        )
    }

    #[test]
    fn original_source_warnings_keep_alias_operands_and_full_variable_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let owner =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
        config.strict_quoting = !config.strict_quoting;
        let input = logical_input(Arc::clone(&owner), config);
        let source = "interp alias {} remove {} file delete --; remove ${a{b}c}";
        let actual = unit(source, &input);
        let warnings = destructive(&actual, owner.commands());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].variable, "a{b");
        assert_eq!(warnings[0].code, crate::compiler_checks::DiagCode::W313);
        assert_eq!(
            &source[warnings[0].span.start() as usize..warnings[0].span.end() as usize],
            "${a{b}c}"
        );
        let tokens = actual
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .native_compilation_admission
                .is_none()
        );
        let replaced = unit("proc file args {return STUB}; file delete -- $path", &input);
        assert!(destructive(&replaced, owner.commands()).is_empty());
        let uncertain = unit("file delete -unknown $path", &input);
        assert!(destructive(&uncertain, owner.commands()).is_empty());
        assert_eq!(
            destructive(&unit("file delete $missing", &input), owner.commands()).len(),
            1
        );
    }

    #[test]
    fn original_source_warnings_require_current_module_function_and_point_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let baseline =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let mut catalogue = CommandRegistry::build_default();
        let mut descriptor = catalogue.get("file").unwrap().clone();
        descriptor.name = "purge";
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        catalogue.insert(descriptor);
        let registry = Arc::new(catalogue);
        let current = Arc::new(baseline.with_command_store(Arc::clone(&registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::resolve_environment("tcl8.4")
                .default_context_registry()
                .with_command_store(Arc::clone(&registry)),
        );
        let config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        let input = logical_input(current, config);
        let old_input = logical_input(older, config);
        let source = "purge delete -- $path";
        let mut actual = unit(source, &input);
        assert_eq!(destructive(&actual, &registry).len(), 1);
        assert!(destructive(&unit(source, &old_input), &registry).is_empty());
        actual.ir_module.source_metadata_input = None;
        assert!(destructive(&actual, &registry).is_empty());
        actual.ir_module.source_metadata_input = Some(input.clone());
        actual.top_level.source_metadata_input = None;
        assert!(destructive(&actual, &registry).is_empty());
        actual.top_level.source_metadata_input = Some(input.clone());
        actual.ir_module.lexer_config.strict_quoting = !config.strict_quoting;
        assert!(destructive(&actual, &registry).is_empty());
        actual.ir_module.lexer_config = config;
        actual.top_level.source_config.strict_quoting = !config.strict_quoting;
        assert!(destructive(&actual, &registry).is_empty());
        actual.top_level.source_config = config;
        actual.ir_module.dialect_profile =
            Some(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
        assert!(destructive(&actual, &registry).is_empty());
        actual.ir_module.dialect_profile = Some(input.unit_profile());
        let mut foreign = CommandRegistry::build_default();
        let mut foreign_descriptor = foreign.get("file").unwrap().clone();
        foreign_descriptor.name = "foreign_source_warning_store";
        foreign.insert(foreign_descriptor);
        assert!(destructive(&actual, &foreign).is_empty());
        assert_eq!(
            destructive(&actual, &registry).len(),
            1,
            "restored genuine owner remains positive"
        );
    }

    #[test]
    fn original_cu_warning_compatibility_requires_explicit_standalone_tags() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // This public compatibility producer has no supplied source authority.
        // Its source warnings remain conditional, without Native execution.
        let registry = CommandRegistry::build_default();
        let source = "file delete $path";
        let mut standalone = CompilationUnit::build_for(source, &registry, false);
        assert!(
            standalone
                .ir_module
                .source_entry
                .metadata_context
                .is_standalone()
        );
        assert!(standalone.top_level.cfg.metadata_context.is_standalone());
        let warnings = |unit: &CompilationUnit| {
            super::super::find_taint_warnings_for_cu(unit, &registry, None)
                .into_iter()
                .filter(|warning| warning.code == crate::compiler_checks::DiagCode::W313)
                .collect::<Vec<_>>()
        };
        assert_eq!(warnings(&standalone).len(), 1);
        standalone.ir_module.source_entry.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        assert!(warnings(&standalone).is_empty());

        let environment = tcl_registry::model::ingress::resolve_environment("tcl9.1");
        let context = Arc::new(
            environment
                .default_context_registry()
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let input = logical_input(context, standalone.ir_module.lexer_config);
        let mut supplied = unit(source, &input);
        assert!(
            !supplied
                .ir_module
                .source_entry
                .metadata_context
                .is_standalone()
        );
        assert!(!supplied.top_level.cfg.metadata_context.is_standalone());
        assert_eq!(warnings(&supplied).len(), 1);
        supplied.ir_module.source_metadata_input = None;
        assert!(warnings(&supplied).is_empty());
        supplied.ir_module.source_metadata_input = Some(input);
        assert_eq!(warnings(&supplied).len(), 1);
        supplied.top_level.source_metadata_input = None;
        assert!(warnings(&supplied).is_empty());
    }

    #[test]
    fn original_setter_warnings_use_the_independent_hosted_source_schema() {
        // naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let owner = environment.default_context_registry();
        let profile = environment.unit_profile();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&owner),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        for (source, expected) in [
            ("HTTP::path bad", 1),
            ("HTTP::path /safe", 0),
            ("HTTP::path $missing", 1),
        ] {
            let actual = unit(source, &input);
            let warnings = find_setter_constraint_warnings_for_module_function(
                &actual.top_level,
                &actual.ir_module,
                owner.commands(),
                &HashMap::<ValueKey, TaintLattice>::new(),
            );
            assert_eq!(warnings.len(), expected, "{source}: {warnings:?}");
            if let Some(warning) = warnings.first() {
                assert_eq!(warning.code, crate::compiler_checks::DiagCode::Irule3101);
                assert_eq!(warning.sink_command, "HTTP::path");
            }
            let mut missing = actual;
            missing.ir_module.source_metadata_input = None;
            assert!(
                find_setter_constraint_warnings_for_module_function(
                    &missing.top_level,
                    &missing.ir_module,
                    owner.commands(),
                    &HashMap::<ValueKey, TaintLattice>::new(),
                )
                .is_empty()
            );
        }
    }
}
