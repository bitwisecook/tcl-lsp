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

//! Lowering an invocation: its native recipe, definition-time commands and cell updates such as `incr`.

use super::super::cells::{CellPlace, name_data_place};
use super::super::elide::{BarrierDecision, BarrierKept};
use super::super::ir::{
    CachedIntegerIncrement, DefinitionDispatch, IntOp, NativeOp, NativeType, NativeValueId,
};
use super::super::representation::{Representation, proven_int_result};
use super::super::{CellAccessKind, NativeLoweringDecline, StatementOutcome};
use super::Lowerer;
use crate::executable_ir::{GenericInvoke, InvocationResolution};
use crate::ir::WordExpr;
use crate::registry_invocation::{
    InvocationMetadataContext, resolved_tokens_invocation_with_metadata_context,
};
use crate::types::TypeShape;
use tcl_registry::{CellUpdate, IntrinsicId, NativeLowering};

impl Lowerer<'_> {
    // Invocations.

    pub(super) fn lower_invoke(
        &mut self,
        invoke: &GenericInvoke,
        block_index: usize,
        position: usize,
    ) -> StatementOutcome {
        if let Some(reason) = self.declined_nodes.get(&invoke.node).copied() {
            self.eval_source(&invoke.source, reason);
            return StatementOutcome::EvalSource(reason);
        }
        let Some(argv) = self.argvs.get(&invoke.argv).cloned() else {
            let reason = NativeLoweringDecline::MissingCommandTokens;
            self.eval_source(&invoke.source, reason);
            return StatementOutcome::EvalSource(reason);
        };
        if invoke.registry_specialisation_arguments_exact()
            && let InvocationResolution::Resolved(facts) = &invoke.resolution
            && let Some(lowering) = self.invocation_native_lowering(invoke, facts)
        {
            // The site proof's operand-admissibility verdict is conservative
            // about command substitutions and variable reads in the operand
            // words; the lowering knows exactly which of those words
            // dispatched a command or read an unproven cell, so a node whose
            // words did neither may take the proof on its domain coverage.
            let words_observed = self.observing_nodes.contains(&invoke.node);
            let proven = self
                .proofs
                .site(
                    u32::try_from(block_index).unwrap_or(u32::MAX),
                    u32::try_from(position).unwrap_or(u32::MAX),
                )
                .is_some_and(|site| {
                    (site.operand_words_admissible || !words_observed)
                        && facts
                            .dispatch_dependencies
                            .iter()
                            .all(|domain| site.covers.contains(domain))
                });
            match lowering {
                NativeLowering::Intrinsic {
                    id: IntrinsicId::ChannelWrite,
                    arity,
                } if proven && argv.len() == 2 && arity.accepts(1) => {
                    self.emit(NativeOp::Puts { src: argv[1] });
                    if !cells_unobserved(&facts.effects) {
                        self.clobber_shadows();
                    }
                    return StatementOutcome::NativeIntrinsic;
                }
                NativeLowering::Completion(code) if proven && argv.len() == 1 => {
                    self.emit(NativeOp::Complete { code, result: None });
                    return StatementOutcome::NativeCompletion;
                }
                NativeLowering::CellReadModifyWrite(update) if proven && argv.len() >= 2 => {
                    if let Some(outcome) = self.lower_cell_update(update, invoke, &argv) {
                        return outcome;
                    }
                }
                NativeLowering::Definition => {
                    let dispatch = if proven {
                        Some(DefinitionDispatch::Proved)
                    } else {
                        self.original_definition_guard(invoke)
                            .map(DefinitionDispatch::RuntimeGuarded)
                    };
                    if let Some(outcome) =
                        dispatch.and_then(|dispatch| self.lower_definition(invoke, &argv, dispatch))
                    {
                        return outcome;
                    }
                }
                _ => {}
            }
        }
        self.max_argc = self.max_argc.max(argv.len());
        self.emit(NativeOp::Invoke { argv });
        self.clobber_shadows();
        StatementOutcome::GenericInvoke
    }

    fn invocation_native_lowering(
        &self,
        invoke: &GenericInvoke,
        facts: &tcl_registry::InvocationFacts,
    ) -> Option<NativeLowering> {
        if self.source_absent_model() {
            return self.input.registry.native_lowering_for_descriptor(
                facts.descriptor_key(),
                self.input.registry.own_surface_query().as_ref(),
            );
        }
        let tokens = self.original_invocations.as_ref()?.command(invoke)?;
        let metadata = self.original_invocation_metadata(tokens)?;
        let selected = resolved_tokens_invocation_with_metadata_context(
            self.input.registry,
            Some(metadata),
            tokens,
        )?;
        (selected.facts.as_ref() == facts).then_some(())?;
        self.input.registry.native_lowering_for_descriptor(
            selected.facts.descriptor_key(),
            Some(&metadata.context().authoring_query()),
        )
    }

    pub(super) fn original_invocation_metadata<'b>(
        &self,
        tokens: &'b crate::ir::CommandTokens,
    ) -> Option<InvocationMetadataContext<'b>> {
        let input = self.input.module.source_metadata_input.as_ref()?;
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(tokens, input, self.input.registry)
    }

    /// A source-owned candidate supplies no live dispatch-domain proof.
    fn original_definition_guard(
        &self,
        invoke: &GenericInvoke,
    ) -> Option<tcl_runtime_api::guard::GuardIdentity> {
        if !self.input.top_level
            || invoke.original_words.len() != 4
            || !invoke.original_words.iter().all(|word| {
                matches!(
                    word,
                    crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
                )
            })
        {
            return None;
        }
        let module = self.input.module;
        let entry = module.source_entry.native_entry.as_deref()?;
        let profile = module.dialect_profile?;
        let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point?);
        if entry.frame != tcl_runtime_api::native_compilation::NativeCompilationFrame::Global
            || !entry.execution_name_policy.is_some_and(|policy| {
                policy.native_recipe().is_some_and(|recipe| {
                    recipe.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
                })
            })
            || entry.invocation_policy != Some(profile.cache_key())
            || !dialect
                .has_same_execution_policy(tcl_registry::InvocationDialect::of_profile(profile))
            || module.native_lexer_config() != tcl_lexer::LexerConfig::for_profile(Some(profile))
            || !entry
                .namespace_context(entry.current_namespace)
                .ok()?
                .path
                .as_segments()
                .is_empty()
        {
            return None;
        }
        let metadata = InvocationMetadataContext::for_module(self.input.registry, module)?;
        (module.original_invocation_source_purpose(self.input.registry, metadata)
            == Some(crate::command_binding::OriginalInvocationSourcePurpose::Execution))
        .then_some(())?;
        NativeLowering::Definition.definition_guard_identity(dialect)
    }

    /// A definition-time command (`proc`) whose body this module also
    /// compiled: the runtime binds the written source body exactly as `proc`
    /// does, and additionally binds the compiled entry the emitter installed
    /// for that body.
    ///
    /// The exact original allocating instruction selects the retained body.
    /// Missing or ambiguous body provenance keeps the generic invocation.
    fn lower_definition(
        &mut self,
        invoke: &GenericInvoke,
        argv: &[NativeValueId],
        dispatch: DefinitionDispatch,
    ) -> Option<StatementOutcome> {
        let site = invoke.source_binding.as_ref()?.invocation_site()?;
        let mut matching = self
            .input
            .module
            .native_source_procedure_bodies_for_compilation_at_declaration(
                site,
                invoke.source.span,
            );
        let (id, procedure) = matching.next()?;
        if matching.next().is_some() {
            return None;
        }

        // A body the front end never captured (a synthetic or dynamic one)
        // has nothing for the runtime to bind; keep the generic invocation.
        let body_source = procedure.body_source.clone()?;
        // Every word this hands the runtime has to be the word the statement
        // actually writes; a substituted one keeps the generic invocation, and
        // the runtime's own `proc` — which evaluates the word at the call site,
        // as Tcl does — defines the procedure.
        if !definition_words_are_written_out(
            &invoke.original_words,
            &procedure.params_raw,
            &body_source,
        ) {
            return None;
        }
        if argv.len() != 4 {
            return None;
        }
        self.max_argc = self.max_argc.max(argv.len());
        self.emit(NativeOp::DefineProc {
            dispatch,
            entry_key: *id,
            argv: argv.to_vec(),
        });
        self.clobber_shadows();
        Some(StatementOutcome::NativeDefinition)
    }

    /// `incr`/`append`/`lappend` reached as an invocation: the place is the
    /// first argument word, which must be a literal name.
    fn lower_cell_update(
        &mut self,
        update: CellUpdate,
        invoke: &GenericInvoke,
        argv: &[NativeValueId],
    ) -> Option<StatementOutcome> {
        let place = if let Some(binding) = &invoke.source_binding {
            let (_, mut tokens) = binding.original_recorded_command()?;
            (tokens.words() == invoke.original_words).then_some(())?;
            tokens.source_binding = Some(binding.clone());
            let value = crate::registry_invocation::original_literal_argument_data_for_module(
                self.input.registry,
                self.input.module,
                &tokens,
                0,
            )?;
            name_data_place(&value)?
        } else {
            self.source_absent_model().then_some(())?;
            let word = invoke.original_words.get(1)?;
            let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.lexer_config.escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&self.lexer_config),
                )
            else {
                return None;
            };
            name_data_place(&value)?
        };
        match update {
            CellUpdate::Increment => {
                if argv.len() > 3 {
                    return None;
                }
                let delta = match argv.get(2) {
                    Some(value) => *value,
                    None => self.const_int(1),
                };
                self.lower_incr(&place, delta, argv.len() == 2);
                Some(StatementOutcome::Native)
            }
            CellUpdate::Append | CellUpdate::ListAppend => {
                let barrier = self.ledger.decide(&place);
                self.emit(NativeOp::CellAppend {
                    place: place.clone(),
                    values: argv[2..].to_vec(),
                    list: update == CellUpdate::ListAppend,
                    barrier,
                });
                self.shadows.forget_base(place.base());
                self.record_cell(&place, CellAccessKind::Update, barrier, false);
                Some(StatementOutcome::Native)
            }
        }
    }

    pub(super) fn remember_cached_increment(
        &mut self,
        place: &CellPlace,
        boxed: NativeValueId,
        integer: NativeValueId,
        barrier: BarrierDecision,
    ) {
        if self.input.top_level
            && self.input.module.source_entry.native_entry.is_some()
            && matches!(place, CellPlace::Named { .. })
            && barrier == BarrierDecision::Kept(BarrierKept::TraceLedgerUnknown)
            && self.boxed_integer_sources.get(&boxed) == Some(&integer)
        {
            self.shadows
                .remember_cached_increment(place.clone(), boxed, integer);
        }
    }

    fn cached_increment_candidate(
        &mut self,
        place: &CellPlace,
        delta: NativeValueId,
    ) -> Option<CachedIntegerIncrement> {
        let (receiver, old) = self.shadows.cached_increment(place)?;
        let interval = proven_int_result(
            IntOp::Add,
            self.rep(old).interval()?,
            self.rep(delta).interval()?,
        )?;
        let sum = self.new_value(NativeType::I64, Representation::NativeInt(interval));
        self.emit(NativeOp::IntBinary {
            dst: sum,
            op: IntOp::Add,
            lhs: old,
            rhs: delta,
        });
        Some(CachedIntegerIncrement { receiver, old, sum })
    }

    /// `incr place delta`: native when the cell's shadow and the delta are
    /// proven-in-range integers, else the guarded runtime read-modify-write.
    pub(super) fn lower_incr(&mut self, place: &CellPlace, delta: NativeValueId, implicit: bool) {
        let shadow = self.shadows.read(place);
        if let (Some(current), NativeType::I64) = (shadow, self.ty(delta))
            && self.ty(current) == NativeType::I64
            && let (Some(lhs), Some(rhs)) =
                (self.rep(current).interval(), self.rep(delta).interval())
            && let Some(result) = proven_int_result(IntOp::Add, lhs, rhs)
        {
            let dst = self.new_value(NativeType::I64, Representation::NativeInt(result));
            self.emit(NativeOp::IntBinary {
                dst,
                op: IntOp::Add,
                lhs: current,
                rhs: delta,
            });
            let boxed = self.boxed(dst);
            self.write_cell(place, boxed, dst);
            return;
        }
        let barrier = self.ledger.decide(place);
        let guard = self.ledger.incr_guard(place);
        let cached = implicit
            .then(|| self.cached_increment_candidate(place, delta))
            .flatten();
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(Some(TypeShape::Int)));
        self.emit(NativeOp::CellIncr {
            dst,
            place: place.clone(),
            delta,
            guard,
            cached,
            barrier,
        });
        if barrier.is_elided() {
            self.shadows.write(place.clone(), dst);
        } else {
            self.shadows.forget_base(place.base());
        }
        self.record_cell(place, CellAccessKind::Update, barrier, shadow.is_some());
    }
}

