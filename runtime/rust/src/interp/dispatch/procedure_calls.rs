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

//! Calling a procedure: preparing the call, binding the formals, running the selected body and the `wrong # args` usage.

use super::super::evaluation::compiling_body_frame;
use super::super::native_procedure_resources::NativeProcedureCommand;
use super::super::{
    native_argument_usage, native_arguments, native_body_artifact, native_compilation,
    native_i64_body, native_jim_import, native_jim_namespace_info, native_procedure_resources,
    CmdFrame, Code, Interp, NativeProcEntry, Param, ProcDef,
};
use super::{CallMeta, ProcFrame};
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_runtime_api::codegen_abi::NATIVE_PROC_STATUS_DECLINED;
use tcl_syntax::value::ValueError;

#[derive(Clone, Copy)]
enum PreparedProcedureBody {
    Enter(Option<NativeProcEntry>),
    Complete(Code),
}

struct PreparedProcedureCall<'a> {
    pub(in super::super) outcome: PreparedProcedureBody,
    issuer_dispatch: u32,
    saved_ns: NsId,
    pub(in super::super) jim_body: Option<*mut TclObj>,
    jim_level: u32,
    // Field order preserves the original owners' retirement order.
    info_frame:
        Result<Option<Rc<native_jim_namespace_info::OriginalNamespaceInfoFrame>>, ValueError>,
    import_traversal: Result<Option<Rc<native_jim_import::OriginalJimImportTraversal>>, ValueError>,
    proc_frame: Option<Box<CmdFrame>>,
    _method_execution: Option<native_procedure_resources::NativeCallableMethodExecution>,
    _native_admission: Box<native_compilation::NativeProcedureAdmission>,
    original_artifact: Option<Rc<native_body_artifact::NativeBodyArtifact>>,
    meta: CallMeta<'a>,
}

