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

//! Command registration: the checked publication service, prepared and plain host commands and the registrar a host command registers through.

use super::value_conversion::export_version;
use super::{
    HostCommandNames, HostCommandShim, VmRegistrar, evaluate_using, provide_package,
    read_variable_using, set_variable, unset_variable,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_engine_api::{
    CommandPublicationKey, CommandPublicationPurpose, CommandPublicationService, CommandRegistrar,
    EngineError, HostCommand, HostOutcome, PreparedCommandPublication, Value,
};
use tcl_vm::Vm;

struct VmPublicationService(tcl_vm::NativePublicationService);

struct VmPublicationReceipt {
    service: tcl_vm::NativePublicationService,
    publication: tcl_vm::NativePreparedPublication,
}

fn native_publication_error(error: tcl_vm::NativePublicationError) -> EngineError {
    EngineError::ExecutionRefusal(error.to_string())
}

fn publication_key(publication: &tcl_vm::NativePreparedPublication) -> CommandPublicationKey {
    let key = publication.key();
    CommandPublicationKey {
        owner: key.owner,
        interpreter: key.interpreter,
        namespace: key.namespace,
        simple: Rc::from(key.simple.as_bytes()),
    }
}

fn publication_receipt(
    publication: &PreparedCommandPublication,
) -> Result<&VmPublicationReceipt, EngineError> {
    let receipt = publication
        .receipt
        .downcast_ref::<VmPublicationReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign command publication receipt".into())
        })?;
    if publication.key != publication_key(&receipt.publication)
        || publication.original != receipt.publication.original()
    {
        return Err(EngineError::ExecutionRefusal(
            "command publication reporting fields do not match its receipt".into(),
        ));
    }
    Ok(receipt)
}

impl CommandPublicationService for VmPublicationService {
    fn prepare(
        &self,
        original: &[u8],
        purpose: CommandPublicationPurpose,
    ) -> Result<PreparedCommandPublication, EngineError> {
        let purpose = match purpose {
            CommandPublicationPurpose::CreateCCommand => {
                tcl_vm::NativePublicationPurpose::CreateCCommand
            }
            CommandPublicationPurpose::DeleteCCommand => {
                tcl_vm::NativePublicationPurpose::DeleteCCommand
            }
        };
        let publication = self
            .0
            .prepare(original, purpose)
            .map_err(native_publication_error)?;
        Ok(PreparedCommandPublication {
            original: publication.original(),
            key: publication_key(&publication),
            receipt: Rc::new(VmPublicationReceipt {
                service: self.0.clone(),
                publication,
            }),
        })
    }

    fn observed_presence(
        &self,
        publication: &PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        self.0
            .observed_presence(&publication_receipt(publication)?.publication)
            .map_err(native_publication_error)
    }

    fn note_publication(
        &self,
        publication: &PreparedCommandPublication,
        present: bool,
    ) -> Result<(), EngineError> {
        self.0
            .note_publication(&publication_receipt(publication)?.publication, present)
            .map_err(native_publication_error)
    }
}

pub(super) fn open_publication_service(
    vm: &Vm,
) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
    vm.native_publication_service()
        .map(|service| Rc::new(VmPublicationService(service)) as Rc<dyn CommandPublicationService>)
        .map_err(native_publication_error)
}

pub(super) fn define_prepared_host_command(
    vm: &mut Vm,
    host_commands: &HostCommandNames,
    publication: &PreparedCommandPublication,
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    let receipt = publication_receipt(publication)?;
    let token = vm
        .define_prepared_native_command(
            &receipt.service,
            &receipt.publication,
            Rc::new(HostCommandShim {
                command,
                host_commands: Rc::clone(host_commands),
            }),
        )
        .map_err(native_publication_error)?;
    if let Some(token) = token {
        host_commands.borrow_mut().push(token);
    }
    Ok(())
}

pub(super) fn remove_prepared_host_command(
    vm: &mut Vm,
    publication: &PreparedCommandPublication,
) -> Result<bool, EngineError> {
    let receipt = publication_receipt(publication)?;
    vm.remove_prepared_native_command(&receipt.service, &receipt.publication)
        .map_err(native_publication_error)
}

pub(super) fn define_host_command(
    vm: &mut Vm,
    host_commands: &HostCommandNames,
    name: &[u8],
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    let service = open_publication_service(vm)?;
    let publication = service.prepare(name, CommandPublicationPurpose::CreateCCommand)?;
    service.note_publication(&publication, true)?;
    define_prepared_host_command(vm, host_commands, &publication, command)
}

pub(super) fn remove_host_command(vm: &mut Vm, name: &[u8]) -> Result<bool, EngineError> {
    let service = open_publication_service(vm)?;
    let publication = service.prepare(name, CommandPublicationPurpose::DeleteCCommand)?;
    let present = service.observed_presence(&publication)?;
    service.note_publication(&publication, false)?;
    let removed = remove_prepared_host_command(vm, &publication)?;
    Ok(present && removed)
}

impl CommandRegistrar for VmRegistrar<'_> {
    fn native_c_version(&self) -> Option<tcl_engine_api::NativeCVersion> {
        self.vm
            .native_scalar_carrier_dialect()
            .native_scalar_getter_protocol()
            .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
            .map(export_version)
    }

    fn command_publication_service(
        &mut self,
    ) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
        open_publication_service(self.vm)
    }
    fn define_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_prepared_host_command(self.vm, &self.host_commands, &publication, command)
    }
    fn remove_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        remove_prepared_host_command(self.vm, &publication)
    }
    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(self.vm, &self.host_commands, name, command)
    }
    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        remove_host_command(self.vm, name)
    }
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.define_command_bytes(name.as_bytes(), command)
    }
    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        self.remove_command_bytes(name.as_bytes())
    }
    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(self.vm, name, version)
    }

    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        self.vm.library_loaded(file_name, prefix);
        Ok(())
    }

    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        read_variable_using(self.vm, self.original, name)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        set_variable(self.vm, name, &value)
    }

    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        unset_variable(self.vm, name)
    }

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        evaluate_using(self.vm, self.original, script)
    }
}

/// Register a host command through the VM's checked publication owner.
pub fn register_host_command(vm: &mut Vm, name: &str, command: Rc<dyn HostCommand>) {
    if let Err(error) = define_host_command(
        vm,
        &Rc::new(RefCell::new(Vec::new())),
        name.as_bytes(),
        command,
    ) {
        let _ = vm.refuse_host_command(error.to_string());
    }
}
