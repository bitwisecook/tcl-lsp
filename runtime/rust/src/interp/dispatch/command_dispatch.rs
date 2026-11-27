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

//! The central dispatch boundary: lookup, the selected binding, and the `unknown` path for a missing command.

use super::super::limits::NATIVE_EVAL_DEPTH_LIMIT;
#[cfg(debug_assertions)]
use super::super::trace_native_depth_refusal;
use super::super::{
    native_arguments, native_operation_currency, native_script, new_string, release_all, Code,
    CommandGenerationLookup, Interp,
};
use super::{CommandDispatchSelection, PreparedMissingCommand};
use crate::namespace::{CommandBinding, NsId, GLOBAL};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// Look up `argv[0]` and invoke it; on a miss, fall to the `unknown` handler
    /// (auto-load / `package` / friendly errors — the pure-Tcl `unknown` proc),
    /// matching C's `TclEvalObjvInternal`.
    pub(crate) fn dispatch(&mut self, argv: &[*mut TclObj]) -> Code {
        self.dispatch_prebound_with_entry(argv, None, true)
    }

    pub(in super::super) fn dispatch_original_source(
        &mut self,
        argv: &[*mut TclObj],
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        self.dispatch_selection_with_source(
            argv,
            CommandDispatchSelection::Unselected,
            true,
            source,
        )
    }

    /// Internal forwarding retains the current ensemble rewrite until its
    /// target evaluates an ordinary command or bytecode body.
    pub(crate) fn dispatch_invoke(&mut self, argv: &[*mut TclObj]) -> Code {
        self.dispatch_prebound(argv, None)
    }

    /// Forward original words through an explicit command lookup namespace,
    /// retaining the caller's variable frame and the real dispatch boundary.
    pub(crate) fn dispatch_in_lookup_namespace(
        &mut self,
        lookup: NsId,
        argv: &[*mut TclObj],
    ) -> Code {
        self.dispatch_selection_with_entry(
            argv,
            CommandDispatchSelection::LookupAt(
                lookup,
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            ),
            false,
        )
    }

    /// Invoke the exact command token named by a shared-runtime `CommandId`.
    /// IDs retain a generation as well as an FQN, so a recreated same-named
    /// namespace command cannot capture a previously resolved handle.
    pub(crate) fn dispatch_command_id(&mut self, id: u32, argv: &[*mut TclObj]) -> Option<Code> {
        let (_, generation) = self.command_identity(id)?;
        let (fqn, command) = match self.command_by_generation(generation) {
            CommandGenerationLookup::Found { fqn, command } => (fqn, command),
            CommandGenerationLookup::Missing | CommandGenerationLookup::Unavailable => {
                return None;
            }
        };
        let mut full = Vec::with_capacity(argv.len() + 1);
        full.push(new_string(&fqn));
        full.extend_from_slice(argv);
        for &word in &full {
            // SAFETY: the head is fresh and every remaining word is borrowed
            // live from the caller for this dispatch.
            unsafe { obj::incr_ref_count(word) };
        }
        let code = self.dispatch_prebound(
            &full,
            Some((
                fqn,
                CommandBinding {
                    generation,
                    command,
                    jim_table_key: None,
                },
            )),
        );
        release_all(&full);
        Some(code)
    }

    /// The central dispatch boundary, optionally with a command binding whose
    /// table is not visible to normal namespace lookup (`interp invokehidden`).
    /// Pre-resolved bindings still pass through command counting and execution
    /// traces; their stored generation selects the exact trace sidecar.
    pub(in super::super) fn dispatch_prebound(
        &mut self,
        argv: &[*mut TclObj],
        prebound: Option<(Vec<u8>, CommandBinding)>,
    ) -> Code {
        self.dispatch_prebound_with_entry(argv, prebound, false)
    }

    fn dispatch_prebound_with_entry(
        &mut self,
        argv: &[*mut TclObj],
        prebound: Option<(Vec<u8>, CommandBinding)>,
        ordinary: bool,
    ) -> Code {
        self.dispatch_selection_with_entry(argv, prebound.into(), ordinary)
    }

    pub(super) fn dispatch_selection_with_entry(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
    ) -> Code {
        self.dispatch_selection_with_source(argv, prebound, ordinary, None)
    }

    fn dispatch_selection_with_source(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return Code::Error;
            }
        };
        let code = (|| {
            let previous = self.native_dispatch_depth.get();
            let depth = previous.saturating_add(1);
            // Unknown handlers and other forwarding entries add real recursive
            // dispatch frames between script-body activations. Bound this shared
            // entry before lookup/rewrite effects, using the existing native-stack
            // budget independently of the body evaluator's depth.
            if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
                #[cfg(debug_assertions)]
                trace_native_depth_refusal("command-dispatch", self.eval_depth.get(), previous);
                return self.error(b"too many nested evaluations (infinite loop?)");
            }
            if ordinary {
                let reset = self.reset_native_ensemble_rewrite(
                    tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
                );
                if scope.currency().ensure_current_or_refuse().is_err() {
                    return Code::Error;
                }
                if let Err(code) = reset {
                    return code;
                }
            }
            self.native_dispatch_depth.set(depth);
            let code = self.dispatch_prebound_inner(argv, prebound, ordinary, source);
            let code = if self.check_entered_native_operation().is_err() {
                Code::Error
            } else {
                self.drain_tailcalls(depth, code)
            };
            self.native_dispatch_depth.set(previous);
            code
        })();
        if scope.currency().ensure_current_or_refuse().is_err() {
            Code::Error
        } else {
            code
        }
    }

    pub(super) fn dispatch_prebound_inner(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        if !self.evaluation_is_live() {
            return Code::Error;
        }
        // C resets before lookup and traces. Jim's selected worker resets only
        // after retaining the actual command; its Script entry owns a separate
        // source reset. The caller independently owns the original argv.
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            self.set_result_bytes(b"");
        }
        if self.check_entered_native_operation().is_err() {
            return Code::Error;
        }
        self.reset_native_global_error_episode();
        if self.check_entered_native_operation().is_err() {
            return Code::Error;
        }
        self.cmd_count.set(self.cmd_count.get() + 1);
        // An embedder's limits are charged here, where every command passes:
        // past one, the command fails instead of running.
        if let Some(code) = self.charge_dispatch() {
            return code;
        }
        // Fast path: nothing is registered, so nothing can fire. Being inside a
        // trace callback is *not* a reason to skip: C's
        // `TclCheckExecutionTraces` never consults `INTERP_TRACE_IN_PROGRESS`,
        // so a command dispatched from a callback still fires its own
        // `enter`/`leave` traces. Only the step machinery is gated, in
        // `dispatch_traced`.
        let traced = !self.traces.borrow().cmd_traces.is_empty();
        // A command's result is charged against the value-size limit once it
        // is built (`charge_result`), so no value over the limit outlives the
        // command that built it.
        let code = if !traced {
            match prebound {
                CommandDispatchSelection::Bound(_, binding) => {
                    if ordinary {
                        if let Err(code) = self.reset_native_ensemble_rewrite(
                            tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                        ) { return code; }
                    }
                    self.invoke_bound_with_source(
                        binding.command,
                        Some(binding.generation),
                        argv,
                        None,
                        source,
                    )
                }
                CommandDispatchSelection::Unselected => self.dispatch_inner_at_with_source(
                    argv,
                    ordinary,
                    self.current_ns.get(),
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                    source,
                ),
                CommandDispatchSelection::LookupAt(lookup, origin) => {
                    self.dispatch_inner_at_with_source(argv, ordinary, lookup, origin, source)
                }
                CommandDispatchSelection::Missing {
                    lookup,
                    caller,
                    origin,
                } => self.dispatch_missing_command(argv, lookup, caller, origin),
            }
        } else {
            self.dispatch_traced(argv, prebound, ordinary, source)
        };
        self.charge_result(code)
    }

    pub(super) fn dispatch_inner_at_with_source(
        &mut self,
        argv: &[*mut TclObj],
        ordinary: bool,
        lookup: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        // The availability gate lives in `resolve_dispatchable`, so a builtin
        // the emulated release does not carry misses here and falls through to
        // the `unknown` machinery below like any other unresolved name.
        match self.resolve_original_command_at(lookup, argv[0]) {
            Ok(Some((cmd, generation))) => {
                if ordinary {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                return self.invoke_bound_with_source(cmd, generation, argv, None, source);
            }
            Ok(None) => {}
            Err(error) => return self.report_cmd_error(error.into()),
        }
        self.dispatch_missing_command(
            argv,
            lookup,
            self.current_ns.get(),
            if origin != tcl_registry::command_lookup::CommandLookupOrigin::Ordinary {
                origin
            } else if ordinary {
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary
            } else {
                tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation
            },
        )
    }

    /// Return the original stored root, installing the native global default
    /// lazily. Namespace storage owns the root independently from the result.
    pub(crate) fn namespace_unknown_root(
        &mut self,
        namespace: NsId,
        default: bool,
    ) -> Option<obj::Owned> {
        let stored = self.namespaces.borrow().unknown_handler(namespace);
        if let Some(stored) = stored {
            return Some(obj::Owned::retain(stored));
        }
        if !default {
            return None;
        }
        let root = obj::Owned::fresh(new_string(b"::unknown"));
        let retired = self
            .namespaces
            .borrow_mut()
            .set_unknown_handler(namespace, Some(root.clone()));
        drop(retired);
        Some(root)
    }

    pub(in super::super) fn retire_namespace_unknown_root(&mut self, namespace: NsId) {
        let retired = self
            .namespaces
            .borrow_mut()
            .set_unknown_handler(namespace, None);
        drop(retired);
    }

    fn prepare_missing_command(
        &mut self,
        argv: &[*mut TclObj],
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Result<Box<PreparedMissingCommand>, Code> {
        use tcl_registry::command_lookup::{
            native_lookup_fallback_policy, UnknownHandlerNamespace,
        };
        use tcl_syntax::value::ValueOps;
        let Some(policy) = native_lookup_fallback_policy(self.native_invocation_dialect(), origin)
        else {
            return Err(self.invalid_original_command(argv[0]));
        };
        let (owned, selected) = if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let context = match self.native_jim_object_context() {
                Ok(context) => context,
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            let head = match crate::native_source::NativeJimObjectContext::unknown_object(&context)
            {
                Ok(original) => original.as_ptr(),
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            let _allocation = obj::NativeObjectLifetime::retain(head);
            let selected = match self.resolve_original_command_at(lookup, head) {
                Ok(Some(selected)) => selected,
                Ok(None) => return Err(Code::Error),
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            let owned = std::iter::once(&head)
                .chain(argv)
                .map(|original| obj::Owned::retain(*original))
                .collect::<Vec<_>>();
            (owned, selected)
        } else {
            let selected = match policy.namespace_handler {
                Some(UnknownHandlerNamespace::Caller) => caller,
                Some(UnknownHandlerNamespace::Lookup) => lookup,
                None => GLOBAL,
            };
            let root = if policy.namespace_handler.is_some() {
                self.namespace_unknown_root(selected, selected == GLOBAL)
                    .or_else(|| self.namespace_unknown_root(GLOBAL, true))
                    .expect("the native global default is installed")
            } else {
                obj::Owned::fresh(new_string(policy.default_handler.as_bytes()))
            };
            let prefix = match self.list_elements(&root.as_ptr()) {
                Ok(prefix) => prefix,
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            let owned: Vec<_> = prefix
                .iter()
                .chain(argv)
                .map(|word| obj::Owned::retain(*word))
                .collect();
            drop(root);
            let Some(head) = owned.first() else {
                return Err(self.invalid_original_command(argv[0]));
            };
            let selected = match self.resolve_original_command_at(lookup, head.as_ptr()) {
                Ok(Some(selected)) => selected,
                Ok(None) => return Err(self.invalid_original_command(argv[0])),
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            (owned, selected)
        };
        self.reset_native_ensemble_rewrite(
            tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
        )?;
        let fqn = selected.1.and_then(|generation| {
            self.namespaces
                .borrow()
                .native_command_at_node(generation)
                .map(|(_, fqn)| fqn)
        });
        let words = owned.iter().map(obj::Owned::as_ptr).collect();
        Ok(Box::new(PreparedMissingCommand {
            owned,
            words,
            selected: Some(selected),
            fqn,
        }))
    }

    pub(super) fn dispatch_missing_command(
        &mut self,
        argv: &[*mut TclObj],
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Code {
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
                Ok(arguments) => arguments,
                Err(error) => return self.report_cmd_error(error),
            };
            let frame = native_arguments::OriginalArgumentFrame::capture(self);
            let namespace = self.current_ns();
            let _ = self.invalid_original_command(argv[0]);
            if let Err(error) = frame.check(self, &arguments) {
                return self.report_cmd_error(error);
            }
            if self.current_ns() != namespace {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim missing-command original namespace changed",
                    )
                    .into(),
                );
            }
            if arguments.finish(Code::Ok) != Code::Ok {
                return Code::Error;
            }
        }
        let mut prepared = match self.prepare_missing_command(argv, lookup, caller, origin) {
            Ok(prepared) => prepared,
            Err(code) => return code,
        };
        let previous_namespace = self.current_ns.replace(lookup);
        let previous_frame_namespace = self.frames.borrow_mut().replace_active_namespace(lookup);
        let (command, generation) = prepared.selected.take().expect("prepared handler");
        let code = match (prepared.fqn.take(), generation) {
            (Some(fqn), Some(generation)) => self.dispatch_prebound(
                &prepared.words,
                Some((
                    fqn,
                    CommandBinding {
                        generation,
                        command,
                        jim_table_key: None,
                    },
                )),
            ),
            _ => self.invoke_bound(command, generation, &prepared.words),
        };
        self.frames
            .borrow_mut()
            .replace_active_namespace(previous_frame_namespace);
        self.current_ns.set(previous_namespace);
        // Keep actual prefix members through the reached invocation, then free
        // them after callback replacement/deletion has released the stored root.
        prepared.owned.clear();
        code
    }
}
