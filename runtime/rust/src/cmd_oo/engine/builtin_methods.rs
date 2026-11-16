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

//! The unexported `oo::object` built-in methods (`variable`, `varname`, `eval`) and the TIP 500 private variable names they map.

use super::super::{
    native_method_cache, wrong_args, CallStep, DeclaredVariableFullName, DeclaredVariableHeader,
    OoFrame,
};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::{
    NativeTclooCoreMethod, NativeTclooMethodPayloadType,
};
use tcl_syntax::value::ValueOps;

impl Interp {
    /// The unexported object built-in methods (`variable`/`varname`/`eval`),
    /// defined on `oo::object` in C (`tclOOBasic.c`) and callable only via
    /// `my`. Returns `None` when `method` is not one of them (so the caller
    /// falls through to the unknown-method error).
    pub(in super::super) fn oo_builtin_method(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
        original_argv: Option<&[*mut TclObj]>,
    ) -> Option<Code> {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(self) {
                Ok(scope) => scope,
                Err(first) => return Some(self.report_cmd_error(first.into())),
            };
        let reached = self.oo_builtin_method_entered(
            obj,
            method,
            args,
            external,
            original_argv,
            scope.currency(),
        );
        match scope.currency().ensure_current_or_refuse() {
            Ok(()) => reached,
            Err(first) => Some(self.report_cmd_error(first.into())),
        }
    }

