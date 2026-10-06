// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual reached math-function implementations used by execution folding.

use crate::ir::Script;

/// Whether two consumed native fixed-table prerequisites can hold together.
/// Missing prerequisites describe independent command-based expressions;
/// actual table dependencies must retain the same interpreter and full image.
#[must_use]
pub fn native_math_prerequisites_compatible(
    previous: Option<&tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    current: Option<&tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => previous == current,
        _ => true,
    }
}

/// Fully resolved native math implementation and frozen alias operands.
/// Missing prefix bytes or an opaque implementation cannot construct this call.
pub struct ResolvedMathFunctionCall<'a> {
    /// Bare implementation identity supplied by the actual registry-backed handler.
    pub function: &'a str,
    /// Frozen prefix values, evaluated when the alias was constructed.
    pub prepended: Vec<&'a str>,
    /// Exact dispatch evidence retained for any consumed fold dependency.
    pub invocation: &'a crate::command_binding::SourceMathInvocation,
}

impl ResolvedMathFunctionCall<'_> {
    /// Materialise the proved mathematical call shape, without resolving names
    /// again or converting any unmaterialised runtime value objects.
    #[must_use]
    pub fn target(&self) -> crate::tcl_expr_eval::NativeMathFunctionTarget {
        crate::tcl_expr_eval::NativeMathFunctionTarget {
            function: self.function.to_owned(),
            prepended: self
                .prepended
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

/// An expression's source instance and exact AST-to-source offset mapping.
/// Missing mappings or unrecorded calls never acquire stock semantics from the
/// function's spelling, the catalogue, or a whole-module declaration summary.
#[derive(Clone, Copy)]
pub struct ExpressionMathBindings<'a> {
    proofs: &'a [crate::command_binding::SourceMathInvocation],
    source: Option<&'a crate::command_binding::ExecutedScriptSource>,
    base: Option<u32>,
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    reads: &'a [crate::command_binding::SourceVariableAccess],
    tokens: Option<&'a crate::ir::CommandTokens>,
}

/// Artifact guard for an actual implicit call consumed by a successful fold.
/// A command lookup and an interpreter-owned fixed table are distinct owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMathFoldDependency {
    /// Revalidate the original relative function lookup and its implementation.
    Command(tcl_runtime_api::CommandBindingIdentity),
    /// Revalidate the actual interpreter and installed function table.
    Fixed(tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite),
}

/// Project a consumed call proof into the dependency retained by an artifact.
/// Obtain the proof from [`ExpressionMathBindings::resolved_call`]; neither a
/// public function spelling nor matching function arity establishes identity.
/// Prefix objects remain unsupported until their coercion effects are proved.
#[must_use]
pub fn native_fold_dependency(
    proof: &crate::command_binding::SourceMathInvocation,
) -> Option<NativeMathFoldDependency> {
    let reached = proof.reached()?;
    if !reached.object_callback_effects_closed() {
        return None;
    }
    resolved_call(proof, &proof.function)?;
    if let Some(binding) = &reached.command_binding {
        let target = binding.proved_target()?;
        return Some(NativeMathFoldDependency::Command(
            tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                &binding.lookup_namespace,
                tcl_registry::mathfunc::qualified_name(&proof.function).trim_start_matches("::"),
                target.command.strip_prefix("::").unwrap_or(&target.command),
            )
            .with_namespace_context(binding.lookup_namespace_key.to_compiled_context()),
        ));
    }
    reached
        .fixed_prerequisite
        .clone()
        .map(NativeMathFoldDependency::Fixed)
}

impl<'a> ExpressionMathBindings<'a> {
    /// Retain the same expression base used to create its parsed AST.
    #[must_use]
    pub fn new(script: &'a Script, base: Option<u32>) -> Self {
        Self::for_origin(
            &script.implicit_math_invocations,
            script.executed_source.as_deref(),
            base,
        )
        .with_preparations(&script.expression_preparations)
    }

    /// Use the retained inventory and original mapping of an IR/CFG expression.
    #[must_use]
    pub const fn for_origin(
        proofs: &'a [crate::command_binding::SourceMathInvocation],
        source: Option<&'a crate::command_binding::ExecutedScriptSource>,
        base: Option<u32>,
    ) -> Self {
        Self {
            proofs,
            source,
            base,
            preparations: &[],
            reads: &[],
            tokens: None,
        }
    }

