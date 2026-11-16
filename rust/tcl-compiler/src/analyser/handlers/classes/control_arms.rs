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

//! The body arms of a control command and which of them a literal environment selects.

use super::super::super::state::Analyser;
use super::{ControlArm, MAX_UNKNOWN_BODY_DEPTH};
use crate::lowering::structured::{braced_word_flags, quoted_word_flags};
use crate::segmenter::SegmentedCommand;
use crate::value_transfer::{WordForm, literal_selection};
use tcl_lexer::{Span, TokenType};

#[derive(Default)]
pub(super) struct ControlArms {
    pub(super) arms: Vec<ControlArm>,
    pub(in super::super) complete: bool,
}

impl Analyser {
    /// Whether the typed control-flow command selects `arm` for the current
    /// literal environment.  Unknown conditions and unsupported control
    /// shapes abstain; no nested body is flattened into an unconditional
    /// statement stream.
    pub(in super::super) fn control_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        match self.control_arm_semantics_for(arm)? {
            tcl_registry::registry::ControlArmSemantics::Always => return Some(true),
            tcl_registry::registry::ControlArmSemantics::Selected => {}
            // A bounded collection loop is no more *selectable* than an
            // unbounded one — which iteration a body run belongs to, or
            // whether the collection was empty, is not decidable here.
            tcl_registry::registry::ControlArmSemantics::FrameBoundary
            | tcl_registry::registry::ControlArmSemantics::BoundedIteration
            | tcl_registry::registry::ControlArmSemantics::Uncertain => return None,
            tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {
                return Some(true);
            }
        }
        let hook =
            words.with_source_schema(&context, |schema| schema.semantics.lowering_hook)??;
        let previous = self.static_eval_site.replace(arm.controller.span);
        let selected = match hook {
            tcl_registry::hooks::LoweringHookId::If => {
                self.if_body_is_selected(proc_qname, arm, env, depth)
            }
            tcl_registry::hooks::LoweringHookId::Switch => {
                self.switch_body_is_selected(proc_qname, arm, env, depth)
            }
            _ => None,
        };
        self.static_eval_site = previous;
        selected
    }

    /// Resolve typed semantics for either a direct body argument or one body
    /// nested inside a registry-described case-list argument. The latter's
    /// token span belongs to a list element, not the controller argv, so its
    /// owning clause-list position must be recovered through `CaseListSpec`.
    pub(in super::super) fn control_arm_semantics_for(
        &self,
        arm: &ControlArm,
    ) -> Option<tcl_registry::registry::ControlArmSemantics> {
        let input = self.result.resolved_input.as_ref()?;
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        let ordinal = self.control_arm_body_index(arm)?;
        words.with_source_schema(&context, |schema| {
            if input.has_logical_source_name_context() {
                schema.authored_logical_source_control_arm_semantics(ordinal)
            } else {
                schema.authored_source_control_arm_semantics(ordinal)
            }
        })?
    }

    /// Recover the owning argv position for a direct body, a case-list body,
    /// or a body nested inside a lambda literal.
    ///
    /// Timing and control-arm semantics are both per script position, so they
    /// must share this mapping rather than falling back to an invocation-wide
    /// trait when a command mixes immediate and deferred scripts.
    pub(in super::super) fn control_arm_body_index(&self, arm: &ControlArm) -> Option<usize> {
        let context = self.retained_static_source_context()?;
        let config = self.file_lexer_config();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            config,
            &arm.controller,
        );
        let bindings = self.head_identities.source_bindings_ref();
        let origin = bindings.source_origin()?;
        let original = origin.original_lexical_command(
            tokens.words(),
            arm.controller.argv.first()?.span.start(),
            config,
        )?;
        let actual = original.command();
        if actual.span != arm.controller.span
            || actual.argv != arm.controller.argv
            || actual.texts != arm.controller.texts
            || actual.word_fragments != arm.controller.word_fragments
            || actual.single_token_word != arm.controller.single_token_word
            || actual.expand_word != arm.controller.expand_word
            || actual.is_partial != arm.controller.is_partial
            || actual.partial_delimiter != arm.controller.partial_delimiter
        {
            return None;
        }
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        if words.origins().len() != arm.controller.argv.len()
            || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                // A proved dispatch states the head as its implementation;
                // with no alias prefix it is still the written head word.
                use crate::registry_invocation::InvocationWordOrigin as Origin;
                !(*origin == Origin::Written(ordinal)
                    || (ordinal == 0 && *origin == Origin::ResolvedHead))
            })
        {
            return None;
        }
        let mut bodies = words
            .source_script_bodies_for(
                &context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            )
            .into_iter()
            .chain(words.original_lambda_body_geometry(&context))
            .filter(|body| body.extent_span() == arm.body_span);
        let body = bodies.next()?;
        if bodies.next().is_some() {
            return None;
        }
        let mut ordinals = words
            .operands()
            .iter()
            .enumerate()
            .filter_map(|(ordinal, operand)| {
                (operand.as_ref()?.word()? == body.original_container()).then_some(ordinal)
            });
        let ordinal = ordinals.next()?;
        ordinals.next().is_none().then_some(ordinal)
    }

    /// Whether a body argument this walk could **not** read makes the arm set
    /// incomplete.
    ///
    /// Two registry facts, in order, and the walk abstains unless both say the
    /// body is irrelevant to it:
    ///
    /// 1. **Typed control semantics.** A body position the registry types
    ///    carries a fact the consumers below consume directly — whether it is
    ///    one arm of a selection (`if {$c} $script`), a frame boundary that
    ///    runs now (`namespace eval $ns $script`), a bounded iteration. An
    ///    unreadable body there is a missing answer, so the arm set really is
    ///    incomplete.
    /// 2. **[`ScriptTiming`].** With no typed semantics the question left is
    ///    whether this exact body argument runs as part of *this* invocation,
    ///    and that is registry data, never an inference. `proc ${ns}::define
    ///    {a} [string map … {…}]` stores its body — nothing in it executes
    ///    here, so an unreadable body word costs the walk nothing about the
    ///    declaration, which falls through like any other command. Legacy
    ///    positional bodies without an exact timing descriptor fall back to
    ///    the command-level [`Traits::DEFERS_BODY`] fact.
    ///
    /// The default is deliberately the abstaining one: a command that has not
    /// declared deferred timing is assumed to run its body now. The first
    /// version of this helper read the *absence* of control-arm semantics as
    /// proof of dormancy, which is not what that absence means —
    /// `uplevel 1 $script` and `oo::define $c $script` have no control-arm
    /// semantics either, and both execute their argument immediately and can
    /// raise or return before the enclosing walk reaches the statement it
    /// cares about. That let the computed-metaclass walk materialise a class
    /// created after `uplevel 1 $script`.
    /// `BodyKind` cannot answer it either: `proc` and `uplevel` are both
    /// `Structural`, because that descriptor answers *which frame*, not
    /// *when*.
    ///
    /// `body_index` may name an [`ArgRole::LambdaLiteral`] word as well as an
    /// [`ArgRole::Body`] one — an unreadable lambda is unreadable script on
    /// the same terms, and `apply` types that position, so it
    /// stops at the first check.
    ///
    /// [`ArgRole::Body`]: tcl_registry::ArgRole::Body
    /// [`ArgRole::LambdaLiteral`]: tcl_registry::ArgRole::LambdaLiteral
    /// [`ScriptTiming`]: tcl_registry::ScriptTiming
    /// [`Traits::DEFERS_BODY`]: tcl_registry::Traits::DEFERS_BODY
    fn unreadable_body_is_material(&self, seg: &SegmentedCommand, body_index: usize) -> bool {
        let Some(context) = self.retained_static_source_context() else {
            return true;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        if registry
            .control_arm_semantics(seg.name(), &args, body_index)
            .is_some()
        {
            return true;
        }
        self.body_runs_now(seg, body_index)
    }

    /// Whether `seg` runs its body argument as part of **this** invocation.
    ///
    /// The one place that answers it, for both the unreadable-body question
    /// above and the untyped-arm question in
    /// [`Self::static_statement_falls_through`] — the two were
    /// the same question asked in two places, and only one of them was asking
    /// it.
    ///
    /// Registry data, never an inference: an option or argument can declare
    /// its exact [`ScriptTiming`], so a command with both immediate and stored
    /// scripts does not force one answer onto both. Legacy positional bodies
    /// fall back to command-level [`Traits::DEFERS_BODY`]. Everything else is
    /// assumed to run now. That default is the abstaining direction — a body
    /// that runs can raise or return before the walk reaches the statement it
    /// cares about, while a body that is merely stored cannot.
    ///
    /// [`Traits::DEFERS_BODY`]: tcl_registry::Traits::DEFERS_BODY
    /// [`ScriptTiming`]: tcl_registry::ScriptTiming
    pub(super) fn body_runs_now(&self, seg: &SegmentedCommand, body_index: usize) -> bool {
        let Some(context) = self.retained_static_source_context() else {
            return true;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        registry
            .script_timing(
                seg.name(),
                &args,
                body_index,
                Some(context.context().authoring_query()),
            )
            .is_none_or(|timing| timing == tcl_registry::ScriptTiming::SameInvocation)
    }

    /// Whether a body the registry gives **no** control-arm semantics for is
    /// proved to stop control before its own end.
    ///
    /// The typed arms above ask the opposite question — *prove* the body falls
    /// through, or abstain — because a descriptor says what the body is for. An
    /// untyped body carries no such promise, and asking it to prove
    /// fall-through would abstain on every script written in a vocabulary this
    /// walk does not model. A class definition script is the case that settles
    /// it: `oo::class create C { superclass D }` is an untyped body running
    /// now, and `superclass` is a definition command, not a command — nothing
    /// here can type it, so demanding a proof of fall-through would abstain on
    /// the one construct the computed-metaclass walk exists to read (the tcllib
    /// clay corpus is built out of exactly these).
    ///
    /// So the burden runs the other way: the enclosing statement is walked past
    /// unless *this* body proves it blocks. Both proofs are registry-owned and
    /// oracle-checked against tclsh 8.6.16 and 9.0.4, which agree exactly:
    ///
    /// * [`InvocationCompletion::Terminates`] — `uplevel 1 {error stop}`,
    ///   `eval {error stop}`, `oo::define C {error stop}`: the statement after
    ///   them never runs;
    /// * [`InvocationCompletion::ReturnsResult`] — `oo::define C {return}`,
    ///   `eval {return}`, `uplevel 1 {return}`: likewise, the completion
    ///   propagates out of the body rather than ending at it.
    ///
    /// A statement whose completion is [`InvocationCompletion::Unknown`] ends
    /// the scan with no proof, which leaves the enclosing walk exactly where it
    /// was before this rule existed. That is the honest limit of an untyped
    /// body, and it is why nested control inside one (`oo::define C {if {$x}
    /// {error stop} else {error stop}}`) is not detected: the `if` itself
    /// completes normally as far as the registry is concerned, and reading its
    /// arms requires control-flow facts unavailable to this lexical walk.
    ///
    /// [`InvocationCompletion::Terminates`]: tcl_registry::registry::InvocationCompletion::Terminates
    /// [`InvocationCompletion::ReturnsResult`]: tcl_registry::registry::InvocationCompletion::ReturnsResult
    /// [`InvocationCompletion::Unknown`]: tcl_registry::registry::InvocationCompletion::Unknown
    pub(super) fn untyped_body_provably_blocks(&mut self, body_span: Span, depth: u32) -> bool {
        if depth >= MAX_UNKNOWN_BODY_DEPTH || self.retained_static_source_context().is_none() {
            return false;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return false;
        };
        for seg in statements {
            if self.static_statement_falls_through(&seg, depth) {
                continue;
            }
            let completion = match self.segment_invocation_completion(&seg) {
                // An untyped body's statements may have no source point of
                // their own on a release profile. Reading the written head
                // there can only make this walk abstain, never claim.
                tcl_registry::registry::InvocationCompletion::Unknown => {
                    self.written_head_completion(&seg)
                }
                completion => completion,
            };
            return matches!(
                completion,
                tcl_registry::registry::InvocationCompletion::Terminates
                    | tcl_registry::registry::InvocationCompletion::ReturnsResult(_)
            );
        }
        false
    }

    /// The registry's completion for the statement's written words, with no
    /// lookup behind it. Only the abstaining direction may read it.
    fn written_head_completion(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::registry::InvocationCompletion {
        let Some(context) = self.retained_static_source_context() else {
            return tcl_registry::registry::InvocationCompletion::Unknown;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        context.commands().invocation_completion(
            seg.name(),
            &args,
            Some(context.context().authoring_query()),
        )
    }

    pub(super) fn control_arms_for_segment(&self, seg: &SegmentedCommand) -> ControlArms {
        let Some(context) = self.retained_static_source_context() else {
            return ControlArms::default();
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let mut indices =
            registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
        indices.sort_unstable();
        indices.dedup();
        let case_call = registry
            .case_invocation(seg.name(), &args, Some(context.context().authoring_query()))
            .and_then(|(_, invocation)| invocation.clause_list_index);
        let mut arms = Vec::new();
        let mut complete = true;
        for idx in indices {
            let (Some(word), Some(token)) =
                (seg.args().get(idx), seg.arg_tokens().get(idx).copied())
            else {
                complete &= !self.unreadable_body_is_material(seg, idx);
                continue;
            };
            if case_call == Some(idx) {
                let Some((case, _)) = registry.case_invocation(
                    seg.name(),
                    &args,
                    Some(context.context().authoring_query()),
                ) else {
                    complete = false;
                    continue;
                };
                for (_, (body_word, body_token)) in crate::segmenter::flatten_case_list_clauses(
                    &self.source,
                    word,
                    token,
                    &case,
                    self.lexer_config(),
                ) {
                    if body_token.kind == TokenType::Str
                        || (body_token.kind == TokenType::Esc
                            && !tcl_syntax::naming::is_dynamic_word(&body_word)
                            && !body_word.contains('\\'))
                    {
                        arms.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                    } else {
                        complete = false;
                    }
                }
            } else if token.kind == TokenType::Str
                || (token.kind == TokenType::Esc
                    && !tcl_syntax::naming::is_dynamic_word(word)
                    && !word.contains('\\'))
            {
                arms.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
            } else {
                complete &= !self.unreadable_body_is_material(seg, idx);
            }
        }
        self.collect_lambda_arms(seg, &args, &mut arms, &mut complete);
        ControlArms { arms, complete }
    }

    /// The [`ArgRole::LambdaLiteral`] half of [`Self::control_arms_for_segment`].
    ///
    /// A lambda argument carries a script that runs *now* — `apply $lambda`
    /// can raise before control ever reaches the next statement — but it is
    /// not an [`ArgRole::Body`], so the loop above never sees it. The walk
    /// therefore neither read such a lambda nor abstained on it, and claimed
    /// past an aborting one: the same soundness class as the `uplevel` shape
    /// above, reached through the other role.
    ///
    /// The argument is one level removed from a body — a `{argList body ?ns?}`
    /// **list** — so a readable one contributes its *body element*'s span,
    /// split by the shared [`crate::lambda_literal`] owner rather than
    /// re-segmented whole. A lambda whose
    /// body cannot be located at exact source offsets — a computed word, a
    /// malformed list, a body element that is not `{braced}` and so has its
    /// escapes collapsed before `apply` sees it — is unreadable, and falls to
    /// the same materiality question a `Body` word does.
    ///
    /// [`ArgRole::Body`]: tcl_registry::ArgRole::Body
    /// [`ArgRole::LambdaLiteral`]: tcl_registry::ArgRole::LambdaLiteral
    fn collect_lambda_arms(
        &self,
        seg: &SegmentedCommand,
        args: &[&str],
        arms: &mut Vec<ControlArm>,
        complete: &mut bool,
    ) {
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let mut indices =
            registry.arg_indices_for_role(seg.name(), args, tcl_registry::ArgRole::LambdaLiteral);
        indices.sort_unstable();
        indices.dedup();
        for idx in indices {
            // Read the body only where the registry says what running it
            // means. `apply` types this position (a frame boundary that runs
            // now); a command that carries the role without typing it has told
            // the walk nothing, and an untyped script that runs now is exactly
            // what `unreadable_body_is_material` abstains on.
            let body = registry
                .control_arm_semantics(seg.name(), args, idx)
                .and_then(|_| seg.arg_tokens().get(idx).copied())
                .and_then(|token| crate::lambda_literal::split_lambda_literal(&self.source, token))
                .and_then(|elements| elements.braced_body());
            if let Some(body_span) = body {
                arms.push(ControlArm {
                    controller: seg.clone(),
                    body_span,
                });
            } else {
                *complete &= !self.unreadable_body_is_material(seg, idx);
            }
        }
    }

    fn if_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        let wanted = self.control_arm_body_index(arm)?;
        let clauses = words.with_source_schema(&context, |schema| {
            if schema.semantics.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::If) {
                return None;
            }
            let plan = schema.clause_plan()?;
            if plan.defect.is_some() || plan.clauses.is_empty() {
                return None;
            }
            plan.clauses
                .iter()
                .map(|clause| {
                    if clause.timing != tcl_registry::clause_grammar::ClauseTiming::Selected
                        || clause.operands(tcl_registry::ArgRole::Body).count() != 1
                        || clause.operands(tcl_registry::ArgRole::Expr).count() > 1
                    {
                        return None;
                    }
                    Some((
                        clause.operand(tcl_registry::ArgRole::Expr),
                        clause.operand(tcl_registry::ArgRole::Body)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()
        })??;
        for (condition, body) in clauses {
            let Some(condition) = condition else {
                return Some(body == wanted);
            };
            let original = words.original_argument_word(condition)?;
            let source = original.image().try_text().ok()?;
            let expression = source.get(original.content_span().ok()?.as_range())?;
            if self.static_expr_value(expression, proc_qname, env, &mut Vec::new(), depth)? {
                return Some(body == wanted);
            }
        }
        Some(false)
    }

    /// Whether the body at `arm.body_span` is the one the `switch` runs, when
    /// the environment states the words the registry's selection reads: the
    /// selection is the command's own (mode, case folding, ordered first
    /// match, the final `default`, fall-through, the regexp engine), never a
    /// match made here. `None` where any word is not stated, the selection
    /// declines, or the subject's members would run different bodies.
    fn switch_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands();
        let args: Vec<&str> = arm.controller.args().iter().map(String::as_str).collect();
        let (case, invocation) = registry.case_invocation(
            arm.controller.name(),
            &args,
            Some(context.context().authoring_query()),
        )?;
        if usize::from(case.subject_args) != 1 {
            return None;
        }
        let values = self.switch_words(proc_qname, arm, &invocation, env, depth)?;
        let words: Vec<(&str, WordForm)> = values
            .iter()
            .map(|(text, form)| (text.as_str(), *form))
            .collect();
        let selection = literal_selection(registry, arm.controller.name(), &words, None)?;
        let bodies = self.switch_clause_bodies(arm, &case, &invocation)?;
        if bodies.len() != selection.clauses || Some(selection.subject) != invocation.subject_index
        {
            return None;
        }
        let mut verdicts = selection.fact.bodies.iter().map(|body| match body {
            None => Some(false),
            Some(clause) => bodies.get(*clause).map(|span| *span == arm.body_span),
        });
        let first = verdicts.next()??;
        verdicts
            .all(|verdict| verdict == Some(first))
            .then_some(first)
    }

    /// The words of a `switch` as its selection reads them, each with how it
    /// was written: the subject and the inline patterns as the environment
    /// states them (a braced one is its own text), every other word as
    /// written. A word left to substitute that is not the subject or a
    /// pattern is unproven, so a `-` body it might spell declines the
    /// selection.
    fn switch_words(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        invocation: &tcl_registry::spec::CaseInvocation,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<Vec<(String, WordForm)>> {
        let controller = &arm.controller;
        let count = controller.args().len();
        let braced = braced_word_flags(
            controller.arg_tokens(),
            controller.arg_single_token(),
            count,
        );
        let quoted = quoted_word_flags(controller.arg_tokens(), count);
        let is_pattern = |index: usize| {
            invocation
                .inline_clause_start
                .is_some_and(|start| index >= start && (index - start).is_multiple_of(2))
        };
        let mut words = Vec::with_capacity(count);
        for (index, text) in controller.args().iter().enumerate() {
            let stated = invocation.subject_index == Some(index) || is_pattern(index);
            let form = if braced[index] {
                WordForm::Braced
            } else if quoted[index] {
                WordForm::Quoted
            } else {
                WordForm::Bare
            };
            words.push(if braced[index] {
                (
                    self.word_rules().collapse_braced_word(text).into_owned(),
                    form,
                )
            } else if stated {
                let value =
                    self.static_word_value(text, proc_qname, env, &mut Vec::new(), depth)?;
                (value, form)
            } else if ['$', '[', '\\'].iter().any(|mark| text.contains(*mark)) {
                (text.clone(), WordForm::Unproven)
            } else {
                (text.clone(), form)
            });
        }
        Some(words)
    }

    /// The body span of each clause of a `switch`, in the command's order:
    /// the clause-list word's bodies, or every second inline word after the
    /// patterns begin.
    fn switch_clause_bodies(
        &self,
        arm: &ControlArm,
        case: &tcl_registry::spec::CaseListSpec,
        invocation: &tcl_registry::spec::CaseInvocation,
    ) -> Option<Vec<Span>> {
        let args = arm.controller.args();
        let tokens = arm.controller.arg_tokens();
        if let Some(list_index) = invocation.clause_list_index {
            let clauses = crate::segmenter::flatten_case_list_clauses(
                &self.source,
                args.get(list_index)?,
                *tokens.get(list_index)?,
                case,
                self.lexer_config(),
            );
            return Some(
                clauses
                    .into_iter()
                    .map(|(_, (_, body))| body.span)
                    .collect(),
            );
        }
        let start = invocation.inline_clause_start?;
        (start..args.len().saturating_sub(1))
            .step_by(2)
            .map(|pattern| tokens.get(pattern + 1).map(|token| token.span))
            .collect()
    }
}