impl Interp {
    pub(in super::super) fn call_bound_proc(
        &mut self,
        binding: &NativeProcedureCommand,
        argv: &[*mut TclObj],
    ) -> Code {
        use tcl_registry::native_procedure::{
            NativeProcedureCompilationPurpose, NativeProcedureRecompileAction,
        };
        use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;
        let mut declaration = binding.declaration();
        if let Err(error) = declaration.check_native_liveness() {
            return self.report_cmd_error(error.into());
        }
        if declaration.native.is_none()
            && !self.original_procedure_artifact_is_current(&declaration)
        {
            if let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
                self.native_invocation_dialect(),
            ) {
                match protocol.recompilation_action(
                    NativeProcedureCompilationPurpose::CommandBody,
                    declaration.native_procedure_role_ledger().references(),
                ) {
                    Some(NativeProcedureRecompileAction::ReplaceSharedDeclaration) => {
                        if declaration.future_i64_add.get().is_some() {
                            native_i64_body::observe("recompile-replace", None);
                        }
                        declaration = match binding.replace_for_recompilation() {
                            Ok(declaration) => declaration,
                            Err(error) => return self.report_cmd_error(error.into()),
                        };
                    }
                    Some(NativeProcedureRecompileAction::RetainDeclaration) | None => {}
                }
            }
        }
        if self.check_entered_native_operation().is_err() {
            return Code::Error;
        }
        self.call_proc(&declaration, argv)
    }

    /// Call a user proc (`TclObjInterpProc`): a thin wrapper over [`run_proc`]
    /// with the proc's `(params, body, ns)` and the call args (`argv[1..]`).
    ///
    /// [`run_proc`]: Interp::run_proc
    pub(in super::super) fn call_proc(&mut self, def: &Rc<ProcDef>, argv: &[*mut TclObj]) -> Code {
        let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let original_body = match def
                .check_native_liveness()
                .and_then(|()| def.body.checked_ptr())
            {
                Ok(original) => original,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let body_lifetime = obj::NativeObjectLifetime::retain(original_body);
            let jim_parameters = match def
                .jim_parameters
                .as_ref()
                .map(obj::ProcedureObject::checked_ptr)
                .transpose()
            {
                Ok(original) => original,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let name = match arguments.bytes(0) {
                Ok(name) => name,
                Err(error) => return self.report_cmd_error(error),
            };
            if let Err(error) = def
                .check_native_liveness()
                .and_then(|()| def.body.checked_ptr())
                .and_then(|current| {
                    if current == original_body {
                        Ok(())
                    } else {
                        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original procedure body replaced",
                        ))
                    }
                })
            {
                return self.report_cmd_error(error.into());
            }
            let body = tcl_syntax::value::ValueOps::native_string_bytes(self, &original_body);
            if let Err(error) = arguments.check() {
                return self.report_cmd_error(error);
            }
            if let Err(error) = def
                .check_native_liveness()
                .and_then(|()| obj::check_native_liveness(body_lifetime.as_ptr()))
            {
                return self.report_cmd_error(error.into());
            }
            let body = match body {
                Ok(body) => body,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let location = def.location();
            self.run_proc(
                &def.params,
                &body,
                location.namespace,
                &argv[1..],
                &name,
                CallMeta {
                    original_argv: Some(argv),
                    err: ProcFrame::Proc(&name),
                    fqn: Some(&location.qualified_name),
                    source: def.source.clone(),
                    body_line_base: def.body_line_base,
                    link_vars: &[],
                    oo_variable_resolver: None,
                    keep_loop_codes: false,
                    same_level: false,
                    usage_prefix: None,
                    level_words: None,
                    quote_name: true,
                    native: def.native,
                    statics: def.native_statics(),
                    c_procedure: Some(def),
                    c_method_client_data: None,
                    jim_parameters,
                    jim_body: self.uses_jim_error_stack().then_some(original_body),
                    jim_namespace: location.jim_namespace.as_ref(),
                },
            )
        })();
        arguments.finish(code)
    }

    /// The shared proc-call protocol (`TclObjInterpProc`), used by both `proc`
    /// dispatch and `apply`: arity-check `call_args` against `params`, push a call
    /// frame in namespace `ns`, bind the params (defaults; an `args` catch-all
    /// collects the rest), run `body`, then pop. A body-level `return` becomes
    /// `Ok`; an escaping `break`/`continue` is an error. `usage_called` is the
    /// prefix of the `wrong # args` message (`name` for a proc, `apply
    /// lambdaExpr` for `apply`). Conservative-first per
    /// `proc-call-and-stack-traces.md` PC-2 (the CmdFrame/stack-trace +
    /// `info level`/`info frame` bookkeeping land with PC-1/PC-4/PC-5).
    pub(crate) fn run_proc<O: obj::ObjectPointer>(
        &mut self,
        params: &[Param<O>],
        body: &[u8],
        ns: NsId,
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        meta: CallMeta,
    ) -> Code {
        match self.prepare_procedure_call(params, body, ns, call_args, usage_called, meta) {
            Ok(prepared) => self.execute_prepared_procedure_call(body, call_args, prepared),
            Err(code) => code,
        }
    }

    #[inline(never)]
    fn prepare_procedure_call<'a, O: obj::ObjectPointer>(
        &mut self,
        params: &[Param<O>],
        body: &[u8],
        ns: NsId,
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        mut meta: CallMeta<'a>,
    ) -> Result<Box<PreparedProcedureCall<'a>>, Code> {
        let requires_source = match meta
            .c_procedure
            .map(|procedure| {
                procedure.requires_original_namespace_helper_source_execution(self, meta.jim_body)
            })
            .transpose()
        {
            Ok(required) => required.unwrap_or(false),
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        let original_import = match meta
            .c_procedure
            .map(|procedure| procedure.original_jim_import_origin(self))
            .transpose()
        {
            Ok(origin) => origin.flatten().is_some(),
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        if original_import && !requires_source {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Import selected Source execution",
                )
                .into(),
            ));
        }
        let issuer_dispatch = self.native_dispatch_depth.get();
        let original_artifact =
            if let Some(procedure) = meta.c_procedure.filter(|_| !original_import) {
                let original = procedure
                    .check_native_liveness()
                    .and_then(|()| procedure.body.checked_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                self.prepare_original_c_body(original, ns, Some(procedure))?
            } else {
                None
            };
        if requires_source && original_artifact.is_some() {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original namespace helper Source execution transport",
                )
                .into(),
            ));
        }
        let mut native_admission = match self.prepare_native_procedure(body, ns) {
            Ok(admission) => admission,
            Err(failure) => {
                return Err(self.admit_native_compilation_error(&failure, Some(usage_called)));
            }
        };
        // InvokeProcedureMethod retains its ProcedureMethod after compiling the
        // body, before the proc core binds formals. Replacement withdraws the
        // table's clientData reference while this entered owner stays live.
        let _method_execution = match meta
            .c_method_client_data
            .map(|method| method.enter_method())
            .transpose()
        {
            Ok(execution) => execution,
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        // A release that compiles a body whole as its procedure is called
        // refuses it before binding a word or running a command
        // (`TclProcCompileProc`).
        if let ProcFrame::Proc(name) = meta.err {
            if let Some(failure) = self.compile_failure_84(body, true, ns) {
                let frame = compiling_body_frame(name, failure.line);
                let code = self.raise_compile_failure(&failure, &frame);
                self.clear_error_logged();
                return Err(code);
            }
        }
        let usage = meta.usage_prefix.as_deref().unwrap_or(usage_called);
        let supplied = call_args.len();
        let Some(grammar) = self.native_invocation_dialect().parameter_grammar() else {
            return Err(self.error(b"formal parameter dialect is not selected"));
        };
        let Ok(bindings) = native_compilation::native_parameter_plan(params, supplied, grammar)
        else {
            return Err(
                match self.proc_wrong_args(usage, params, supplied, meta.quote_name) {
                    Ok(message) => self.wrong_arguments_message(&message),
                    Err(error) => self.report_cmd_error(error),
                },
            );
        };
        if grammar.skips_empty_body_activation() && body.is_empty() {
            self.set_result_bytes(b"");
            return Err(Code::Ok);
        }
        let jim_body = meta.jim_body;
        let info_receiver = match self.original_namespace_info_receiver(meta.original_argv) {
            Ok(receiver) => receiver,
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        let (saved_ns, proc_frame) = self.enter_native_procedure_frame(
            params,
            call_args,
            usage_called,
            (ns, &mut meta),
            bindings,
            original_artifact
                .as_ref()
                .and_then(|artifact| artifact.compiled_local_layout()),
        )?;
        if tcl_registry::release_compilation::compile_procedures_modelled(self.runtime_version()) {
            let unit = crate::compiled_locals::ProcUnit::new(
                body,
                params.iter().map(|param| param.name.clone()).collect(),
            );
            self.frames.borrow_mut().set_unit(Rc::new(unit));
        }
        if matches!(meta.err, ProcFrame::Method { .. }) {
            self.oo_bind_method_activation();
        }
        // Run the compiled body when there is one, else the source body. The
        // two are interchangeable here precisely because everything a body can
        // observe about its call — the variable frame, its namespace, `info
        // level`'s words, the bound formals, `wrong # args` — was decided
        // above, and everything about its completion is decided below.
        //
        // Source execution is required in these cases:
        //
        // - a live step trace. `dispatch_traced` installs this proc's own
        //   `enterstep`/`leavestep` into `step_active` *before* dispatching, so
        //   one read covers both "this proc is step-traced" and "an outer step
        //   trace is running". A natively lowered `set`/`incr`/`expr` never
        //   reaches `dispatch`, so it would fire no step trace and the
        //   transcript would silently lose lines. `enter`/`leave` traces need
        //   nothing: they fire around the whole call either way.
        // - the entry declining, which it may only do before any observable
        //   effect, so falling through to the source body is not a partial
        //   re-run.
        // - an actual installed Source owner requires token/operation receipts
        //   that the compiled entry does not transport.
        let native = meta
            .native
            .filter(|_| !requires_source && self.traces.borrow().step_active.is_empty());
        let import_traversal = self
            .frames
            .borrow()
            .current_original_jim_procedure()
            .and_then(|procedure| match procedure {
                Some(procedure) => {
                    native_jim_import::OriginalJimImportTraversal::capture(self, &procedure)
                }
                None => Ok(None),
            });
        if let Ok(Some(traversal)) = &import_traversal {
            self.frames
                .borrow_mut()
                .observe_original_jim_import_traversal(Rc::clone(traversal));
        }
        let info_frame = if let Some(receiver) = info_receiver {
            self.frames.borrow().current_original_jim_procedure().and_then(|procedure| {
                match procedure {
                    Some(procedure) if procedure.original_jim_namespace_helper_origin(self)?.is_some_and(|(kind, _)| kind == tcl_native_model::native_jim_lookup::NativeJimNamespaceHelperKind::Info) => {
                        native_jim_namespace_info::OriginalNamespaceInfoFrame::capture(self, &procedure, receiver).map(Some)
                    }
                    _ => Ok(None),
                }
            })
        } else {
            Ok(None)
        };
        if let Ok(Some(frame)) = &info_frame {
            self.frames
                .borrow_mut()
                .observe_original_namespace_info_frame(Rc::clone(frame));
        }
        let body_options = self.clear_return_options_checked();
        native_admission.activate();
        let jim_level = self.jim_procedure_level.get();
        if self.uses_jim_error_stack() {
            self.jim_procedure_level.set(jim_level + 1);
        }
        let execution: Result<(), tcl_cmd_core::CmdError> = match body_options {
            Ok(()) => self
                .acquire_native_procedure_execution(meta.c_procedure)
                .map_err(tcl_cmd_core::CmdError::from)
                .and_then(|()| {
                    let traversal = import_traversal
                        .as_ref()
                        .map_err(|error| tcl_cmd_core::CmdError::from(error.clone()))?;
                    if let Some(procedure) = meta.c_procedure {
                        procedure
                            .require_original_jim_import_traversal(self, traversal.as_ref())
                            .map_err(tcl_cmd_core::CmdError::from)?;
                    }
                    Ok(())
                })
                .and_then(|()| {
                    info_frame
                        .as_ref()
                        .map(|_| ())
                        .map_err(|error| error.clone().into())
                }),
            Err(first) => Err(first.into()),
        };
        // The body is compiled as one unit making its own locals
        // ([`InterpState::proc_body`]); the source path's script loop takes the
        // flag, and it is cleared after for a body that never reached one.
        self.next_proc_body.set(true);
        let outcome = match execution {
            Err(error) => PreparedProcedureBody::Complete(self.report_cmd_error(error)),
            Ok(()) => PreparedProcedureBody::Enter(native),
        };
        Ok(Box::new(PreparedProcedureCall {
            outcome,
            issuer_dispatch,
            saved_ns,
            jim_body,
            jim_level,
            info_frame,
            import_traversal,
            proc_frame: Some(proc_frame),
            _method_execution,
            _native_admission: native_admission,
            original_artifact,
            meta,
        }))
    }

    fn execute_prepared_procedure_call(
        &mut self,
        body: &[u8],
        call_args: &[*mut TclObj],
        mut prepared: Box<PreparedProcedureCall<'_>>,
    ) -> Code {
        let code = match prepared.outcome {
            PreparedProcedureBody::Complete(code) => code,
            PreparedProcedureBody::Enter(native) => match native {
                Some(entry) => match self.run_native_body(
                    entry,
                    call_args,
                    *prepared
                        .proc_frame
                        .take()
                        .expect("prepared procedure frame"),
                ) {
                    Ok(code) => code,
                    Err(frame) => self.run_selected_procedure_body(
                        body,
                        prepared.original_artifact.as_ref(),
                        prepared.jim_body,
                        *frame,
                    ),
                },
                None => self.run_selected_procedure_body(
                    body,
                    prepared.original_artifact.as_ref(),
                    prepared.jim_body,
                    *prepared
                        .proc_frame
                        .take()
                        .expect("prepared procedure frame"),
                ),
            },
        };
        self.finish_prepared_procedure_call(prepared, code)
    }

    fn finish_prepared_procedure_call(
        &mut self,
        prepared: Box<PreparedProcedureCall<'_>>,
        code: Code,
    ) -> Code {
        self.next_proc_body.set(false);
        let import_completion = prepared
            .import_traversal
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|traversal| traversal.normal_completion(self));
        let info_completion = prepared
            .info_frame
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|frame| frame.completion(self));
        self.release_native_procedure_execution();
        self.jim_procedure_level.set(prepared.jim_level);
        // The frame's local variables (and any traces on them) die with it.
        let proc_level = self.frames.borrow().current_level();
        // Capture `[info level 0]` (the invocation words) before the frame is
        // popped — the TIP 348 `CALL` entry if this body unwinds with an error.
        // The locals' unset traces are collected while the frame — and its
        // arrays' elements — still exist, and fire once it is gone, as C's
        // `TclDeleteVars` runs over a frame that is on its way out.
        let teardown = if self.traces.borrow().traces.is_empty() {
            Vec::new()
        } else {
            self.frame_teardown_unset_traces(proc_level)
        };
        let tailcall = self.frames.borrow_mut().take_tailcall();
        self.clean_current_jim_local_commands();
        let popped = self.pop_native_call_frame();
        if self.has_variable_traces() {
            self.clear_frame_var_traces(proc_level);
        }
        self.current_ns.set(prepared.saved_ns);
        self.fire_unset_callbacks(teardown);
        self.leave_namespace_activation(popped);
        self.recursion_depth.set(self.recursion_depth.get() - 1);
        // Apply the return boundary (`return`/`return -code -level`), then a
        // bare `break`/`continue` that escaped the body (no enclosing loop) is an
        // error (C Tcl: `invoked "break" outside of a loop`).
        // A *bare* `break`/`continue` command escaping the body (no enclosing
        // loop) is the `invoked "break" outside of a loop` error; but `return
        // -code break` (the body completed with `Code::Return`) propagates the
        // raw completion code unchanged — C distinguishes these, e.g. an
        // ensemble `-unknown` handler that does `return -code break` yields code
        // 3, not the loop error (namespace-47.4).
        let tailcall_code = self
            .native_invocation_dialect()
            .tailcall_protocol()
            .map(|protocol| i64::from(protocol.pending_code()));
        if let Some(tailcall) = tailcall.filter(|_| {
            code == Code::Ok || code == Code::Return || Some(code.as_int()) == tailcall_code
        }) {
            if let Some(Err(error)) = import_completion {
                return self.report_cmd_error(error.into());
            }
            if let Some(Err(error)) = info_completion {
                return self.report_cmd_error(error.into());
            }
            if let Err(first) = self.clear_return_options_checked() {
                return self.refuse_native_execution(first);
            }
            self.deferred_tailcalls
                .borrow_mut()
                .push((prepared.issuer_dispatch, tailcall));
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let from_return = code == Code::Return;
        let settled = match self.settle_return(code) {
            Code::Break if !prepared.meta.keep_loop_codes && !from_return => {
                self.error(b"invoked \"break\" outside of a loop")
            }
            Code::Continue if !prepared.meta.keep_loop_codes && !from_return => {
                self.error(b"invoked \"continue\" outside of a loop")
            }
            other => other,
        };
        let settled = if settled == Code::Ok {
            match import_completion {
                Some(Err(error)) => self.report_cmd_error(error.into()),
                _ => settled,
            }
        } else {
            settled
        };
        let settled = if settled == Code::Ok {
            match info_completion {
                Some(Ok(completion)) => match completion.publish(self) {
                    Ok(()) => settled,
                    Err(error) => self.report_cmd_error(error.into()),
                },
                Some(Err(error)) => self.report_cmd_error(error.into()),
                None => settled,
            }
        } else {
            settled
        };
        // On error, append the `(procedure "name" line N)` / `(lambda term ...)`
        // frame and clear `already_logged` so the proc-call command logs next.
        // Skipped when the error was produced by the return boundary itself
        // (`return -code error`, i.e. the body completed with `Code::Return`):
        // C only adds the procedure frame when the error unwinds *through* the
        // body, not when `return` synthesises it (error-6.7, result-6.2).
        if settled == Code::Error {
            if code != Code::Return {
                self.make_proc_error(prepared.meta.err);
            } else {
                // `return -code error`: no procedure frame, but the *caller* still
                // logs its own `invoked from within "<call>"` frame, so release
                // the already-logged flag that `process_return_error` may have set
                // for an explicit `-errorinfo` (error-6.7).
                self.clear_error_logged();
            }
        }
        settled
    }

    /// Execute the body selected before activation and formal binding. A native
    /// entry that declines uses this same retained selection and untouched frame.
    fn run_selected_procedure_body(
        &mut self,
        body: &[u8],
        original_artifact: Option<&Rc<native_body_artifact::NativeBodyArtifact>>,
        jim_body: Option<*mut TclObj>,
        frame: CmdFrame,
    ) -> Code {
        if let Some(artifact) = original_artifact {
            self.execute_original_c_body(artifact, frame)
        } else if let Some(original) = jim_body {
            self.eval_original_body_framed(
                tcl_native_model::native_eval_object::EvalObjectPurpose::ControlBody,
                original,
                frame,
            )
        } else {
            self.eval_framed(body, frame)
        }
    }

    /// Run a proc's **compiled** body in the call frame
    /// [`run_proc`](Self::run_proc) has already prepared — the native-tier
    /// mirror of [`eval_framed`](Self::eval_framed) +
    /// [`eval_script_mode`](Self::eval_script_mode), minus the parse and the
    /// command loop.
    ///
    /// Ownership, stated here because the emitted side has to match it
    /// exactly: `run_proc` owns the Tcl variable frame, `info level`'s words
    /// and the formal binding; this method owns the compiled activation and
    /// the body's `CmdFrame`; the entry owns **neither** (see
    /// [`NativeProcEntry`]).
    ///
    /// `Ok(code)` is the body's raw completion — `2` for a body-level
    /// `return`, exactly as the interpreted body reports it, so `run_proc`'s
    /// `settle_return` cannot tell the two apart. `Err(frame)` is a decline:
    /// nothing observable happened, and the untouched frame comes back so the
    /// source body can run in it. Handing the frame back rather than cloning
    /// it up front keeps the common path allocation-free; the box is what
    /// keeps a rarely-taken `Err` from widening every return of this function.
    pub(in super::super) fn run_native_body(
        &mut self,
        entry: NativeProcEntry,
        call_args: &[*mut TclObj],
        mut frame: CmdFrame,
    ) -> Result<Code, Box<CmdFrame>> {
        // A freshly pushed frame is its own `codePtr->source`, so `errorLine`
        // is measured from this body's base (`eval_framed`).
        frame.proc_line_base = frame.line_base;
        self.observe_codegen_depth("native-body-before", entry as usize);
        // The activation the eval loop would hold for this body. A refusal is
        // `eval_script_mode`'s: the error is already set, no activation is
        // held, and nothing has been pushed, so `run_proc`'s ordinary error
        // tail is exactly right — including the `(procedure "p" line N)` frame.
        if !self.codegen_activation_enter() {
            return Ok(Code::Error);
        }
        self.observe_codegen_depth("native-body-entered", entry as usize);
        self.cmd_frames.borrow_mut().push(frame);
        // `Tcl_EvalEx` resets the result at entry; `eval_script_mode` can skip
        // that because its first command always sets one. A compiled body
        // cannot promise the same — `set`/`incr`/`puts` write no interpreter
        // result — so a completion that comes back with a null result means
        // "the body left the result alone", and that has to be the empty
        // string the eval loop would have produced, never the caller's.
        self.set_result_bytes(b"");
        let mut out = crate::codegen_abi::TclCompletionAbi {
            code: 0,
            result: core::ptr::null_mut(),
            options: core::ptr::null_mut(),
        };
        let argc = i32::try_from(call_args.len()).unwrap_or(i32::MAX);
        // SAFETY: `entry` was supplied by a generated module through
        // `tcl_codegen_proc_define_native`; `argv[..argc]` are the live bound
        // call arguments it borrows; `out` is live, zeroed completion storage.
        let status = unsafe { entry(call_args.as_ptr(), argc, core::ptr::from_mut(&mut out)) };
        let popped = self.cmd_frames.borrow_mut().pop();
        if status == tcl_runtime_api::codegen_abi::NATIVE_PROC_STATUS_HOST_REFUSED {
            self.codegen_activation_leave(Code::Error);
            return Ok(Code::Error);
        }
        if status == NATIVE_PROC_STATUS_DECLINED {
            self.codegen_activation_leave(Code::Ok);
            return Err(Box::new(
                popped.expect("the CmdFrame this method just pushed"),
            ));
        }
        // Counted here, not before the call: a decline is not a native run.
        self.native_proc_dispatches
            .set(self.native_proc_dispatches.get().saturating_add(1));
        let code = Code::from_int(out.code);
        if !out.result.is_null() {
            // `set_result` takes the interpreter's own reference, so the
            // entry's is released after the store rather than transferred.
            self.set_result(out.result);
            // SAFETY: the entry transferred one owned reference on `result`.
            unsafe { obj::decr_ref_count(out.result) };
        }
        if !out.options.is_null() {
            // The options dict is a snapshot of state the runtime already
            // holds: `return`'s `-code`/`-level` were recorded by the `return`
            // command when the body invoked it, and `settle_return` below
            // reads those, not this dict. So it is released, never consulted.
            // SAFETY: the entry transferred one owned reference on `options`.
            unsafe { obj::decr_ref_count(out.options) };
        }
        self.codegen_activation_leave(code);
        Ok(code)
    }

    /// Number of actual compiled Native procedure-body executions.
    pub(crate) fn native_proc_dispatches(&self) -> u64 {
        self.native_proc_dispatches.get()
    }
}

