// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original procedure and alias query owners across reached String callbacks.

use super::native_operation_currency::NativeOperationScope;
use super::{Command, ImportQueryPurpose, Interp, ProcDef};
use crate::obj::{self, NativeObjectLifetime, TclObj};
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{ProcInfo, ProcParam};
use tcl_syntax::value::ValueError;

pub(crate) enum JimNamespaceOriginResult {
    Found(Vec<u8>),
    Missing(Vec<u8>),
}
impl JimNamespaceOriginResult {
    pub(crate) fn publish(self, interpreter: &mut Interp) -> super::Code {
        match self {
            Self::Found(bytes) => {
                interpreter.set_result_bytes(&bytes);
                super::Code::Ok
            }
            Self::Missing(message) => interpreter.error(&message),
        }
    }
}

struct JimOriginQuery<'a> {
    interpreter: &'a mut Interp,
    namespace: obj::Owned,
    namespace_scope: super::native_jim_lookup::OriginalJimLookupNamespace,
    protocol: tcl_syntax::native_jim_lookup::NativeJimLookupProtocol,
    scope: NativeOperationScope,
}
impl JimOriginQuery<'_> {
    fn check(&mut self) -> Result<(), CmdError> {
        self.scope.currency().ensure_current_or_refuse()?;
        self.namespace_scope.check(self.interpreter)?;
        obj::check_native_liveness(self.namespace.as_ptr())?;
        Ok(())
    }
    fn bytes(&mut self, original: &Rc<obj::Owned>) -> Result<Vec<u8>, CmdError> {
        self.check()?;
        let reached = self
            .interpreter
            .native_object_string_bytes(original.as_ptr());
        self.check()?;
        reached.map(|bytes| bytes.to_vec()).map_err(Into::into)
    }
}
impl tcl_registry::native_jim_lookup::NativeJimNamespaceOriginBackend for JimOriginQuery<'_> {
    type Original = Rc<obj::Owned>;
    type Error = CmdError;

    fn canonical(&mut self, original: &Self::Original) -> Result<Self::Original, Self::Error> {
        self.check()?;
        let reached = self
            .interpreter
            .jim_canonical_namespace_object(&self.namespace, original.as_ptr());
        self.check()?;
        Ok(Rc::new(reached?))
    }
    fn alias_prefix(
        &mut self,
        canonical: &Self::Original,
    ) -> Result<Option<Self::Original>, Self::Error> {
        self.check()?;
        let is_alias = self
            .interpreter
            .resolve_original_command_at(crate::namespace::GLOBAL, canonical.as_ptr())?
            .is_some_and(|(command, _)| matches!(command, Command::Alias { .. }));
        self.check()?;
        if !is_alias {
            return Ok(None);
        }
        // The scripted helper reaches info-alias separately on this SAME object.
        let reached = self
            .interpreter
            .original_alias_prefix_value_at(crate::namespace::GLOBAL, canonical.as_ptr());
        self.check()?;
        let failure = match reached? {
            tcl_runtime_api::AliasPrefixLookup::Prefix(original) => {
                return Ok(Some(Rc::new(obj::Owned::retain(original))));
            }
            tcl_runtime_api::AliasPrefixLookup::MissingCommand => {
                tcl_runtime_api::NativeJimAliasLookupFailure::MissingCommand
            }
            tcl_runtime_api::AliasPrefixLookup::NotAlias => {
                tcl_runtime_api::NativeJimAliasLookupFailure::NotAlias
            }
        };
        let name = self.bytes(canonical)?;
        Err(CmdError::new_bytes(failure.message(&name))
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084))
    }
    fn command_exists(&mut self, original: &Self::Original) -> Result<bool, Self::Error> {
        self.check()?;
        let found = self
            .interpreter
            .resolve_original_command_at(crate::namespace::GLOBAL, original.as_ptr())?
            .is_some();
        self.check()?;
        Ok(found)
    }
    fn report(&mut self, original: &Self::Original) -> Result<Vec<u8>, Self::Error> {
        let bytes = self.bytes(original)?;
        Ok(self.protocol.namespace_origin_result(&bytes))
    }
    fn same_original(&self, left: &Self::Original, right: &Self::Original) -> bool {
        left.as_ptr() == right.as_ptr()
    }
    fn alias_cycle_unavailable(&mut self) -> Self::Error {
        self.interpreter.retain_name_query_refusal(
            "Jim namespace origin alias cycle has no bounded original result",
        )
    }
}