    /// Attach reached preparation evidence from the same source inventory.
    /// A call binding cannot substitute for whole-expression entry validation.
    #[must_use]
    pub const fn with_preparations(
        self,
        preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    ) -> Self {
        Self {
            preparations,
            ..self
        }
    }

    /// Whether the original expression has an exact source instance and parser base.
    #[must_use]
    pub const fn is_positioned(&self) -> bool {
        self.source.is_some() && self.base.is_some()
    }

    /// Native numeric operand receipts from the retained original statement.
    /// Preparation offsets, actual read extents and physical alternatives must
    /// agree; rendered proposed source cannot construct these receipts.
    #[must_use]
    pub fn source_numeric_operands(
        &self,
        context: &tcl_syntax::expr::parser::ExprParseContext,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        crate::tcl_expr_eval::Env,
        crate::tcl_expr_eval::NativeOperandProofs,
    )> {
        let preparation = self.preparation_for_context(context)?;
        crate::native_numeric::expression_operands(preparation.witness.tree(), |node| {
            let (_, source) = preparation.original_variable_source(node)?;
            let mut accesses = self
                .reads
                .iter()
                .filter(|access| {
                    access.source == source
                        && matches!(&access.owner, crate::command_binding::SourceVariableEvaluationOwner::NativeExpression { invocation, .. } if invocation == &preparation.invocation)
                });
            let access = accesses.next()?;
            if accesses.next().is_some() {
                return None;
            }
            crate::native_numeric::source_read_operand(access, registry)
        })
    }