    fn oo_builtin_method_entered(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
        original_argv: Option<&[*mut TclObj]>,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Option<Code> {
        let var_ns = self.oo.borrow().objects.get(&obj).map(|o| o.var_ns)?;
        match method {
            // Link each named instance variable into the calling method frame.
            b"variable" => {
                let Some(protocol) = self.native_c_variable_name_protocol() else {
                    return Some(
                        self.report_cmd_error(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "actual TclOO variable names",
                            )
                            .into(),
                        ),
                    );
                };
                for &original in args {
                    let bytes = match self.native_string_bytes(&original) {
                        Ok(bytes) => bytes.to_vec(),
                        Err(error) => return Some(self.report_cmd_error(error.into())),
                    };
                    let local = match tcl_syntax::naming::native_oo_explicit_variable_local_name(
                        tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                        &bytes,
                    ) {
                        Ok(Ok(local)) => local.to_vec(),
                        Ok(Err(error)) => {
                            let words = error
                                .error_code
                                .iter()
                                .map(Vec::as_slice)
                                .collect::<Vec<_>>();
                            return Some(
                                self.report_cmd_error(
                                    tcl_cmd_core::CmdError::with_error_code_bytes(
                                        error.message,
                                        crate::interp::error_code_list(&words),
                                    )
                                    .with_native_string_result(
                                        tcl_syntax::native_string::NativeStringProtocol::C(
                                            protocol.version(),
                                        ),
                                    ),
                                ),
                            );
                        }
                        Err(_) => {
                            return Some(
                                self.report_cmd_error(
                                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                        "TclOO variable local name",
                                    )
                                    .into(),
                                ),
                            );
                        }
                    };
                    let mapped = match self.private_storage_name_original(
                        &bytes,
                        tcl_syntax::naming::NativeOoPrivateVariablePurpose::ExplicitLink,
                    ) {
                        Ok(mapped) => mapped,
                        Err(error) => return Some(self.report_cmd_error(error)),
                    };
                    let target = mapped
                        .as_ref()
                        .map_or(original, |full| full.original.as_ptr());
                    let code = self.link_original_c_oo_variable(target, var_ns, &local);
                    if code != Code::Ok {
                        return Some(code);
                    }
                }
                self.set_result_bytes(b"");
                Some(Code::Ok)
            }
            // Varname creates the selected cell and follows its actual link.
            b"varname" => {
                if args.len() != 1 {
                    return Some(wrong_args(self, b"my varname varName"));
                }
                let bytes = match self.native_string_bytes(&args[0]) {
                    Ok(bytes) => bytes.to_vec(),
                    Err(error) => return Some(self.report_cmd_error(error.into())),
                };
                let storage = match self.private_storage_name(
                    &bytes,
                    tcl_syntax::naming::NativeOoPrivateVariablePurpose::Varname,
                ) {
                    Ok(storage) => storage.unwrap_or(bytes),
                    Err(error) => return Some(self.report_cmd_error(error)),
                };
                Some(self.original_c_oo_varname(args[0], var_ns, &storage))
            }
            // Evaluate a script in the object's namespace (concatenating multiple
            // arguments with spaces, as `Tcl_ConcatObj` does).
            b"eval" => {
                if args.is_empty() {
                    return Some(wrong_args(self, b"my eval arg ?arg ...?"));
                }
                let concatenated;
                let script = if let [original] = args {
                    *original
                } else {
                    let script = match self.concat_original_arguments(args) {
                        Ok(script) => script,
                        Err(error) => return Some(self.report_cmd_error(error)),
                    };
                    concatenated = obj::Owned::fresh(script);
                    concatenated.as_ptr()
                };
                let caller = self.current_ns();
                self.set_current_ns(var_ns);
                self.enter_namespace_activation(var_ns);
                self.frames.borrow_mut().push_namespace(var_ns);
                if let Some(argv) = original_argv {
                    let installed = self
                        .frames
                        .borrow_mut()
                        .install_original_error_stack_argv(argv);
                    if let Err(error) = installed {
                        let popped = self.pop_native_call_frame();
                        self.set_current_ns(caller);
                        self.leave_namespace_activation(popped);
                        return Some(self.report_cmd_error(error.into()));
                    }
                }
                let activation = self.frames.borrow().current_activation();
                // Run the script with an OO context frame so `self`/`my` work
                // inside it (`$obj eval {self}` returns the object; oo-18.12).
                let name_owner = self.oo_name_owner(obj);
                let provider = self
                    .oo
                    .borrow()
                    .object_root
                    .expect("native object eval provider");
                let declaring_facet = self
                    .oo
                    .borrow()
                    .classes
                    .get(&provider)
                    .map(|class| Rc::downgrade(&class.facet_birth));
                self.oo.borrow_mut().call_stack.push(OoFrame {
                    name_owner,
                    activation: Some(activation),
                    object: obj,
                    chain: native_method_cache::ReachedMethodChain::from(vec![CallStep {
                        method_owner: None,
                        provider,
                        method: b"eval".to_vec(),
                        is_object: false,
                        is_filter: false,
                        synthetic_type: Some(NativeTclooMethodPayloadType::Core(
                            NativeTclooCoreMethod::Eval,
                        )),
                        declaring_facet,
                    }]),
                    index: 0,
                    target: Vec::new(),
                    external,
                });
                let code = self.eval_generic_control_body(script);
                self.retire_oo_call_frame(currency);
                let popped = self.pop_native_call_frame();
                self.set_current_ns(caller);
                self.leave_namespace_activation(popped);
                if let Err(first) = currency.ensure_current_or_refuse() {
                    return Some(self.report_cmd_error(first.into()));
                }
                // C's FinalizeEval: on error append `(in "<name> eval" script
                // line N)`, where name is the object's name for a public call
                // and the literal `my` for an internal (non-public) one.
                if code == Code::Error {
                    let mut inner = b"in \"".to_vec();
                    if external {
                        inner.extend_from_slice(&self.oo_name(obj));
                    } else {
                        inner.extend_from_slice(b"my");
                    }
                    inner.extend_from_slice(b" eval\" script");
                    self.append_frame_line(&inner);
                    self.clear_error_logged();
                }
                Some(code)
            }
            _ => None,
        }
    }

    /// If `name` is a TIP 500 private instance variable of the currently-running
    /// method's declaring scope, its mangled storage name (`"<creationEpoch> :
    /// name"`); otherwise `None`. The scope is the provider of the call-stack's
    /// top frame (the method that invoked the built-in).
    fn private_storage_name_original(
        &self,
        name: &[u8],
        purpose: tcl_syntax::naming::NativeOoPrivateVariablePurpose,
    ) -> Result<Option<DeclaredVariableFullName>, tcl_cmd_core::CmdError> {
        let Some(selected) = self.native_c_variable_name_protocol() else {
            return Ok(None);
        };
        let protocol = tcl_syntax::naming::NativeNameProtocol::C(selected.version());
        let reached = {
            let oo = self.oo.borrow();
            let frame =
                oo.call_stack.iter().rev().find(|frame| {
                    frame.activation == Some(self.frames.borrow().current_activation())
                });
            frame
                .and_then(|frame| {
                    frame
                        .chain
                        .get(frame.index)
                        .map(|step| (frame.object, step.provider))
                })
                .and_then(|(object, provider)| {
                    let declared = if provider == object {
                        &oo.objects.get(&provider)?.private_variables
                    } else {
                        &oo.classes.get(&provider)?.private_variables
                    };
                    let declaration = declared.iter().find(|variable| {
                        tcl_syntax::naming::native_oo_private_variable_matches(
                            protocol,
                            purpose,
                            &variable.name,
                            name,
                        ) == Ok(true)
                    })?;
                    Some((
                        declaration.check_originals(),
                        declaration
                            .full_name
                            .as_ref()
                            .map(|full| DeclaredVariableFullName {
                                original: full.original.observation(),
                                name: full.name.clone(),
                            }),
                        oo.objects.get(&provider)?.creation_id,
                        declaration.name.clone(),
                    ))
                })
        };
        let Some((current, full, epoch, declared)) = reached else {
            return Ok(None);
        };
        if let Err(error) = current {
            return Err(tcl_cmd_core::CmdError::from_execution_refusal(
                self.clone().refuse_completion_value_access(error),
            ));
        }
        if let Some(full) = full {
            return Ok(Some(full));
        }
        // Neutral declarations retain their separate reconstructed-input contract.
        let mut bytes = format!("{epoch} : ").into_bytes();
        bytes.extend_from_slice(&declared);
        Ok(Some(DeclaredVariableFullName {
            original: DeclaredVariableHeader::Stored(obj::Owned::fresh(obj::new_string_bytes(
                &bytes,
            ))),
            name: bytes,
        }))
    }

    fn private_storage_name(
        &self,
        name: &[u8],
        purpose: tcl_syntax::naming::NativeOoPrivateVariablePurpose,
    ) -> Result<Option<Vec<u8>>, tcl_cmd_core::CmdError> {
        self.private_storage_name_original(name, purpose)
            .map(|full| full.map(|full| full.name))
    }
}
