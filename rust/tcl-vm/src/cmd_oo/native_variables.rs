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

//! Declaring-provider variable resolution in the actual selected method frame.

use super::{DeclaredVariable, NsId, Vm};
use tcl_core_types::NameBytes;
use tcl_syntax::naming::{
    NameProjectionUnavailable, NativeNameProtocol, NativeOoVariableResolverPurpose,
    native_oo_variable_resolver_matches,
};

type ProviderDeclarations<'a> = (NsId, u64, &'a [DeclaredVariable], &'a [DeclaredVariable]);

fn declarations(
    vm: &Vm,
    start: usize,
) -> Result<Option<ProviderDeclarations<'_>>, NameProjectionUnavailable> {
    let (activation, frame_namespace) = vm
        .native_oo_frame_identity(start)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let Some(context) = vm
        .oo
        .call_stack
        .iter()
        .rev()
        .find(|context| context.activation == Some(activation))
    else {
        return Ok(None);
    };
    let object = vm
        .oo
        .objects
        .get(&context.object)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let namespace = object
        .namespace_token
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    if namespace != frame_namespace {
        return Ok(None);
    }
    let step = context
        .chain
        .get(context.index)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let epoch = vm
        .oo
        .objects
        .get(&step.provider)
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?
        .creation_id;
    let (public, private) = if step.is_object {
        let owner = vm
            .oo
            .objects
            .get(&step.provider)
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
        (&owner.variables, &owner.private_variables)
    } else {
        let owner = vm
            .oo
            .classes
            .get(&step.provider)
            .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
        (&owner.variables, &owner.private_variables)
    };
    Ok(Some((namespace, epoch, public, private)))
}

/// The runtime resolver gets a `CString` root; declaration keys stay counted.
/// This selects a namespace binding, without inventing a local alias or cell.
pub(crate) fn runtime_target(
    vm: &Vm,
    start: usize,
    protocol: NativeNameProtocol,
    root: &[u8],
) -> Result<Option<(NsId, NameBytes)>, NameProjectionUnavailable> {
    let Some((namespace, epoch, public, private)) = declarations(vm, start)? else {
        return Ok(None);
    };
    for declaration in private {
        if native_oo_variable_resolver_matches(
            protocol,
            NativeOoVariableResolverPurpose::RuntimeRoot,
            declaration.name.as_bytes(),
            root,
        )? {
            return Ok(Some((namespace, declaration.private_storage(epoch))));
        }
    }
    for declaration in public {
        if native_oo_variable_resolver_matches(
            protocol,
            NativeOoVariableResolverPurpose::RuntimeRoot,
            declaration.name.as_bytes(),
            root,
        )? {
            return Ok(Some((namespace, declaration.name.clone())));
        }
    }
    Ok(None)
}

/// Native `AppendLocals` reports declaring names independently of installed aliases.
pub(crate) fn reported_bindings(
    vm: &Vm,
    start: usize,
) -> Result<Vec<(NsId, NameBytes, NameBytes)>, NameProjectionUnavailable> {
    let Some((namespace, epoch, public, private)) = declarations(vm, start)? else {
        return Ok(Vec::new());
    };
    Ok(public
        .iter()
        .map(|entry| (namespace, entry.name.clone(), entry.name.clone()))
        .chain(
            private
                .iter()
                .map(|entry| (namespace, entry.name.clone(), entry.private_storage(epoch))),
        )
        .collect())
}

pub(crate) fn reported_names(
    vm: &Vm,
    start: usize,
) -> Result<Vec<NameBytes>, NameProjectionUnavailable> {
    reported_bindings(vm, start)
        .map(|bindings| bindings.into_iter().map(|(_, name, _)| name).collect())
}