    /// Already numeric at the original expression read, independently of its
    /// value. This purpose-only check cannot construct a frozen operand or fold
    /// another expression. Missing/observed/alternative reads withdraw it.
    pub(crate) fn source_read_already_numeric(
        &self,
        node: &crate::expr_ast::ExprNode,
        context: &tcl_syntax::expr::parser::ExprParseContext,
        policy: crate::tcl_expr_eval::FoldPolicy,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Some(preparation) = self.preparation_for_context(context) else {
            return false;
        };
        if !self.unobserved_preparation_invocation(preparation) {
            return false;
        }
        let config = tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar);
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return false;
        };
        if !matches!(
            crate::native_lowering::cells::variable_reference_place(text, config),
            Ok(crate::native_lowering::cells::CellPlace::Named { .. })
        ) {
            return false;
        }
        let Some((_, source)) = preparation.original_variable_source(node) else {
            return false;
        };
        let mut accesses = self.reads.iter().filter(|access| {
            access.source == source
                && matches!(&access.owner, crate::command_binding::SourceVariableEvaluationOwner::NativeExpression { invocation, .. } if invocation == &preparation.invocation)
                && access.original_spelling == *text
        });
        let Some(access) = accesses.next() else {
            return false;
        };
        if accesses.next().is_some()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return false;
        }
        !access.context_alternatives().is_empty()
            && access.context_alternatives().iter().all(|state| {
                if state
                    .invocation_dialect
                    .map(|dialect| dialect.expression_parse_context(policy.dialect))
                    .as_ref()
                    != Some(context)
                {
                    return false;
                }
                let place = access.place_in_context(state, registry);
                state.contents_already_native_numeric_at(&place, registry)
            })
    }

    /// The actual native invocation whose argv includes this expression must
    /// not expose its rewritten text to an execution observer. Missing exact
    /// sites or conflicting nested carriers decline independently of read traces.
    fn unobserved_preparation_invocation(
        &self,
        preparation: &crate::command_binding::SourceExpressionPreparation,
    ) -> bool {
        let Some(tokens) = self.tokens else {
            return false;
        };
        let mut bindings = tokens
            .source_binding
            .iter()
            .chain(tokens.nested_bindings.iter().map(|(_, binding)| binding))
            .filter(|binding| binding.invocation_site() == Some(&preparation.invocation));
        let Some(first) = bindings.next() else {
            return false;
        };
        first.unobserved_native_dispatch() && bindings.all(|binding| binding == first)
    }

    /// Select the preparation of these exact source bytes at their parser base.
    /// Conflicting alternatives remain unproved, including a different native
    /// term tree, engine axes or installed function-table owner/generation.
    #[must_use]
    pub fn preparation(&self) -> Option<&'a crate::command_binding::SourceExpressionPreparation> {
        let source = self.source?;
        let base = self.base?;
        let mut candidates = self.preparations.iter().filter(|preparation| {
            ((preparation.source.origin == source.origin && preparation.source.base() == base)
                || preparation.original_expression_mapping().is_some_and(
                    |(origin, original_base)| origin == &source.origin && original_base == base,
                ))
                && preparation.has_closed_script_compilation()
        });
        let first = candidates.next()?;
        candidates
            .all(|candidate| candidate == first)
            .then_some(first)
    }

    /// Require the exact native preparation axes before consuming its term tree.
    /// This does not establish compiler-entry acceptance or reached handler identity.
    #[must_use]
    pub fn preparation_for_context(
        &self,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Option<&'a crate::command_binding::SourceExpressionPreparation> {
        let proof = self.preparation()?;
        (proof.witness.context() == context
            && proof.witness.source().as_bytes() == proof.source.text.bytes())
        .then_some(proof)
    }

    /// Select another exact parser base within the same retained source instance.
    /// The caller must supply the original expression's mapping, not an offset
    /// recovered from rendered or substituted text.
    #[must_use]
    pub const fn at_base(self, base: Option<u32>) -> Self {
        Self { base, ..self }
    }

    /// Select the prepared source owned by an exact reached invocation.
    /// Materialised bytes retain their derived origin rather than borrowing
    /// the original word's parser offsets. Conflicting alternatives decline.
    #[must_use]
    pub fn at_invocation(self, offset: u32) -> Option<Self> {
        let origin = &self.source?.origin;
        let mut candidates = self.preparations.iter().filter(|proof| {
            &proof.invocation.source == origin && proof.invocation.offset == offset
        });
        let first = candidates.next()?;
        candidates.all(|proof| proof == first).then_some(Self {
            source: Some(&first.source),
            base: Some(first.source.base()),
            ..self
        })
    }

    /// Select a retained typed expression statement without reparsing rendered
    /// text. Conflicting source occurrences or mappings remain unproved.
    #[must_use]
    pub fn for_module_statement(
        module: &'a crate::ir::Module,
        span: tcl_lexer::Span,
    ) -> Option<Self> {
        let mut pending = vec![&module.top_level];
        pending.extend(module.procedures.values().map(|procedure| &procedure.body));
        let mut selected: Option<Self> = None;
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                let candidate = match statement {
                    crate::ir::Statement::AssignExpr {
                        span: actual,
                        expr_base,
                        ..
                    }
                    | crate::ir::Statement::ExprEval {
                        span: actual,
                        expr_base,
                        ..
                    }
                    | crate::ir::Statement::Return {
                        span: actual,
                        expr: Some(_),
                        expr_base,
                        ..
                    } if *actual == span => Some(Self::new(script, *expr_base)),
                    crate::ir::Statement::If { clauses, .. } => clauses
                        .iter()
                        .find(|clause| clause.condition_span == span)
                        .map(|clause| Self::new(script, clause.condition_base)),
                    crate::ir::Statement::While {
                        condition_span,
                        condition_base,
                        ..
                    }
                    | crate::ir::Statement::For {
                        condition_span,
                        condition_base,
                        ..
                    } if *condition_span == span => Some(Self::new(script, *condition_base)),
                    _ => None,
                };
                if let Some(candidate) = candidate {
                    let candidate = Self {
                        tokens: script.retained_source_tokens_for_statement(statement),
                        reads: script
                            .retained_source_tokens_for_statement(statement)
                            .map_or(&[], |tokens| &tokens.variable_accesses),
                        ..candidate
                    };
                    let candidate = candidate.at_retained_preparation();
                    if selected.is_some_and(|previous| {
                        previous.base != candidate.base
                            || previous.source != candidate.source
                            || previous.proofs != candidate.proofs
                            || previous.preparations != candidate.preparations
                            || previous.reads != candidate.reads
                            || previous.tokens != candidate.tokens
                    }) {
                        return None;
                    }
                    selected = Some(candidate);
                }
                pending.extend(statement.child_scripts());
            }
        }
        selected
    }

    fn at_retained_preparation(self) -> Self {
        let Some(proof) = self.preparation() else {
            return self;
        };
        let Some(tokens) = self.tokens else {
            return self;
        };
        let selects_parent = tokens
            .source_binding
            .iter()
            .chain(tokens.nested_bindings.iter().map(|(_, binding)| binding))
            .any(|binding| binding.invocation_site() == Some(&proof.invocation));
        if !selects_parent {
            return self;
        }
        Self {
            source: Some(&proof.source),
            base: Some(proof.source.base()),
            ..self
        }
    }

    /// Exact scoped normalization of the original expression tree. This query
    /// consumes the source owner's original frame/world/handler envelope;
    /// actual compiler preparation remains a separate executable obligation.
    pub(crate) fn nested_numeric_normalisation(
        &self,
        context: &tcl_syntax::expr::parser::ExprParseContext,
        registry: &tcl_registry::CommandRegistry,
        expected: &crate::expr_ast::ExprNode,
    ) -> bool {
        self.tokens
            .and_then(|tokens| {
                Some(
                    tokens
                        .source_binding
                        .as_ref()?
                        .nested_expression_normalisation(registry, tokens, context, Some(expected)),
                )
            })
            .unwrap_or(false)
    }

    /// Prove the actual native implementation after reached operand evaluations.
    #[must_use]
    pub fn proves_intrinsic(&self, function: &str, start: u32) -> bool {
        self.proved_invocation(function, start).is_some()
    }

    /// Prove the unchanged native call and closed operand effects for erasure.
    /// Alias prefixes and implementation-only value evidence do not suffice.
    #[must_use]
    pub fn proves_intrinsic_for_erasure(&self, function: &str, start: u32) -> bool {
        self.proved_invocation(function, start)
            .and_then(native_fold_dependency)
            .is_some()
    }

    /// Resolve a native math implementation independently of its public alias
    /// spelling. Unknown prefix values stay unknown and are never materialised.
    #[must_use]
    pub fn resolved_call(
        &self,
        function: &str,
        start: u32,
    ) -> Option<ResolvedMathFunctionCall<'a>> {
        let call = self.resolved_call_for_value_analysis(function, start)?;
        call.invocation
            .reached()?
            .object_callback_effects_closed()
            .then_some(call)
    }

    /// Select implementation identity for conditional value analysis only.
    /// This query establishes neither operand conversion effects nor permission
    /// to erase a call; executable consumers use [`Self::resolved_call`].
    #[must_use]
    pub fn resolved_call_for_value_analysis(
        &self,
        function: &str,
        start: u32,
    ) -> Option<ResolvedMathFunctionCall<'a>> {
        let site = self.base?.checked_add(start)?;
        let source = self.source?;
        let mut candidates = self.proofs.iter().filter(|proof| {
            proof.origin == source.origin && proof.site == site && proof.function == function
        });
        let first = resolved_call(candidates.next()?, function)?;
        for candidate in candidates {
            let other = resolved_call(candidate, function)?;
            if first.function != other.function
                || first.prepended != other.prepended
                || first.invocation != other.invocation
            {
                return None;
            }
        }
        Some(first)
    }

    /// Retain the exact dependency consumed by one successful fold query.
    /// Emitters call this only for calls the evaluator actually reached; the
    /// fixed prerequisite includes its real interpreter owner, while mutable
    /// bindings retain their exact namespace/token dispatch evidence.
    #[must_use]
    pub fn proved_invocation(
        &self,
        function: &str,
        start: u32,
    ) -> Option<&'a crate::command_binding::SourceMathInvocation> {
        let site = self.base.and_then(|base| base.checked_add(start))?;
        let source = self.source?;
        let mut proofs = self.proofs.iter().filter(|proof| {
            proof.origin == source.origin && proof.site == site && proof.function == function
        });
        let first = proofs.next()?;
        (proves_intrinsic(first, function)
            && proofs.all(|proof| proof == first && proves_intrinsic(proof, function)))
        .then_some(first)
    }
}

