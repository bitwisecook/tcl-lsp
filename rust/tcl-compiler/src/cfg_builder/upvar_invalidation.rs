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

//! Widening a call statement's defs with the caller-frame names its callees write, and the barriers where no per-name list is sound.

use super::{CfgBuilder, EmbeddedSubstExtras, global_write_info};
use crate::ir::{CommandTokens, Statement};

impl CfgBuilder<'_> {
    /// Augment a statement's effective `defs` with caller-side
    /// variable names that any callee proc will modify via `upvar`.
    /// Returns a list of statements — the original (possibly with
    /// merged `defs` for the direct-call form) plus, ahead of it, the
    /// statement's word effects ([`Statement::word_effects`]) when the
    /// embedded-substitution form contributes defs that can't be merged
    /// into the host statement (e.g. an `AssignValue` whose `value` text
    /// contains `[upvar_proc arg]`).
    ///
    /// Direct-call form: resolves the statement through the module command
    /// state, then maps the effective user-procedure target and its
    /// alias-prepended structured arguments through `UpvarInfo` into the
    /// call's own `defs`.
    ///
    /// Embedded-substitution form: scans the call's args / the
    /// `AssignValue`'s value text for `[command_substitution]`
    /// tokens whose head is a known upvar proc; merges those defs
    /// into the host Call when possible, or puts the statement's word
    /// effects, paired with it, ahead of a non-Call host.
    ///
    /// The same two forms also widen `defs` with `global_write_procs`
    /// (a callee that writes an outer-scope name via `global`/`variable`/
    /// `upvar #0`, see [`global_write_info`]) — a global name doesn't
    /// depend on call-site arguments, so no params-based mapping is
    /// needed, just the literal name list.
    pub(super) fn apply_upvar_invalidation(&mut self, stmt: Statement) -> Vec<Statement> {
        self.record_caller_frame_barrier(&stmt);
        self.record_alias_observed(&stmt);
        let original = matches!(
            &stmt,
            Statement::Call {
                foreach_groups: None,
                ..
            }
        )
        .then(|| self.original_tokens_for_statement(&stmt).cloned())
        .flatten();
        let logical_call = matches!(&stmt, Statement::Call { tokens, foreach_groups: None, .. } if tokens.as_ref().is_none_or(|tokens| tokens.synthetic.is_none()))
            && self
                .command_classes
                .metadata
                .source_analysis_input()
                .is_some_and(
                    crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
                );
        let span = stmt.span();
        let mut produced = self.upvar_invalidated(stmt);
        let mut unknown_locations = logical_call;
        if let Some(original) = original {
            for statement in &produced {
                if let Some(receipt) = self.source_call_may_writes(&original, statement) {
                    unknown_locations = !receipt.writes().is_empty();
                    let same_site = self.source_may_calls.entry(receipt.site_key()).or_default();
                    if !same_site.iter().any(|held| held.same_owner(&receipt)) {
                        same_site.push(receipt);
                    }
                }
            }
        }
        if unknown_locations
            && !produced
                .last()
                .is_some_and(crate::ssa::is_unseen_call_marker)
        {
            produced.push(Statement::unseen_call_marker(span));
        }
        produced
    }

    fn source_call_may_writes(
        &self,
        original: &CommandTokens,
        emitted: &Statement,
    ) -> Option<global_write_info::OriginalSourceCallMayWrites> {
        let module = self.procedure_summary_owners.module(self.registry)?;
        let calls = self
            .procedure_summary_owners
            .may_calls(original, self.registry)?;
        if calls.unknown_alternatives {
            return None;
        }
        let mut writes = Vec::new();
        for body in &calls.bodies {
            for place in self
                .global_write_procs
                .get(body)?
                .original_source_writes()?
            {
                if !writes.iter().any(|held: &crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence| held.symbol() == place.symbol()) {
                    writes.push(place.clone());
                }
            }
        }
        global_write_info::OriginalSourceCallMayWrites::for_emission(
            module,
            self.registry,
            original,
            emitted,
            calls.bodies,
            writes,
        )
    }

    /// Record every caller-frame name a callee invoked by `stmt` may touch
    /// through an `upvar` alias or an `uplevel` write, so the dead-store /
    /// unused-assignment passes never delete a store such a callee can
    /// observe (`set callervar 5; get` where `get` runs `upvar 1 callervar
    /// m; return $m`).  The names land on
    /// [`crate::cfg::Function::alias_observed_vars`]; recording them as
    /// *reads* on the call statement instead would fabricate
    /// read-before-set uses (a false W210) for the pure out-param shape.
    pub(super) fn record_alias_observed(&mut self, stmt: &Statement) {
        if self.upvar_procs.is_empty() && !self.global_writers {
            return;
        }
        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        if self.global_writers {
            // A callee that writes an outer-scope name may read it first —
            // `incr ::hits`, `global g; incr g 5` — so a store the caller
            // made to it is observed, not dead: O109 deleted `set hits 0`
            // ahead of a `bump` whose body is `set y [incr ::hits]`
            // (#2214). The summary records writes, not reads, so every
            // name it holds counts as observed.
            let observed: Vec<String> = self
                .direct_global_writes(stmt)
                .into_iter()
                .chain(
                    self.global_write_defs_from_commands(
                        &embedded.commands,
                        self.original_tokens_for_statement(stmt),
                    )
                    .0,
                )
                .collect();
            self.alias_observed_vars.extend(observed);
        }
        if self.upvar_procs.is_empty() {
            return;
        }
        self.alias_observed_vars
            .extend(self.direct_upvar_effects(stmt).defs);
        self.alias_observed_vars.extend(
            self.upvar_effects_from_commands(
                &embedded.commands,
                self.original_tokens_for_statement(stmt),
            )
            .defs,
        );
    }

    pub(super) fn record_caller_frame_barrier(&mut self, stmt: &Statement) {
        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        let mut raised = crate::dynamic_names::DynamicNameBarrier::default();
        if embedded.opaque {
            raised = raised.union(crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT);
        }
        let direct = self.direct_upvar_effects(stmt);
        let embedded = self.upvar_effects_from_commands(
            &embedded.commands,
            self.original_tokens_for_statement(stmt),
        );
        raised = raised
            .union(direct.frame_barrier)
            .union(embedded.frame_barrier);
        if direct.opaque_arguments || embedded.opaque_arguments {
            raised = raised.union(crate::dynamic_names::DynamicNameBarrier {
                writes: true,
                destroys: false,
                reads: false,
            });
        }
        self.caller_frame_barrier = self.caller_frame_barrier.union(raised);
        match &self.barrier_block {
            Some(block) => {
                let own = self.caller_frame_barriers.entry(block.clone()).or_default();
                *own = own.union(raised);
            }
            None => {
                self.caller_frame_barrier_unplaced =
                    self.caller_frame_barrier_unplaced.union(raised);
            }
        }
    }

    /// Run `f` with the caller-frame barriers it records owned by `block`,
    /// the block its statements are pushed into.
    pub(super) fn in_barrier_block<R>(&mut self, block: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let outer = self.barrier_block.replace(block.to_owned());
        let result = f(self);
        self.barrier_block = outer;
        result
    }

    /// The `defs`-widening half of [`Self::apply_upvar_invalidation`], with
    /// no side effect on the function-level barrier — so a speculative query
    /// ([`Self::init_written_names`]) can ask what a statement writes without
    /// recording the statement twice.
    pub(super) fn upvar_invalidated(&self, stmt: Statement) -> Vec<Statement> {
        let call_unseen = self.call_is_unseen(&stmt);
        let substitution_last = self.substitution_runs_last(&stmt);
        let span = stmt.span();
        let (mut out, substitution_unseen) = self.upvar_effect_statements(stmt);
        // An embedded command runs while the host's words are still being
        // evaluated, so its marker goes ahead of the host's own reads; the
        // call's own head runs after its words are, so its marker follows:
        // the words read the versions before it, and a read after the call
        // finds the fresh one the marker leaves
        // ([`crate::ssa::SsaFunction::value_clobbers`]). Where the embedded
        // command is the last thing the host evaluates before it runs, every
        // read the host makes precedes it, and its marker follows the host as
        // a call's own does: the host's writes come after it (#2451).
        if substitution_unseen && !substitution_last {
            out.insert(0, Statement::unseen_call_marker(span));
        }
        if call_unseen || (substitution_unseen && substitution_last) {
            out.push(Statement::unseen_call_marker(span));
        }
        out
    }

    /// Whether every read `stmt` makes precedes the command its last word
    /// substitutes: the earlier words are literal, the last is one `[…]`
    /// holding no further substitution, and the host reads no variable of its
    /// own (`set r [cmd $x]`, `puts [cmd $x]`). The host then reads nothing
    /// that command can change before it runs; only its own writes follow.
    fn substitution_runs_last(&self, stmt: &Statement) -> bool {
        let reads_by_name = match stmt {
            Statement::AssignValue { .. } => false,
            Statement::Call {
                reads,
                reads_own_defs,
                foreach_groups: None,
                ..
            } => *reads_own_defs || !reads.is_empty(),
            _ => return false,
        };
        let Some(tokens) = self.original_tokens_for_statement(stmt) else {
            return false;
        };
        let Some((crate::ir::WordExpr::CommandSubstitution { spelling, .. }, earlier)) =
            tokens.word_exprs.split_last()
        else {
            return false;
        };
        !reads_by_name
            && tokens.expand_word.is_none()
            && tokens.word_exprs.len() == tokens.argv.len()
            && earlier.iter().all(|word| {
                matches!(
                    word,
                    crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
                )
            })
            && spelling
                .strip_prefix('[')
                .is_some_and(|inner| !inner.contains(['[', '\\']))
    }

    /// The statements [`Self::upvar_invalidated`] puts for `stmt` before its
    /// call is marked, and whether a command a `[…]` substitution of it runs is
    /// one the module cannot see.
    fn upvar_effect_statements(&self, mut stmt: Statement) -> (Vec<Statement>, bool) {
        // 1. Direct-call extras: command is a known upvar proc / a proc
        //    that writes outer-scope names.
        let direct_extras = self.direct_call_extras(&stmt);

        // 2. A callee whose caller-frame effect cannot be fully enumerated per
        //    name widens the call site with an opaque barrier after the call
        //    (SCCP/propagation widen every tracked value at a
        //    `Statement::Barrier`). Keep processing the call, though: a
        //    summary may contain both precise caller-side defs and an opaque
        //    remainder, and dropping the known defs loses useful facts such
        //    as `uplevel 1 [list set $parameter value]`.
        let direct_opaque_barrier = self.opaque_call_barrier(&stmt);

        // 3. Embedded-substitution extras: walk text for
        //    `[upvar_proc arg]` / `[global_write_proc arg]` substitutions.
        let EmbeddedSubstExtras {
            defs: embedded_extras,
            reads: embedded_reads,
            opaque_global: embedded_opaque_global,
            unseen: substitution_unseen,
        } = self.embedded_subst_extras(&stmt);

        if direct_extras.is_empty()
            && embedded_extras.is_empty()
            && embedded_reads.is_empty()
            && !embedded_opaque_global
        {
            let out = match direct_opaque_barrier {
                Some(barrier) => vec![stmt, barrier],
                None => vec![stmt],
            };
            return (out, substitution_unseen);
        }

        // 2b. An embedded call to a proc that runs an unreadable script at
        //     the global frame (`set y [setter]` where `setter` does
        //     `uplevel #0 $body`): no def list can enumerate
        //     what it clobbers, so prepend an opaque barrier — the same
        //     program-order position the statement's word effects take,
        //     so the host statement's own reads already see the widened
        //     state.
        let opaque_barrier = embedded_opaque_global.then(|| Statement::Barrier {
            span: stmt.span(),
            reason: "embedded call runs an unreadable script at the global frame".to_owned(),
            command: "<global-frame-script>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::GlobalFrameScript,
            )),
        });

        // 3. Merge into the host statement when it's a Call.
        if let Statement::Call { defs, reads, .. } = &mut stmt {
            for d in direct_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            for d in embedded_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            // `puts [incr n]` writes `n` *and* reads the value it increments.
            // Recording only the write makes the feeding `set n 1` look
            // overwritten-before-read, and O109 deletes it (#2050).
            for r in embedded_reads {
                if !reads.contains(&r) {
                    reads.push(r);
                }
            }
            let mut out = Vec::new();
            if let Some(barrier) = opaque_barrier {
                out.push(barrier);
            }
            out.push(stmt);
            if let Some(barrier) = direct_opaque_barrier {
                out.push(barrier);
            }
            return (out, substitution_unseen);
        }

        // 4. Non-Call host (e.g. AssignValue) with embedded extras — its
        //    word effects go right ahead of it as a definition point paired
        //    with it, so the affected vars are written in program order.
        let mut out = Vec::new();
        if let Some(barrier) = opaque_barrier {
            out.push(barrier);
        }
        if !embedded_extras.is_empty() || !embedded_reads.is_empty() {
            out.push(Statement::word_effects(
                stmt.span(),
                (embedded_extras, embedded_reads),
                crate::ir::WordEffectsHost::Statement,
            ));
        }
        out.push(stmt);
        if let Some(barrier) = direct_opaque_barrier {
            out.push(barrier);
        }
        (out, substitution_unseen)
    }

    /// The opaque widening barrier for a direct call whose callee's
    /// caller-frame effect has no sound per-name def list: a callee whose
    /// `upvar` caller-side name is unresolvable (`upvar 1 $computed x`) can
    /// write ANY caller variable, and a callee that runs an unreadable
    /// script at the global frame (`uplevel #0 $body`) can write or read ANY
    /// global/namespace name.
    fn opaque_call_barrier(&self, stmt: &Statement) -> Option<Statement> {
        let Statement::Call {
            command,
            span,
            tokens,
            ..
        } = stmt
        else {
            return None;
        };
        let literal_head = tokens.as_ref().is_none_or(|tokens| {
            tokens.synthetic.is_none()
                && tokens.words().first().is_none_or(|head| {
                    crate::registry_invocation::invocation_word(head)
                        .literal()
                        .is_some()
                })
        });
        let direct_upvar = self.direct_upvar_effects(stmt);
        let unresolvable_upvar = direct_upvar.has_unresolvable_target;
        let opaque_global = literal_head
            && self
                .source_global_writes(self.original_tokens_for_statement(stmt))
                .1;
        let source_opaque_upvar = direct_upvar.opaque_arguments;
        let opaque_variable_write = self.variable_write_projection(stmt).opaque_variable_frame;
        if !unresolvable_upvar && !source_opaque_upvar && !opaque_global && !opaque_variable_write {
            return None;
        }
        let reason = if unresolvable_upvar || source_opaque_upvar {
            format!("{command} upvar-aliases a dynamic caller variable")
        } else if opaque_global {
            format!("{command} runs an unreadable script at the global frame")
        } else {
            format!("{command} writes a source-opaque variable name")
        };
        // Not a command to run: the call itself is already in the statement
        // stream immediately beside this barrier, so naming the callee here
        // would make codegen invoke it a second time: `proc p {} { upvar 1 {a
        // b} v ; puts "u=$v" }; p` would print `u=…` twice on the VM where
        // tclsh 8.6.14 / 9.0.4 print it once; `proc setter {body} { uplevel #0
        // $body }; setter {set q 1}` failed with `wrong # args` from the
        // re-invoke. The typed `SyntheticMarker` on the tokens is what stops
        // codegen dispatching it, and `reason` keeps the callee's name.
        Some(Self::caller_frame_opaque(*span, reason))
    }

    /// The direct-call half of [`Self::upvar_invalidated`]: the caller-side
    /// names a known upvar proc / global-writing proc defines at this call.
    /// Extras are merged as DEFS only; the "the callee may also *read* the
    /// aliased cell" half is recorded on
    /// [`crate::cfg::Function::alias_observed_vars`] by
    /// [`Self::record_alias_observed`] (a read here would fabricate
    /// read-before-set uses — a false W210 — for the pure out-param shape).
    fn direct_call_extras(&self, stmt: &Statement) -> Vec<String> {
        if !matches!(stmt, Statement::Call { .. }) {
            return Vec::new();
        }
        let mut extras = self.direct_upvar_effects(stmt).defs;
        for name in self.variable_write_projection(stmt).literal_names {
            if !extras.contains(&name) {
                extras.push(name);
            }
        }
        for name in self.direct_global_writes(stmt) {
            if !extras.contains(&name) {
                extras.push(name);
            }
        }
        extras
    }
}
