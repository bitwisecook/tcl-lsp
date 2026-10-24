// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Native values and stores retain distinct ownership checks.

use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, LexerConfig, Module,
    OriginalSummaryScript, Statement,
};
use crate::command_binding::OriginalInvocationSourcePurpose;
use crate::ir::WordExpr;
use crate::registry_invocation::{
    NormalTransferInvocation, ResolvedStatementInvocation,
    normal_transfer_invocation_with_metadata_context,
    resolved_tokens_invocation_with_metadata_context,
};
use crate::ssa::SsaFunction;
use crate::ssa::ValueKey;

macro_rules! n17_native_trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        if std::env::var("TCL_LSP_TRACE_N17_FOLD").as_deref() == Ok("first-gate-v1") {
            eprintln!("N17_NATIVE {}", format_args!($($arg)*));
        }
    };
}

fn n17_native_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    if value.is_none() {
        n17_native_trace!("decline={stage}");
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

fn n17_native_decline(stage: &str, value: bool) -> bool {
    if value {
        n17_native_trace!("decline={stage}");
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

fn n17_native_outcome<T>(stage: &str, value: Option<T>) -> Option<T> {
    n17_native_trace!("stage={stage} present={}", value.is_some());
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

enum NativeTransferPurpose<'a> {
    ExecutionEntry,
    FreshSource(crate::command_binding::FreshNativeSourceAnalysis<'a>),
}

pub(in crate::interprocedural::transfer) struct NativeSummaryContext<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    config: LexerConfig,
    purpose: NativeTransferPurpose<'a>,
}

impl<'a> NativeSummaryContext<'a> {
    pub(in crate::interprocedural::transfer) fn for_module(
        module: &'a Module,
        registry: &'a CommandRegistry,
        config: LexerConfig,
    ) -> Option<Self> {
        n17_native_trace!(
            "factory.enter top={} bodies={}",
            module.top_level.statements.len(),
            module.body_units.len()
        );
        if n17_native_decline(
            "factory.config",
            config.normalized() != module.lexer_config.normalized(),
        ) || n17_native_decline("factory.dynamic-trace", module.has_dynamic_trace)
            || n17_native_decline(
                "factory.traced-commands",
                !module.traced_commands.is_empty(),
            )
            || n17_native_decline(
                "factory.retained-match",
                !n17_native_option("factory.retained", module.retained_source_bindings.as_ref())?
                    .matches_module(module, registry),
            )
        {
            return None;
        }
        let purpose = match module.original_invocation_source_purpose(
            registry,
            n17_native_option(
                "factory.metadata",
                InvocationMetadataContext::for_module(registry, module),
            )?,
        ) {
            Some(OriginalInvocationSourcePurpose::Execution) => {
                NativeTransferPurpose::ExecutionEntry
            }
            None | Some(OriginalInvocationSourcePurpose::AuthoredSimulation) => {
                NativeTransferPurpose::FreshSource(n17_native_option(
                    "factory.fresh-projection",
                    n17_native_option(
                        "factory.fresh-retained",
                        module.retained_source_bindings.as_ref(),
                    )?
                    .fresh_native_source_analysis(module, registry),
                )?)
            }
            _ => return n17_native_option("factory.other-purpose", None),
        };
        n17_native_trace!(
            "factory.accepted fresh={}",
            matches!(&purpose, NativeTransferPurpose::FreshSource(_))
        );
        Some(Self {
            module,
            registry,
            config,
            purpose,
        })
    }

    fn metadata<'w>(&'w self, tokens: &'w CommandTokens) -> Option<InvocationMetadataContext<'w>> {
        let retained = n17_native_option(
            "metadata.retained",
            self.module.retained_source_bindings.as_ref(),
        )?;
        n17_native_option(
            "metadata.tokens-owner",
            retained.owns_original_tokens(tokens).then_some(()),
        )?;
        let origin = n17_native_option(
            "metadata.root-origin",
            n17_native_option(
                "metadata.positioned",
                retained.positioned_for_module(self.module, self.registry),
            )?
            .source_origin(),
        )?;
        let binding = n17_native_option("metadata.binding", tokens.source_binding.as_ref())?;
        n17_native_option(
            "metadata.origin-Arc",
            std::sync::Arc::ptr_eq(
                origin,
                &n17_native_option("metadata.site", binding.invocation_site())?.source,
            )
            .then_some(()),
        )?;
        let metadata = n17_native_option(
            "metadata.for-module",
            binding.original_invocation_metadata_for_module(tokens, self.module, self.registry),
        )?;
        if let NativeTransferPurpose::FreshSource(source) = &self.purpose {
            n17_native_option(
                "metadata.fresh-attachment",
                source.matches_tokens(tokens, metadata).then_some(()),
            )?;
        }
        Some(metadata)
    }

    fn operation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<(ResolvedStatementInvocation, NormalTransferInvocation)> {
        let metadata = n17_native_option("operation.metadata", self.metadata(tokens))?;
        Some((
            n17_native_option(
                "operation.strict",
                resolved_tokens_invocation_with_metadata_context(
                    self.registry,
                    Some(metadata),
                    tokens,
                ),
            )?,
            n17_native_option(
                "operation.normal",
                normal_transfer_invocation_with_metadata_context(
                    self.registry,
                    Some(metadata),
                    tokens,
                ),
            )?,
        ))
    }

    pub(in crate::interprocedural::transfer) fn with_store_operation<T>(
        &self,
        body: Option<crate::ir::ProcedureId>,
        tokens: &CommandTokens,
        defs: &[(String, ValueKey)],
        ssa: &SsaFunction,
        apply: impl FnOnce(
            &ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        n17_native_trace!("store.enter defs={} body={}", defs.len(), body.is_some());
        let (operation, normal) = n17_native_option("store.operation", self.operation(tokens))?;
        n17_native_option(
            "store.receiver",
            self.receiver(body, tokens, &normal, defs, ssa),
        )?;
        let stored = n17_native_option("store.value-index", normal.stored_value_argument())?
            .checked_add(1)?;
        for (index, word) in operation.effective.words.iter().enumerate() {
            if index == stored {
                n17_native_option("store.sole-child", word.sole_command_substitution())?;
            } else if n17_native_decline("store.nonliteral-argv", !literal_word(word)) {
                return None;
            }
        }
        let realm = n17_native_option(
            "store.realm",
            n17_native_option("store.binding", tokens.source_binding.as_ref())?.invocation_realm(),
        )?;
        n17_native_outcome(
            "store.schema-or-consumer",
            operation.with_metadata_schema(
                self.registry,
                n17_native_option("store.schema-metadata", self.metadata(tokens))?,
                realm,
                |schema| apply(&operation, schema),
            ),
        )
    }

    /// Identity is proved before allowing the existing name-based place adapter.
    fn receiver(
        &self,
        body: Option<crate::ir::ProcedureId>,
        tokens: &CommandTokens,
        normal: &NormalTransferInvocation,
        defs: &[(String, ValueKey)],
        ssa: &SsaFunction,
    ) -> Option<()> {
        use tcl_registry::native_compilation::VariableOperandBindingPhase;
        if n17_native_decline(
            "receiver.phase-or-roles",
            normal.variable_binding_phase() != VariableOperandBindingPhase::AfterArguments
                || normal.variable_roles().as_slice() != [(0, tcl_registry::ArgRole::VarWrite)],
        ) {
            return None;
        }
        let [(display, (symbol, version))] = defs else {
            return n17_native_option("receiver.single-definition", None);
        };
        let binding = n17_native_option("receiver.binding", tokens.source_binding.as_ref())?;
        n17_native_option(
            "receiver.stored-child",
            n17_native_option(
                "receiver.stored-word",
                normal.stored_value_word(&binding.variable_context, self.registry),
            )?
            .sole_command_substitution(),
        )?;
        let place = n17_native_option(
            "receiver.place",
            normal.variable_operand_place(0, &binding.variable_context, self.registry),
        )?;
        if n17_native_decline(
            "receiver.unbounded-place",
            place.observed
                || place.dynamic
                || place.index.is_some()
                || place.kind != crate::place::PlaceKind::Scalar,
        ) {
            return None;
        }
        let key = n17_native_option(
            "receiver.canonical-key",
            crate::var_resolve::canonical_binding_value_key(&place),
        )?;
        if n17_native_decline("receiver.SSA-cell-key", ssa.cell_key(*symbol) != Some(&key)) {
            return None;
        }
        let name = n17_native_option("receiver.literal-name", normal.argument_literal(0))?;
        if n17_native_decline(
            "receiver.name-projection",
            name != *display
                || ssa.var_name(*symbol) != name
                || ssa.var_symbol(&name) != Some(*symbol)
                || crate::naming::element_var_name(&name) != name,
        ) {
            return None;
        }
        let mut point = None;
        for (&block, ssa_block) in &ssa.blocks {
            for (index, statement) in ssa_block.statements.iter().enumerate() {
                if statement.defs.get(symbol) != Some(version) {
                    continue;
                }
                if n17_native_decline(
                    "receiver.current-point",
                    point.is_some()
                        || statement.defs.len() != 1
                        || statement.may_defs.contains(symbol)
                        || statement.destruction_defs.contains(symbol)
                        || statement.statement.tokens() != Some(tokens)
                        || self.statement_tokens(body, &statement.statement).as_ref()
                            != Some(tokens)
                        || ssa.var_symbol_at(block, index, &name) != Some(*symbol),
                ) {
                    return None;
                }
                point = Some((block, index));
            }
        }
        n17_native_option("receiver.owned-point", point.map(|_| ()))
    }

    pub(in crate::interprocedural::transfer) fn with_value_operation<T>(
        &self,
        tokens: &CommandTokens,
        apply: impl FnOnce(
            &ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        let (operation, normal) = n17_native_option("child.operation", self.operation(tokens))?;
        if n17_native_decline(
            "child.roles-arguments-or-literals",
            !normal.variable_roles().is_empty()
                || !tokens
                    .source_binding
                    .as_ref()?
                    .original_arguments_complete_normally(tokens)
                || !operation.effective.words.iter().all(literal_word),
        ) {
            return None;
        }
        let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
        n17_native_outcome(
            "child.schema-or-consumer",
            operation.with_metadata_schema(
                self.registry,
                n17_native_option("child.schema-metadata", self.metadata(tokens))?,
                realm,
                |schema| apply(&operation, schema),
            ),
        )
    }

    pub(in crate::interprocedural::transfer) fn statement_tokens(
        &self,
        body: Option<crate::ir::ProcedureId>,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        let script = match body {
            Some(body) => {
                n17_native_option(
                    "statement.body-owner",
                    self.module.procedure_ids.owns(body).then_some(()),
                )?;
                let procedure =
                    n17_native_option("statement.body", self.module.body_units.get(&body))?;
                n17_native_option(
                    "statement.card-match",
                    n17_native_option(
                        "statement.card",
                        self.module
                            .original_native_declaration_body_units
                            .get(&body),
                    )?
                    .matches_procedure(self.module, procedure)
                    .then_some(()),
                )?;
                &procedure.body
            }
            None => &self.module.top_level,
        };
        let mut selected: Option<CommandTokens> = None;
        for original in &script.statements {
            if original.span() != statement.span() {
                continue;
            }
            let tokens = n17_native_option(
                "statement.tokens",
                script.retained_source_tokens_for_statement(original),
            )?;
            n17_native_option("statement.metadata", self.metadata(tokens))?;
            if statement
                .tokens()
                .is_some_and(|supplied| supplied != tokens)
                || selected.as_ref().is_some_and(|prior| prior != tokens)
            {
                return None;
            }
            selected = Some(tokens.clone());
        }
        n17_native_option("statement.selected", selected)
    }

    pub(in crate::interprocedural::transfer) fn substitution(
        &self,
        parent: &CommandTokens,
        site: &crate::ir::SourceSite,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        let metadata = n17_native_option("substitution.parent-metadata", self.metadata(parent))?;
        if n17_native_decline(
            "substitution.provenance",
            site.provenance != crate::ir::Provenance::Source,
        ) {
            return None;
        }
        let origin = &parent.source_binding.as_ref()?.invocation_site()?.source;
        let spelling = origin
            .source_image()
            .try_text()
            .ok()?
            .get(site.span.as_range())?;
        if spelling.strip_prefix('[')?.strip_suffix(']')? != script {
            return None;
        }
        let calls = n17_native_option(
            "substitution.inventory",
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                parent,
                self.config.nested(),
                self.registry,
                metadata,
            ),
        )?;
        let mut selected = calls.into_iter().filter(|call| call.span == site.span);
        let call = n17_native_option("substitution.selected-child", selected.next())?;
        if selected.next().is_some() {
            return None;
        }
        let child = n17_native_option("substitution.child-tokens", call.tokens)?;
        n17_native_option("substitution.child-metadata", self.metadata(&child))?;
        std::sync::Arc::ptr_eq(
            origin,
            &child.source_binding.as_ref()?.invocation_site()?.source,
        )
        .then_some(())?;
        Some(OriginalSummaryScript {
            text: script.to_owned(),
            base: site.span.start().checked_add(1)?,
            commands: vec![child],
        })
    }
}

fn literal_word(word: &WordExpr) -> bool {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Template { parts, .. } => parts
            .iter()
            .all(|part| matches!(part, crate::ir::WordPart::Text { .. })),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::{ConstValue, LatticeValue};
    use crate::compilation_unit::CompilationUnit;

    #[test]
    fn native_normal_store_joins_one_actual_ssa_receiver() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = current.commands();
        let text = "proc p {} {set known [string length abcde]; return $known}\n";
        let unit = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let (body, _) = unit
            .ir_module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let function = unit.function_for_body(Some(*body)).unwrap();
        let symbol = function.ssa.var_symbol("known").unwrap();
        assert!(matches!(
            function.sccp.values.get(&(symbol, 1)),
            Some(LatticeValue::Const(ConstValue::Int(5)))
        ));
        let context = NativeSummaryContext::for_module(
            &unit.ir_module,
            registry,
            unit.ir_module.lexer_config,
        )
        .unwrap();
        let original = unit.ir_module.body_units.get(body).unwrap().body.statements[0]
            .tokens()
            .unwrap();
        let defs = vec![("known".to_owned(), (symbol, 1))];
        assert!(
            context
                .with_store_operation(Some(*body), original, &defs, &function.ssa, |_, _| Some(()))
                .is_some()
        );
        for defs in [
            vec![],
            vec![("foreign".to_owned(), (symbol, 1))],
            vec![("known".to_owned(), (symbol, 0))],
            vec![defs[0].clone(), defs[0].clone()],
        ] {
            assert!(
                context
                    .with_store_operation(Some(*body), original, &defs, &function.ssa, |_, _| Some(
                        ()
                    ))
                    .is_none()
            );
        }
        let foreign = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let foreign_body = foreign
            .ir_module
            .original_native_declaration_body_units
            .keys()
            .next()
            .unwrap();
        let foreign_tokens = foreign.ir_module.body_units[foreign_body].body.statements[0]
            .tokens()
            .unwrap();
        assert!(
            context
                .with_store_operation(Some(*body), foreign_tokens, &defs, &function.ssa, |_, _| {
                    Some(())
                })
                .is_none()
        );
        assert!(
            context
                .with_value_operation(original, |_, _| Some(()))
                .is_none()
        );
        assert!(
            context
                .with_store_operation(None, original, &defs, &function.ssa, |_, _| Some(()))
                .is_none()
        );
        let foreign_function = foreign.function_for_body(Some(*foreign_body)).unwrap();
        assert!(
            context
                .with_store_operation(
                    Some(*body),
                    original,
                    &defs,
                    &foreign_function.ssa,
                    |_, _| Some(())
                )
                .is_none()
        );
    }

    #[test]
    fn fresh_native_source_analysis_refuses_foreign_or_changed_interpretations() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = current.commands();
        let unit = CompilationUnit::build_for_dialect(
            "proc p {} {set known [string length abcde]}",
            registry,
            false,
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            NativeSummaryContext::for_module(module, registry, module.lexer_config).unwrap();
        assert!(module.source_entry.native_entry.is_none());
        assert_eq!(
            module.original_invocation_source_purpose(
                registry,
                InvocationMetadataContext::for_module(registry, module).unwrap()
            ),
            Some(OriginalInvocationSourcePurpose::AuthoredSimulation)
        );
        assert!(matches!(
            source.purpose,
            NativeTransferPurpose::FreshSource(_)
        ));
        assert!(
            module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .fresh_native_source_analysis(module, registry)
                .is_some()
        );
        let mut unknown = module.clone();
        unknown.source_entry.unknown_entry = true;
        assert!(
            NativeSummaryContext::for_module(&unknown, registry, unknown.lexer_config).is_none()
        );
        let mut changed = module.clone();
        changed.source_metadata_input = None;
        assert!(
            NativeSummaryContext::for_module(&changed, registry, changed.lexer_config).is_none()
        );
        let foreign = tcl_registry::CommandRegistry::build_default();
        assert!(NativeSummaryContext::for_module(module, &foreign, module.lexer_config).is_none());
        let (context, logical) = crate::interprocedural::logical_completion_unit(
            "proc p {} {set known VALUE}",
            "tcl8.6",
        );
        assert!(
            logical
                .ir_module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .fresh_native_source_analysis(&logical.ir_module, context.commands())
                .is_none()
        );
    }

    #[test]
    fn equal_explicit_entry_and_standalone_do_not_acquire_fresh_issuance() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let text = "proc p {} {set known [string length abcde]}";
        let fresh = CompilationUnit::build_for_dialect(text, registry, false, "tcl8.6");
        let module = &fresh.ir_module;
        let input = module.source_metadata_input.as_ref().unwrap();
        let options = || crate::compilation_unit::UnitBuildOptions {
            registry,
            config: module.lexer_config,
            dialect: Some(input.unit_profile()),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        };
        let explicit = CompilationUnit::build_with_analysis_input(
            text,
            options(),
            Some(&module.source_entry),
            input,
        );
        assert_eq!(explicit.ir_module.source_entry, module.source_entry);
        assert!(
            NativeSummaryContext::for_module(&explicit.ir_module, registry, module.lexer_config,)
                .is_none()
        );
        let standalone =
            CompilationUnit::build_standalone_with_analysis_input(text, options(), input).unwrap();
        assert!(
            NativeSummaryContext::for_module(&standalone.ir_module, registry, module.lexer_config,)
                .is_none()
        );
    }

    #[test]
    fn native_variable_unknown_or_stateful_children_do_not_supply_values() {
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        for text in [
            "proc p {} {set known [missing-command abcde]; return $known}\n",
            "proc p {value} {set known [string length $value]; return $known}\n",
            "proc p {} {set known [set other VALUE]; return $known}\n",
            "proc p {} {set known [error FAILED]; return $known}\n",
        ] {
            let unit =
                CompilationUnit::build_for_dialect(text, current.commands(), false, "tcl8.6");
            let body = *unit
                .ir_module
                .original_native_declaration_body_units
                .keys()
                .next()
                .unwrap();
            let function = unit.function_for_body(Some(body)).unwrap();
            let symbol = function.ssa.var_symbol("known").unwrap();
            assert!(!matches!(
                function.sccp.values.get(&(symbol, 1)),
                Some(LatticeValue::Const(_))
            ));
        }
    }
}
