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

//! Original procedure and alias query owners across reached String callbacks.

use super::native_operation_currency::NativeOperationScope;
use super::{Command, ImportQueryPurpose, Interp, ProcDef};
use crate::obj::{self, NativeObjectLifetime, TclObj};
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{ProcInfo, ProcParam};
use tcl_syntax::value::ValueError;

enum ProcedureQueryNamespace {
    C {
        token: tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
        lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
    },
    Jim {
        namespace: crate::namespace::NsId,
        holder: Weak<obj::Owned>,
        original: NativeObjectLifetime,
        context: Weak<crate::native_source::NativeJimObjectContext>,
    },
}
impl ProcedureQueryNamespace {
    fn capture(interpreter: &Interp, declaration: &ProcDef) -> Result<Self, CmdError> {
        if interpreter
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let location = declaration.location();
            let holder = location
                .jim_namespace
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim procedure declaration namespace",
                ))?;
            obj::check_native_liveness(holder.as_ptr())?;
            let context = interpreter.native_jim_object_context()?;
            obj::validate_jim_allocator(holder.as_ptr(), &context.allocation_pool())?;
            if crate::native_source::selected_string_protocol(holder.as_ptr()).is_some()
                && !Rc::ptr_eq(&crate::native_source::context(holder.as_ptr())?, &context)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim procedure namespace object context",
                )
                .into());
            }
            return Ok(Self::Jim {
                namespace: location.namespace,
                holder: Rc::downgrade(&holder),
                original: NativeObjectLifetime::retain(holder.as_ptr()),
                context: Rc::downgrade(&context),
            });
        }
        let token = interpreter
            .native_namespace_token_at(declaration.namespace())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original procedure query namespace",
            ))?;
        let lifecycle = token.lifecycle();
        Ok(Self::C { token, lifecycle })
    }

    fn check(&self, interpreter: &Interp, declaration: &ProcDef) -> Result<(), CmdError> {
        let same = match self {
            Self::C { token, lifecycle } => {
                token.lifecycle() == *lifecycle
                    && interpreter.namespaces().owns_namespace_name_token(token)
                    && declaration.namespace() as u64 == token.token()
            }
            Self::Jim {
                namespace,
                holder,
                original,
                context,
            } => {
                original.checked_ptr()?;
                let location = declaration.location();
                let actual_context = interpreter.native_jim_object_context()?;
                obj::validate_jim_allocator(original.as_ptr(), &actual_context.allocation_pool())?;
                if crate::native_source::selected_string_protocol(original.as_ptr()).is_some()
                    && !Rc::ptr_eq(
                        &crate::native_source::context(original.as_ptr())?,
                        &actual_context,
                    )
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "foreign Jim procedure namespace object context",
                    )
                    .into());
                }
                location.namespace == *namespace
                    && location.jim_namespace.as_ref().is_some_and(|current| {
                        current.as_ptr() == original.as_ptr()
                            && Weak::ptr_eq(holder, &Rc::downgrade(current))
                    })
                    && Weak::ptr_eq(context, &Rc::downgrade(&actual_context))
            }
        };
        if same {
            Ok(())
        } else {
            Err(
                ValueError::CommandProtocolUnavailable("original procedure query namespace owner")
                    .into(),
            )
        }
    }
}

struct ProcedureQuery {
    generation: u64,
    declaration: Rc<ProcDef>,
    namespace: ProcedureQueryNamespace,
}
impl ProcedureQuery {
    fn capture(interpreter: &Interp, name: &[u8]) -> Result<Option<Self>, CmdError> {
        let Some(binding) = interpreter.import_query_binding(name) else {
            return Ok(None);
        };
        Self::capture_binding(interpreter, binding)
    }