impl Interp {
    /// The `wrong # args` message for a proc/method, applying any active
    /// ensemble-rewrite so the call is reported as the user wrote it (C's
    /// `Tcl_WrongNumArgs` rewrite path). When a rewrite is active and all the
    /// inserted words are accounted for, the leading `removed` words of the
    /// original `source` replace the rewritten prefix, and the formal parameters
    /// already satisfied by the inserted arguments are dropped.
    pub(crate) fn proc_wrong_args<O: obj::ObjectPointer>(
        &mut self,
        called: &[u8],
        params: &[Param<O>],
        supplied: usize,
        quote_name: bool,
    ) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        let protocol = self
            .native_invocation_dialect()
            .usage_protocol(Some(
                tcl_native_model::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
            ))
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "procedure usage",
            ))?;
        let activation = tcl_registry::native_procedure::procedure_activation_protocol(
            self.native_invocation_dialect(),
        )
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "procedure activation",
        ))?;
        let variadic = activation.variadic_usage_word();
        let original = native_argument_usage::OriginalArgumentUsage::capture(self, &[], 0, false)?;
        if let Some((words, drop_parameters)) = original.procedure_prefix(supplied, params.len())? {
            let words: Vec<_> = words.iter().map(|word| word.as_ref()).collect();
            original.check()?;
            return Ok(proc_usage_words(
                protocol,
                &words,
                &params[drop_parameters..],
                self.native_invocation_dialect().parameter_grammar(),
                variadic,
            ));
        }
        original.check()?;
        Ok(proc_usage(
            protocol,
            called,
            params,
            quote_name,
            self.native_invocation_dialect().parameter_grammar(),
            variadic,
        ))
    }
}