fn proves_intrinsic(proof: &crate::command_binding::SourceMathInvocation, function: &str) -> bool {
    let Some(reached) = proof.reached() else {
        return false;
    };
    if !reached.unobserved {
        return false;
    }
    let identity = tcl_registry::mathfunc::qualified_name(function);
    if let Some(binding) = &reached.command_binding {
        return binding.proved_target().is_some_and(|target| {
            target.registry_backed
                && target.kind == crate::command_binding::BindingKind::Builtin
                && target.command == identity
                && target.prepended.is_empty()
        });
    }
    matches!(
        reached.fixed_prerequisite.as_ref().map(|prerequisite| prerequisite.table.lookup(function)),
        Some(tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row))
            if row.registry_identity.as_deref()
                .and_then(tcl_registry::mathfunc::global_command_bare_name) == Some(function)
    )
}

fn resolved_call<'a>(
    proof: &'a crate::command_binding::SourceMathInvocation,
    function: &str,
) -> Option<ResolvedMathFunctionCall<'a>> {
    let reached = proof.reached()?;
    if !reached.unobserved {
        return None;
    }
    if let Some(binding) = &reached.command_binding {
        let target = binding.proved_target()?;
        if !target.registry_backed || target.kind != crate::command_binding::BindingKind::Builtin {
            return None;
        }
        // Frozen bytes establish argv contents, but an alias may retain a
        // shared list/double object whose numeric conversion is observable.
        // A prefix needs an object/representation effect proof before folding.
        if !target.prepended.is_empty() {
            return None;
        }
        let function = tcl_registry::mathfunc::global_command_bare_name(&target.command)?;
        let prepended = target
            .prepended
            .iter()
            .map(|word| match word {
                crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                    Some(value.as_str())
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        return Some(ResolvedMathFunctionCall {
            function,
            prepended,
            invocation: proof,
        });
    }
    if !proves_intrinsic(proof, function) {
        return None;
    }
    Some(ResolvedMathFunctionCall {
        function: &proof.function,
        prepended: Vec::new(),
        invocation: proof,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{
        ExecutedScriptSource, SourceImplicitMathInvocation, SourceOriginId,
    };
    use std::sync::Arc;
    use tcl_runtime_api::native_compilation::{
        NativeInterpreterIdentity, NativeMathFunctionBinding, NativeMathFunctionPrerequisite,
        NativeMathFunctionTable,
    };

    #[test]
    fn fixed_math_name_and_arity_do_not_establish_native_implementation_or_owner() {
        let origin = Arc::new(SourceOriginId::authored(&Arc::from("abs(-3)")));
        let source = ExecutedScriptSource::contiguous(Arc::clone(&origin), "abs(-3)", 0).unwrap();
        let table = NativeMathFunctionTable {
            closed: true,
            generation: 7,
            functions: vec![NativeMathFunctionBinding {
                name: "abs".into(),
                token: 4,
                implementation_generation: 7,
                registry_identity: None,
                arity: Some(1),
            }],
        };
        let mut proof = SourceImplicitMathInvocation {
            origin,
            site: 0,
            function: "abs".into(),
            unobserved: true,
            object_callback_effects: None,
            command_binding: None,
            fixed_functions: Some(table.clone()),
            fixed_prerequisite: Some(NativeMathFunctionPrerequisite {
                interpreter: NativeInterpreterIdentity {
                    owner: 12,
                    interpreter: 3,
                },
                table,
            }),
        };
        let query = |proof: &SourceImplicitMathInvocation| {
            ExpressionMathBindings::for_origin(
                std::slice::from_ref(&crate::command_binding::SourceMathInvocation::from_reached(
                    proof.clone(),
                )),
                Some(&source),
                Some(0),
            )
            .proves_intrinsic("abs", 0)
        };
        assert!(
            !query(&proof),
            "an opaque same-name/same-arity registration is not stock"
        );
        proof.fixed_prerequisite.as_mut().unwrap().table.functions[0].registry_identity =
            Some(tcl_registry::mathfunc::qualified_name("abs"));
        assert!(query(&proof));
        for stamp in ["tcl::mathfunc::abs", "::tcl::mathfunc::abs"] {
            proof.fixed_prerequisite.as_mut().unwrap().table.functions[0].registry_identity =
                Some(stamp.into());
            assert!(query(&proof), "actual global registry contract: {stamp}");
        }
        for stamp in [
            "abs",
            "::other::tcl::mathfunc::abs",
            "::tcl::mathfunc::other::abs",
        ] {
            proof.fixed_prerequisite.as_mut().unwrap().table.functions[0].registry_identity =
                Some(stamp.into());
            assert!(
                !query(&proof),
                "foreign or incomplete registry contract: {stamp}"
            );
        }
        proof.fixed_prerequisite.as_mut().unwrap().table.functions[0].registry_identity =
            Some(tcl_registry::mathfunc::qualified_name("abs"));
        let original = crate::command_binding::SourceMathInvocation::from_reached(proof.clone());
        let unclosed = ExpressionMathBindings::for_origin(
            std::slice::from_ref(&original),
            Some(&source),
            Some(0),
        );
        assert!(
            unclosed
                .resolved_call_for_value_analysis("abs", 0)
                .is_some()
        );
        assert!(unclosed.resolved_call("abs", 0).is_none());
        assert!(
            native_fold_dependency(&crate::command_binding::SourceMathInvocation::from_reached(
                proof.clone()
            ))
            .is_none()
        );
        let mut other_owner = proof.clone();
        other_owner
            .fixed_prerequisite
            .as_mut()
            .unwrap()
            .interpreter
            .owner += 1;
        let alternatives = [proof.clone(), other_owner]
            .map(crate::command_binding::SourceMathInvocation::from_reached);
        let alternatives_query =
            ExpressionMathBindings::for_origin(&alternatives, Some(&source), Some(0));
        assert!(alternatives_query.proved_invocation("abs", 0).is_none());
        assert!(alternatives_query.resolved_call("abs", 0).is_none());
        proof.unobserved = false;
        assert!(!query(&proof));
        proof.unobserved = true;
        proof.fixed_prerequisite = None;
        assert!(
            !query(&proof),
            "a table without its interpreter owner cannot license a fold"
        );
    }

    #[test]
    fn executable_math_requires_actual_closed_operand_effects() {
        for profile in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let selected_profile = tcl_dialect::DialectProfile::find(profile).unwrap();
            let entry = crate::environment_ingress::captured_native_entry(selected_profile);
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            for (source, closed, hosted) in [
                ("expr {sqrt(49)}", profile != "tcl8.4", profile == "tcl8.4"),
                ("expr {sqrt(49)}", false, true),
                ("proc f {} {incr x; expr {sqrt($x)}}; f", true, true),
                ("proc f {input} {expr {sqrt($input)}}", false, true),
            ] {
                let inventory = crate::command_binding::SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    registry,
                    crate::command_binding::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            selected_profile,
                        )),
                        native_entry: hosted.then_some(&entry),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let script = ExecutedScriptSource::contiguous(
                    Arc::clone(inventory.source_origin().unwrap()),
                    source,
                    0,
                )
                .unwrap();
                let calls = inventory.implicit_math_invocations_for_script(&script);
                let site = u32::try_from(source.find("sqrt").unwrap()).unwrap();
                let bindings =
                    ExpressionMathBindings::for_origin(&calls, Some(&script), Some(site));
                assert!(
                    bindings
                        .resolved_call_for_value_analysis("sqrt", 0)
                        .is_some(),
                    "actual implementation must remain known: {profile}: {source}"
                );
                assert_eq!(
                    bindings.resolved_call("sqrt", 0).is_some(),
                    closed,
                    "{profile}: {source}; hosted={hosted}"
                );
                if closed {
                    let call = bindings.resolved_call("sqrt", 0).unwrap();
                    assert!(native_fold_dependency(call.invocation).is_some());
                    assert!(bindings.proves_intrinsic_for_erasure("sqrt", 0));
                    let mut custom = call.invocation.reached().unwrap().clone();
                    custom.object_callback_effects = None;
                    let custom = crate::command_binding::SourceMathInvocation::from_reached(custom);
                    assert!(native_fold_dependency(&custom).is_none());
                } else {
                    assert!(!bindings.proves_intrinsic_for_erasure("sqrt", 0));
                }
            }
        }
    }

    #[test]
    fn materialised_preparation_requires_the_exact_invocation_and_source() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            r#"set a 3; set b 4; set r [expr "$a + $b"]"#,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let mut script = unit.ir_module.top_level.clone();
        let proof = script.expression_preparations.first().unwrap().clone();
        let query = ExpressionMathBindings::new(&script, None);
        let prepared = query.at_invocation(proof.invocation.offset).unwrap();
        assert_eq!(prepared.preparation(), Some(&proof));
        assert_eq!(proof.witness.source(), "3 + 4");
        assert!(query.at_invocation(proof.invocation.offset + 1).is_none());
        let mut conflicting = proof.clone();
        let origin = Arc::new(SourceOriginId::authored(&Arc::from("3 + 4")));
        conflicting.source =
            Arc::new(ExecutedScriptSource::contiguous(origin, "3 + 4", 0).unwrap());
        script.expression_preparations.push(conflicting);
        assert!(
            ExpressionMathBindings::new(&script, None)
                .at_invocation(proof.invocation.offset)
                .is_none()
        );
        script.executed_source = None;
        assert!(
            ExpressionMathBindings::new(&script, None)
                .at_invocation(proof.invocation.offset)
                .is_none()
        );
    }
}