/// Whether an invocation's registry effect footprint proves it neither
/// writes a variable cell nor runs a callback that could.
fn cells_unobserved(effects: &tcl_registry::world_effect::EffectFootprint) -> bool {
    use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};
    !effects.requires_world_barrier()
        && effects.accesses().iter().all(|access| {
            !matches!(
                access.domain,
                WorldStateDomain::VariableStore | WorldStateDomain::VariableTraces
            ) || access.mode == EffectAccessMode::Read
        })
}

/// The cell a variable word reads, in this tier's decline vocabulary.
///
/// [`variable_word_place`] is the one owner of the reading; this only names
/// the refusal.
/// Whether the word is written out literally, so what the front end recorded
/// for it is the text the statement itself carries.
fn is_written_literal(word: &WordExpr) -> bool {
    matches!(
        word,
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
    )
}

/// Whether the word is a literal whose content is exactly `recorded`.
fn word_is_literally(word: &WordExpr, recorded: &str) -> bool {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => text == recorded,
        _ => false,
    }
}

/// Whether a `proc name params body` statement wrote every word out, and wrote
/// the parameter list and body the surviving [`crate::ir::Procedure`] recorded.
///
/// The condition under which a tier may register a definition itself rather
/// than leave it to the runtime's own `proc`. `Procedure` records the
/// *written* name, parameter list and body text, but a lowering may have
/// compiled the body from a value it materialised instead — a const-mapped
/// `$body`, or a `[subst -nocommands …]` template — recording the original
/// word beside that compiled body rather than the text it compiled.
/// Registering that word would report the wrong `info body`, and any later run
/// of the source body (a step trace, or a declined native entry) would
/// evaluate the substitution *in the procedure's own frame*, where its
/// operands do not exist.
///
/// Shared by the native tier and the general wasm tier because it is one rule:
/// a second copy is a second thing to go out of step.
pub(crate) fn definition_words_are_written_out(
    words: &[WordExpr],
    params_raw: &str,
    body_source: &str,
) -> bool {
    // The runtime's `proc` owns every error the other shapes can raise, so
    // only the exact four-word form is taken.
    let [_head, name, params, body] = words else {
        return false;
    };
    is_written_literal(name)
        && word_is_literally(params, params_raw)
        && word_is_literally(body, body_source)
}
