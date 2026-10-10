// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original preparation visits for compiler name effects only.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::{ModuleCommandBindings, SourceExecutionContext};
use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeOriginalCompilerNameEffects,
};
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_compiler_words::NativeCompilerWords;
use tcl_registry::native_control_compilation::NativeControlPreparationStep;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScriptVisit {
    image: SourceImage,
    span: Span,
    config: tcl_lexer::LexerConfig,
    compilation: NativeCompilationContext,
}

/// Derived proofs belong to one immutable lookup point. Detached point clones
/// start empty, and neither this memo nor its population is semantic state.
#[derive(Debug, Default)]
pub(super) struct SourceCompilerNameEffectsCache {
    outcomes: Mutex<HashMap<ScriptProofKey, bool>>,
    #[cfg(test)]
    parsed_scripts: std::sync::atomic::AtomicUsize,
}

impl Clone for SourceCompilerNameEffectsCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScriptProofKey {
    script: ScriptVisit,
    origin: Arc<super::SourceOriginId>,
    namespace: super::SourceNamespaceKey,
    registry: tcl_registry::RegistrySemanticKey,
    realm: tcl_dialect::model::InvocationRealm,
}

struct ProofScope<'a> {
    memo: Option<&'a SourceCompilerNameEffectsCache>,
    origin: &'a Arc<super::SourceOriginId>,
    namespace: super::SourceNamespaceKey,
    registry: tcl_registry::RegistrySemanticKey,
    realm: tcl_dialect::model::InvocationRealm,
}

impl ProofScope<'_> {
    fn key(&self, script: &ScriptVisit) -> ScriptProofKey {
        ScriptProofKey {
            script: script.clone(),
            origin: Arc::clone(self.origin),
            namespace: self.namespace.clone(),
            registry: self.registry.clone(),
            realm: self.realm,
        }
    }

    fn known(&self, script: &ScriptVisit) -> Option<bool> {
        self.memo?
            .outcomes
            .lock()
            .unwrap()
            .get(&self.key(script))
            .copied()
    }

    fn retain(&self, script: &ScriptVisit, proved: bool) {
        if let Some(memo) = self.memo {
            memo.outcomes
                .lock()
                .unwrap()
                .insert(self.key(script), proved);
        }
    }
}

enum ProofTask {
    Enter(ScriptVisit),
    Finish(ScriptVisit, Vec<ScriptVisit>),
}

