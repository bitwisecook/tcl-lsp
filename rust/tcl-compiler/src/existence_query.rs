// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook/tcl-lsp>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-resolved variable-existence query recognition.
//!
//! The parser owns expression structure, while the registry owns which
//! invocation denotes an existence operation. Rooted spellings such as
//! `::info exists name` therefore follow the same resolved invocation as
//! every other compiler consumer.

use crate::expr_ast::{ExprNode, UnaryOp};

/// The fact an existence query asks about a variable name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistenceKind {
    /// The name is bound to either a scalar or an array.
    AnyVariable,
    /// The name is bound specifically to an array.
    Array,
}

/// A registry-resolved command-substitution existence query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistenceQuery {
    /// The queried name, exactly as written.
    pub(crate) var: String,
    /// Whether the containing condition negates the query.
    pub(crate) negated: bool,
    /// The query's registry-owned semantic distinction.
    pub(crate) kind: ExistenceKind,
}

/// Recognise an existence query at its exact retained nested dispatch point.
/// Explicit unknown or absent implementations never recover facts from spelling.
#[must_use]
pub(crate) fn in_expr_at_with_metadata_context(
    node: &ExprNode,
    expression_base: u32,
    parent: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    let metadata = metadata.filter(|context| context.matches_registry(registry))?;
    in_expr_at_inner(
        node,
        expression_base,
        parent,
        registry,
        config,
        Some(metadata),
    )
}

fn in_expr_at_inner(
    node: &ExprNode,
    expression_base: u32,
    parent: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => {
            let (mut query, context) =
                in_expr_at_inner(operand, expression_base, parent, registry, config, metadata)?;
            query.negated = !query.negated;
            Some((query, context))
        }
        ExprNode::Command { text, start, end } => {
            let source = crate::ir::SourceSite::source(tcl_lexer::Span::new(
                expression_base.checked_add(*start)?,
                expression_base.checked_add(*end)?,
            ));
            let mut nested = crate::word_subst::nested_command_words(text, &source, config).ok()?;
            nested.inherit_nested_bindings(parent);
            in_tokens_with_metadata_context(&nested, registry, metadata)
        }
        _ => None,
    }
}

/// Registry operation and its selected relative variable-name operand.
/// Name existence contributes a dependency, never a required contents read.
pub(crate) fn operand(facts: &tcl_registry::InvocationFacts) -> Option<(ExistenceKind, usize)> {
    let kind = match facts.operation {
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists) => {
            ExistenceKind::AnyVariable
        }
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ArrayExists) => {
            ExistenceKind::Array
        }
        _ => return None,
    };
    Some((kind, facts.argument_offset))
}

#[cfg(test)]
fn in_tokens(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    in_tokens_inner(tokens, registry, standalone_context(registry))
}

pub(crate) fn in_tokens_with_metadata_context(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    let metadata = metadata.filter(|context| context.matches_registry(registry))?;
    in_tokens_inner(tokens, registry, Some(metadata))
}

fn in_tokens_inner(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry, metadata, tokens,
    )?;
    let (kind, argument) = operand(&invocation.facts)?;
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    Some((
        ExistenceQuery {
            var: invocation.argument_literal(argument)?,
            negated: false,
            kind,
        },
        std::sync::Arc::clone(&tokens.source_binding.as_ref()?.variable_context),
    ))
}

/// Diagnostic-only selection under the exact original declaration lookup.
/// Its context cannot establish actual contents presence or permit erasure.
#[cfg(test)]
pub(crate) fn in_tokens_for_diagnostics(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    in_tokens_for_diagnostics_inner(tokens, registry, standalone_context(registry))
}

pub(crate) fn in_tokens_for_diagnostics_with_metadata_context(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    let metadata = metadata.filter(|context| context.matches_registry(registry))?;
    in_tokens_for_diagnostics_inner(tokens, registry, Some(metadata))
}

fn in_tokens_for_diagnostics_inner(
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    if let Some(actual) = in_tokens_inner(tokens, registry, metadata) {
        return Some(actual);
    }
    let binding = tokens.source_binding.as_ref()?;
    let advice = binding.declaration_operand_layout_advice(tokens)?;
    if !advice.closed_lookup() || advice.has_opaque_handler_alternatives() {
        return None;
    }
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        if !target.registry_backed {
            return None;
        }
        let effective = crate::registry_invocation::effective_words_for_target(tokens, target)?;
        let values =
            crate::registry_invocation::declared_argument_words(tokens, &effective, dialect);
        let mut words = vec![tcl_registry::InvocationWord::Literal(&target.command)];
        words.extend(
            values
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word),
        );
        let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
            crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
                registry,
                metadata,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let (kind, argument) = operand(&facts)?;
        if facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let var = words.get(argument.checked_add(1)?)?.literal()?.to_owned();
        let query = ExistenceQuery {
            var,
            negated: false,
            kind,
        };
        if agreed.as_ref().is_some_and(|previous| previous != &query) {
            return None;
        }
        agreed = Some(query);
    }
    Some((
        agreed?,
        std::sync::Arc::clone(advice.original_variable_context()),
    ))
}