/// Build a `wrong # args` message from explicit leading `words` followed by the
/// formal `shown` parameters for an ensemble-rewritten call.
fn proc_usage_words<O: obj::ObjectPointer>(
    protocol: tcl_native_model::native_usage::NativeUsageProtocol,
    words: &[&[u8]],
    shown: &[Param<O>],
    grammar: Option<tcl_dialect::ParameterGrammar>,
    variadic: &[u8],
) -> Vec<u8> {
    let mut header = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            header.push(b' ');
        }
        header.extend_from_slice(word);
    }
    protocol.render_procedure_message(&header, &formal_usage(shown, grammar, variadic))
}

fn formal_usage<O: obj::ObjectPointer>(
    params: &[Param<O>],
    grammar: Option<tcl_dialect::ParameterGrammar>,
    variadic: &[u8],
) -> Vec<u8> {
    let Some(grammar) = grammar else {
        return Vec::new();
    };
    let parameters = native_compilation::formal_parameters(params, grammar);
    tcl_syntax::formal_params::formal_parameter_usage_bytes(&parameters, grammar, variadic)
}

fn proc_usage<O: obj::ObjectPointer>(
    protocol: tcl_native_model::native_usage::NativeUsageProtocol,
    called: &[u8],
    params: &[Param<O>],
    quote_name: bool,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    variadic: &[u8],
) -> Vec<u8> {
    let header = protocol.render_procedure_name(called, quote_name);
    protocol.render_procedure_message(&header, &formal_usage(params, grammar, variadic))
}