/// Original compile-time visits share completed descendant proofs under the
/// same immutable table, exact source origin and complete compiler context.
/// No command executes, source scope advances, or inline receipt is created.
/// Physical native entries remain uncached because provider currency can
/// change outside the immutable source point.
pub(super) fn preserves_visits(
    words: &NativeCompilerWords<'_>,
    visits: Vec<NativeControlPreparationStep>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    origin: &Arc<super::SourceOriginId>,
) -> bool {
    let mut roots = Vec::new();
    if !append_visits(words, visits, &mut roots, state, context.compilation) {
        return false;
    }
    if !roots.is_empty() && !lookup_has_no_callbacks(state) {
        return false;
    }
    let scope = ProofScope {
        memo: context.compilation_snapshot.and_then(|snapshot| {
            (state.baseline.native_entry.is_none() && std::ptr::eq(state, &snapshot.table.state))
                .then_some(&snapshot.table.compiler_name_effects)
        }),
        origin,
        namespace: context.namespace_identity(),
        registry: context.registry.snapshot().semantic_key(),
        realm: context.realm,
    };
    let mut outcomes = HashMap::new();
    let mut active = HashSet::new();
    let mut pending = roots
        .iter()
        .cloned()
        .map(ProofTask::Enter)
        .collect::<Vec<_>>();
    while let Some(task) = pending.pop() {
        match task {
            ProofTask::Enter(script) => {
                if outcomes.contains_key(&script) {
                    continue;
                }
                if let Some(proved) = scope.known(&script) {
                    outcomes.insert(script, proved);
                    continue;
                }
                // A recursive source dependency cannot manufacture a proof.
                if !active.insert(script.clone()) {
                    return false;
                }
                #[cfg(test)]
                if let Some(memo) = scope.memo {
                    memo.parsed_scripts
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let Some(children) = script_visits(&script, state, context) else {
                    active.remove(&script);
                    scope.retain(&script, false);
                    outcomes.insert(script, false);
                    continue;
                };
                pending.push(ProofTask::Finish(script, children.clone()));
                pending.extend(children.into_iter().rev().map(ProofTask::Enter));
            }
            ProofTask::Finish(script, children) => {
                let proved = children
                    .iter()
                    .all(|child| outcomes.get(child) == Some(&true));
                active.remove(&script);
                scope.retain(&script, proved);
                outcomes.insert(script, proved);
            }
        }
    }
    roots
        .iter()
        .all(|script| outcomes.get(script) == Some(&true))
}

fn script_visits(
    script: &ScriptVisit,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<ScriptVisit>> {
    if !lookup_has_no_callbacks(state) {
        return None;
    }
    let parsed =
        tcl_lexer::native_script_words_in(script.image.clone(), script.span, script.config).ok()?;
    if parsed.fatal_tail.is_some() {
        return None;
    }
    let context = SourceExecutionContext {
        compilation: script.compilation,
        ..context
    };
    let mut children = Vec::new();
    for command in parsed.commands {
        if !append_command(&command.words, state, context, &mut children) {
            return None;
        }
    }
    Some(children)
}

pub(super) fn lookup_has_no_callbacks(state: &ModuleCommandBindings) -> bool {
    !state.baseline.unknown_entry
        && state.baseline.native_entry.as_ref().is_none_or(|entry| {
            entry
                .command_resolvers
                .is_some_and(|inventory| inventory.permits_no_callbacks(entry))
        })
}

fn append_command(
    original: &[NativeWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    pending: &mut Vec<ScriptVisit>,
) -> bool {
    let Some(dialect) = state.baseline.compilation_dialect() else {
        return false;
    };
    let Some(protocol) =
        super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)
    else {
        return false;
    };
    let Ok(words) = NativeCompilerWords::capture(original, protocol) else {
        return false;
    };
    let Some(policy) = state
        .baseline
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return false;
    };
    let Some(head) = original.first().and_then(|word| {
        SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            policy,
        )
    }) else {
        return false;
    };
    let input = SignatureSourceNameInput::OriginalWord(head);
    let targets =
        state.native_compiler_targets_for_original_input(&input, &context.namespace_identity());
    if targets.unknown {
        return false;
    }
    if targets.may_be_generic {
        let visits = (0..original.len())
            .map(|index| {
                NativeControlPreparationStep::Word(NativeCompilerWordOperand::Original(index))
            })
            .collect();
        if !append_visits(&words, visits, pending, state, context.compilation) {
            return false;
        }
    }
    for target in targets.targets {
        if !target.registry_backed || !target.prepended.is_empty() {
            return false;
        }
        let Some(spec) = context
            .registry
            .native_compilation_for_original_registration(&target.command, &words, 1, dialect)
        else {
            return false;
        };
        let dependencies =
            super::compiled_invocation::compiler_dependencies(spec, &target, state, context);
        if dependencies.is_none() && spec.compiler_hook_presence(dialect) != Some(false) {
            return false;
        }
        let effects = spec.original_argument_name_effects_in_context(
            &words,
            1,
            Some(dialect),
            context.compilation,
        );
        if !append_effects(effects, &words, pending, state, context.compilation) {
            return false;
        }
    }
    true
}

fn append_effects(
    effects: NativeOriginalCompilerNameEffects,
    words: &NativeCompilerWords<'_>,
    pending: &mut Vec<ScriptVisit>,
    state: &ModuleCommandBindings,
    compilation: NativeCompilationContext,
) -> bool {
    match effects {
        NativeOriginalCompilerNameEffects::Preserved => true,
        NativeOriginalCompilerNameEffects::FixedLookup(lookup) => {
            super::compiled_invocation::original_fixed_compiler_lookup_preserves_names(
                state,
                lookup,
                state
                    .baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe),
            )
        }
        NativeOriginalCompilerNameEffects::Visits(visits) => {
            append_visits(words, visits, pending, state, compilation)
        }
        NativeOriginalCompilerNameEffects::Unknown => false,
    }
}