/// Conditional branch guard advice under actual supplied metadata. Runtime
/// presence and edit queries retain their own original invocation premises.
pub(crate) fn in_expr_for_diagnostics_at_with_metadata_context(
    node: &ExprNode,
    expression_base: u32,
    parent: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    let metadata = metadata.filter(|context| context.matches_registry(registry))?;
    in_expr_for_diagnostics_at_inner(
        node,
        expression_base,
        parent,
        registry,
        config,
        Some(metadata),
    )
}

fn in_expr_for_diagnostics_at_inner(
    node: &ExprNode,
    expression_base: u32,
    parent: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<(
    ExistenceQuery,
    std::sync::Arc<crate::var_resolve::ResolveContext>,
)> {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => {
            let (mut query, context) = in_expr_for_diagnostics_at_inner(
                operand,
                expression_base,
                parent,
                registry,
                config,
                metadata,
            )?;
            query.negated = !query.negated;
            Some((query, context))
        }
        ExprNode::Command { text, start, end } => {
            let source = crate::ir::SourceSite::source(tcl_lexer::Span::new(
                expression_base.checked_add(*start)?,
                expression_base.checked_add(*end)?,
            ));
            let mut nested = crate::word_subst::nested_command_words(text, &source, config).ok()?;
            nested.inherit_nested_bindings(parent);
            in_tokens_for_diagnostics_inner(&nested, registry, metadata)
        }
        _ => None,
    }
}

/// Recognise source syntax only under an explicitly retained Logical input.
/// There is no positioned Native lookup or contents fact in an expression label.
pub(crate) fn in_expr_with_metadata_context(
    node: &ExprNode,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<ExistenceQuery> {
    let metadata = metadata.filter(|context| {
        context.matches_registry(registry) && context.permits_logical_source_names()
    })?;
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => {
            let mut query =
                in_expr_with_metadata_context(operand, registry, config, Some(metadata))?;
            query.negated = !query.negated;
            Some(query)
        }
        ExprNode::Command { text, .. } => {
            let inner = text.strip_prefix('[')?.strip_suffix(']')?;
            let length = u32::try_from(inner.len()).ok()?;
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(inner),
                tcl_lexer::Span::new(0, length),
                config,
            )
            .ok()?;
            let [command] = plan.commands.as_slice() else {
                return None;
            };
            let values = command
                .words
                .iter()
                .map(tcl_syntax::word_rules::original_static_word_ascii_presentation)
                .collect::<Option<Vec<_>>>()?;
            let text = values
                .iter()
                .map(|value| core::str::from_utf8(value).ok())
                .collect::<Option<Vec<_>>>()?;
            let words = text
                .iter()
                .map(|word| tcl_registry::InvocationWord::Literal(word))
                .collect::<Vec<_>>();
            let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
                crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
                    registry,
                    Some(metadata),
                    &words,
                    None,
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .ok()?
            else {
                return None;
            };
            let (kind, argument) = operand(&facts)?;
            if facts.arity_accepts_frozen_arguments() != Some(true) {
                return None;
            }
            Some(ExistenceQuery {
                var: words.get(argument.checked_add(1)?)?.literal()?.to_owned(),
                negated: false,
                kind,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
fn standalone_context(
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
    registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile)
        .map(Into::into)
}

/// Recognise one bracketed command substitution as an existence query.
#[must_use]
#[cfg(test)]
pub(crate) fn in_text(
    text: &str,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, ExistenceKind)> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    let words: Vec<&str> = command.texts.iter().map(String::as_str).collect();
    let (head, args) = words.split_first()?;
    let [_subcommand, variable] = args else {
        return None;
    };
    let operation = registry
        .resolve_invocation(head, args, registry.own_surface_query())?
        .semantics
        .operation;
    let kind = match operation {
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists) => {
            ExistenceKind::AnyVariable
        }
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ArrayExists) => {
            ExistenceKind::Array
        }
        _ => return None,
    };
    Some(((*variable).to_owned(), kind))
}

#[cfg(test)]
mod tests {
    use super::{ExistenceKind, in_text};

    fn original_query_tokens(
        engine: &str,
        prefix: &str,
        command: &str,
    ) -> (
        crate::ir::CommandTokens,
        std::sync::Arc<tcl_registry::CommandRegistry>,
    ) {
        let registry = tcl_registry::model::ingress::static_context_for(engine)
            .commands()
            .clone();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(engine)).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let source = format!("{prefix}proc f {{}} {{{command}}}");
        let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
            &source,
            config,
            &registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind(command).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(&source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        (tokens, registry)
    }

