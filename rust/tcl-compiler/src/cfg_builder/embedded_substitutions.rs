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

//! The effects of the `[…]` substitutions a statement's words or a condition run before it: registry barriers, caller-side defs and protected reads.

use super::{CfgBuilder, ConditionEffects, EmbeddedSubstExtras, REGISTRY_BARRIER_TRAITS};
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use std::collections::BTreeSet;
use tcl_lexer::Span;

impl CfgBuilder<'_> {
    /// Whether any recovered command substitution reaches a registry operation
    /// with a barrier/evaluation trait. Substitutions execute before their
    /// host statement, so callers place the synthetic barrier before that
    /// host in the CFG.
    pub(super) fn embedded_registry_barrier(&self, stmt: &Statement) -> bool {
        let bindings = self
            .source_binding_timeline
            .as_ref()
            .and_then(|timeline| timeline.before_substitutions(stmt.span()))
            .cloned()
            .unwrap_or_else(|| self.command_bindings.clone());
        let Some(metadata) = self.command_classes.metadata_context() else {
            return true;
        };
        let metadata = metadata.context();
        let state = std::cell::RefCell::new(bindings);
        let barrier = std::cell::Cell::new(false);
        let resolve = |head: &str| {
            state
                .borrow()
                .resolved_embedded_head(head, &self.invocation_namespace)
        };
        let observe = |words: &[crate::ir_helpers::CommandWord], conditional: bool| {
            let found = state
                .borrow_mut()
                .source_order_registry_barrier_for_command_with_metadata_context(
                    words,
                    conditional,
                    self.registry,
                    &self.invocation_namespace,
                    REGISTRY_BARRIER_TRAITS,
                    metadata,
                );
            barrier.set(barrier.get() || found);
        };
        let embedded =
            crate::ir_helpers::evaluated_command_substitutions_with_replay_and_metadata_context(
                stmt,
                self.registry,
                Some(&resolve),
                Some(&observe),
                metadata,
                self.config,
            );
        embedded.opaque || barrier.get()
    }

    pub(super) fn registry_barrier_statement(stmt: &Statement, reason: &str) -> Statement {
        Self::registry_barrier_statement_at(stmt.span(), reason)
    }

    fn registry_barrier_statement_at(span: Span, reason: &str) -> Statement {
        Statement::Barrier {
            span,
            reason: reason.to_owned(),
            command: "<registry-barrier>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::RegistryBarrier,
            )),
        }
    }

    /// Resolve a recovered substitution's head through the module's command
    /// bindings, so a registry-owned role question about it is asked of the
    /// command it really reaches.
    ///
    /// Without this, `interp alias {} e {} expr` hid an expression word from
    /// the in-frame descent: the raw spelling `e` has no `ArgRole::Expr`, so
    /// the `[incr x]` of `[e {$x + [incr x]}]` was invisible and the later
    /// read folded to the stale literal — tclsh 8.6.18 / 9.0.4 print `3` then
    /// `2`.
    ///
    /// Answers only for a spelling with exactly one statically known
    /// registry-backed target. Several possible bindings mean the role
    /// question has no single answer, and a user procedure carries no
    /// registry roles at all; both fall back to the raw spelling.
    pub(super) fn embedded_head_resolver(
        &self,
    ) -> impl Fn(&str) -> Option<crate::ir_helpers::ResolvedEmbeddedHead> + '_ {
        |head: &str| {
            self.command_bindings
                .resolved_embedded_head(head, &self.invocation_namespace)
        }
    }

    /// The embedded-substitution half of [`Self::upvar_invalidated`]: the
    /// caller-side defs contributed by `[…]` substitutions in the
    /// statement's argument words (or an assignment's value), the subset of
    /// those the embedded command reads before writing, plus whether any
    /// embedded callee runs an unreadable script at the global frame.
    pub(super) fn embedded_subst_extras(&self, stmt: &Statement) -> EmbeddedSubstExtras {
        let resolve = self.embedded_head_resolver();
        let embedded = crate::ir_helpers::evaluated_command_substitutions_with_heads(
            stmt,
            self.registry,
            Some(&resolve),
        );
        let mut embedded_extras: Vec<String> = Vec::new();
        let mut embedded_opaque_global = embedded.opaque;
        let upvar = self.upvar_effects_from_commands(
            &embedded.commands,
            self.original_tokens_for_statement(stmt),
        );
        embedded_opaque_global |= upvar.opaque_arguments;
        for d in upvar.defs {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        let (global_defs, opaque) = self.global_write_defs_from_commands(
            &embedded.commands,
            self.original_tokens_for_statement(stmt),
        );
        embedded_opaque_global |= opaque;
        for d in global_defs {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        // A var-mutating builtin inside a command substitution
        // (`set y [append x b]`, `[incr x]`, `[lset l …]`) writes its
        // target variable as a side effect; record it so copy / constant
        // propagation (O100) does not propagate a stale value past the
        // mutation (FP-OPT-06).
        // The variable-effect view takes the in-frame words too: a `[incr x]`
        // inside `[expr {…}]`, or the `incr x` of `[catch {incr x}]`, writes
        // `x` whatever word carried it.
        // The call-graph consumers above deliberately do not — see
        // `EvaluatedCommandSubstitutions::in_frame_commands`.
        // A call the walk binds only to source procedures writes through its
        // body, which the summaries above already account for; the registry
        // knows nothing of it and would call it opaque.
        let summarised = self.summarised_embedded_heads(self.original_tokens_for_statement(stmt));
        let writes = crate::ir_helpers::variable_write_effects_from_commands(
            embedded.all_commands_keeping(|words| {
                words
                    .first()
                    .and_then(crate::ir_helpers::CommandWord::literal)
                    .is_none_or(|head| {
                        !summarised.contains(head) && !self.declared_head_has_no_caller_frame(head)
                    })
            }),
            self.registry,
        );
        embedded_opaque_global |= writes.opaque;
        for d in writes
            .names
            .into_iter()
            .chain(embedded.may_bound_names.iter().cloned())
        {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        // A `VarRead`-role word inside a substitution observes its target
        // without writing it (`set e [info exists x]`), so it is never among
        // the write effects. Same rule as the condition path (#2132).
        let mut reads = writes.read_names;
        let role_reads = crate::ir_helpers::variable_read_effects_from_commands(
            embedded.all_commands(),
            self.registry,
        );
        // As above, deliberately *not* folded into `opaque`: an unnameable **read**
        // (`[info exists $p]`) observes a cell we cannot name, which is a
        // precision loss, not a claim that anything is written. The
        // `opaque_global` barrier means "this may write any name anywhere",
        // and asserting that for a read made an `[info exists Params($k)]`
        // guard stop folding. A dynamic read's effect on dead-store
        // elimination is already owned by `dynamic_names.reads`, which
        // abstains for the whole function.
        for r in role_reads.names {
            if !reads.contains(&r) {
                reads.push(r);
            }
        }
        let beside = self.read_beside_writes(
            &embedded_extras,
            self.original_tokens_for_statement(stmt),
            |scanner| {
                crate::ir_helpers::statement_substituted_reads(
                    stmt,
                    &embedded,
                    scanner,
                    self.registry,
                )
            },
        );
        for name in beside.into_iter().chain(self.protected_writes(&embedded)) {
            if !reads.contains(&name) {
                reads.push(name);
            }
        }
        EmbeddedSubstExtras {
            defs: embedded_extras,
            reads,
            opaque_global: embedded_opaque_global,
            unseen: self.embedded_reaches_unseen(stmt),
        }
    }

    /// The places a protected script among `embedded` writes — a `catch` or
    /// `try` body inside a substitution — each of which the statement reads
    /// as well as defines. The script stops at its first error and the
    /// `catch` or `try` absorbs it, so a place written only after a command
    /// that may fail keeps what it held, and the store before the statement
    /// is what a later read sees there: `set c old; set r [catch {lassign {x
    /// y z} a b c} m]` leaves `c` old where `b` is an array (tclsh 8.5 to
    /// 9.1). Read by name, like a read beside a write, of the version before
    /// it.
    fn protected_writes(
        &self,
        embedded: &crate::ir_helpers::EvaluatedCommandSubstitutions,
    ) -> Vec<String> {
        let mut names = crate::ir_helpers::variable_write_effects_from_commands(
            embedded.protected_commands.iter(),
            self.registry,
        )
        .names;
        for name in &embedded.may_bound_names {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }

    /// Protect incoming values read around a substitution write.
    /// Original roles and lexical reads contribute MAY protection; missing
    /// ownership keeps every written name's incoming version live.
    pub(super) fn read_beside_writes(
        &self,
        written: &[String],
        tokens: Option<&CommandTokens>,
        host_reads: impl FnOnce(&mut VarReferenceScanner) -> BTreeSet<String>,
    ) -> Vec<String> {
        if written.is_empty() {
            return Vec::new();
        }
        let mut scanner = VarReferenceScanner::with_config(
            VarScanOptions {
                include_var_read_roles: false,
                recurse_cmd_substitutions: true,
                include_reads_before_write: false,
                element_qualified: true,
            },
            self.config,
        );
        let base = |name: &str| {
            crate::variable_observation_names::module_observation_key(
                tcl_syntax::naming::normalise_var_name_braced(name, true),
            )
            .to_owned()
        };
        let mut read: BTreeSet<String> = host_reads(&mut scanner)
            .iter()
            .map(|name| base(name))
            .collect();
        let Some(children) = tokens.and_then(|tokens| {
            self.procedure_summary_owners
                .children(tokens, self.registry)
        }) else {
            return written.to_vec();
        };
        for child in &children {
            let Some(binding) = child.source_binding.as_ref() else {
                return written.to_vec();
            };
            let metadata = match self.procedure_summary_owners.module(self.registry) {
                Some(module) => {
                    binding.original_invocation_metadata_for_module(child, module, self.registry)
                }
                None => binding
                    .original_source_metadata_for_advice(child, self.registry)
                    .ok()
                    .flatten(),
            };
            let Some(metadata) = metadata else {
                return written.to_vec();
            };
            let Some(roles) =
                crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                    self.registry,
                    Some(metadata),
                    child,
                )
            else {
                return written.to_vec();
            };
            if roles.unknown_residual()
                || roles
                    .unresolved_roles()
                    .any(|role| role == tcl_registry::ArgRole::VarRead)
            {
                return written.to_vec();
            }
            for (role, word) in roles.operands() {
                if role != tcl_registry::ArgRole::VarRead {
                    continue;
                }
                match word {
                    crate::registry_invocation::EffectiveInvocationWord::Literal(name) => {
                        read.insert(base(name));
                    }
                    crate::registry_invocation::EffectiveInvocationWord::ArrayElementName {
                        root,
                    } => {
                        read.insert(base(root));
                    }
                    _ => return written.to_vec(),
                }
            }
        }
        written
            .iter()
            .filter(|name| read.contains(&base(name)))
            .cloned()
            .collect()
    }

    /// Condition-position effects combine registry variable roles, resolved
    /// procedure summaries, and whether a command the condition's
    /// substitutions run, in source order, is one the module cannot see.
    pub(super) fn condition_out_vars(&self, condition: &ExprNode, span: Span) -> ConditionEffects {
        let bindings = self
            .source_binding_timeline
            .as_ref()
            .and_then(|timeline| timeline.before_substitutions(span))
            .cloned()
            .unwrap_or_else(|| self.command_bindings.clone());
        let metadata = self
            .command_classes
            .metadata_context()
            .and_then(crate::registry_invocation::InvocationMetadataSelection::context);
        let writes = crate::ir_helpers::expression_possible_writes_with_metadata_context(
            condition,
            self.registry,
            &bindings,
            &self.invocation_namespace,
            metadata,
            self.config,
        );
        let mut defs = writes.names;
        let state = std::cell::RefCell::new(bindings);
        let unseen = std::cell::Cell::new(false);
        let registry_barrier = std::cell::Cell::new(false);
        let resolve = |head: &str| {
            state
                .borrow()
                .resolved_embedded_head(head, &self.invocation_namespace)
        };
        let observe = |words: &[crate::ir_helpers::CommandWord], conditional: bool| {
            let found = state
                .borrow_mut()
                .source_order_registry_barrier_for_command_with_metadata_context(
                    words,
                    conditional,
                    self.registry,
                    &self.invocation_namespace,
                    REGISTRY_BARRIER_TRAITS,
                    metadata,
                );
            registry_barrier.set(registry_barrier.get() || found);
            self.observe_in_source_order(&state, &unseen, words, conditional);
        };
        let embedded =
            crate::ir_helpers::expression_command_substitutions_with_replay_and_metadata_context(
                condition,
                self.registry,
                Some(&resolve),
                Some(&observe),
                metadata,
                self.config,
            );
        let upvar =
            self.upvar_effects_from_commands(&embedded.commands, self.original_tokens_at(span));
        let opaque_upvar = upvar.opaque_arguments;
        for name in upvar.defs {
            if !defs.contains(&name) {
                defs.push(name);
            }
        }
        let (global_defs, mut opaque_global) =
            self.global_write_defs_from_commands(&embedded.commands, self.original_tokens_at(span));
        opaque_global |= embedded.opaque || opaque_upvar;
        opaque_global |= writes.opaque;
        for name in global_defs
            .into_iter()
            .chain(embedded.may_bound_names.iter().cloned())
        {
            if !defs.contains(&name) {
                defs.push(name);
            }
        }
        // A condition's `[incr n]` observes `n` before overwriting it, exactly
        // as the same substitution does in an argument word. Recording only
        // the write made the store feeding the condition look
        // overwritten-before-read, and O109 deleted it: tclsh 9.0.4 prints `6`
        // for `proc p {} {set n 5; if {[incr n]} {puts $n}}` and the optimised
        // program printed `1` (#2132).
        // An `[info exists n]` / `[array size a]` in the condition reads its
        // target without writing it at all, so the read never appears among
        // the write effects above. `proc p {} {set x 1; if {[info exists x]}
        // {puts yes}}` prints `yes` on tclsh 9.0.4; without this the store was
        // removed as unused and the program printed nothing.
        //
        // Deliberately *not* folded into `opaque_global`: an unnameable
        // **read** (`[info exists $p]`) observes a cell we cannot name, which
        // is a precision loss, not a claim that anything is written. A
        // dynamic read's effect on dead-store elimination is already owned by
        // `dynamic_names.reads`, which abstains for the whole function.
        // A condition's `[info exists a(y)]` observes the element it names,
        // not every element `a` holds (#2268).
        let mut reads = writes.exact_read_names;
        let beside = self.read_beside_writes(&defs, self.original_tokens_at(span), |scanner| {
            crate::ir_helpers::condition_substituted_reads(
                condition,
                &embedded,
                scanner,
                self.registry,
            )
        });
        for name in beside.into_iter().chain(self.protected_writes(&embedded)) {
            if !reads.contains(&name) {
                reads.push(name);
            }
        }
        ConditionEffects {
            defs,
            reads,
            opaque_global,
            unseen: unseen.get(),
            registry_barrier: embedded.opaque || registry_barrier.get(),
        }
    }

    /// Push the synthetic condition effects and analysis-only barriers.
    pub(super) fn push_condition_effects(&mut self, condition: &ExprNode, span: Span, block: &str) {
        let ConditionEffects {
            defs,
            reads,
            opaque_global,
            unseen,
            registry_barrier,
        } = self.condition_out_vars(condition, span);
        if unseen {
            self.block_mut(block)
                .statements
                .push(Statement::unseen_call_marker(span));
        }
        if !defs.is_empty() || !reads.is_empty() {
            self.push_statement(
                block,
                Statement::Call {
                    span,
                    command: "<cond>".into(),
                    canonical_command: None,
                    args: Vec::new(),
                    defs,
                    reads,
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: Some(crate::ir::CommandTokens::marker(
                        crate::ir::SyntheticMarker::Condition,
                    )),
                    foreach_groups: None,
                },
            );
        }
        if opaque_global {
            self.push_statement(
                block,
                Statement::Barrier {
                    span,
                    reason: "condition runs an unreadable script at the global frame".into(),
                    command: "<global-frame-script>".into(),
                    canonical_command: None,
                    args: Vec::new(),
                    tokens: Some(crate::ir::CommandTokens::marker(
                        crate::ir::SyntheticMarker::GlobalFrameScript,
                    )),
                },
            );
        }
        if registry_barrier {
            self.block_mut(block)
                .statements
                .push(Self::registry_barrier_statement_at(
                    span,
                    "condition reaches a registry-declared evaluation barrier",
                ));
        }
    }

    /// Materialise the variable effects of substitutions executed by a
    /// control statement before its terminator or dispatch is installed.
    pub(super) fn push_embedded_control_effects(
        &mut self,
        stmt: &Statement,
        current: &str,
        opaque_reason: &str,
    ) {
        self.record_caller_frame_barrier(stmt);
        self.record_alias_observed(stmt);
        let EmbeddedSubstExtras {
            defs: extras,
            reads: extra_reads,
            opaque_global: opaque,
            unseen,
        } = self.embedded_subst_extras(stmt);

        if unseen {
            self.block_mut(current)
                .statements
                .push(Statement::unseen_call_marker(stmt.span()));
        }
        if opaque {
            self.push_statement(
                current,
                Statement::Barrier {
                    span: stmt.span(),
                    reason: opaque_reason.to_owned(),
                    command: "<global-frame-script>".to_owned(),
                    canonical_command: None,
                    args: Vec::new(),
                    tokens: Some(crate::ir::CommandTokens::marker(
                        crate::ir::SyntheticMarker::GlobalFrameScript,
                    )),
                },
            );
        }
        if !extras.is_empty() || !extra_reads.is_empty() {
            self.block_mut(current)
                .statements
                .push(Statement::word_effects(
                    stmt.span(),
                    (extras, extra_reads),
                    crate::ir::WordEffectsHost::Dispatch,
                ));
        }
    }
}
