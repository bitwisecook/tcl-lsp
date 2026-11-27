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

//! The proof that a metaclass's unknown-dispatch member creates and names an instance.

use super::super::super::state::Analyser;
use super::super::super::types::{ClassDef, MemberSide};
use super::{MAX_UNKNOWN_BODY_DEPTH, UserMetaclass};
use tcl_lexer::Span;
use tcl_source::segment::SegmentedCommand;

/// What one statement of an unknown-dispatch body proves — see
/// [`Analyser::unknown_dispatch_binds_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnknownBodyEvidence {
    /// It constructs an object named from the fallback's first parameter.
    Constructs,
    /// It completes the member with exactly that parameter.
    ReturnsWord,
    /// It completes the member with something else — fatal to the proof.
    ReturnsSomethingElse,
    /// It ends this path with a non-normal completion and returns no handle.
    Terminates,
    /// It says nothing either way.
    Nothing,
}

#[derive(Default)]
struct UnknownPathProof {
    proved_return: bool,
    invalid_return: bool,
    fallthrough_constructed: Vec<bool>,
}

/// Whether `head` is a bracketed self-receiver word — `[self]` / `[self
/// object]` written as a command head, which dispatches on the current object
/// exactly as `my` does.
///
/// Registry data via [`tcl_registry::CommandRegistry::is_self_receiver_call`];
/// the bracket peeling is this function's whole contribution.
fn bracketed_self_receiver(registry: &tcl_registry::CommandRegistry, head: &str) -> bool {
    let Some(inner) = head
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .map(str::trim)
    else {
        return false;
    };
    let mut words = inner.split_whitespace();
    let Some(cmd) = words.next() else {
        return false;
    };
    let arg = words.next();
    words.next().is_none() && registry.is_self_receiver_call(cmd, arg)
}

impl Analyser {
    /// Whether this metaclass's **unrecognised-word fallback member** proves
    /// that calling one of the classes it makes with a bare word both
    /// constructs an object and hands that same word back — Tk's
    /// `::tk::IconList .il` idiom.
    ///
    /// Two facts have to hold, and both are read off the metaclass's own
    /// body, where they are provable:
    ///
    /// 1. it declares the family's unknown-dispatch member
    ///    ([`DefinitionBodyGrammar::unknown_dispatch_method`], registry data —
    ///    `unknown` for `TclOO`) with at least one parameter, so an
    ///    unrecognised first word reaches code at all; and
    /// 2. that body **constructs** an object named from the first parameter
    ///    (a manufacturer call — again registry data — whose instance-name
    ///    word interpolates the parameter) **and** completes with exactly
    ///    that parameter, proved through
    ///    [`tcl_registry::ArgRole::Result`] rather than assumed.
    ///
    /// Fact 2 is the one the issue insists on: an `unknown` that returns
    /// something else — `return [self]`, a formatted string, a bare `next`
    /// fall-through — must **not** be read this way, because the value the
    /// caller binds is then not an object handle at all. Every reachable
    /// normal-result path must return the first parameter after constructing
    /// it; a different result collapses the proof to `false`, and a body with
    /// no such path proves nothing. Registry completion semantics distinguish
    /// that normal result from `return -code error`, `error`, loop transfer,
    /// and other terminating completions.
    ///
    /// Deliberate residuals, both in the abstaining direction except where
    /// noted:
    ///
    /// * a body that also dispatches through `next` (Tk's does, for a word
    ///   that is not a widget path) is accepted only when the actual class MRO
    ///   proves that the next provider terminates. A visible user provider
    ///   that returns normally invalidates the proof; an incomplete hierarchy
    ///   abstains. At a proved built-in chain end, completion comes from the
    ///   definition grammar rather than from the dispatch keyword;
    /// * a construction written through a variable holding the name
    ///   (`set n $w; [self] create $n`) is not followed, and abstains;
    /// * the member is looked up on the **metaclass**, because it is
    ///   *instances* of the metaclass — the manufactured classes — that
    ///   dispatch through it. An `unknown` on an ordinary class governs that
    ///   class's own instances, which is a different question this does not
    ///   answer.
    ///
    /// [`DefinitionBodyGrammar::unknown_dispatch_method`]: tcl_registry::definer::DefinitionBodyGrammar::unknown_dispatch_method
    pub(super) fn unknown_dispatch_binds_instance(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
    ) -> bool {
        let Some(keyword) = meta.grammar.unknown_dispatch_method else {
            return false;
        };
        let Some(members) =
            super::super::super::class_hierarchy::original_metadata::original_definition_members_for_class(
                &self.result,
                class,
                MemberSide::Instance,
            )
        else {
            return false;
        };
        let Some(original) = members.iter().find(|method| {
            method.original_name_input().policy() == meta.root_policy
                && method.original_name_input().bytes() == keyword.as_bytes()
        }) else {
            return false;
        };
        let Some(member) = original.original_parameter_names_and_body(&self.result) else {
            return false;
        };
        let Some(word_param) = member.parameter_names().next() else {
            return false;
        };
        let Some(body_token) = member.body_word().tokens().first() else {
            return false;
        };
        let Some(proof) = self.unknown_script_path_proof(
            meta,
            class,
            body_token.span,
            word_param,
            vec![false],
            0,
        ) else {
            return false;
        };
        proof.proved_return && !proof.invalid_return && proof.fallthrough_constructed.is_empty()
    }