    #[test]
    fn original_existence_advice_is_a_name_dependency_without_actual_presence() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            for command in ["info exists u", "array exists u"] {
                let (tokens, registry) = original_query_tokens(engine, "", command);
                let (query, context) = super::in_tokens_for_diagnostics(&tokens, &registry)
                    .unwrap_or_else(|| panic!("{engine}: {command}"));
                assert_eq!(query.var, "u");
                let binding = tokens.source_binding.as_ref().unwrap();
                assert_eq!(
                    context.frame_kind,
                    crate::var_resolve::VariableFrameKind::Local
                );
                assert!(super::in_tokens(&tokens, &registry).is_none());
                assert!(binding.proved_execution_target().is_none());
                let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
                let flow = crate::registry_invocation::declaration_invocation_flow(
                    &registry, &tokens, &advice,
                )
                .unwrap();
                assert!(flow.reads.is_empty(), "{engine}: {command}");
                assert_eq!(flow.name_queries, ["u"]);
            }
        }
    }

    #[test]
    fn original_existence_advice_tracks_selected_alias_and_imported_handlers() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for (prefix, command) in [
                ("interp alias {} query {} info exists; ", "query u"),
                (
                    "namespace eval Q {interp alias {} ::Q::query {} info exists; namespace export query}; namespace import Q::query; ",
                    "query u",
                ),
            ] {
                let (tokens, registry) = original_query_tokens(engine, prefix, command);
                assert_eq!(
                    super::in_tokens_for_diagnostics(&tokens, &registry)
                        .unwrap()
                        .0
                        .var,
                    "u"
                );
            }
            for prefix in [
                "rename info stock_info; proc info args {return 1}; ",
                "proc query {name} {return 1}; ",
                "unknown_mutation; ",
            ] {
                let command = if prefix.starts_with("proc query") {
                    "query u"
                } else {
                    "info exists u"
                };
                let (tokens, registry) = original_query_tokens(engine, prefix, command);
                assert!(
                    super::in_tokens_for_diagnostics(&tokens, &registry).is_none(),
                    "{engine}: {prefix}"
                );
            }
            let (mut tokens, registry) = original_query_tokens(engine, "", "info exists u");
            tokens.word_exprs.pop();
            assert!(super::in_tokens_for_diagnostics(&tokens, &registry).is_none());
        }
    }

    fn diagnostic_query_summary(source: &str, engine: &str) -> Vec<String> {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let Some(function) = unit.function("::f") else {
            return vec!["no function".into()];
        };
        let mut rows = Vec::new();
        for (&block, data) in &function.cfg.blocks {
            for (index, statement) in data.statements.iter().enumerate() {
                let Some(tokens) = statement.tokens() else {
                    continue;
                };
                let query = super::in_tokens_for_diagnostics(tokens, registry);
                let query_cell = query.as_ref().and_then(|(query, context)| {
                    crate::var_resolve::canonical_place_key(
                        &crate::var_resolve::resolve_literal_place(
                            &query.var, context, false, registry,
                        ),
                    )
                });
                let symbol = function.ssa.var_symbol_at(block, index, "u");
                rows.push(format!("statement={index} synthetic={:?} query={} actual={} query-cell={query_cell:?} ssa-cell={:?} accesses={}",
                    tokens.synthetic, query.is_some(), super::in_tokens(tokens, registry).is_some(),
                    symbol.map(|symbol| function.ssa.cell_key(symbol)), tokens.variable_accesses.len(),
                ));
            }
        }
        rows
    }

    #[test]
    fn diagnostic_existence_guards_are_conditional_and_handler_specific() {
        use tcl_core_types::DiagCode;
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "proc f {} {info exists u}",
                "proc f {} {array exists u}",
                "proc f {} {set answer [info exists u]; puts $answer}",
                "interp alias {} query {} info exists; proc f {} {query u}",
                "proc f {} {if {[info exists u]} {puts $u}}",
                "proc f {} {if {![info exists u]} {puts absent} else {puts $u}}",
                "interp alias {} query {} info exists; proc f {} {if {[query u]} {puts $u}}",
                "namespace eval Q {interp alias {} ::Q::query {} info exists; namespace export query}; namespace import Q::query; proc f {} {if {[query u]} {puts $u}}",
            ] {
                let result = crate::analyser::Analyser::new().analyse(source, engine);
                assert!(
                    !result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagCode::W210),
                    "{engine}: {source}: {:?}: {:?}",
                    diagnostic_query_summary(source, engine),
                    result
                        .diagnostics
                        .iter()
                        .map(|diagnostic| (
                            diagnostic.code,
                            diagnostic.span,
                            diagnostic.message.as_str()
                        ))
                        .collect::<Vec<_>>()
                );
            }
            for source in [
                "proc f {} {info exists $u}",
                "proc f {} {set answer [info exists u]$u}",
                "proc f {} {set answer [info exists u]; puts $u}",
                "proc f {} {if {[info exists u]} {puts present}; puts $u}",
                "rename info stock_info; proc info args {return 1}; proc f {} {if {[info exists u]} {puts $u}}",
                "proc query {name} {return 1}; proc f {} {if {[query u]} {puts $u}}",
            ] {
                let result = crate::analyser::Analyser::new().analyse(source, engine);
                assert!(
                    result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagCode::W210),
                    "{engine}: {source}"
                );
            }
        }
    }

    #[test]
    fn rooted_core_existence_queries_resolve_by_operation() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert_eq!(
            in_text(
                "[::info exists name]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name".to_owned(), ExistenceKind::AnyVariable))
        );
        assert_eq!(
            in_text(
                "[::array exists name]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name".to_owned(), ExistenceKind::Array))
        );
    }

    #[test]
    fn existence_words_follow_the_exact_document_grammar() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let irules = tcl_lexer::LexerConfig::for_dialect("f5-irules");
        assert_eq!(
            in_text("[info {exists}{name}]", &registry, irules),
            Some(("name".to_owned(), ExistenceKind::AnyVariable)),
        );
        assert_eq!(
            in_text(
                "[info {exists}{name}]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            None,
        );
        assert_eq!(
            in_text(
                "[info exists {name with spaces}]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name with spaces".to_owned(), ExistenceKind::AnyVariable,)),
        );
    }
    #[test]
    fn original_existence_metadata_keeps_genuine_advice_and_declines_missing_foreign_input() {
        // naming.compiler.retained-existence-metadata
        // docs/design/analysis/name-resolution-proofs/retained-existence-metadata.md
        let (tokens, registry) = original_query_tokens("tcl8.6", "", "info exists u");
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let metadata = crate::registry_invocation::InvocationMetadataContext::from(context);
        let (query, _) = super::in_tokens_for_diagnostics_with_metadata_context(
            &tokens,
            &registry,
            Some(metadata),
        )
        .expect("genuine source query remains a name dependency");
        assert_eq!(query.var, "u");
        assert_eq!(query.kind, ExistenceKind::AnyVariable);
        assert!(
            super::in_tokens_with_metadata_context(&tokens, &registry, Some(metadata)).is_none(),
            "source advice supplies no actual Native query"
        );
        assert!(
            super::in_tokens_for_diagnostics_with_metadata_context(&tokens, &registry, None)
                .is_none()
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let foreign = crate::registry_invocation::InvocationMetadataContext::from(foreign);
        assert!(
            super::in_tokens_for_diagnostics_with_metadata_context(
                &tokens,
                &registry,
                Some(foreign)
            )
            .is_none()
        );
        assert!(
            super::in_tokens_with_metadata_context(&tokens, &registry, Some(foreign)).is_none()
        );
    }

    #[test]
    fn original_unpositioned_existence_requires_retained_logical_input() {
        // naming.compiler.retained-existence-metadata
        // docs/design/analysis/name-resolution-proofs/retained-existence-metadata.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let registry = context.commands();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &input,
        )
        .unwrap();
        let node =
            crate::expr_parser::parse_expr_for_profile("![::info exists {x}]", Some(profile));
        let query = super::in_expr_with_metadata_context(&node, registry, config, Some(metadata))
            .expect("positive Logical source query");
        assert_eq!(query.var, "x");
        assert!(query.negated);
        assert!(super::in_expr_with_metadata_context(&node, registry, config, None).is_none());
        let availability_only =
            crate::registry_invocation::InvocationMetadataContext::from(context.as_ref());
        assert!(
            super::in_expr_with_metadata_context(&node, registry, config, Some(availability_only))
                .is_none()
        );
        let dynamic =
            crate::expr_parser::parse_expr_for_profile("[info exists $name]", Some(profile));
        assert!(
            super::in_expr_with_metadata_context(&dynamic, registry, config, Some(metadata))
                .is_none()
        );
        let native =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let native_profile = native.commands().profile().unwrap();
        let native_input = crate::analyser::ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            std::sync::Arc::clone(&native),
            config,
        );
        let native_metadata =
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                native.commands(),
                &native_input,
            )
            .unwrap();
        assert!(
            super::in_expr_with_metadata_context(
                &node,
                native.commands(),
                config,
                Some(native_metadata)
            )
            .is_none()
        );
    }
}