    fn capture_binding(
        interpreter: &Interp,
        binding: (Vec<u8>, u64, Command),
    ) -> Result<Option<Self>, CmdError> {
        let Some(target) =
            interpreter.resolve_import_query(binding, ImportQueryPurpose::Introspection)
        else {
            return Ok(None);
        };
        let Command::Proc(procedure) = target.command else {
            return Ok(None);
        };
        let declaration = procedure.declaration();
        declaration.check_native_liveness()?;
        let namespace = ProcedureQueryNamespace::capture(interpreter, &declaration)?;
        Ok(Some(Self {
            generation: target.generation,
            declaration,
            namespace,
        }))
    }
    fn check(&self, interpreter: &Interp, scope: &NativeOperationScope) -> Result<(), CmdError> {
        Self::check_selected(
            interpreter,
            scope,
            self.generation,
            &self.declaration,
            &self.namespace,
        )
    }
    fn check_selected(
        interpreter: &Interp,
        scope: &NativeOperationScope,
        generation: u64,
        declaration: &Rc<ProcDef>,
        namespace: &ProcedureQueryNamespace,
    ) -> Result<(), CmdError> {
        scope.currency().ensure_current_or_refuse()?;
        let same = interpreter.raw_command_by_generation(generation)
            .is_some_and(|command| matches!(command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), declaration)));
        if !same {
            return Err(interpreter
                .clone()
                .retain_name_query_refusal("original procedure query owner retired"));
        }
        namespace
            .check(interpreter, declaration)
            .map_err(|error| interpreter.clone().retain_name_query_error(error))?;
        declaration
            .check_native_liveness()
            .map_err(|error| interpreter.clone().retain_name_query_error(error.into()))
    }
    fn bytes(
        &self,
        interpreter: &mut Interp,
        scope: &NativeOperationScope,
        original: *mut TclObj,
        stage: Option<&dyn Fn() -> Result<(), CmdError>>,
    ) -> Result<Vec<u8>, CmdError> {
        let current = || {
            if let Some(stage) = stage {
                stage()?;
            }
            self.check(interpreter, scope)
        };
        current()?;
        let arguments =
            super::native_arguments::OriginalArguments::enter(interpreter, &[original])?;
        let reached = arguments.bytes_using(0, &current);
        current()?;
        drop(arguments);
        current()?;
        reached.map(|bytes| bytes.to_vec())
    }
}

struct OriginalProcedureBodyOperand {
    scope: NativeOperationScope,
    arguments: super::native_arguments::OriginalArguments,
    caller: super::native_jim_lookup::OriginalJimLookupNamespace,
    actual: Interp,
    current: crate::namespace::NsId,
    original: *mut TclObj,
}
impl OriginalProcedureBodyOperand {
    fn enter(actual: &Interp, original: *mut TclObj) -> Result<Self, CmdError> {
        let arguments = super::native_arguments::OriginalArguments::enter(actual, &[original])?;
        let scope =
            NativeOperationScope::enter(actual).map_err(CmdError::from_execution_refusal)?;
        if actual
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
            || actual
                .native_invocation_dialect()
                .native_jim_info_protocol()
                .is_none()
        {
            return Err(arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("original Jim procedure body query issuer")
                    .into(),
            ));
        }
        let current = actual.current_ns();
        let result = Self {
            caller: super::native_jim_lookup::OriginalJimLookupNamespace::capture(actual, current)?,
            actual: actual.clone(),
            current,
            original,
            scope,
            arguments,
        };
        result.check()?;
        Ok(result)
    }
    fn check(&self) -> Result<(), CmdError> {
        self.arguments
            .check_original_for(&self.actual, 0, self.original)?;
        self.scope.currency().ensure_current_or_refuse()?;
        self.caller
            .check_current_arguments(&self.actual, &self.arguments)
    }
    fn select(&self, interpreter: &mut Interp) -> Result<Option<ProcedureQuery>, CmdError> {
        let selected = crate::list::with_native_conversion_check(
            &|| self.check(),
            &|error| self.arguments.retain_refusal(error.into()),
            |stage, _retain| {
                interpreter.resolve_original_command_at_using(
                    self.current,
                    self.original,
                    Some(&|_actual| stage()),
                )
            },
        )?;
        self.check()?;
        let query = match selected {
            Some((command, Some(generation))) => {
                let (name, _) = interpreter
                    .raw_command_location_by_generation(generation)
                    .ok_or_else(|| {
                        self.arguments.retain_refusal(
                            ValueError::CommandProtocolUnavailable(
                                "original procedure query command placement",
                            )
                            .into(),
                        )
                    })?;
                ProcedureQuery::capture_binding(interpreter, (name, generation, command))?
            }
            Some((_command, None)) => {
                return Err(self.arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable(
                        "original procedure query command generation",
                    )
                    .into(),
                ));
            }
            None => None,
        };
        Ok(query)
    }
}

