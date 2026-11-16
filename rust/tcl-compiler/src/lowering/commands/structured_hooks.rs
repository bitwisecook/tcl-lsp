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

//! Dispatching a structured lowering hook: argv-shape commitment, expansion barriers and the consumed command binding.

use super::super::{Lowerer, seg_word_is_static_literal};
use crate::ir::{CommandBindingSite, CommandTokens, Statement};
use crate::segmenter::SegmentedCommand;
use tcl_registry::hooks::LoweringHookId;

/// Recursive dispatch retains only the selected contract, never the registry's
/// full materialization frame while lowering another body.
struct StructuredHookSelection<'a> {
    hook: LoweringHookId,
    requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'a>,
    error_context: Option<tcl_registry::InlineBodyErrorContext>,
}

impl Lowerer<'_> {
    /// Whether a resolved lowering hook commits to a **positional argv
    /// shape** — its lowerer reads "the body is word N", "the loop variables
    /// are word 0", "the result variable is word 1" — so a `{*}` expansion
    /// among the words makes the shape unknowable and the call must barrier
    /// instead.
    ///
    /// Registry-derived by construction: the answer is keyed on the typed
    /// [`LoweringHookId`] that [`Self::try_dispatch_structured_hook`] already
    /// dispatches on, not on a command-name list: a name list drifts against
    /// the structured hook IDs, letting a form such as `lmap`, `dict for`,
    /// `array for`, `foreachLine`, `catch`, `try`, `eval`, `uplevel` or
    /// `apply` commit to its un-expanded argv.  The match is exhaustive with
    /// no wildcard arm, so a new hook ID cannot be added without classifying
    /// it here.
    ///
    /// The `false` arm is the non-structured group: those hooks run in
    /// [`try_lower_hook`] before this dispatcher is reached, and each already
    /// declines an expanded call itself via `has_expansion`.
    fn hook_commits_to_argv_shape(hook: LoweringHookId) -> bool {
        match hook {
            LoweringHookId::Proc
            | LoweringHookId::When
            | LoweringHookId::NamespaceEval
            | LoweringHookId::If
            | LoweringHookId::Switch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine
            | LoweringHookId::Catch
            | LoweringHookId::Try
            | LoweringHookId::Dict
            | LoweringHookId::Eval
            | LoweringHookId::Uplevel
            | LoweringHookId::Apply
            | LoweringHookId::ArrayFor => true,
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar => false,
        }
    }

    /// A proved builtin whose lowering commits to its argv shape keeps the
    /// opaque runtime barrier when a `{*}` word leaves no structured
    /// selection: the expanded argv decides what the command runs.
    pub(super) fn expanded_structured_barrier(
        &self,
        seg: &SegmentedCommand,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        if self.target.is_trace_visible()
            || !seg
                .expand_word
                .as_ref()
                .is_some_and(|expanded| expanded.iter().any(|&e| e))
        {
            return None;
        }
        let target = tokens
            .source_binding
            .as_ref()?
            .proved_execution_target()
            .filter(|target| target.registry_backed && target.prepended.is_empty())?;
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let hook = self
            .registry
            .resolve_call(&target.command, &args, self.registry.own_surface_query())?
            .lowering_hook?;
        Self::hook_commits_to_argv_shape(hook)
            .then(|| Box::new(self.structured_expand_barrier(seg.name(), seg.args(), seg)))
    }

    /// `{*}` expansion on a structured command lowers to a barrier so
    /// downstream analyses can't reason about the expanded form.
    fn structured_expand_barrier(
        &self,
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
    ) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: format!("{cmd_name} with argument expansion"),
            command: cmd_name.into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// Try to dispatch *`cmd_name`* through the registry's typed
    /// [`LoweringHookId`] for structured commands dispatched from
    /// [`lower_command`](Self::lower_command).  Returns
    /// `Some(stmt)` when the hook ID matches a structured form;
    /// returns `None` otherwise so `lower_command` falls back to
    /// [`lower_default`](Self::lower_default).
    ///
    /// Every structured form routes through
    /// `resolve_call().lowering_hook`, and unmatched commands flow
    /// straight to `lower_default`.  Forms with shape preconditions
    /// (`proc`, `namespace eval`, `foreach`, `lmap`, `dict`,
    /// `when`, `foreachLine`) return `None` on a failed
    /// precondition so `lower_default` catches them.
    ///
    /// Dispatching through the hook ID picks up two benefits over a
    /// bare name match:
    ///
    /// 1. Canonical resolution via [`CommandRegistry::resolve_call`]
    ///    instead of bare name comparison, so any future spec
    ///    that aliases an existing form (e.g. an iRules-specific
    ///    `eval` variant) automatically dispatches correctly
    ///    when its `lowering_hook` is stamped.
    /// 2. The hook ID is the canonical key consumed by downstream
    ///    audit / LSP / compiler-explorer surfaces.
    // Registry-hook dispatcher keeps uniform fallback policy in one place.
    #[allow(clippy::too_many_lines)]
    pub(in super::super) fn try_dispatch_structured_hook(
        &mut self,
        cmd_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Box<Statement>> {
        let args = seg.args();
        // Release availability is selected before recursive body lowering.
        let tokens = self.cmd_tokens_boxed(seg);
        if self.target.is_bytecode()
            && self.native_entry.is_some()
            && tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_structured_compilation(&tokens))
                .is_some_and(|preparation| {
                    matches!(
                        preparation.recipe(),
                        tcl_registry::native_instruction_plan::NativeInstructionPlan::Control(_)
                    )
                })
        {
            // Under a live native entry the selected compiler owns expression
            // preparation and body visits: emission recomputes its plan from
            // the call's original vector, so the call is kept. A unit with no
            // native entry has no such emission, and its analyses (escape
            // summary, inliners, optimiser) need the structured statement.
            return Some(self.lower_default_boxed(seg, namespace));
        }
        let admitted = self.target.is_bytecode().then(|| {
            crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                &tokens,
            )
        });
        let logical =
            (!self.target.is_bytecode()).then(|| self.logical_structured_invocation(&tokens));
        let resolved = if let Some(admitted) = &admitted {
            let admitted = admitted.as_ref()?;
            StructuredHookSelection {
                hook: admitted.lowering_hook()?,
                requirement: crate::registry_invocation::SelectedLoweringBindingIdentity::Native(
                    admitted,
                ),
                error_context: admitted.compiler_operand_error_context(),
            }
        } else {
            let logical = logical.as_ref()?.as_ref()?;
            StructuredHookSelection {
                hook: logical.lowering_hook()?,
                requirement: logical.lowering_identity(),
                error_context: logical.error_context(),
            }
        };
        let hook = resolved.hook;
        // The expansion gate lives here, keyed on the same typed hook the
        // dispatch below uses, so it can never name a different set of
        // commands than the lowerers it protects.
        if Self::hook_commits_to_argv_shape(hook)
            && seg
                .expand_word
                .as_ref()
                .is_some_and(|ew| ew.iter().any(|&e| e))
        {
            return Some(self.box_lowered_statement(|this| {
                this.structured_expand_barrier(cmd_name, args, seg)
            }));
        }
        self.with_original_body_parent(tokens, |this| {
            let inline_body_error_context = resolved.error_context;
            let statement = match hook {
                // Static-body uplevel.  Match `uplevel 1 {body}`,
                // `uplevel #0 {body}`, and the canonical no-level form
                // `uplevel {body}` (level defaults to 1) when the body
                // arg is a brace-string literal token.  Dynamic forms
                // (`uplevel 1 $body` / `uplevel $lvl {body}`) fall
                // through to the default lowering so a runtime
                // `Call` / `Barrier` carries the unresolved arguments.
                LoweringHookId::Uplevel => this.box_lowered_statement(|this| {
                    this.try_lower_uplevel_static(seg, namespace)
                        .unwrap_or_else(|| this.lower_default(seg, namespace))
                }),

                // `eval $body` / `eval {body}` with a literal /
                // const-folded body relaxes to a `Statement::Block` so
                // downstream analyses see the inlined script.  Dynamic
                // bodies (`eval $dyn` with no const-map binding,
                // `eval [cmd]`) fall through to the default barrier
                // dispatch.
                LoweringHookId::Eval => this.box_lowered_statement(|this| {
                    this.lower_eval_structured(
                        seg,
                        namespace,
                        inline_body_error_context,
                        resolved.requirement,
                    )
                }),

                // `apply {{params} body ?ns?} …` — walk the braced body so nested
                // definitions register (like `namespace eval`), keeping the call a
                // runtime barrier because the body runs in a separate frame.
                LoweringHookId::Apply => this.box_lowered_statement(|this| this.lower_apply(seg)),

                // `array for {k v} arr body` (Tcl 9.0) — iterate array entries.
                // Like `apply`, the call stays a runtime barrier (C Tcl invokes
                // `::tcl::array::for` with the body as an unparsed literal), but a
                // braced literal body is walked in a fresh frame bound to the two
                // loop variables so it is analysable.
                LoweringHookId::ArrayFor => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_array_for(seg, namespace))
                }

                // Straightforward control-flow forms.  Each is a
                // single-method dispatch with no arity / shared-method /
                // subcommand complications.
                LoweringHookId::If => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_if(seg, namespace))
                }
                LoweringHookId::Switch => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_switch(seg, namespace))
                }
                LoweringHookId::For => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_for(seg, namespace))
                }
                LoweringHookId::While => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_while(seg, namespace))
                }
                // A count the release's `catch` refuses is its `wrong # args`, which
                // the command raises before it runs the body.
                LoweringHookId::Catch => {
                    if !this
                        .release_admits_arity(resolved.requirement.binding_identity(), args.len())
                    {
                        return None;
                    }
                    this.box_lowered_statement(|this| {
                        this.lower_catch_with_binding(seg, namespace, resolved.requirement)
                    })
                }
                LoweringHookId::Try => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_try(seg, namespace))
                }

                // Forms with shape preconditions.  Calls with the wrong
                // arity / shape must fall through to `lower_default`
                // instead of crashing the dedicated lowerer, so each arm
                // returns `None` on precondition failure and
                // `lower_command` then falls through to `lower_default`.
                //
                // `proc name params body` — exactly three args and
                // at least three token slices (the body needs to
                // be a real token, not synthesised whitespace).
                LoweringHookId::Proc => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_proc_declaration(
                            seg,
                            namespace,
                            resolved.requirement.descriptor_key().label(),
                        )
                    });
                }
                // `namespace eval ns body` — the subcommand match
                // is already handled by `resolve_call`, so the
                // hook only fires for the `eval` subcommand.  Keep
                // the arity / token-count guards (a tokenless body
                // would derail body lowering).
                LoweringHookId::NamespaceEval => {
                    if args.len() >= 3 && seg.arg_tokens().len() >= 3 {
                        this.box_lowered_statement(|this| this.lower_namespace_eval(seg, namespace))
                    } else {
                        return None;
                    }
                }
                // `foreach vars list body` / `lmap vars list body`
                // — share `lower_foreach(... is_lmap)`.  The
                // dedicated lowerer handles its own shape errors,
                // so no precondition here.
                LoweringHookId::Foreach => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_foreach(seg, namespace, false))
                }
                LoweringHookId::Lmap => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_foreach(seg, namespace, true))
                }
                // `dict <subcommand> ...` — must have at least one
                // arg so the subcommand can be picked.  Bare `dict`
                // falls through to `lower_default`.
                LoweringHookId::Dict => {
                    if args.is_empty() {
                        return None;
                    }
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_dict(seg, namespace))
                }

                // `when EVENT ?priority N? body` — iRules event
                // handler.  The hook stamp lives on the `when` spec
                // in `tcl-registry/src/commands/irules/when.rs`,
                // which `load_dialect(SpecSurface::IRULES)` brings
                // into the registry.  Production callers (the LSP
                // server) load the active dialect
                // before lowering; tests that lower iRule code call
                // `registry.load_irules()` explicitly (see
                // `irules_checks::tests::registry`).  Callers that
                // lower iRule code against a vanilla `build_default()`
                // registry now silently flow through to
                // `lower_default` — that path was always a
                // misconfiguration; the dialect needs to match the
                // source.
                LoweringHookId::When => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_when_declaration(
                            seg,
                            namespace,
                            resolved.requirement.descriptor_key().label(),
                        )
                    });
                }
                // `foreachLine varName filename body` — Tcl 9.0
                // (TIP 670).  Always registered in `build_default()`
                // via `tcl::foreachline::spec`, so no dialect-load
                // dance is needed.  The `lower_foreach_line`
                // emitter handles its own shape errors and
                // dynamic-body fallback, but the `args.len() == 3`
                // guard keeps an under- or over-argued `foreachLine`
                // flowing to `lower_default` instead of triggering a
                // barrier inside the dedicated emitter.
                LoweringHookId::ForeachLine => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_foreach_line_structured(seg, namespace, resolved.requirement)
                    });
                }

                // Non-structured hooks (`Expr` / `Return` / `Set` /
                // `Incr` / `AppendOrLappend` / `Unset` / `Global` /
                // `Variable` / `Upvar`) are handled by
                // [`try_lower_hook`] before [`lower_command`] reaches
                // this dispatcher.  They can still reach here when
                // their static dispatcher returned `None` for the
                // input shape (e.g. `try_lower_expr` rejects
                // multi-arg `expr`); fall through to `lower_default`.
                LoweringHookId::Expr
                | LoweringHookId::Return
                | LoweringHookId::Set
                | LoweringHookId::Incr
                | LoweringHookId::AppendOrLappend
                | LoweringHookId::Unset
                | LoweringHookId::Global
                | LoweringHookId::Variable
                | LoweringHookId::Upvar => return None,
            };
            Some(statement)
        })
    }

    /// Keep the genuine parent vector through all structured body callbacks,
    /// restoring it even when a hook declines before producing a statement.
    fn with_original_body_parent<T>(
        &mut self,
        parent: Box<CommandTokens>,
        lower: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let enclosing = self.structured_source_parent.replace(parent);
        let result = lower(self);
        self.structured_source_parent = enclosing;
        result
    }

    /// Materialise statement return storage outside the recursive dispatcher.
    /// Retaining one inline return slot per hook arm otherwise grows every
    /// nested structured lowering frame as the IR's statement variants grow.
    #[inline(never)]
    fn box_lowered_statement(
        &mut self,
        lower: impl FnOnce(&mut Self) -> Statement,
    ) -> Box<Statement> {
        Box::new(lower(self))
    }

    #[inline(never)]
    fn box_optional_lowered_statement(
        &mut self,
        lower: impl FnOnce(&mut Self) -> Option<Statement>,
    ) -> Option<Box<Statement>> {
        lower(self).map(Box::new)
    }

    /// Retain the resolved head of a command whose typed lowering consumes its
    /// live command semantics. This is recorded before lowering nested bodies,
    /// while the enclosing script's site scope is current.
    pub(in super::super) fn record_consumed_command_binding(
        &mut self,
        seg: &SegmentedCommand,
        resolution_namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) {
        let source_tokens = Some(Box::new(self.cmd_tokens(seg)));
        let name = crate::registry_invocation::static_command_word(
            source_tokens.as_deref().expect("retained invocation"),
            self.config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
        .unwrap_or_else(|| seg.name().to_owned());
        let guard = crate::registry_invocation::command_binding_guard(
            source_tokens.as_deref().expect("retained invocation"),
        );
        let namespace_context = crate::registry_invocation::compiled_namespace_context(
            source_tokens.as_deref().expect("retained invocation"),
        );
        let shape = self.invocation_shape(seg);
        let Some(sites) = self.command_binding_site_stack.last_mut() else {
            return;
        };
        sites.push(CommandBindingSite {
            span: seg.span,
            source_tokens,
            known_namespaces: self
                .source_bindings
                .as_ref()
                .map(|bindings| bindings.known_namespaces_at(seg.span.start())),
            variable_frame: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .variable_frame
            }),
            variable_context: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .variable_context
            }),
            existing_namespace_cells: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .existing_namespace_cells
            }),
            binding: tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                resolution_namespace,
                name,
                requirement.binding_identity(),
            )
            .with_namespace_context(namespace_context)
            .with_guard(guard),
            shape,
        });
    }

    /// Whether the release's signature of `command` admits `count` arguments.
    fn release_admits_arity(&self, command: &str, count: usize) -> bool {
        self.registry.get(command).is_none_or(|spec| {
            self.registry
                .release_arity(spec)
                .accepts(u16::try_from(count).unwrap_or(u16::MAX))
        })
    }

    /// The words after `seg`'s command, as a release's compile reads them.
    pub(super) fn invocation_shape(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::release_compilation::InvocationShape {
        let release = self
            .dialect
            .map_or(tcl_dialect::TclVersion::V9_0, |profile| {
                profile.vm_runtime_version
            });
        let args = seg.texts.get(1..).unwrap_or_default();
        tcl_registry::release_compilation::InvocationShape::of(release, args, |index| {
            seg_word_is_static_literal(seg, index + 1)
        })
    }

    fn lower_catch_with_binding(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Statement {
        let lowered = self.lower_catch(seg, namespace);
        if matches!(lowered, Statement::Catch { .. }) {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        lowered
    }

    fn lower_eval_structured(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        inline_body_error_context: Option<tcl_registry::InlineBodyErrorContext>,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Statement {
        let lowered = self.try_lower_eval_static(seg, namespace, inline_body_error_context);
        if lowered.is_some() {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        lowered.unwrap_or_else(|| self.lower_default(seg, namespace))
    }

    fn try_lower_foreach_line_structured(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Option<Statement> {
        if seg.args().len() != 3 {
            return None;
        }

        let lowered = self.lower_foreach_line(seg, namespace);
        if matches!(lowered, Statement::Foreach { .. }) {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        Some(lowered)
    }
}