fn append_visits(
    words: &NativeCompilerWords<'_>,
    visits: Vec<NativeControlPreparationStep>,
    pending: &mut Vec<ScriptVisit>,
    state: &ModuleCommandBindings,
    compilation: NativeCompilationContext,
) -> bool {
    use NativeControlPreparationStep as Visit;
    let Some(first) = words.original_words().first() else {
        return false;
    };
    for visit in visits {
        match visit {
            Visit::Script {
                operand,
                span,
                context,
            }
            | Visit::SpeculativeScript {
                operand,
                span,
                context,
            } => {
                // Changed braced continuations or expansion child geometry need
                // a separate produced-script mapping; source equality is exact.
                let NativeCompilerWordOperand::Original(index) = operand else {
                    return false;
                };
                let Some(value) = words.literal(index) else {
                    return false;
                };
                if words.original_literal_extent(index, 0..value.len()) != Some(span) {
                    return false;
                }
                let Some(compilation) = context.entered_context(compilation) else {
                    return false;
                };
                pending.push(ScriptVisit {
                    image: first.image().clone(),
                    span,
                    config: first.config(),
                    compilation,
                });
            }
            Visit::Word(NativeCompilerWordOperand::Original(index)) => {
                let Some(word) = words.original_words().get(index) else {
                    return false;
                };
                for component in word.executable_parts().all_parts() {
                    match component.part {
                        ExecutablePart::Command { body } => pending.push(ScriptVisit {
                            image: word.image().clone(),
                            span: body,
                            config: word.config(),
                            compilation,
                        }),
                        ExecutablePart::ParseError(_) | ExecutablePart::Expression { .. } => {
                            return false;
                        }
                        ExecutablePart::Text(_) | ExecutablePart::Variable { .. } => {}
                    }
                }
            }
            Visit::Expression(operand) => {
                if !append_expression_visits(words, &operand, first, pending, state, compilation) {
                    return false;
                }
            }
            Visit::Word(NativeCompilerWordOperand::LiteralExpansion { .. })
            | Visit::BooleanProbe(_)
            | Visit::DeclareLocal(_)
            | Visit::DeclareAnonymousLocal
            | Visit::Literal(_)
            | Visit::Integer(_)
            | Visit::List(_) => {}
        }
    }
    true
}

fn append_expression_visits(
    words: &NativeCompilerWords<'_>,
    operand: &NativeCompilerWordOperand,
    first: &NativeWord,
    pending: &mut Vec<ScriptVisit>,
    state: &ModuleCommandBindings,
    compilation: NativeCompilationContext,
) -> bool {
    let Some(dialect) = state.baseline.compilation_dialect() else {
        return false;
    };
    let Ok(program) = tcl_registry::native_expression_program::prepare_native_expression_program(
        words, operand, dialect,
    ) else {
        return false;
    };
    if !expression_compiler_names_preserved(&program, dialect, state) {
        return false;
    }
    let Some(scripts) = program.original_command_substitutions() else {
        return false;
    };
    for script in scripts {
        // The checked program owns this entire original operand.
        // A bracket child inherits only its unchanged source body,
        // not a produced value, runtime frame or inline licence.
        let Some(bytes) = program.source.get(script.as_range()) else {
            return false;
        };
        if bytes.first() != Some(&b'[') || bytes.last() != Some(&b']') {
            return false;
        }
        let Some(start) = program
            .span
            .start()
            .checked_add(script.start())
            .and_then(|start| start.checked_add(1))
        else {
            return false;
        };
        let Some(end) = script
            .end()
            .checked_sub(1)
            .and_then(|end| program.span.start().checked_add(end))
        else {
            return false;
        };
        if start > end {
            return false;
        }
        pending.push(ScriptVisit {
            image: first.image().clone(),
            span: Span::new(start, end),
            config: first.config(),
            compilation,
        });
    }
    true
}

