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

//! Preparing a procedure's body artifact (the cache stamp and the local table) and entering it.

use super::super::{obj, CmdFrame, Code, Interp, NsId, ProcDef, Rc, TclObj};
use super::{
    cache, install, native_list_conversion, next_layout_token, unavailable, BodyContext,
    BodyExecution, Builder, CacheStamp, NativeBodyArtifact, RetainedBodyLocalTable, BYTECODE_TYPE,
};
use std::collections::HashMap;
use std::rc::Weak;
use tcl_bytecode_tables::{LiteralTable, LocalVarTable};
use tcl_lexer::{SourceImage, Span};
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_runtime_api::native_compilation::{
    NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind,
};
use tcl_syntax::naming::NativeCompiledVariableEnvironment as Environment;

impl Interp {
    pub(in super::super) fn original_procedure_artifact_is_current(
        &self,
        procedure: &Rc<ProcDef>,
    ) -> bool {
        let Ok(original) = procedure.body.checked_ptr() else {
            return false;
        };
        let Some(current) = cache(original) else {
            return false;
        };
        matches!(&current.owner, BodyContext::Procedure(owner) if Weak::ptr_eq(owner, &Rc::downgrade(procedure)))
            && self
                .native_body_stamp(procedure.namespace(), Some(procedure))
                .as_ref()
                == Some(&current.stamp)
    }

    pub(super) fn native_body_stamp(
        &self,
        namespace: NsId,
        procedure: Option<&Rc<ProcDef>>,
    ) -> Option<CacheStamp> {
        let traces = self.traces.borrow();
        Some(CacheStamp {
            execution_name_policy: self.execution_name_policy(),
            interpreter: self.native_command_interpreter,
            namespace,
            epochs: self.native_compiler_cache_epochs(namespace)?,
            physical: self.native_invocation_dialect().tcl_version?,
            grammar: self.lexer_config(),
            source_protocol: self.source_string_protocol()?,
            fixed_math: self.native_math_function_table(),
            expression_policy: self.expression_evaluation_policy(),
            observers: traces
                .cmd_traces
                .iter()
                .filter(|trace| trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0)
                .map(|trace| (trace.id, trace.token, trace.ops))
                .collect(),
            steps: traces
                .step_active
                .iter()
                .map(|trace| (trace.token, trace.ops, trace.command.clone()))
                .collect(),
            borrowed_table: if procedure.is_none() && self.frames.borrow().in_proc() {
                let frames = self.frames.borrow();
                match frames.native_local_name_table() {
                    Some(table) => Some(Rc::as_ptr(table) as usize),
                    None if self
                        .native_invocation_dialect()
                        .native_compiled_variable_protocol()
                        .is_some_and(|compiler| {
                            compiler.supports_environment(Environment::BorrowFrameSlots)
                        }) =>
                    {
                        return None;
                    }
                    // C8.4/8.5 non-body scripts use original name access and
                    // have no indexed frame borrowing requirement. Any actual
                    // table above retains its existing authentication schedule.
                    None => None,
                }
            } else {
                None
            },
        })
    }