struct ProcedureQuery {
    generation: u64,
    declaration: Rc<ProcDef>,
    namespace: tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
    lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
}
impl ProcedureQuery {
    fn capture(interpreter: &Interp, name: &[u8]) -> Result<Option<Self>, CmdError> {
        let Some(binding) = interpreter.import_query_binding(name) else {
            return Ok(None);
        };
        let Some(target) =
            interpreter.resolve_import_query(binding, ImportQueryPurpose::Introspection)
        else {
            return Ok(None);
        };
        let Command::Proc(procedure) = target.command else {
            return Ok(None);
        };
        let declaration = procedure.declaration();
        let namespace = interpreter
            .native_namespace_token_at(declaration.namespace())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original procedure query namespace",
            ))?;
        let lifecycle = namespace.lifecycle();
        Ok(Some(Self {
            generation: target.generation,
            declaration,
            namespace,
            lifecycle,
        }))
    }
    fn check(
        &self,
        interpreter: &mut Interp,
        scope: &NativeOperationScope,
    ) -> Result<(), CmdError> {
        scope.currency().ensure_current_or_refuse()?;
        let same = interpreter.raw_command_by_generation(self.generation)
            .is_some_and(|command| matches!(command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), &self.declaration)));
        if !same
            || self.namespace.lifecycle() != self.lifecycle
            || !interpreter
                .namespaces()
                .owns_namespace_name_token(&self.namespace)
            || self.declaration.namespace() as u64 != self.namespace.token()
        {
            return Err(
                interpreter.retain_name_query_refusal("original procedure query owner retired")
            );
        }
        self.declaration
            .check_native_liveness()
            .map_err(|error| interpreter.retain_name_query_error(error.into()))
    }
    fn bytes(
        &self,
        interpreter: &mut Interp,
        scope: &NativeOperationScope,
        original: *mut TclObj,
    ) -> Result<Vec<u8>, CmdError> {
        self.check(interpreter, scope)?;
        let arguments =
            super::native_arguments::OriginalArguments::enter(interpreter, &[original])?;
        let reached = arguments.bytes(0);
        self.check(interpreter, scope)?;
        drop(arguments);
        self.check(interpreter, scope)?;
        reached.map(|bytes| bytes.to_vec())
    }
}

impl Interp {
    pub(crate) fn original_jim_namespace_origin(
        &mut self,
        original: *mut TclObj,
    ) -> Result<JimNamespaceOriginResult, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        obj::check_native_liveness(original)?;
        let original = Rc::new(obj::Owned::retain(original));
        let namespace = self.jim_current_namespace_object()?;
        let namespace_scope =
            super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, self.current_ns())?;
        let protocol = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim namespace origin helper",
            ))?;
        let mut query = JimOriginQuery {
            interpreter: self,
            namespace,
            namespace_scope,
            protocol,
            scope,
        };
        let reached =
            tcl_registry::native_jim_lookup::native_jim_namespace_origin(&mut query, &original);
        query.check()?;
        let result = match reached? {
            tcl_registry::native_jim_lookup::NativeJimNamespaceOriginOutcome::Found(bytes) => {
                JimNamespaceOriginResult::Found(bytes)
            }
            tcl_registry::native_jim_lookup::NativeJimNamespaceOriginOutcome::Missing(original) => {
                let bytes = query.bytes(&original)?;
                JimNamespaceOriginResult::Missing(query.protocol.namespace_origin_missing(&bytes))
            }
        };
        query.check()?;
        Ok(result)
    }
    fn retain_name_query_error(&mut self, error: CmdError) -> CmdError {
        if let Some(cause) = error.native_execution_refusal() {
            self.refuse_native_execution(cause.clone());
        }
        error
    }
    fn retain_name_query_refusal(&mut self, reason: &'static str) -> CmdError {
        self.retain_name_query_error(ValueError::CommandProtocolUnavailable(reason).into())
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
                query.bytes(self, &scope, query.declaration.body.checked_ptr()?)
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
                    .map(|value| query.bytes(self, &scope, value.checked_ptr()?))
                    .transpose()?;
                params.push(ProcParam {
                    name: parameter.name.clone(),
                    default,
                });
            }
            query.check(self, &scope)?;
            let body = query.bytes(self, &scope, query.declaration.body.checked_ptr()?)?;
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
