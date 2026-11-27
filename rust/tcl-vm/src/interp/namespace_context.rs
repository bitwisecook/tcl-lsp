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

//! Original execution namespaces remain distinct from materialized coordinates.

use super::{
    Vm, native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
};
use crate::interp::native_name_world::NameWorldCell;
use crate::{Value, error::TclHostFailure, value::NativeObjectLifetimeLease};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;
use tcl_core_types::{NameBytes, NsId};
use tcl_runtime_api::{NativeExecutionError, native_compilation::NativeInterpreterIdentity};
use tcl_syntax::{
    naming::{NamePolicyAuthority, NamePolicyProtocol},
    raw_string::NativeValueAccessRefusal,
};

#[derive(Clone)]
pub(crate) enum NamespaceExecutionContext {
    Resolved(NsId),
    OriginalJim(Rc<OriginalJimNamespaceContext>),
}

impl From<NsId> for NamespaceExecutionContext {
    fn from(token: NsId) -> Self {
        Self::Resolved(token)
    }
}

impl PartialEq for NamespaceExecutionContext {
    fn eq(&self, other: &Self) -> bool {
        self.same_context(other)
    }
}
impl Eq for NamespaceExecutionContext {}

impl std::fmt::Debug for NamespaceExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved(token) => f.debug_tuple("Resolved").field(token).finish(),
            Self::OriginalJim(_) => f.write_str("OriginalJim"),
        }
    }
}

impl NamespaceExecutionContext {
    pub(crate) fn same_context(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Resolved(left), Self::Resolved(right)) => left == right,
            (Self::OriginalJim(left), Self::OriginalJim(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }

    pub(crate) fn check_owner(&self, vm: &mut Vm) -> Result<(), CmdError> {
        match self {
            Self::Resolved(token) => {
                let scope = NativeOperationScope::enter(vm)?;
                if !vm.native_namespace_token_has_owner(*token) {
                    return Err(refuse(vm, "original namespace token owner"));
                }
                scope.check(vm)
            }
            Self::OriginalJim(original) => original.check_owner(vm),
        }
    }

    /// Observe an existing coordinate without obtaining a namespace getter.
    pub(crate) fn resolved_token(&self) -> Option<NsId> {
        match self {
            Self::Resolved(token) => Some(*token),
            Self::OriginalJim(original) => original.resolved_token(),
        }
    }
}

pub(crate) struct OriginalJimNamespaceContext {
    original: NativeObjectLifetimeLease,
    interpreter: NativeInterpreterIdentity,
    world: Weak<NameWorldCell>,
    policy: NamePolicyProtocol,
    materialized: RefCell<Option<(NameBytes, NsId)>>,
}

impl OriginalJimNamespaceContext {
    pub(crate) fn for_lambda(vm: &mut Vm, original: &Value) -> Result<Rc<Self>, CmdError> {
        let scope = NativeOperationScope::enter(vm)?;
        let policy = vm.name_policy_protocol().filter(|policy| {
            policy.authority() == NamePolicyAuthority::Native && policy.recipe().is_jim084()
        });
        let Some(policy) = policy else {
            return Err(refuse(vm, "original Jim namespace issuer"));
        };
        OriginalNumericOps::in_original_scope(vm, &scope)?.current_for_value(original)?;
        let context = Rc::new(Self {
            original: original.native_lifetime_lease(),
            interpreter: vm.native_interpreter_identity(),
            world: Rc::downgrade(&vm.name_world),
            policy,
            materialized: RefCell::new(None),
        });
        scope.check(vm)?;
        context.check_owner(vm)?;
        Ok(context)
    }

    pub(crate) fn original(&self) -> &Value {
        self.original.value()
    }

    pub(crate) fn owner_is_current(&self, vm: &Vm) -> bool {
        self.interpreter == vm.native_interpreter_identity()
            && vm.name_policy_protocol() == Some(self.policy)
            && self.original().check_native_header().is_ok()
            && self
                .world
                .upgrade()
                .is_some_and(|world| Rc::ptr_eq(&world, &vm.name_world) && world.borrow().is_live())
    }

    pub(crate) fn check_owner(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let scope = NativeOperationScope::enter(vm)?;
        if !self.owner_is_current(vm) {
            return Err(refuse(vm, "original Jim namespace owner"));
        }
        OriginalNumericOps::in_original_scope(vm, &scope)?.current_for_value(self.original())?;
        scope.check(vm)
    }

    fn resolved_token(&self) -> Option<NsId> {
        let bytes = self.original().resident_string_bytes()?;
        self.materialized
            .borrow()
            .as_ref()
            .filter(|(selected, _)| selected.as_bytes() == bytes.as_ref())
            .map(|(_, token)| *token)
    }

    /// A reached original namespace operation owns this getter independently of capture.
    pub(crate) fn materialize_using(
        &self,
        vm: &mut Vm,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<NsId, CmdError> {
        self.check_owner(vm)?;
        check(vm)?;
        let bytes = vm.native_name_operand_bytes_checked_using(self.original(), &|vm| {
            self.check_owner(vm)?;
            check(vm)
        })?;
        self.check_owner(vm)?;
        check(vm)?;
        let selected = self
            .policy
            .recipe()
            .jim_lambda_namespace_input(&bytes)
            .map_err(|_| refuse(vm, "original Jim namespace context input"))?;
        let token = vm.intern_jim_namespace_object(selected.selected());
        self.check_owner(vm)?;
        *self.materialized.borrow_mut() = Some((NameBytes::from(bytes.as_ref()), token));
        vm.note_original_jim_namespace_materialized(self, token);
        check(vm)?;
        Ok(token)
    }
}

fn refuse(vm: &mut Vm, purpose: &'static str) -> CmdError {
    let cause = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
    );
    vm.retain_tcl_host_failure(TclHostFailure::Execution(cause.clone()));
    CmdError::from_execution_refusal(cause)
}

#[derive(Clone)]
pub(crate) enum ProcedureNamespace {
    Resolved {
        path: tcl_core_types::ByteNamespacePath,
        token: NsId,
    },
    OriginalJim(Rc<OriginalJimNamespaceContext>),
}

impl ProcedureNamespace {
    pub(crate) fn context(&self) -> NamespaceExecutionContext {
        match self {
            Self::Resolved { token, .. } => NamespaceExecutionContext::Resolved(*token),
            Self::OriginalJim(original) => {
                NamespaceExecutionContext::OriginalJim(Rc::clone(original))
            }
        }
    }
}