    pub(super) fn native_body_local_table(
        &mut self,
        stamp: &CacheStamp,
        names: &[tcl_runtime_api::NameBytes],
        entered: &native_list_conversion::BodyObjectOperation,
    ) -> Result<(LocalVarTable, Option<RetainedBodyLocalTable>), Code> {
        let borrowed_table = if stamp.borrowed_table.is_some() {
            self.frames.borrow().native_local_name_table().cloned()
        } else {
            None
        };
        let borrowed_names = borrowed_table
            .as_ref()
            .map(|table| {
                table
                    .names
                    .iter()
                    .map(|name| {
                        name.as_ref()
                            .map(|name| {
                                entered
                                    .bytes_using(self, name.as_ptr(), &|| {
                                        entered.check(self)?;
                                        let same_table = self
                                            .frames
                                            .borrow()
                                            .native_local_name_table()
                                            .is_some_and(|current| Rc::ptr_eq(current, table));
                                        if !same_table
                                            || self
                                                .native_body_stamp(stamp.namespace, None)
                                                .as_ref()
                                                != Some(stamp)
                                        {
                                            return Err(entered.refuse(unavailable(
                                                "native body borrowed table changed",
                                            )));
                                        }
                                        Ok(())
                                    })
                                    .map(|bytes| tcl_runtime_api::NameBytes::from(bytes.as_ref()))
                            })
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, tcl_cmd_core::CmdError>>()
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error))?;
        // Canonical name materialisation may call a native updater. The same
        // table and physical compiler context must still own the compilation.
        if stamp.borrowed_table.is_some()
            && self.native_body_stamp(stamp.namespace, None).as_ref() != Some(stamp)
        {
            return Err(
                self.report_cmd_error(unavailable("native body borrowed table changed").into())
            );
        }
        let mut lvt = borrowed_names.as_ref().map_or_else(
            || LocalVarTable::from_native_names(names),
            |names| LocalVarTable::from_native_slot_names(names),
        );
        lvt.set_native_protocol(
            self.native_invocation_dialect()
                .native_compiled_variable_protocol(),
        );
        Ok((lvt, borrowed_table))
    }

    /// Prepare a complete executable artifact before installing its native
    /// primary. Unsupported registered instructions preserve the interpreter's
    /// existing uncached path; unavailable geometry is a typed host refusal.
    pub(in super::super) fn prepare_original_c_body(
        &mut self,
        original: *mut TclObj,
        namespace: NsId,
        procedure: Option<&Rc<ProcDef>>,
    ) -> Result<Option<Rc<NativeBodyArtifact>>, Code> {
        let entered = native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|first| self.report_cmd_error(first))?;
        let reached = (|| -> Result<Option<Rc<NativeBodyArtifact>>, Code> {
            obj::check_native_liveness(original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let original_lifetime = obj::NativeObjectLifetime::retain(original);
            if let Some(procedure) = procedure {
                let live = procedure
                    .body
                    .checked_ptr()
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                if live != original {
                    return Err(self.report_cmd_error(
                        unavailable("native procedure body original identity").into(),
                    ));
                }
            }
            let Some(mut stamp) = self.native_body_stamp(namespace, procedure) else {
                return Ok(None);
            };
            let matches_owner = |owner: &BodyContext| match (owner, procedure) {
                (BodyContext::Script, None) => true,
                (BodyContext::Procedure(owner), Some(procedure)) => {
                    Weak::ptr_eq(owner, &Rc::downgrade(procedure))
                }
                _ => false,
            };
            if let Some(current) = cache(original)
                .filter(|current| current.stamp == stamp && matches_owner(&current.owner))
            {
                return Ok(Some(current));
            }
            let bytes = entered.bytes(self, original);
            entered
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            obj::check_native_liveness(original_lifetime.as_ptr())
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let bytes = bytes.map_err(|error| self.report_cmd_error(error))?;
            if cache(original).is_some() {
                obj::change_type_with_stage_check(original, core::ptr::null(), 0, &mut || {
                    entered.check(self)
                })
                .map_err(|error| self.report_cmd_error(entered.retain(error)))?;
                entered
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                obj::check_native_liveness(original_lifetime.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
            }
            stamp = self
                .native_body_stamp(namespace, procedure)
                .ok_or_else(|| {
                    self.report_cmd_error(unavailable("native body source callback context").into())
                })?;
            let image = SourceImage::native(bytes.as_ref());
            let end = u32::try_from(image.len()).map_err(|_| {
                self.report_cmd_error(unavailable("native body source extent").into())
            })?;
            let region = Span::new(0, end);
            if stamp.physical == tcl_dialect::TclVersion::V8_4 {
                let plan = tcl_lexer::native_script_words_in(image.clone(), region, stamp.grammar)
                    .map_err(|_| {
                        self.report_cmd_error(
                            unavailable("native body executable script geometry").into(),
                        )
                    })?;
                if let Some(fatal) = plan.fatal_tail {
                    // C8.4 parses the complete compiled body before entering its
                    // first command. A later malformed command therefore prevents
                    // an earlier return from executing and leaves no Bytecode.
                    return Err(self.error(fatal.cut.message.as_bytes()));
                }
            }
            let context = Context {
                mode: NativeCompilationMode::BytecodeObject,
                frame: if procedure.is_some() {
                    NativeCompilationFrame::ProcedureCode
                } else {
                    NativeCompilationFrame::ScriptCode
                },
                loop_depth: 0,
                catch_depth: Some(0),
            };
            let names: Vec<_> = procedure.map_or_else(Vec::new, |procedure| {
                procedure
                    .params
                    .iter()
                    .map(|param| tcl_runtime_api::NameBytes::from(param.name.as_slice()))
                    .collect()
            });
            let (lvt, borrowed_table) = self.native_body_local_table(&stamp, &names, &entered)?;
            let mut builder = Builder {
                interp: self,
                image: image.clone(),
                context,
                stamp: stamp.clone(),
                lvt,
                literals: LiteralTable::new(),
                scripts: HashMap::new(),
                parse_failure: None,
                compilation_failure: None,
                private_objects: HashMap::new(),
                procedure: procedure.map(Rc::downgrade),
            };
            let compilation = builder
                .script(region, 0)
                .and_then(|()| builder.finish_compactible_body(region, original));
            let compilation = builder.check_compilation_stage(compilation);
            entered
                .check(builder.interp)
                .map_err(|first| builder.interp.report_cmd_error(first))?;
            let literals = match compilation {
                Ok(literals) => literals,
                Err(error) => {
                    if builder.compilation_failure.is_some() || builder.parse_failure.is_some() {
                        return Err(builder.report_compilation_error(error));
                    }
                    if matches!(
                    error.native_access_refusal(),
                    Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native body registered instruction capability"
                    ))
                ) {
                    return Ok(None);
                }
                    return Err(builder.interp.report_cmd_error(error));
                }
            };
            if builder
                .interp
                .native_body_stamp(namespace, procedure)
                .as_ref()
                != Some(&stamp)
            {
                return Err(builder.interp.report_cmd_error(
                    unavailable("native body compiler context changed during publication").into(),
                ));
            }
            let locals = procedure.map(|_| NativeCompiledLocalLayout {
                owner: stamp.interpreter,
                token: next_layout_token(),
                epoch: stamp.epochs.0,
                kind: NativeCompiledLocalLayoutKind::Procedure,
                names: builder.lvt.native_slot_names(),
            });
            let artifact = Rc::new(NativeBodyArtifact {
                stamp,
                owner: procedure.map_or(BodyContext::Script, |procedure| {
                    BodyContext::Procedure(Rc::downgrade(procedure))
                }),
                image,
                region,
                scripts: builder.scripts,
                literals,
                locals,
                _borrowed_table: borrowed_table.as_ref().map(Rc::downgrade),
            });
            if let (Some(procedure), Some(layout)) = (procedure, artifact.locals.as_ref()) {
                procedure.retain_native_compiled_names(layout);
            }
            install(original, &artifact, &BYTECODE_TYPE, &entered, self)
                .map_err(|error| self.report_cmd_error(error))?;
            entered
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            obj::check_native_liveness(original_lifetime.as_ptr())
                .map_err(|error| self.report_cmd_error(error.into()))?;
            drop(original_lifetime);
            entered
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            Ok(Some(artifact))
        })();
        entered
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    pub(in super::super) fn execute_original_c_body(
        &mut self,
        artifact: &Rc<NativeBodyArtifact>,
        frame: CmdFrame,
    ) -> Code {
        if !self.codegen_activation_enter() {
            return Code::Error;
        }
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
        ) {
            self.codegen_activation_leave(code);
            return code;
        }
        self.cmd_frames.borrow_mut().push(frame);
        let mut execution = BodyExecution::default();
        #[cfg(have_tommath)]
        {
            execution.native_i64_add = self.capture_entered_i64_body(artifact);
        }
        #[cfg(have_tommath)]
        let observed_i64 = execution.native_i64_add.is_some();
        let code = self.execute_body_region(artifact, artifact.region, &mut execution);
        #[cfg(have_tommath)]
        if observed_i64 {
            crate::interp::native_i64_body::observe(
                "body-settled",
                Some(
                    u64::from(execution.native_i64_ran)
                        | (u64::from(execution.done) << 1)
                        | (u64::from(code == Code::Return) << 2),
                ),
            );
        }
        #[cfg(have_tommath)]
        if execution.native_i64_ran && execution.done && code == Code::Return {
            let count = self.native_proc_dispatches.get().saturating_add(1);
            self.native_proc_dispatches.set(count);
            crate::interp::native_i64_body::observe("dispatch-count", Some(count));
        }
        let code = if execution.done && code == Code::Return {
            Code::Ok
        } else {
            code
        };
        self.cmd_frames.borrow_mut().pop();
        self.codegen_activation_leave(code);
        code
    }
}