    fn unknown_script_path_proof(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        body_span: Span,
        word_param: &str,
        mut states: Vec<bool>,
        depth: u32,
    ) -> Option<UnknownPathProof> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let context = self.retained_static_source_context()?;
        let mut proof = UnknownPathProof::default();
        for seg in self.direct_statements_in_span(body_span)? {
            if states.is_empty() {
                break;
            }
            let words = crate::registry_invocation::source_structure::source_registry_words_at(
                &self.source,
                &self.result,
                seg.argv.first()?.span.start(),
            )?;
            let hook =
                words.with_source_schema(&context, |schema| schema.semantics.lowering_hook)?;
            if hook == Some(tcl_registry::hooks::LoweringHookId::If) {
                let (body_spans, has_fallthrough) =
                    self.possible_if_body_spans(&seg, word_param)?;
                let mut next_states = Vec::new();
                for state in states {
                    for span in &body_spans {
                        let nested = self.unknown_script_path_proof(
                            meta,
                            class,
                            *span,
                            word_param,
                            vec![state],
                            depth + 1,
                        )?;
                        proof.proved_return |= nested.proved_return;
                        proof.invalid_return |= nested.invalid_return;
                        next_states.extend(nested.fallthrough_constructed);
                    }
                    if has_fallthrough {
                        next_states.push(state);
                    }
                }
                states = next_states;
                continue;
            }
            if hook.is_some_and(|hook| {
                matches!(
                    hook,
                    tcl_registry::hooks::LoweringHookId::Switch
                        | tcl_registry::hooks::LoweringHookId::Try
                )
            }) {
                return None;
            }
            match self.unknown_body_evidence(meta, class, &seg, word_param) {
                UnknownBodyEvidence::Constructs => states.fill(true),
                UnknownBodyEvidence::ReturnsWord => {
                    proof.invalid_return |= states.iter().any(|constructed| !constructed);
                    proof.proved_return |= states.iter().any(|constructed| *constructed);
                    states.clear();
                }
                UnknownBodyEvidence::ReturnsSomethingElse => {
                    proof.invalid_return = true;
                    states.clear();
                }
                UnknownBodyEvidence::Terminates => states.clear(),
                UnknownBodyEvidence::Nothing => {}
            }
        }
        proof.fallthrough_constructed = states;
        Some(proof)
    }

    pub(in super::super) fn possible_if_body_spans(
        &self,
        seg: &SegmentedCommand,
        _word_param: &str,
    ) -> Option<(Vec<Span>, bool)> {
        let context = self.retained_static_source_context()?;
        let input = self.result.resolved_input.as_ref()?;
        if self.segment_invocation_completion(seg)
            != tcl_registry::registry::InvocationCompletion::FallsThrough
        {
            return None;
        }
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            seg.argv.first()?.span.start(),
        )?;
        if words.origins().len() != seg.argv.len()
            || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                *origin != crate::registry_invocation::InvocationWordOrigin::Written(ordinal)
            })
        {
            return None;
        }
        let clauses = words.with_source_schema(&context, |schema| {
            if schema.semantics.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::If) {
                return None;
            }
            let plan = schema.clause_plan()?;
            if plan.defect.is_some() || plan.clauses.is_empty() {
                return None;
            }
            let clauses = plan
                .clauses
                .iter()
                .map(|clause| {
                    if clause.timing != tcl_registry::clause_grammar::ClauseTiming::Selected
                        || clause.operands(tcl_registry::ArgRole::Body).count() != 1
                        || clause.operands(tcl_registry::ArgRole::Expr).count() > 1
                    {
                        return None;
                    }
                    let body = clause.operand(tcl_registry::ArgRole::Body)?;
                    let semantics = if input.has_logical_source_name_context() {
                        schema.authored_logical_source_control_arm_semantics(body)
                    } else {
                        schema.authored_source_control_arm_semantics(body)
                    };
                    (semantics == Some(tcl_registry::registry::ControlArmSemantics::Selected))
                        .then_some(())?;
                    Some((clause.operand(tcl_registry::ArgRole::Expr), body))
                })
                .collect::<Option<Vec<_>>>()?;
            let (mut scripts, prefixes, mut expressions) =
                if input.has_logical_source_name_context() {
                    (
                        schema.authored_logical_source_script_arguments()?,
                        schema.authored_logical_source_command_prefix_arguments()?,
                        schema
                            .authored_logical_source_expression_arguments()?
                            .arguments,
                    )
                } else {
                    (
                        schema.authored_source_script_arguments()?,
                        schema.authored_source_command_prefix_arguments()?,
                        schema.authored_source_expression_arguments()?.arguments,
                    )
                };
            let mut expected_scripts = clauses.iter().map(|(_, body)| *body).collect::<Vec<_>>();
            let mut expected_expressions = clauses
                .iter()
                .filter_map(|(condition, _)| *condition)
                .collect::<Vec<_>>();
            scripts.sort_unstable();
            expressions.sort_unstable();
            expected_scripts.sort_unstable();
            expected_expressions.sort_unstable();
            (prefixes.is_empty()
                && scripts == expected_scripts
                && expressions == expected_expressions)
                .then_some(clauses)
        })??;
        let branches = Self::original_if_body_branches(&words, clauses, &context)?;
        self.resolve_possible_if_branches(seg, &branches)
    }

    fn original_if_body_branches(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        clauses: Vec<(Option<usize>, usize)>,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Vec<(Option<usize>, Span)>> {
        let bodies = words.source_script_bodies_for_mutation_coverage(context)?;
        if bodies.len() != clauses.len() {
            return None;
        }
        let mut branches = Vec::with_capacity(clauses.len());
        for (condition, ordinal) in clauses {
            let original = words.original_argument_word(ordinal)?;
            let mut matches = bodies
                .iter()
                .filter(|body| body.original_container() == original);
            let body = matches.next()?;
            if matches.next().is_some()
                || branches.iter().any(|(_, span)| *span == body.extent_span())
            {
                return None;
            }
            branches.push((condition, body.extent_span()));
        }
        Some(branches)
    }

    fn resolve_possible_if_branches(
        &self,
        seg: &SegmentedCommand,
        branches: &[(Option<usize>, Span)],
    ) -> Option<(Vec<Span>, bool)> {
        for &(condition, body) in branches {
            match condition {
                None => return Some((vec![body], false)),
                Some(condition) => {
                    match self.original_condition_boolean(seg, condition.checked_add(1)?) {
                        Some(true) => return Some((vec![body], false)),
                        Some(false) => {}
                        None => {
                            return Some((
                                branches.iter().map(|(_, span)| *span).collect(),
                                branches.iter().all(|(condition, _)| condition.is_some()),
                            ));
                        }
                    }
                }
            }
        }
        Some((Vec::new(), true))
    }

    pub(in super::super) fn original_condition_boolean(
        &self,
        seg: &SegmentedCommand,
        written: usize,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            self.file_lexer_config(),
            seg,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        let original = crate::registry_invocation::original_expression_operand_advice_for_word(
            context.commands(),
            &tokens,
            written,
        )?;
        if !original.lookup_closed
            || original.expression.evaluation_nodes().iter().any(|node| {
                !matches!(
                    node,
                    crate::expr_ast::ExprNode::Literal { .. }
                        | crate::expr_ast::ExprNode::Unary { .. }
                        | crate::expr_ast::ExprNode::Binary { .. }
                        | crate::expr_ast::ExprNode::Ternary { .. }
                )
            })
        {
            return None;
        }
        let mut policy = crate::tcl_expr_eval::FoldPolicy::from_registry(context.commands())
            .with_invocation_dialect(original.dialect);
        policy.word_rules =
            tcl_syntax::word_rules::WordValueRules::from_config(&self.file_lexer_config());
        crate::tcl_expr_eval::evaluate_expr_with_constants(
            &original.expression,
            &crate::tcl_expr_eval::Env::new(),
            policy,
        )
        .map(|value| value != 0)
    }

    /// Resolve a method-chain continuation from the current metaclass's
    /// actual linearised hierarchy. A user-defined next provider is judged
    /// from its body; only a proved chain end may use the registry-declared
    /// completion of the class system's built-in fallback.
    fn next_chain_terminates(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        args: &[&str],
        anchor_arg: Option<usize>,
    ) -> Option<bool> {
        if class.inheritance_unknown {
            return None;
        }
        let method = meta.grammar.unknown_dispatch_method?;
        let mut classes = self.result.all_classes.clone();
        classes.insert(class.qualified_name.clone(), class.clone());
        let hierarchy =
            super::super::super::class_hierarchy::build_class_hierarchy(classes.clone());
        if !hierarchy.errors.is_empty() {
            return None;
        }
        if hierarchy.method_target(&class.qualified_name, method)
            != Some(class.qualified_name.as_str())
        {
            return None;
        }
        let mro = hierarchy.mro_map.get(&class.qualified_name)?;
        let current = mro.iter().position(|name| name == &class.qualified_name)?;
        let scan_from = if let Some(anchor_arg) = anchor_arg {
            let written = args.get(anchor_arg)?;
            if crate::naming::is_dynamic_word(written) {
                return None;
            }
            let (namespace, _) = crate::naming::key_holder_and_tail(&class.qualified_name);
            let qualified = crate::naming::qualify(namespace, written);
            let anchor = mro.iter().position(|name| name == &qualified)?;
            (anchor > current).then_some(anchor)?
        } else {
            current + 1
        };
        for (index, ancestor) in mro.iter().enumerate().skip(scan_from) {
            if let Some(provider) = classes.get(ancestor) {
                if let Some(definition) = provider.methods.get(method) {
                    return self.method_body_terminates(definition);
                }
                continue;
            }
            let lookup = hierarchy.original_unresolved_relation(ancestor)?;
            if lookup.policy() == meta.root_policy
                && lookup
                    .candidates()?
                    .iter()
                    .any(|slot| slot == &meta.root_slot)
                && !classes.values().any(|class| {
                    class.source_name.as_ref().is_some_and(|source| {
                        source.policy() == lookup.policy() && source.slot() == &meta.root_slot
                    })
                })
            {
                return Self::builtin_next_completion(meta.grammar, method, index, mro.len());
            }
            return None;
        }
        None
    }

    pub(in super::super) fn builtin_next_completion(
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        method: &str,
        provider_index: usize,
        mro_len: usize,
    ) -> Option<bool> {
        (provider_index + 1 == mro_len).then_some(grammar.builtin_method_terminates(method))
    }

    fn method_body_terminates(
        &self,
        method: &super::super::super::types::MethodDef,
    ) -> Option<bool> {
        self.registry.as_deref()?;
        for seg in self.direct_statements_in_span(method.body_span)? {
            match self.segment_invocation_completion(&seg) {
                tcl_registry::registry::InvocationCompletion::FallsThrough => {
                    let controls = self.control_arms_for_segment(&seg);
                    if !controls.complete || !controls.arms.is_empty() {
                        return None;
                    }
                }
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_) => {
                    return Some(false);
                }
                tcl_registry::registry::InvocationCompletion::Terminates => return Some(true),
                tcl_registry::registry::InvocationCompletion::Unknown => return None,
            }
        }
        Some(false)
    }

    /// What one statement of an unknown-dispatch body contributes to
    /// [`Self::unknown_dispatch_binds_instance`]'s proof.
    fn unknown_body_evidence(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        seg: &SegmentedCommand,
        word_param: &str,
    ) -> UnknownBodyEvidence {
        let Some(registry) = self.registry.as_ref() else {
            return UnknownBodyEvidence::Nothing;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        // A compound self receiver is not itself a registry command head, so
        // classify its registry-described manufacturer method before asking
        // for ordinary command completion. The full dispatch shape remains
        // typed by the self-receiver hook and the definition grammar.
        let head = seg.name();
        let receiver_is_self = registry
            .method_dispatch_keyword(head)
            .is_some_and(|kind| kind == tcl_registry::registry::MethodDispatchKind::SelfDispatch)
            || bracketed_self_receiver(registry, head);
        if receiver_is_self
            && let Some(manufacturer) = args.first().and_then(|kw| meta.grammar.manufacturer(kw))
        {
            let Some(name_arg) = manufacturer.names_instance_at else {
                return UnknownBodyEvidence::Nothing;
            };
            let config = self.file_lexer_config();
            let bindings = self.head_identities.source_bindings_ref();
            let Some(origin) = bindings.source_origin() else {
                return UnknownBodyEvidence::Nothing;
            };
            let image = origin.source_image();
            if image.bytes() != self.source.as_bytes()
                || !bindings.matches_original_source_image(image, config)
            {
                return UnknownBodyEvidence::Nothing;
            }
            let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, seg);
            let original = crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                seg.span.start(),
                config,
            );
            return if original
                .as_ref()
                .and_then(|words| {
                    tcl_syntax::word_rules::original_scalar_variable_with_prefix(
                        words.get(usize::from(name_arg).checked_add(1)?)?,
                        &[b"", b"::"],
                    )
                })
                .is_some_and(|name| name == word_param.as_bytes())
            {
                UnknownBodyEvidence::Constructs
            } else {
                UnknownBodyEvidence::Nothing
            };
        }
        let completion = self.segment_invocation_completion(seg);
        if registry.method_dispatch_keyword(seg.name())
            == Some(tcl_registry::registry::MethodDispatchKind::NextChain)
        {
            if completion != tcl_registry::registry::InvocationCompletion::FallsThrough {
                return UnknownBodyEvidence::ReturnsSomethingElse;
            }
            let anchor_arg = registry
                .arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Name)
                .into_iter()
                .next();
            return self
                .next_chain_terminates(meta, class, &args, anchor_arg)
                .map_or(UnknownBodyEvidence::ReturnsSomethingElse, |terminates| {
                    if terminates {
                        UnknownBodyEvidence::Terminates
                    } else {
                        UnknownBodyEvidence::ReturnsSomethingElse
                    }
                });
        }
        match completion {
            tcl_registry::registry::InvocationCompletion::ReturnsResult(Some(idx)) => {
                let Some(word) = args.get(idx) else {
                    return UnknownBodyEvidence::ReturnsSomethingElse;
                };
                return if crate::value_shapes::whole_word_scalar_var_name(
                    word,
                    self.file_lexer_config(),
                ) == Some(word_param)
                {
                    UnknownBodyEvidence::ReturnsWord
                } else {
                    UnknownBodyEvidence::ReturnsSomethingElse
                };
            }
            tcl_registry::registry::InvocationCompletion::ReturnsResult(None)
            | tcl_registry::registry::InvocationCompletion::Unknown => {
                return UnknownBodyEvidence::ReturnsSomethingElse;
            }
            tcl_registry::registry::InvocationCompletion::Terminates => {
                return UnknownBodyEvidence::Terminates;
            }
            tcl_registry::registry::InvocationCompletion::FallsThrough => {}
        }
        UnknownBodyEvidence::Nothing
    }
}