struct OriginalProcedureBodyResult {
    operand: OriginalProcedureBodyOperand,
    generation: u64,
    declaration: Weak<ProcDef>,
    namespace: ProcedureQueryNamespace,
    body: NativeObjectLifetime,
}
impl tcl_runtime_api::OriginalProcedureBodyValue<*mut TclObj, CmdError>
    for OriginalProcedureBodyResult
{
    fn checked_value(&self) -> Result<*mut TclObj, CmdError> {
        let reached = (|| {
            self.operand.check()?;
            let actual = &self.operand.actual;
            let declaration =
                self.declaration
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original body-result declaration retired",
                    ))?;
            ProcedureQuery::check_selected(
                actual,
                &self.operand.scope,
                self.generation,
                &declaration,
                &self.namespace,
            )?;
            let original = self.body.checked_ptr()?;
            if declaration.body.checked_ptr()? != original {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original body-result header changed",
                )
                .into());
            }
            let context = actual.native_jim_object_context()?;
            obj::validate_jim_allocator(original, &context.allocation_pool())?;
            if crate::native_source::selected_string_protocol(original).is_some()
                && !Rc::ptr_eq(&crate::native_source::context(original)?, &context)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign original body-result context",
                )
                .into());
            }
            drop(declaration);
            self.operand.check()?;
            Ok(original)
        })();
        reached.map_err(|error| self.operand.arguments.retain_refusal(error))
    }
    fn finish(self: Box<Self>) -> Result<(), CmdError> {
        self.operand.check()?;
        let original = self
            .body
            .checked_ptr()
            .map_err(|error| self.operand.arguments.retain_refusal(error.into()))?;
        if self.operand.actual.result_obj() != original {
            return Err(self.operand.arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("original body-result publication header")
                    .into(),
            ));
        }
        Ok(())
    }
}

impl Interp {
    fn retain_name_query_error(&mut self, error: CmdError) -> CmdError {
        if let Some(cause) = error.native_execution_refusal() {
            self.refuse_native_execution(cause.clone());
        }
        error
    }
    fn retain_name_query_refusal(&mut self, reason: &'static str) -> CmdError {
        self.retain_name_query_error(ValueError::CommandProtocolUnavailable(reason).into())
    }
    pub(crate) fn original_proc_body_operand_query(
        &mut self,
        original: *mut TclObj,
    ) -> Result<tcl_runtime_api::ProcBodyBytesQuery, CmdError> {
        let operand = OriginalProcedureBodyOperand::enter(self, original)?;
        let reached: Result<tcl_runtime_api::ProcBodyBytesQuery, CmdError> = (|| {
            let query = operand.select(self)?;
            let result = if let Some(query) = &query {
                tcl_runtime_api::ProcBodyBytesQuery::Body(query.bytes(
                    self,
                    &operand.scope,
                    query.declaration.body.checked_ptr()?,
                    Some(&|| operand.check()),
                )?)
            } else {
                tcl_runtime_api::ProcBodyBytesQuery::NotProcedure(
                    operand
                        .arguments
                        .bytes_using(0, &|| operand.check())?
                        .to_vec(),
                )
            };
            operand.check()?;
            drop(query);
            operand.check()?;
            Ok(result)
        })();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        operand.check()?;
        reached
    }

    pub(crate) fn original_proc_body_value_query(
        &mut self,
        original: *mut TclObj,
    ) -> Result<tcl_runtime_api::ProcBodyValueQuery<*mut TclObj, CmdError>, CmdError> {
        let operand = OriginalProcedureBodyOperand::enter(self, original)?;
        let query = operand
            .select(self)
            .map_err(|error| operand.arguments.retain_refusal(error))?;
        let Some(query) = query else {
            let name = operand
                .arguments
                .bytes_using(0, &|| operand.check())?
                .to_vec();
            operand.check()?;
            return Ok(tcl_runtime_api::ProcBodyValueQuery::NotProcedure(name));
        };
        query.check(self, &operand.scope)?;
        let body = NativeObjectLifetime::retain(query.declaration.body.checked_ptr()?);
        let declaration = Rc::downgrade(&query.declaration);
        let ProcedureQuery {
            generation,
            declaration: selected,
            namespace,
        } = query;
        drop(selected);
        let result = OriginalProcedureBodyResult {
            operand,
            generation,
            declaration,
            namespace,
            body,
        };
        tcl_runtime_api::OriginalProcedureBodyValue::checked_value(&result)?;
        Ok(tcl_runtime_api::ProcBodyValueQuery::Body(Box::new(result)))
    }