fn expression_compiler_names_preserved(
    program: &tcl_registry::native_expression_program::NativeExpressionProgram,
    dialect: tcl_registry::InvocationDialect,
    state: &ModuleCommandBindings,
) -> bool {
    // naming.expression.compiler-function-name-literals
    // docs/design/analysis/name-resolution-proofs/expression-compiler-function-name-literals.md
    use tcl_registry::native_expression_program::NativeExpressionCompilerNameEffects as Effects;
    match program.compiler_name_effects(dialect) {
        Effects::CallFree => true,
        Effects::RegistersCommandLiterals => {
            !state.has_opaque_domain()
                && state.command_observers.is_quiet()
                && lookup_has_no_callbacks(state)
                && state.ordinary_literal_pool.as_ref().is_some_and(
                    super::literal_object_pool::SourceOrdinaryLiteralPool::effects_current,
                )
        }
        Effects::Unknown => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings};
    use super::*;
    use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
    use tcl_registry::native_expression_program::{
        NativeExpressionProgram, prepare_native_expression_program,
    };

    fn words(source: &str, dialect: tcl_registry::InvocationDialect) -> Vec<NativeWord> {
        let image = SourceImage::native(source.as_bytes());
        tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, image.len().try_into().unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words
    }

    fn state(source: &str, dialect: tcl_registry::InvocationDialect) -> ModuleCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
        .final_state
        .as_ref()
        .clone()
    }

    fn program(
        words: &[NativeWord],
        dialect: tcl_registry::InvocationDialect,
    ) -> NativeExpressionProgram {
        let captured =
            NativeCompilerWords::capture(words, dialect.native_source_string_protocol().unwrap())
                .unwrap();
        prepare_native_expression_program(
            &captured,
            &NativeCompilerWordOperand::Original(1),
            dialect,
        )
        .unwrap()
    }

    #[test]
    fn function_name_registration_requires_original_pool_and_quiet_lookup() {
        // naming.expression.compiler-function-name-literals
        // docs/design/analysis/name-resolution-proofs/expression-compiler-function-name-literals.md
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let source = words("expr {Pi()}", dialect);
        let program = program(&source, dialect);
        let ordinary = state("set checkpoint READY", dialect);
        assert!(expression_compiler_names_preserved(
            &program, dialect, &ordinary
        ));
        let mut missing_pool = ordinary.clone();
        missing_pool.ordinary_literal_pool = None;
        assert!(!expression_compiler_names_preserved(
            &program,
            dialect,
            &missing_pool
        ));
        let mut unknown = ordinary.clone();
        std::sync::Arc::make_mut(&mut unknown.baseline).unknown_entry = true;
        assert!(!expression_compiler_names_preserved(
            &program, dialect, &unknown
        ));
        let observed = state(
            "proc cb args {}; trace add execution expr enter cb",
            dialect,
        );
        assert!(!observed.command_observers.is_quiet());
        assert!(!expression_compiler_names_preserved(
            &program, dialect, &observed
        ));
    }

    #[test]
    fn function_argument_compilation_keeps_original_nested_script_visits() {
        // naming.expression.compiler-function-name-literals
        // docs/design/analysis/name-resolution-proofs/expression-compiler-function-name-literals.md
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        let original = words("expr {missing([set checkpoint READY])}", dialect);
        let captured = NativeCompilerWords::capture(
            &original,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        let current = state("set checkpoint READY", dialect);
        let mut pending = Vec::new();
        assert!(append_visits(
            &captured,
            vec![NativeControlPreparationStep::Expression(
                NativeCompilerWordOperand::Original(1)
            )],
            &mut pending,
            &current,
            crate::environment_ingress::authoring_native_compilation()
        ));
        assert_eq!(pending.len(), 1);
        assert_eq!(
            &pending[0].image.bytes()[pending[0].span.as_range()],
            b"set checkpoint READY"
        );
        let mut unknown = current;
        unknown.ordinary_literal_pool = None;
        assert!(!append_visits(
            &captured,
            vec![NativeControlPreparationStep::Expression(
                NativeCompilerWordOperand::Original(1)
            )],
            &mut Vec::new(),
            &unknown,
            crate::environment_ingress::authoring_native_compilation()
        ));
    }

    fn body_visit(words: &NativeCompilerWords<'_>) -> Vec<NativeControlPreparationStep> {
        let body = words.literal(2).expect("original literal conditional body");
        vec![NativeControlPreparationStep::Script {
            operand: NativeCompilerWordOperand::Original(2),
            span: words.original_literal_extent(2, 0..body.len()).unwrap(),
            context: tcl_registry::native_compilation::NativeCompiledBodyContext::Inherit,
        }]
    }

    fn proof_context<'a>(
        registry: &'a tcl_registry::CommandRegistry,
        namespace: &'a super::super::SourceNamespaceKey,
        snapshot: &'a super::super::NativeCompilationSnapshot,
        frame: &'a crate::var_resolve::VariableExecutionFrame,
    ) -> SourceExecutionContext<'a> {
        let mut context = super::super::root_source_execution_context(
            frame,
            "::",
            namespace,
            tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar),
            registry,
            SourceAnalysisOptions::default(),
        );
        context.compilation = tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
            ..crate::environment_ingress::authoring_native_compilation()
        };
        context.compilation_snapshot = Some(snapshot);
        context
    }

    #[test]
    fn original_compiler_descendant_proofs_parse_each_region_once_per_lookup_point() {
        // naming.source.original-compiler-descendant-proof-reuse
        // docs/design/analysis/name-resolution-proofs/source-original-compiler-descendant-proof-reuse.md
        // Source-contract control: exact visits can be reused; they do not issue
        // a native execution, source-frame or compiler-admission receipt.
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let depth = 128;
        let source = format!(
            "{}set checkpoint READY{}",
            "if 1 {".repeat(depth),
            "}".repeat(depth)
        );
        let original = words(&source, dialect);
        let origin = Arc::new(super::super::SourceOriginId::authored_image(
            original[0].image().clone(),
        ));
        let mut current = state("set checkpoint READY", dialect);
        current.current_source_origin = Some(Arc::clone(&origin));
        let snapshot = current.native_compilation_snapshot();
        let namespace = current.source_root_namespace_key().unwrap();
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = proof_context(registry, &namespace, &snapshot, &frame);
        let protocol = dialect.native_source_string_protocol().unwrap();
        let captured = NativeCompilerWords::capture(&original, protocol).unwrap();
        assert!(preserves_visits(
            &captured,
            body_visit(&captured),
            &snapshot.table.state,
            context,
            &origin
        ));
        let memo = &snapshot.table.compiler_name_effects;
        assert_eq!(
            memo.parsed_scripts
                .load(std::sync::atomic::Ordering::Relaxed),
            depth
        );
        // Query the actual nested original operands independently, as successive
        // compiler selections do, rather than repeating only the outer request.
        let mut nested = original;
        for _ in 0..depth {
            let captured = NativeCompilerWords::capture(&nested, protocol).unwrap();
            assert!(preserves_visits(
                &captured,
                body_visit(&captured),
                &snapshot.table.state,
                context,
                &origin
            ));
            let span = captured
                .original_literal_extent(2, 0..captured.literal(2).unwrap().len())
                .unwrap();
            nested =
                tcl_lexer::native_script_words_in(nested[0].image().clone(), span, context.config)
                    .unwrap()
                    .commands
                    .remove(0)
                    .words;
        }
        assert_eq!(
            memo.parsed_scripts
                .load(std::sync::atomic::Ordering::Relaxed),
            depth
        );
        let detached = snapshot.table.as_ref().clone();
        assert!(
            detached
                .compiler_name_effects
                .outcomes
                .lock()
                .unwrap()
                .is_empty()
        );
        assert_eq!(&detached, snapshot.table.as_ref());
    }

    #[test]
    fn original_compiler_proof_memo_retains_refusals_and_exact_source_context() {
        // naming.source.original-compiler-descendant-proof-reuse
        // docs/design/analysis/name-resolution-proofs/source-original-compiler-descendant-proof-reuse.md
        // Source-contract controls; memo population supplies no execution facts.
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = words("if 1 {$unknown}", dialect);
        let origin = Arc::new(super::super::SourceOriginId::authored_image(
            original[0].image().clone(),
        ));
        let mut current = state("set checkpoint READY", dialect);
        current.current_source_origin = Some(Arc::clone(&origin));
        let snapshot = current.native_compilation_snapshot();
        let namespace = current.source_root_namespace_key().unwrap();
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = proof_context(registry, &namespace, &snapshot, &frame);
        let captured = NativeCompilerWords::capture(
            &original,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        let check = |words: &NativeCompilerWords<'_>, context, origin| {
            let proved = preserves_visits(
                words,
                body_visit(words),
                &snapshot.table.state,
                context,
                origin,
            );
            let parsed = snapshot
                .table
                .compiler_name_effects
                .parsed_scripts
                .load(std::sync::atomic::Ordering::Relaxed);
            (proved, parsed)
        };
        assert_eq!(check(&captured, context, &origin), (false, 1));
        assert_eq!(check(&captured, context, &origin), (false, 1));
        let mut changed = context;
        changed.compilation.loop_depth += 1;
        assert_eq!(check(&captured, changed, &origin), (false, 2));
        changed = context;
        changed.realm = tcl_dialect::model::InvocationRealm::InterpreterRuntime;
        assert_eq!(check(&captured, changed, &origin), (false, 3));
        let other_registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        changed = SourceExecutionContext {
            registry: other_registry,
            ..context
        };
        assert_eq!(check(&captured, changed, &origin), (false, 4));
        let other_origin = Arc::new(super::super::SourceOriginId::loaded_image(
            Arc::from("other.tcl"),
            Arc::from("source-contract"),
            original[0].image().clone(),
        ));
        assert_eq!(check(&captured, context, &other_origin), (false, 5));
        let other_namespace = super::super::SourceNamespaceKey::authored("::other");
        changed = SourceExecutionContext {
            namespace_key: Some(&other_namespace),
            ..context
        };
        assert_eq!(check(&captured, changed, &origin), (false, 6));
        let mut config = context.config;
        config.strict_quoting = !config.strict_quoting;
        let alternate = tcl_lexer::native_script_words_in(
            original[0].image().clone(),
            Span::new(0, original[0].image().len().try_into().unwrap()),
            config,
        )
        .unwrap()
        .commands
        .remove(0)
        .words;
        let alternate = NativeCompilerWords::capture(
            &alternate,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        assert_eq!(check(&alternate, context, &origin), (false, 7));
        let good = words("if 1 {set checkpoint READY}", dialect);
        let good_origin = Arc::new(super::super::SourceOriginId::authored_image(
            good[0].image().clone(),
        ));
        let good =
            NativeCompilerWords::capture(&good, dialect.native_source_string_protocol().unwrap())
                .unwrap();
        assert_eq!(check(&good, context, &good_origin), (true, 8));
        let mut opaque = snapshot.table.state.clone();
        Arc::make_mut(&mut opaque.baseline).unknown_entry = true;
        assert!(!preserves_visits(
            &good,
            body_visit(&good),
            &opaque,
            context,
            &good_origin
        ));
        assert_eq!(check(&good, context, &good_origin), (true, 8));
    }

    #[test]
    fn original_compiler_proof_memo_cannot_cross_a_changed_literal_or_lookup_world() {
        // naming.source.original-compiler-descendant-proof-reuse
        // docs/design/analysis/name-resolution-proofs/source-original-compiler-descendant-proof-reuse.md
        // A preservation memo is derived source planning, not an entry receipt.
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = words("if 1 {expr {Pi()}}", dialect);
        let origin = Arc::new(super::super::SourceOriginId::authored_image(
            original[0].image().clone(),
        ));
        let mut current = state("set checkpoint READY", dialect);
        current.current_source_origin = Some(Arc::clone(&origin));
        let snapshot = current.native_compilation_snapshot();
        let namespace = current.source_root_namespace_key().unwrap();
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = proof_context(registry, &namespace, &snapshot, &frame);
        let captured = NativeCompilerWords::capture(
            &original,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        assert!(preserves_visits(
            &captured,
            body_visit(&captured),
            &snapshot.table.state,
            context,
            &origin
        ));
        let mut changed = snapshot.table.state.clone();
        changed.ordinary_literal_pool = None;
        // The original snapshot is supplied deliberately: it cannot authorise
        // reuse when the inquiry's actual immutable state is another object.
        assert!(!preserves_visits(
            &captured,
            body_visit(&captured),
            &changed,
            context,
            &origin
        ));
        let other_snapshot = changed.native_compilation_snapshot();
        let other_context = proof_context(registry, &namespace, &other_snapshot, &frame);
        assert!(!preserves_visits(
            &captured,
            body_visit(&captured),
            &other_snapshot.table.state,
            other_context,
            &origin
        ));
        assert_eq!(
            other_snapshot
                .table
                .compiler_name_effects
                .parsed_scripts
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert!(preserves_visits(
            &captured,
            body_visit(&captured),
            &snapshot.table.state,
            context,
            &origin
        ));
        assert_eq!(
            snapshot
                .table
                .compiler_name_effects
                .parsed_scripts
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }
}