    pub(crate) fn original_proc_body_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<Vec<u8>>, CmdError> = query
            .as_ref()
            .map(|query| {
                query.check(self, &scope)?;
                query.bytes(self, &scope, query.declaration.body.checked_ptr()?, None)
            })
            .transpose();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(query);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
    pub(crate) fn original_proc_info_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<ProcInfo>, CmdError> = (|| {
            let Some(query) = query.as_ref() else {
                return Ok(None);
            };
            let mut params = Vec::with_capacity(query.declaration.params.len());
            for parameter in &query.declaration.params {
                query.check(self, &scope)?;
                let default = parameter
                    .default
                    .as_ref()
                    .map(|value| query.bytes(self, &scope, value.checked_ptr()?, None))
                    .transpose()?;
                params.push(ProcParam {
                    name: parameter.name.clone(),
                    default,
                });
            }
            query.check(self, &scope)?;
            let body = query.bytes(self, &scope, query.declaration.body.checked_ptr()?, None)?;
            Ok(Some(ProcInfo { body, params }))
        })();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(query);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
    pub(crate) fn original_alias_bytes_query(
        &mut self,
        id: u32,
    ) -> Result<Option<Vec<Vec<u8>>>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let (_, generation) =
            self.command_identity(id)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "alias prefix command token",
                ))?;
        let namespace_id = self
            .namespaces()
            .native_command_slot_at_node(generation)
            .map(|(namespace, _)| namespace)
            .or_else(|| {
                self.hidden
                    .borrow()
                    .values()
                    .any(|binding| binding.generation == generation)
                    .then_some(crate::namespace::GLOBAL)
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original alias query namespace",
            ))?;
        let namespace = self.native_namespace_token_at(namespace_id).ok_or(
            ValueError::CommandProtocolUnavailable("original alias query namespace owner"),
        )?;
        let lifecycle = namespace.lifecycle();
        let command = self.raw_command_by_generation(generation);
        let Some(Command::Alias {
            original_prefix, ..
        }) = command.as_ref()
        else {
            return Ok(None);
        };
        let original_header = std::rc::Rc::clone(original_prefix);
        let original = NativeObjectLifetime::retain(original_header.checked_ptr()?);
        drop(command);
        let check = |interpreter: &mut Interp| -> Result<(), CmdError> {
            scope.currency().ensure_current_or_refuse()?;
            original_header.checked_ptr()?;
            let same = interpreter.raw_command_by_generation(generation).is_some_and(|command| matches!(command, Command::Alias { original_prefix, .. } if std::rc::Rc::ptr_eq(&original_prefix, &original_header) && original_prefix.checked_ptr().is_ok()));
            if !same
                || namespace.lifecycle() != lifecycle
                || !interpreter
                    .namespaces()
                    .owns_namespace_name_token(&namespace)
                || interpreter
                    .namespaces()
                    .native_command_slot_at_node(generation)
                    .is_some_and(|(current, _)| current != namespace_id)
            {
                return Err(
                    interpreter.retain_name_query_refusal("original alias query owner retired")
                );
            }
            Ok(())
        };
        let reached: Result<Option<Vec<Vec<u8>>>, CmdError> = (|| {
            check(self)?;
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias List protocol",
                ))?;
            let members = crate::list::list_elements_native_with_currency(
                original.as_ptr(),
                protocol,
                Some(scope.currency()),
            );
            check(self)?;
            let members = members?;
            let backing = crate::list::native_list_backing(original.as_ptr()).ok_or(
                ValueError::CommandProtocolUnavailable("original alias List backing"),
            )?;
            let members = members
                .into_iter()
                .map(|member| {
                    obj::check_native_liveness(member)?;
                    Ok(NativeObjectLifetime::retain(member))
                })
                .collect::<Result<Vec<_>, ValueError>>()?;
            let mut result = Vec::with_capacity(members.len());
            for member in &members {
                backing.check_generation()?;
                check(self)?;
                let arguments =
                    super::native_arguments::OriginalArguments::enter(self, &[member.as_ptr()])?;
                let reached = arguments.bytes(0);
                check(self)?;
                drop(arguments);
                check(self)?;
                backing.check_generation()?;
                result.push(reached?.to_vec());
            }
            Ok(Some(result))
        })();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
}

#[cfg(test)]
mod tests;
