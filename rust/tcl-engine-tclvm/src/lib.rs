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

//! `tcl-engine-tclvm` — the bytecode VM behind the Tcl extension interface.
//!
//! The first implementation of [`tcl_engine_api::Engine`], and the one the
//! interface was built against: `docs/design/registry/spec-packs.md` names `tcl-vm` the
//! canonical engine everywhere (server, CLI, and studio, which ships it to
//! wasm already).
//!
//! Everything the interface asks for maps onto an embedder API the VM has,
//! rather than onto a shim around a gap:
//!
//! | interface | VM |
//! |---|---|
//! | [`Engine::compile`] | [`Vm::define_procedure`] — one compile, then bytecode |
//! | [`Engine::invoke`] | [`Vm::invoke_command`] — the public call path |
//! | [`Engine::define_command`] | [`Vm::register_native_command`] — stateful host commands |
//! | [`CommandRegistrar`] during an invocation | the `&mut Vm` the native-command seam hands over: [`Vm::register_native_command`], [`Vm::remove_command`], [`Vm::package_provide`], [`Vm::library_loaded`], [`Vm::get_var`], [`Vm::set_var`], [`Vm::unset_var`] and [`Vm::eval_source`], all in the calling frame |
//! | a [`HostCommand`]'s [`CompletionCode`] | the [`Code`] of the [`Completion`] the native command answers |
//! | [`Engine::restrict_commands`] | [`Vm::retain_commands`] — a closed whitelist |
//! | [`Budget::commands`] | [`Vm::set_command_limit`] — enforced, not merely stored |
//! | [`Budget::wall_clock`] | [`Vm::set_wall_clock_budget`] |
//! | [`Engine::set_release`] | [`Vm::set_dialect_profile`] — the profile resolved through the one dialect ingress |
//! | [`Engine::confine_stores`] | [`Vm::set_stores_confined`] — a store outside the activation is a Tcl error |
//!
//! **What each budget bounds.** The command limit counts *dispatched*
//! commands, which is what C Tcl's `interp limit commands` counts too — a loop
//! the compiler inlines into bytecode dispatches nothing and is not charged.
//! The wall-clock cap is what bounds that case: the VM polls it inside the
//! bytecode trampoline, so `while {1} {}` is stopped even though it never
//! dispatches. A host that wants containment therefore sets both, and
//! [`tcl_spec_hooks`]'s default config does.
//!
//! Values cross as structure, never as text: a `words` list arrives as a Tcl
//! list value and a `ctx` dict as a Tcl dict value, both built directly.
//!
//! **The thread's numeral grammar stays the thread's.** The VM installs its
//! release's numeral grammar per thread (`tcl_syntax::number`), and an engine
//! runs on the analysis thread that owns it, so an engine pinned to 8.6 would
//! otherwise leave every later numeral on that thread read as 8.6 reads it.
//! Every operation that runs the VM claims the engine's own grammar and hands
//! the thread's back on the way out ([`GrammarGuard`]).

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_engine_api::OriginalObject;
use tcl_engine_api::{
    Budget, BudgetKind, CommandPublicationKey, CommandPublicationPurpose,
    CommandPublicationService, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, PreparedCommandPublication, Value,
};
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Completion, NativeCommand, Vm};

/// A compiled unit: the VM procedure the body was defined as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmHandle {
    procedure: String,
    parameters: usize,
    token: tcl_vm::NativeRegisteredCommandToken,
}

impl VmHandle {
    /// The VM procedure name this handle invokes. Exposed for diagnostics.
    #[must_use]
    pub fn procedure(&self) -> &str {
        &self.procedure
    }
}

/// The actual generations registered through [`Engine::define_command`] — shared with
/// every [`HostCommandShim`] so a command registered from inside another's
/// invocation is kept by a later [`Engine::restrict_commands`] too.
type HostCommandNames = Rc<RefCell<Vec<tcl_vm::NativeRegisteredCommandToken>>>;

/// Adapts a [`HostCommand`] to the VM's [`NativeCommand`], converting values
/// at the boundary in both directions.
struct HostCommandShim {
    command: Rc<dyn HostCommand>,
    host_commands: HostCommandNames,
}

/// The registration door a host command gets while it runs: the VM the
/// native-command seam already hands over, plus the engine's name list.
struct VmOriginalObject {
    value: VmOriginalValue,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
}

enum VmOriginalValue {
    Header(tcl_vm::NativeObjectLifetimeLease),
    ListMember {
        backing: tcl_vm::NativeListItems,
        index: usize,
    },
}
impl std::ops::Deref for VmOriginalValue {
    type Target = tcl_vm::Value;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Header(lease) => lease.value(),
            Self::ListMember { backing, index } => &backing[*index],
        }
    }
}
struct VmListBackingReceipt {
    backing: tcl_vm::NativeListItems,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    interpreter: (u64, u64),
}

// This private receipt can duplicate a descriptor, but owns no callable or Value.
struct VmCommandNameCacheReceipt {
    cache: Option<tcl_runtime_api::native_command_name::NativeCommandNameCache>,
    version: tcl_dialect::TclVersion,
    interpreter: (u64, u64),
}

struct VmNamespaceNameCacheReceipt {
    cache: tcl_runtime_api::native_namespace_name::NativeNamespaceNameCache,
    interpreter: (u64, u64),
}

impl tcl_engine_api::OriginalObject for VmOriginalObject {
    fn native_c_version(&self) -> Option<tcl_engine_api::NativeCVersion> {
        self.dialect
            .native_scalar_getter_protocol()
            .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
            .map(export_version)
    }
    fn scope_identity(&self) -> (u64, u64) {
        self.interpreter
    }
    fn resident_string(&self) -> Option<(Rc<[u8]>, tcl_engine_api::NativeStringStorageIdentity)> {
        self.value
            .resident_string_bytes()
            .map(|bytes| (bytes, export_storage(&self.value)))
    }
    fn identity(&self) -> usize {
        self.value.native_object_identity()
    }
    fn is_shared(&self) -> bool {
        self.value.native_object_is_shared()
    }
    fn snapshot(&self) -> Result<Value, EngineError> {
        if !self.value.native_object_is_live() {
            return Err(EngineError::ExecutionRefusal(
                "retired original native object header".into(),
            ));
        }
        from_vm_object_value(&self.value)
    }
    fn engine_receipt(&self) -> &dyn std::any::Any {
        self
    }
    fn index_cache(
        &self,
    ) -> Option<(
        tcl_engine_api::NativeIndexCache,
        tcl_engine_api::NativeCVersion,
    )> {
        self.value
            .native_index_cache()
            .map(|(cache, version)| (cache, export_version(version)))
    }
    fn string_cache(&self) -> Option<tcl_engine_api::NativeStringCache> {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as S;
        match self.value.native_object_snapshot().cache {
            S::String {
                protocol,
                num_chars,
                unicode,
            } => protocol
                .tcl_version()
                .map(|version| tcl_engine_api::NativeStringCache::C {
                    origin: export_version(version),
                    num_chars,
                    unicode,
                }),
            S::JimString { num_chars } => {
                Some(tcl_engine_api::NativeStringCache::Jim { num_chars })
            }
            _ => None,
        }
    }
    fn command_name_cache_origin(&self) -> Option<tcl_engine_api::NativeCVersion> {
        self.value
            .native_command_name_cache_origin()
            .map(export_version)
    }
    fn command_name_cache(&self) -> Option<tcl_engine_api::NativeCommandNameCache> {
        let version = self.value.native_command_name_cache_origin()?;
        Some(tcl_engine_api::NativeCommandNameCache::new(
            export_version(version),
            self.interpreter,
            Rc::new(VmCommandNameCacheReceipt {
                cache: self.value.native_command_name_cache(),
                version,
                interpreter: self.interpreter,
            }),
        ))
    }
    fn namespace_name_cache(&self) -> Option<tcl_engine_api::NativeNamespaceNameCache> {
        let cache = self.value.native_namespace_name_cache()?;
        Some(tcl_engine_api::NativeNamespaceNameCache::new(
            export_version(cache.version()),
            self.interpreter,
            Rc::new(VmNamespaceNameCacheReceipt {
                cache,
                interpreter: self.interpreter,
            }),
        ))
    }
    fn list_backing(&self) -> Result<Option<tcl_engine_api::NativeListBacking>, EngineError> {
        let Some(protocol) = self.dialect.native_string_protocol() else {
            return Err(EngineError::ExecutionRefusal(
                "original List issuer unavailable".into(),
            ));
        };
        let Some(backing) = self
            .value
            .native_list_backing_in(protocol)
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?
        else {
            return Ok(None);
        };
        let version = protocol.tcl_version().ok_or_else(|| {
            EngineError::ExecutionRefusal("C List backing issuer unavailable".into())
        })?;
        let canonical = backing.canonical_state();
        let sharing = backing.lifetime_view();
        let count = backing.len();
        Ok(Some(tcl_engine_api::NativeListBacking::new(
            export_version(version),
            self.interpreter,
            count,
            canonical,
            Rc::new(move || sharing.native_is_shared()),
            Rc::new(VmListBackingReceipt {
                backing,
                protocol,
                interpreter: self.interpreter,
            }),
        )))
    }
    fn list_backing_matches(
        &self,
        carrier: &tcl_engine_api::NativeListBacking,
    ) -> Result<bool, EngineError> {
        let receipt = checked_list_backing_receipt(carrier, self.dialect, self.interpreter)?;
        Ok(receipt.backing.has_native_header()
            && self.value.native_list_attachment_matches(&receipt.backing))
    }

    fn duplicate_native_header(
        &self,
    ) -> Result<Rc<dyn tcl_engine_api::OriginalObject>, EngineError> {
        let protocol = self.dialect.native_string_protocol().ok_or_else(|| {
            EngineError::ExecutionRefusal("native duplicate issuer unavailable".into())
        })?;
        drop(
            self.value
                .native_list_backing_in(protocol)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        );
        let duplicate = self.value.duplicate_native_object_in(protocol);
        Ok(Rc::new(Self {
            value: VmOriginalValue::Header(duplicate.native_lifetime_lease()),
            dialect: self.dialect,
            interpreter: self.interpreter,
        }))
    }
    fn list_members(
        &self,
    ) -> Result<Option<(Vec<Rc<dyn tcl_engine_api::OriginalObject>>, bool)>, EngineError> {
        Ok(self
            .value
            .cached_list_representation()
            .map(|(items, canonical)| {
                let members = (0..items.len())
                    .map(|index| {
                        Rc::new(Self {
                            value: VmOriginalValue::ListMember {
                                backing: items.lifetime_view(),
                                index,
                            },
                            dialect: self.dialect,
                            interpreter: self.interpreter,
                        }) as Rc<dyn tcl_engine_api::OriginalObject>
                    })
                    .collect();
                (members, canonical)
            }))
    }
    fn dictionary_members(
        &self,
    ) -> Result<
        Option<(
            Vec<(
                Rc<dyn tcl_engine_api::OriginalObject>,
                Rc<dyn tcl_engine_api::OriginalObject>,
            )>,
            Option<usize>,
        )>,
        EngineError,
    > {
        Ok(self
            .value
            .with_cached_dictionary_representation(|entries, buckets| {
                let members = entries
                    .iter()
                    .map(|(key, value)| {
                        (
                            Rc::new(Self {
                                value: VmOriginalValue::Header(key.native_lifetime_lease()),
                                dialect: self.dialect,
                                interpreter: self.interpreter,
                            })
                                as Rc<dyn tcl_engine_api::OriginalObject>,
                            Rc::new(Self {
                                value: VmOriginalValue::Header(value.native_lifetime_lease()),
                                dialect: self.dialect,
                                interpreter: self.interpreter,
                            })
                                as Rc<dyn tcl_engine_api::OriginalObject>,
                        )
                    })
                    .collect();
                (members, Some(buckets))
            }))
    }
    fn apply(
        &self,
        value: &tcl_engine_api::OriginalObjectResult,
        string_mutation: tcl_engine_api::ResidentStringMutation,
    ) -> Result<(), EngineError> {
        let mut reached = value;
        loop {
            reached = match reached {
                tcl_engine_api::OriginalObjectResult::Resident { value, .. } => value,
                tcl_engine_api::OriginalObjectResult::Shared(value) => value,
                _ => break,
            };
        }
        if let tcl_engine_api::OriginalObjectResult::RetainedList(backing) = reached
            && !self.list_backing_matches(backing)?
        {
            return Err(EngineError::ExecutionRefusal(
                "retired original List attachment".into(),
            ));
        }
        let donor = recover_original_result(value, self.dialect, self.interpreter)?;
        self.value
            .adopt_native_object_representation_with_string_mutation(
                &donor,
                self.dialect,
                string_mutation,
            )
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
    }
}

struct VmRegistrar<'a> {
    vm: &'a mut Vm,
    host_commands: HostCommandNames,
}

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

fn open_publication_service(vm: &Vm) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
    vm.native_publication_service()
        .map(|service| Rc::new(VmPublicationService(service)) as Rc<dyn CommandPublicationService>)
        .map_err(native_publication_error)
}

fn define_prepared_host_command(
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

fn remove_prepared_host_command(
    vm: &mut Vm,
    publication: &PreparedCommandPublication,
) -> Result<bool, EngineError> {
    let receipt = publication_receipt(publication)?;
    vm.remove_prepared_native_command(&receipt.service, &receipt.publication)
        .map_err(native_publication_error)
}

fn provide_package(vm: &mut Vm, name: &str, version: &str) -> Result<(), EngineError> {
    vm.package_provide(name, version)
        .map_err(|error| internal_error(error, vm.native_scalar_carrier_dialect()))
}

fn read_variable(vm: &mut Vm, name: &str) -> Result<Value, EngineError> {
    vm.read_variable(name)
        .and_then(|value| {
            from_vm_value(&value, vm.native_scalar_carrier_dialect())
                .map_err(|error| vm.refuse_host_command(error.to_string()))
        })
        .map_err(|completion| script_error(&completion, vm.native_scalar_carrier_dialect()))
}

fn set_variable(vm: &mut Vm, name: &str, value: &Value) -> Result<(), EngineError> {
    vm.write_variable(
        name,
        to_vm_value(value, vm.native_scalar_carrier_dialect())?,
    )
    .map_err(|completion| script_error(&completion, vm.native_scalar_carrier_dialect()))
}

fn unset_variable(vm: &mut Vm, name: &str) -> Result<(), EngineError> {
    vm.unset_variable(name)
        .map_err(|completion| script_error(&completion, vm.native_scalar_carrier_dialect()))
}

/// Evaluate `script` in the VM's current frame and report how it completed: a
/// normal completion, `return`, `break` and `continue` as the code they are, and
/// an error as the error, a budget one as the budget it outran.
fn evaluate(vm: &mut Vm, script: &str) -> Result<HostOutcome, EngineError> {
    let completion = match vm.eval_source(script) {
        Ok(completion) => completion,
        Err(error) => error
            .into_completion()
            .map_err(|failure| EngineError::ExecutionRefusal(failure.to_string()))?,
    };
    let code = match completion.code {
        Code::Ok => CompletionCode::Ok,
        Code::Return => CompletionCode::Return,
        Code::Break => CompletionCode::Break,
        Code::Continue => CompletionCode::Continue,
        Code::Error => {
            let failure = script_error(&completion, vm.native_scalar_carrier_dialect());
            if matches!(
                failure,
                EngineError::Script { .. } | EngineError::ScriptBytes { .. }
            ) {
                // The host command takes the error as its own, so `$errorCode`
                // and `$errorInfo` are what a `catch` of the script would leave.
                vm.publish_caught_error(&completion);
            }
            return Err(failure);
        }
        Code::Other(other) => CompletionCode::Other(other),
    };
    let value = from_vm_value(&completion.result, vm.native_scalar_carrier_dialect())?;
    Ok(if code == CompletionCode::Return {
        HostOutcome::returning(
            value,
            from_vm_value(&completion.options, vm.native_scalar_carrier_dialect())?,
        )
    } else {
        HostOutcome::completing(code, value)
    })
}

/// The error a failed completion is, with the `-errorcode` its options carry and
/// a budget the VM reported as the budget it outran.
fn script_error(
    completion: &Completion<tcl_vm::Value>,
    dialect: tcl_registry::InvocationDialect,
) -> EngineError {
    TclVmEngine::completion_to_result(completion, dialect)
        .expect_err("an error completion has no normal value")
}

/// Export an internal VM failure without rebuilding its guest completion.
fn internal_error(
    error: tcl_vm::TclError,
    dialect: tcl_registry::InvocationDialect,
) -> EngineError {
    match error.into_completion() {
        Ok(completion) => script_error(&completion, dialect),
        Err(failure) => EngineError::ExecutionRefusal(failure.to_string()),
    }
}

/// The message the VM reports when a body outruns `kind` of budget.
fn budget_message(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Commands => "command count limit exceeded",
        BudgetKind::WallClock => "time limit exceeded",
        BudgetKind::ValueSize => "value size limit exceeded",
    }
}

/// The VM's code for a host command's [`CompletionCode`].
fn to_vm_code(code: CompletionCode) -> Code {
    Code::from_int(code.as_int())
}

fn define_host_command(
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

fn remove_host_command(vm: &mut Vm, name: &[u8]) -> Result<bool, EngineError> {
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
        read_variable(self.vm, name)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        set_variable(self.vm, name, &value)
    }

    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        unset_variable(self.vm, name)
    }

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        evaluate(self.vm, script)
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

impl NativeCommand for HostCommandShim {
    fn retire(&self, vm: &mut Vm) -> Completion<tcl_vm::Value> {
        let mut registrar = VmRegistrar {
            vm,
            host_commands: Rc::clone(&self.host_commands),
        };
        match self.command.retire_with_registrar(&mut registrar) {
            Ok(()) => Completion::new(Code::Ok, tcl_vm::Value::empty(), tcl_vm::Value::empty()),
            Err(error) => registrar.vm.refuse_host_command(error.to_string()),
        }
    }

    fn invoke(&self, vm: &mut Vm, arguments: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let dialect = vm.native_scalar_carrier_dialect();
        let interpreter = vm.native_object_bridge_identity();
        let originals: Vec<Rc<dyn tcl_engine_api::OriginalObject>> =
            if self.command.argument_view() == tcl_engine_api::HostArgumentView::OriginalObjects {
                arguments
                    .iter()
                    .map(|value| {
                        Rc::new(VmOriginalObject {
                            value: VmOriginalValue::Header(value.native_lifetime_lease()),
                            dialect,
                            interpreter,
                        }) as Rc<dyn tcl_engine_api::OriginalObject>
                    })
                    .collect()
            } else {
                Vec::new()
            };
        let arguments: Vec<Value> = match arguments
            .iter()
            .map(|value| match self.command.argument_view() {
                tcl_engine_api::HostArgumentView::MaterializedStrings => {
                    from_vm_value(value, dialect)
                }
                tcl_engine_api::HostArgumentView::NativeObjectSnapshots => {
                    from_vm_object_value(value)
                }
                tcl_engine_api::HostArgumentView::OriginalObjects => Ok(Value::Empty),
            })
            .collect::<Result<_, _>>()
        {
            Ok(arguments) => arguments,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        let mut registrar = VmRegistrar {
            vm,
            host_commands: Rc::clone(&self.host_commands),
        };
        let outcome =
            if self.command.argument_view() == tcl_engine_api::HostArgumentView::OriginalObjects {
                self.command
                    .invoke_original_completion_with_registrar(&mut registrar, &originals)
            } else {
                self.command
                    .invoke_with_registrar(&mut registrar, &arguments)
                    .map(|outcome| {
                        Completion::new(
                            to_vm_code(outcome.code),
                            tcl_engine_api::OriginalObjectResult::Value(outcome.value),
                            tcl_engine_api::OriginalObjectResult::Value(outcome.options),
                        )
                    })
            };
        match outcome {
            Ok(result) => {
                match recover_original_completion(
                    &result,
                    registrar.vm.native_scalar_carrier_dialect(),
                    interpreter,
                ) {
                    Ok(completion) => completion,
                    Err(error) => registrar.vm.refuse_host_command(error.to_string()),
                }
            }
            // A script error crosses as the Tcl error it is: the message
            // verbatim, and its `-errorcode` in the completion's options so
            // `catch` and `$errorCode` see what the host command set.
            Err(EngineError::Script { message, code }) => {
                let options = code.map_or_else(tcl_vm::Value::empty, |code| {
                    tcl_vm::Value::list(vec![
                        tcl_vm::Value::string("-code"),
                        tcl_vm::Value::int(1),
                        tcl_vm::Value::string("-level"),
                        tcl_vm::Value::int(0),
                        tcl_vm::Value::string("-errorcode"),
                        tcl_vm::Value::string(code),
                    ])
                });
                Completion::new(Code::Error, tcl_vm::Value::string(message), options)
            }
            Err(EngineError::ScriptBytes {
                message,
                code,
                options,
            }) => {
                let options =
                    options.map_or_else(|| error_options(code), tcl_vm::Value::from_string_bytes);
                Completion::new(
                    Code::Error,
                    tcl_vm::Value::from_string_bytes(message),
                    options,
                )
            }
            Err(EngineError::ExecutionRefusal(reason)) => registrar.vm.refuse_host_command(reason),
            Err(EngineError::BudgetExceeded(kind)) => Completion::new(
                Code::Error,
                tcl_vm::Value::string(budget_message(kind)),
                tcl_vm::Value::empty(),
            ),
            Err(error) => registrar.vm.refuse_host_command(error.to_string()),
        }
    }
}

fn error_options(code: Option<Vec<u8>>) -> tcl_vm::Value {
    code.map_or_else(tcl_vm::Value::empty, |code| {
        tcl_vm::Value::list(vec![
            tcl_vm::Value::string("-code"),
            tcl_vm::Value::int(1),
            tcl_vm::Value::string("-level"),
            tcl_vm::Value::int(0),
            tcl_vm::Value::string("-errorcode"),
            tcl_vm::Value::from_string_bytes(code),
        ])
    })
}

fn recover_original_result(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<tcl_vm::Value, EngineError> {
    recover_original_result_in(
        result,
        dialect,
        interpreter,
        &mut std::collections::BTreeMap::new(),
    )
}

fn recover_original_completion(
    completion: &tcl_engine_api::OriginalObjectCompletion,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<Completion<tcl_vm::Value>, EngineError> {
    let mut memo = std::collections::BTreeMap::new();
    let result = recover_original_result_in(&completion.result, dialect, interpreter, &mut memo)?;
    let options = recover_original_result_in(&completion.options, dialect, interpreter, &mut memo)?;
    Ok(Completion::new(completion.code, result, options))
}

enum OriginalRecoveryWork<'a> {
    Visit(&'a tcl_engine_api::OriginalObjectResult),
    List(usize, bool),
    Dictionary(usize, Option<usize>),
    Resident(&'a Rc<[u8]>, tcl_engine_api::NativeStringStorageIdentity),
    Shared(usize),
}

fn recover_original_leaf(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    Ok(Some(match result {
        R::Value(value) => to_vm_value(value, dialect)?,
        R::Index { cache, origin } => tcl_vm::Value::from_native_index_cache(
            cache.clone(),
            import_version(*origin),
            dialect,
            None,
        )
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        R::String(cache) => recover_native_string_cache(cache, dialect, None)?,
        R::CommandName(_) => {
            return Err(EngineError::ExecutionRefusal(
                "native command-name cache requires resident string storage".into(),
            ));
        }
        R::NamespaceName(cache) => {
            recover_native_namespace_name_cache(cache, dialect, interpreter, None)?
        }
        R::Original(original) => recover_original_handle(original, dialect, interpreter)?,
        R::RetainedList(carrier) => {
            let receipt = checked_list_backing(carrier, dialect, interpreter)?;
            tcl_vm::Value::from_retained_native_list_backing(&receipt.backing, receipt.protocol)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?
        }
        R::Shared(_) | R::List { .. } | R::Dictionary { .. } | R::Resident { .. } => {
            return Ok(None);
        }
    }))
}

fn recover_original_resident(
    value: &tcl_engine_api::OriginalObjectResult,
    string: &Rc<[u8]>,
    storage: tcl_engine_api::NativeStringStorageIdentity,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    let value = match value {
        R::CommandName(cache) => recover_native_command_name_cache(
            cache,
            dialect,
            interpreter,
            Rc::clone(string),
            import_storage(storage),
        )?,
        R::NamespaceName(cache) => recover_native_namespace_name_cache(
            cache,
            dialect,
            interpreter,
            Some((Rc::clone(string), import_storage(storage))),
        )?,
        R::String(cache) => recover_native_string_cache(
            cache,
            dialect,
            Some((Rc::clone(string), import_storage(storage))),
        )?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn recover_original_result_in(
    result: &tcl_engine_api::OriginalObjectResult,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
    memo: &mut std::collections::BTreeMap<usize, tcl_vm::Value>,
) -> Result<tcl_vm::Value, EngineError> {
    use OriginalRecoveryWork as Work;
    use tcl_engine_api::OriginalObjectResult as R;
    let mut work = vec![Work::Visit(result)];
    let mut values = Vec::new();
    let mut active = std::collections::BTreeSet::new();
    while let Some(next) = work.pop() {
        if let Work::Visit(result) = &next
            && let Some(value) = recover_original_leaf(result, dialect, interpreter)?
        {
            values.push(value);
            continue;
        }
        match next {
            Work::Visit(R::Shared(node)) => {
                let identity = Rc::as_ptr(node) as usize;
                if let Some(value) = memo.get(&identity) {
                    values.push(value.clone());
                } else {
                    if !active.insert(identity) {
                        return Err(EngineError::ExecutionRefusal(
                            "cyclic callback object graph".into(),
                        ));
                    }
                    work.push(Work::Shared(identity));
                    work.push(Work::Visit(node));
                }
            }
            Work::Visit(R::List { items, canonical }) => {
                work.push(Work::List(items.len(), *canonical));
                work.extend(items.iter().rev().map(Work::Visit));
            }
            Work::Visit(R::Dictionary { entries, buckets }) => {
                work.push(Work::Dictionary(entries.len(), *buckets));
                for (key, value) in entries.iter().rev() {
                    work.push(Work::Visit(value));
                    work.push(Work::Visit(key));
                }
            }
            Work::Visit(R::Resident {
                value,
                string,
                storage,
            }) => {
                if let Some(recovered) =
                    recover_original_resident(value, string, *storage, dialect, interpreter)?
                {
                    values.push(recovered);
                } else {
                    work.push(Work::Resident(string, *storage));
                    work.push(Work::Visit(value));
                }
            }
            Work::Visit(_) => unreachable!("original leaf already recovered"),
            Work::List(count, canonical) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .expect("visited callback List children");
                let items = values.split_off(start);
                let protocol = dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal("callback List updater issuer unavailable".into())
                })?;
                values.push(tcl_vm::Value::list_with_native_canonical_in(
                    items, canonical, protocol,
                ));
            }
            Work::Dictionary(count, buckets) => {
                let protocol = dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal(
                        "callback Dictionary updater issuer unavailable".into(),
                    )
                })?;
                let value = recover_dictionary_children(&mut values, count, buckets, protocol)?;
                values.push(value);
            }
            Work::Resident(string, storage) => {
                let value = values
                    .pop()
                    .expect("visited resident callback representation");
                values.push(
                    value
                        .with_resident_string_bytes_and_storage(
                            Rc::clone(string),
                            import_storage(storage),
                        )
                        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
                );
            }
            Work::Shared(identity) => {
                memo.insert(
                    identity,
                    values.last().expect("visited callback graph node").clone(),
                );
                active.remove(&identity);
            }
        }
    }
    Ok(values.pop().expect("visited callback result"))
}

fn checked_list_backing_receipt(
    carrier: &tcl_engine_api::NativeListBacking,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<&VmListBackingReceipt, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmListBackingReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign native List backing authority".into())
        })?;
    let protocol = dialect
        .native_string_protocol()
        .ok_or_else(|| EngineError::ExecutionRefusal("native List issuer unavailable".into()))?;
    if receipt.interpreter != interpreter
        || carrier.scope_identity() != interpreter
        || receipt.protocol != protocol
        || protocol.tcl_version().map(export_version) != Some(carrier.origin())
        || carrier.member_count() != receipt.backing.len()
        || !Rc::ptr_eq(
            &carrier.canonical_state(),
            &receipt.backing.canonical_state(),
        )
        || !receipt.backing.is_whole_backing()
    {
        return Err(EngineError::ExecutionRefusal(
            "native List backing scope, issuer or shape mismatch".into(),
        ));
    }
    Ok(receipt)
}

fn checked_list_backing(
    carrier: &tcl_engine_api::NativeListBacking,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<&VmListBackingReceipt, EngineError> {
    let receipt = checked_list_backing_receipt(carrier, dialect, interpreter)?;
    if !receipt.backing.has_native_header() {
        return Err(EngineError::ExecutionRefusal(
            "retired native List backing".into(),
        ));
    }
    Ok(receipt)
}

fn recover_original_handle(
    original: &Rc<dyn tcl_engine_api::OriginalObject>,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
) -> Result<tcl_vm::Value, EngineError> {
    let Some(receipt) = original.engine_receipt().downcast_ref::<VmOriginalObject>() else {
        return Err(EngineError::ExecutionRefusal(
            "foreign original-object engine authority".into(),
        ));
    };
    if receipt.interpreter != interpreter
        || receipt.native_c_version()
            != dialect
                .native_scalar_getter_protocol()
                .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
                .map(export_version)
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign original-object interpreter or native issuer".into(),
        ));
    }
    Ok((*receipt.value).clone())
}

fn recover_dictionary_children(
    values: &mut Vec<tcl_vm::Value>,
    count: usize,
    buckets: Option<usize>,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<tcl_vm::Value, EngineError> {
    let start = values
        .len()
        .checked_sub(count * 2)
        .expect("visited callback Dictionary children");
    let children = values.split_off(start);
    let mut children = children.into_iter();
    let entries = (0..count)
        .map(|_| {
            (
                children.next().expect("key"),
                children.next().expect("value"),
            )
        })
        .collect();
    tcl_vm::Value::native_dictionary_constructor(entries, buckets, protocol)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

fn recover_native_namespace_name_cache(
    carrier: &tcl_engine_api::NativeNamespaceNameCache,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
) -> Result<tcl_vm::Value, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmNamespaceNameCacheReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign namespace-name cache authority".into())
        })?;
    let cache = &receipt.cache;
    if carrier.scope_identity() != interpreter
        || receipt.interpreter != interpreter
        || (cache.interpreter().owner, cache.interpreter().interpreter) != interpreter
        || carrier.origin() != export_version(cache.version())
        || dialect
            .native_namespace_name_protocol()
            .is_none_or(|protocol| protocol.recipe().version() != cache.version())
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign namespace-name cache interpreter or origin".into(),
        ));
    }
    tcl_vm::Value::from_native_namespace_name_cache(cache.clone(), dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

fn recover_native_command_name_cache(
    carrier: &tcl_engine_api::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
    resident: Rc<[u8]>,
    storage: tcl_syntax::native_string::NativeStringStorageIdentity,
) -> Result<tcl_vm::Value, EngineError> {
    let receipt = carrier
        .engine_receipt()
        .downcast_ref::<VmCommandNameCacheReceipt>()
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("foreign command-name cache authority".into())
        })?;
    if carrier.scope_identity() != interpreter
        || receipt.interpreter != interpreter
        || carrier.origin() != export_version(receipt.version)
        || !dialect
            .native_command_name_protocol()
            .is_some_and(|protocol| protocol.accepts_cache_origin(receipt.version))
        || receipt.cache.as_ref().is_some_and(|cache| {
            (cache.interpreter.owner, cache.interpreter.interpreter) != interpreter
                || cache.version != receipt.version
        })
    {
        return Err(EngineError::ExecutionRefusal(
            "foreign command-name cache interpreter or origin".into(),
        ));
    }
    let value = tcl_vm::Value::new_native_string_bytes(Rc::clone(&resident))
        .with_resident_string_bytes_and_storage(resident, storage)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
    if let Some(cache) = &receipt.cache {
        value.install_native_command_name_cache(cache.clone(), dialect)
    } else {
        value.install_unresolved_native_command_name_cache(dialect)
    }
    .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
    Ok(value)
}

fn recover_native_string_cache(
    cache: &tcl_engine_api::NativeStringCache,
    dialect: tcl_registry::InvocationDialect,
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
) -> Result<tcl_vm::Value, EngineError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as S;
    let cache = match cache {
        tcl_engine_api::NativeStringCache::C {
            origin,
            num_chars,
            unicode,
        } => S::String {
            protocol: tcl_syntax::native_string::NativeStringProtocol::C(import_version(*origin)),
            num_chars: *num_chars,
            unicode: unicode.clone(),
        },
        tcl_engine_api::NativeStringCache::Jim { num_chars } => S::JimString {
            num_chars: *num_chars,
        },
    };
    tcl_vm::Value::from_native_string_cache(cache, dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

/// Convert an interface value to the VM's representation.
///
/// A Dictionary retains its member objects without a string round-trip.
fn to_vm_value(
    value: &Value,
    dialect: tcl_registry::InvocationDialect,
) -> Result<tcl_vm::Value, EngineError> {
    let refusal = |error: &dyn std::fmt::Display| EngineError::ExecutionRefusal(error.to_string());
    Ok(match value {
        Value::Empty => tcl_vm::Value::string(String::new()),
        Value::Str(text) => tcl_vm::Value::from_string_bytes(text.as_bytes()),
        Value::StringBytes(bytes) => tcl_vm::Value::from_string_bytes(bytes.as_ref()),
        Value::ByteArray(bytes) => tcl_vm::Value::byte_array(Rc::clone(bytes)),
        Value::NativeScalar(cache) => {
            let (cache, origin) = import_scalar_cache(cache);
            tcl_vm::Value::from_native_scalar_cache(cache, origin, dialect)
                .map_err(|error| refusal(&error))?
        }
        Value::Resident {
            value,
            string,
            storage,
        } => {
            let storage = import_storage(*storage);
            if let Value::NativeScalar(cache) = value.as_ref() {
                let (cache, origin) = import_scalar_cache(cache);
                tcl_vm::Value::from_native_scalar_cache_with_storage(
                    cache,
                    origin,
                    dialect,
                    Some((Rc::clone(string), storage)),
                )
                .map_err(|error| refusal(&error))?
            } else {
                to_vm_value(value, dialect)?
                    .with_resident_string_bytes_and_storage(Rc::clone(string), storage)
                    .map_err(|error| refusal(&error))?
            }
        }
        Value::Int(number) => tcl_vm::Value::int(*number),
        Value::Double(number) => tcl_vm::Value::double(*number),
        Value::List(items) => tcl_vm::Value::native_list_constructor(
            items
                .iter()
                .map(|value| to_vm_value(value, dialect))
                .collect::<Result<_, _>>()?,
            dialect.native_string_protocol().ok_or_else(|| {
                EngineError::ExecutionRefusal("List import updater issuer unavailable".into())
            })?,
        ),
        Value::Dict(entries) => tcl_vm::Value::native_dictionary_constructor(
            entries
                .iter()
                .map(|(key, value)| Ok((to_vm_value(key, dialect)?, to_vm_value(value, dialect)?)))
                .collect::<Result<_, EngineError>>()?,
            None,
            dialect.native_string_protocol().ok_or_else(|| {
                EngineError::ExecutionRefusal("Dictionary import updater issuer unavailable".into())
            })?,
        )
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
    })
}

/// Convert a VM value to the interface's representation.
///
/// Deliberately string-shaped. The VM's value is dual-rep, and asking it "are
/// you *really* a list" is not a question — it is a *conversion*, which
/// installs a list intrep on every string that happens to parse as one and
/// changes what the body's own later use of that value costs. The host parses
/// what its protocol expects; the boundary reports what the body produced.
fn import_version(version: tcl_engine_api::NativeCVersion) -> tcl_dialect::TclVersion {
    tcl_syntax::scalar_getter::carrier::import_version(version)
}
fn export_version(version: tcl_dialect::TclVersion) -> tcl_engine_api::NativeCVersion {
    tcl_syntax::scalar_getter::carrier::export_version(version)
}
fn import_storage(
    storage: tcl_engine_api::NativeStringStorageIdentity,
) -> tcl_syntax::native_string::NativeStringStorageIdentity {
    tcl_syntax::scalar_getter::carrier::import_storage(storage)
}
fn export_storage(value: &tcl_vm::Value) -> tcl_engine_api::NativeStringStorageIdentity {
    tcl_syntax::scalar_getter::carrier::export_storage(
        value
            .resident_string_storage_identity()
            .unwrap_or(tcl_syntax::native_string::NativeStringStorageIdentity::Unknown),
    )
}
fn import_scalar_cache(
    cache: &tcl_engine_api::NativeScalarCache,
) -> (
    tcl_syntax::scalar_getter::NativeScalarCache,
    Option<tcl_dialect::TclVersion>,
) {
    tcl_syntax::scalar_getter::carrier::import_scalar(cache)
}
fn export_scalar_cache(
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    origin: Option<tcl_dialect::TclVersion>,
) -> Result<tcl_engine_api::NativeScalarCache, EngineError> {
    tcl_syntax::scalar_getter::carrier::export_scalar(cache, origin)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

fn from_vm_object_value(value: &tcl_vm::Value) -> Result<Value, EngineError> {
    let payload = if let Some(bytes) = value.byte_array_representation() {
        Value::byte_array(bytes)
    } else if let Some(cache) = value.native_scalar_cache() {
        Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_word_boolean_version(),
        )?)
    } else if value.has_list_representation() {
        if !matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::List {
                canonical: true,
                ..
            }
        ) {
            return Err(EngineError::ExecutionRefusal(
                "native noncanonical list carrier is unavailable".into(),
            ));
        }
        let (items, _) = value.cached_list_representation().ok_or_else(|| {
            EngineError::ExecutionRefusal("native List carrier cache is unavailable".into())
        })?;
        Value::list(
            items
                .iter()
                .map(from_vm_object_value)
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        // Other primary caches need their own explicit carrier. This view may
        // not convert a stringless object to obtain a convenient representation.
        if !matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        ) {
            return Err(EngineError::ExecutionRefusal(
                "native object cache carrier is unavailable".into(),
            ));
        }
        let Some(bytes) = value.resident_string_bytes() else {
            return Err(EngineError::ExecutionRefusal(
                "native object cache carrier is unavailable".into(),
            ));
        };
        return Ok(Value::Empty.with_resident_string_storage(bytes, export_storage(value)));
    };
    Ok(if let Some(bytes) = value.resident_string_bytes() {
        payload.with_resident_string_storage(bytes, export_storage(value))
    } else {
        payload
    })
}

fn from_vm_value(
    value: &tcl_vm::Value,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Value, EngineError> {
    if let Some(bytes) = value.byte_array_representation() {
        let payload = Value::byte_array(bytes);
        return Ok(if let Some(string) = value.resident_string_bytes() {
            payload.with_resident_string_storage(string, export_storage(value))
        } else {
            payload
        });
    }
    if let Some(cache) = value.native_scalar_cache() {
        let payload = Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_word_boolean_version(),
        )?);
        // Retain the bridge's string-valued host view while carrying the full
        // primary cache. Numeric string generation does not narrow that cache.
        let string = native_string_bytes(value, dialect)?;
        return Ok(payload.with_resident_string_storage(string, export_storage(value)));
    }
    Ok(Value::string_bytes(native_string_bytes(value, dialect)?))
}

fn native_string_bytes(
    value: &tcl_vm::Value,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Rc<[u8]>, EngineError> {
    let protocol = dialect.native_string_protocol().ok_or_else(|| {
        EngineError::ExecutionRefusal(
            "native string materialisation protocol is unavailable".into(),
        )
    })?;
    value
        .native_string_bytes(protocol)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

/// Hands the thread back the numeral grammar it had when dropped.
///
/// `Vm::set_dialect_profile` installs the pinned release's grammar for the
/// whole thread, and the VM claims its own again only at its script entry
/// points, not at [`Vm::invoke_command`]. An engine operation therefore
/// claims the engine's grammar for its duration and restores the caller's on
/// every exit path, a caught panic included.
struct GrammarGuard(tcl_syntax::number::NumberSyntax);

impl GrammarGuard {
    /// Install `syntax` for the guard's lifetime.
    fn claim(syntax: tcl_syntax::number::NumberSyntax) -> Self {
        let saved = tcl_syntax::number::runtime_syntax();
        tcl_syntax::number::set_runtime_syntax(syntax);
        Self(saved)
    }

    /// Keep whatever the guarded operation installs, restoring the
    /// caller's grammar afterwards.
    fn keep() -> Self {
        Self(tcl_syntax::number::runtime_syntax())
    }
}

impl Drop for GrammarGuard {
    fn drop(&mut self) {
        tcl_syntax::number::set_runtime_syntax(self.0);
    }
}

/// The `tcl-vm` engine.
pub struct TclVmEngine {
    vm: Vm,
    budget: Budget,
    /// The profile [`Engine::set_release`] pinned, by canonical name.
    release: Option<&'static str>,
    /// Mints the internal procedure name each compiled unit is defined as.
    units: u32,
    /// Command generations registered through [`Engine::define_command`] or a
    /// running command's registrar, kept so a later
    /// [`Engine::restrict_commands`] does not remove them.
    host_commands: HostCommandNames,
    /// Actual procedure generations captured for compiled units — likewise kept, since a
    /// unit compiled before the whitelist was applied must still be callable
    /// after it.
    unit_commands: Vec<tcl_vm::NativeRegisteredCommandToken>,
}

impl TclVmEngine {
    /// A fresh engine with the default command registry driving its compiler.
    #[must_use]
    pub fn new() -> Self {
        Self::with_registry(CommandRegistry::build_default())
    }

    /// A fresh engine whose compiler resolves against `registry` and whose
    /// native issuer is the C9.0 contract exposed by this adapter.
    #[must_use]
    pub fn with_registry(registry: CommandRegistry) -> Self {
        // Building the VM pins its default release, which installs that
        // release's grammar for the whole thread.
        let _grammar = GrammarGuard::keep();
        let mut vm = Vm::new();
        let native = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        vm.set_dialect_profile(native);
        assert!(vm.set_native_engine_profile(native));
        vm.set_compiler(Box::new(BytecodeCompileService::new(registry)));
        Self {
            vm,
            budget: Budget::default(),
            release: None,
            units: 0,
            host_commands: Rc::new(RefCell::new(Vec::new())),
            unit_commands: Vec::new(),
        }
    }

    /// The underlying VM, for an embedder that needs a VM-specific facility
    /// the interface does not expose. Using it makes that code engine-specific
    /// by construction, which is the point of it being a separate accessor.
    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }

    /// The profile [`Engine::set_release`] pinned, by canonical name.
    #[must_use]
    pub fn release(&self) -> Option<&'static str> {
        self.release
    }

    /// Claim this engine's release grammar for the length of one operation.
    fn claim_grammar(&self) -> GrammarGuard {
        GrammarGuard::claim(self.vm.runtime_version().number_syntax())
    }

    /// Translate a VM completion into the interface's result, mapping the
    /// budget errors the VM reports as ordinary Tcl errors back to
    /// [`EngineError::BudgetExceeded`] — the host must be able to tell "your
    /// hook is too expensive" from "your hook is wrong".
    fn completion_to_result(
        completion: &Completion<tcl_vm::Value>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Value, EngineError> {
        if completion.code.is_ok() || completion.code == Code::Return {
            return from_vm_value(&completion.result, dialect);
        }
        let message = native_string_bytes(&completion.result, dialect)?;
        match message.as_ref() {
            b"command count limit exceeded" => {
                Err(EngineError::BudgetExceeded(BudgetKind::Commands))
            }
            b"time limit exceeded" => Err(EngineError::BudgetExceeded(BudgetKind::WallClock)),
            b"value size limit exceeded" => Err(EngineError::BudgetExceeded(BudgetKind::ValueSize)),
            _ => {
                let protocol = dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal(
                        "native completion option protocol is unavailable".into(),
                    )
                })?;
                let code = if let Some(code) = completion
                    .options
                    .with_cached_dictionary_representation(|pairs, _| {
                        Self::completion_error_code(
                            pairs.iter().map(|(key, value)| (key, value)),
                            dialect,
                        )
                    }) {
                    code?
                } else {
                    let items = completion
                        .options
                        .native_object_list_elements(protocol)
                        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
                    Self::completion_error_code(
                        items
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|[key, value]| (key, value)),
                        dialect,
                    )?
                };
                let options = native_string_bytes(&completion.options, dialect)?;
                if options.is_empty() {
                    Err(EngineError::script_bytes(message.to_vec(), code))
                } else {
                    Err(EngineError::ScriptBytes {
                        message: message.to_vec(),
                        code,
                        options: Some(options.to_vec()),
                    })
                }
            }
        }
    }

    fn completion_error_code<'a>(
        pairs: impl Iterator<Item = (&'a tcl_vm::Value, &'a tcl_vm::Value)>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Option<Vec<u8>>, EngineError> {
        for (key, value) in pairs {
            if native_string_bytes(key, dialect)?.as_ref() == b"-errorcode" {
                return Ok(Some(native_string_bytes(value, dialect)?.to_vec()));
            }
        }
        Ok(None)
    }
}

impl Default for TclVmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for TclVmEngine {
    type Handle = VmHandle;

    fn name(&self) -> &'static str {
        "tclvm"
    }

    fn command_publication_service(
        &mut self,
    ) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
        open_publication_service(&self.vm)
    }
    fn define_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_prepared_host_command(&mut self.vm, &self.host_commands, &publication, command)
    }
    fn remove_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        remove_prepared_host_command(&mut self.vm, &publication)
    }
    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(&mut self.vm, &self.host_commands, name, command)
    }
    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        remove_host_command(&mut self.vm, name)
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
        provide_package(&mut self.vm, name, version)
    }

    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        self.vm.library_loaded(file_name, prefix);
        Ok(())
    }

    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        let _grammar = self.claim_grammar();
        read_variable(&mut self.vm, name)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        let _grammar = self.claim_grammar();
        set_variable(&mut self.vm, name, &value)
    }

    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        let _grammar = self.claim_grammar();
        unset_variable(&mut self.vm, name)
    }

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        let _grammar = self.claim_grammar();
        evaluate(&mut self.vm, script)
    }

    /// Keep only the `allowed` commands, the host's and the compiled units'.
    /// An allowed `expr` keeps its math functions: from 8.5 each is a
    /// command, `tcl::mathfunc::NAME`, which the expression calls, so
    /// stripping them would make `expr {abs(-1)}` fold under an engine
    /// pinned to 8.4, whose functions are builtins, and decline under every
    /// later one. `rand` and `srand` go: their seed is interpreter state
    /// one invocation would leave for the next (a confined VM refuses them
    /// under 8.4 as well). An allowed ensemble keeps its subcommands: the
    /// compiler lowers `string trim` to a call of `::tcl::string::trim`, so
    /// naming `string` names them too.
    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        let mut tokens = self.host_commands.borrow().clone();
        tokens.extend(self.unit_commands.iter().cloned());
        tokens.extend(
            allowed
                .iter()
                .filter_map(|name| self.vm.registered_command_token_bytes(name.as_bytes())),
        );
        let tokens = self.vm.registered_stock_implementation_tokens(&tokens);
        self.vm.retain_registered_command_tokens(&tokens);
        Ok(())
    }

    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        let _grammar = self.claim_grammar();
        self.units += 1;
        // An ordinary qualified name the restriction keeps, so a body can
        // spell a sibling unit's (`::spectcl::unit::N`) and call it. An
        // engine serves one pack, pinned to one release, so what it reaches
        // is the same pack's code, never another pack's; a call is charged
        // to the invocation's command budget like any other, so a unit that
        // recurses raises — the budget or the nesting limit — and its
        // caller declines. Nothing in the sandbox defines a command, so no
        // body can shadow a unit.
        let procedure = format!("::spectcl::unit::{}", self.units);
        self.vm
            .try_define_procedure(&procedure, unit.parameters, unit.body)
            .map_err(|error| EngineError::Compile(error.to_string()))?;
        // The VM's command table is keyed by the canonical *unrooted* name, so
        // that is what the whitelist sweep compares against.
        let token = self
            .vm
            .registered_command_token_bytes(procedure.as_bytes())
            .ok_or_else(|| {
                EngineError::ExecutionRefusal("compiled procedure token is unavailable".into())
            })?;
        self.unit_commands.push(token.clone());
        Ok(VmHandle {
            procedure,
            parameters: unit.parameters.len(),
            token,
        })
    }

    fn invoke(&mut self, handle: &Self::Handle, arguments: &[Value]) -> Result<Value, EngineError> {
        if self
            .vm
            .registered_command_token_bytes(handle.procedure.as_bytes())
            != Some(handle.token.clone())
        {
            return Err(EngineError::ExecutionRefusal(
                "compiled procedure binding was retired or replaced".into(),
            ));
        }
        if arguments.len() != handle.parameters {
            return Err(EngineError::Script {
                message: format!(
                    "wrong # args: unit takes {} argument(s), got {}",
                    handle.parameters,
                    arguments.len()
                ),
                code: None,
            });
        }
        let dialect = self.vm.native_scalar_carrier_dialect();
        let arguments: Vec<tcl_vm::Value> = arguments
            .iter()
            .map(|value| to_vm_value(value, dialect))
            .collect::<Result<_, _>>()?;
        let _grammar = self.claim_grammar();
        // Fuel is per invocation, so refill before every call rather than
        // letting a long-lived engine starve its own later hooks.
        self.vm.reset_command_count();
        if let Some(wall_clock) = self.budget.wall_clock {
            self.vm.set_wall_clock_budget(Some(wall_clock));
        }
        let completion = self
            .vm
            .try_invoke_command(&handle.procedure, &arguments)
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
        Self::completion_to_result(&completion, self.vm.native_scalar_carrier_dialect())
    }

    fn set_budget(&mut self, budget: Budget) -> Result<(), EngineError> {
        self.budget = budget;
        self.vm.set_command_limit(budget.commands);
        self.vm.set_wall_clock_budget(budget.wall_clock);
        self.vm.set_value_size_limit(budget.max_value_bytes);
        Ok(())
    }

    fn commands_spent(&self) -> Option<u64> {
        Some(self.vm.commands_run())
    }

    /// Pin the VM to the profile `profile` names. The name resolves through
    /// the one dialect ingress to its catalogue profile. The lenient sink,
    /// the `tk` library environment, a ladder-less dialect (`jim`) and an
    /// unknown name all name no release this VM can run, so each is
    /// `Unsupported` rather than a silent run at the VM's default. A second,
    /// different pin after a unit was compiled is refused as well: the VM
    /// does not switch release under compiled code.
    fn set_release(&mut self, profile: &str) -> Result<(), EngineError> {
        let Some(resolved) = tcl_registry::model::resolve_known_environment(profile)
            .and_then(|environment| environment.catalogue_profile())
        else {
            return Err(EngineError::Unsupported("pinning a release"));
        };
        if self.release == Some(resolved.name) {
            return Ok(());
        }
        if self.units > 0 {
            return Err(EngineError::Unsupported(
                "pinning a release after a unit was compiled",
            ));
        }
        let _grammar = GrammarGuard::keep();
        self.vm.set_dialect_profile(resolved);
        self.release = Some(resolved.name);
        Ok(())
    }

    fn confine_stores(&mut self) -> Result<(), EngineError> {
        self.vm.set_stores_confined(true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use super::{Engine, TclVmEngine, recover_dictionary_children, register_host_command};
    use tcl_engine_api::{
        Budget, BudgetKind, CommandRegistrar, CompileUnit, EngineError, HostCommand, HostOutcome,
        Value,
    };

    #[test]
    fn retained_list_bridge_keeps_actual_storage_and_native_header_sharing() {
        use tcl_engine_api::{OriginalObject, OriginalObjectResult as R, ResidentStringMutation};
        let vm = tcl_vm::Vm::new();
        let dialect = vm.native_scalar_carrier_dialect();
        let scope = vm.native_object_bridge_identity();
        let protocol = dialect.native_string_protocol().unwrap();
        let child = tcl_vm::Value::new_native_string_bytes(b"member".as_slice());
        let original =
            tcl_vm::Value::list_with_native_canonical_in(vec![child.clone()], false, protocol);
        drop(child);
        let capability = super::VmOriginalObject {
            value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
            dialect,
            interpreter: scope,
        };
        let carrier = capability.list_backing().unwrap().unwrap();
        assert!(!carrier.is_shared());
        let members = capability.list_members().unwrap().unwrap().0;
        assert!(!members[0].is_shared());
        let second_view = capability.list_members().unwrap().unwrap().0;
        assert!(!members[0].is_shared());
        let duplicate = capability.duplicate_native_header().unwrap();
        assert!(carrier.is_shared());
        let duplicate_backing = duplicate.list_backing().unwrap().unwrap();
        assert!(duplicate.list_backing_matches(&carrier).unwrap());
        assert!(capability.list_backing_matches(&duplicate_backing).unwrap());
        assert!(!members[0].is_shared());
        drop(duplicate);
        assert!(!carrier.is_shared());
        assert_eq!(
            original.native_string_bytes(protocol).unwrap().as_ref(),
            b"member"
        );
        assert!(carrier.canonical_state().get());
        let resident = R::Resident {
            value: Box::new(R::RetainedList(carrier.clone())),
            string: original.resident_string_bytes().unwrap(),
            storage: super::export_storage(&original),
        };
        capability
            .apply(&resident, ResidentStringMutation::Preserve)
            .unwrap();
        assert!(capability.list_backing_matches(&carrier).unwrap());
        assert!(!carrier.is_shared());
        assert_eq!(members[0].identity(), second_view[0].identity());
        assert!(super::checked_list_backing(&carrier, dialect, (scope.0 + 1, scope.1)).is_err());
        let forged = tcl_engine_api::NativeListBacking::new(
            carrier.origin(),
            scope,
            1,
            carrier.canonical_state(),
            Rc::new(|| false),
            Rc::new(()),
        );
        assert!(super::checked_list_backing(&forged, dialect, scope).is_err());
        let replacement = tcl_vm::Value::new_native_string_bytes(b"replacement".as_slice());
        original
            .adopt_native_object_representation(&replacement, dialect)
            .unwrap();
        assert!(!members[0].is_shared());
        assert!(!capability.list_backing_matches(&carrier).unwrap());
        assert!(super::checked_list_backing(&carrier, dialect, scope).is_err());
        assert!(
            capability
                .apply(&R::RetainedList(carrier), ResidentStringMutation::Preserve)
                .is_err()
        );
    }

    #[test]
    fn callback_cmdname_retains_private_scope_and_distinct_duplicate_identity() {
        use tcl_engine_api::{OriginalObject, OriginalObjectResult as R};
        use tcl_runtime_api::native_command_name::NativeCommandNameCache;
        use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
        let vm = tcl_vm::Vm::new();
        let dialect = vm.native_scalar_carrier_dialect();
        let scope = vm.native_object_bridge_identity();
        let original = tcl_vm::Value::new_native_string_bytes(b"head".as_slice());
        let cache = NativeCommandNameCache {
            interpreter: NativeInterpreterIdentity {
                owner: scope.0,
                interpreter: scope.1,
            },
            version: tcl_dialect::TclVersion::V9_0,
            slot: tcl_runtime_api::CommandSlot::new(
                tcl_runtime_api::ByteNamespacePath::root(),
                b"head".as_slice().into(),
            ),
            namespace_token: 0,
            token: 17,
            implementation_generation: 17,
            command_epoch: 0,
            reference: None,
        };
        original
            .install_native_command_name_cache(cache.clone(), dialect)
            .unwrap();
        let capability = Rc::new(super::VmOriginalObject {
            value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
            dialect,
            interpreter: scope,
        });
        let retained = R::Original(capability.clone());
        let same = super::recover_original_result(&retained, dialect, scope).unwrap();
        assert_eq!(
            same.native_object_identity(),
            original.native_object_identity()
        );
        let carrier = capability.command_name_cache().unwrap();
        let duplicate = R::Resident {
            value: Box::new(R::CommandName(carrier.clone())),
            string: Rc::from(b"head".as_slice()),
            storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
        };
        let restored = super::recover_original_result(&duplicate, dialect, scope).unwrap();
        assert_ne!(
            restored.native_object_identity(),
            original.native_object_identity()
        );
        assert_eq!(restored.native_command_name_cache(), Some(cache));
        capability
            .apply(&duplicate, tcl_engine_api::ResidentStringMutation::Preserve)
            .unwrap();
        assert!(original.native_command_name_cache().is_some());
        assert!(
            duplicate.snapshot().is_err(),
            "opaque native descriptor is not a data snapshot"
        );
        let foreign = tcl_vm::Vm::new().native_object_bridge_identity();
        assert!(super::recover_original_result(&duplicate, dialect, foreign).is_err());
        let forged = R::Resident {
            value: Box::new(R::CommandName(tcl_engine_api::NativeCommandNameCache::new(
                tcl_engine_api::NativeCVersion::V9_0,
                scope,
                Rc::new(()),
            ))),
            string: Rc::from(b"head".as_slice()),
            storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
        };
        assert!(super::recover_original_result(&forged, dialect, scope).is_err());
    }

    #[test]
    fn callback_namespace_primary_retains_private_scope_and_descriptor() {
        use tcl_engine_api::{OriginalObject, OriginalObjectResult as R};
        let mut vm = tcl_vm::Vm::new();
        let dialect = vm.native_scalar_carrier_dialect();
        let scope = vm.native_object_bridge_identity();
        let completion = vm.invoke_command("namespace", &[tcl_vm::Value::string("current")]);
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        let original = completion.result;
        let cache = original
            .native_namespace_name_cache()
            .expect("actual C9 current producer");
        let capability = Rc::new(super::VmOriginalObject {
            value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
            dialect,
            interpreter: scope,
        });
        let carrier = capability.namespace_name_cache().unwrap();
        let retained = R::Resident {
            value: Box::new(R::NamespaceName(carrier.clone())),
            string: original.resident_string_bytes().unwrap(),
            storage: super::export_storage(&original),
        };
        let restored = super::recover_original_result(&retained, dialect, scope).unwrap();
        assert_ne!(
            restored.native_object_identity(),
            original.native_object_identity()
        );
        assert!(cache.same_descriptor(&restored.native_namespace_name_cache().unwrap()));
        assert!(retained.snapshot().is_err());
        assert!(
            super::recover_original_result(&R::NamespaceName(carrier.clone()), dialect, scope)
                .is_err()
        );
        assert!(
            super::recover_original_result(&retained, dialect, (scope.0 + 1, scope.1)).is_err()
        );
        let forged = R::Resident {
            value: Box::new(R::NamespaceName(
                tcl_engine_api::NativeNamespaceNameCache::new(carrier.origin(), scope, Rc::new(())),
            )),
            string: Rc::from(b"::".as_slice()),
            storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
        };
        assert!(super::recover_original_result(&forged, dialect, scope).is_err());
        let duplicate = capability.duplicate_native_header().unwrap();
        let duplicate = duplicate.namespace_name_cache().unwrap();
        let receipt = duplicate
            .engine_receipt()
            .downcast_ref::<super::VmNamespaceNameCacheReceipt>()
            .unwrap();
        assert!(cache.same_descriptor(&receipt.cache));
    }

    #[test]
    fn original_result_requires_its_actual_interpreter_and_native_issuer() {
        let first = tcl_vm::Vm::new();
        let second = tcl_vm::Vm::new();
        let original = tcl_vm::Value::from_native_string_bytes(b"a\0\xff".as_slice());
        let dialect = first.native_scalar_carrier_dialect();
        let scope = first.native_object_bridge_identity();
        let result = tcl_engine_api::OriginalObjectResult::Original(std::rc::Rc::new(
            super::VmOriginalObject {
                value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
                dialect,
                interpreter: scope,
            },
        ));
        let result = tcl_engine_api::OriginalObjectResult::Shared(std::rc::Rc::new(result));
        let returned = super::recover_original_result(&result, dialect, scope).unwrap();
        assert_eq!(
            returned.native_object_identity(),
            original.native_object_identity()
        );
        assert_eq!(
            returned.resident_string_bytes().unwrap().as_ref(),
            b"a\0\xff"
        );
        assert!(matches!(
            super::recover_original_result(
                &result,
                dialect,
                second.native_object_bridge_identity()
            ),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(matches!(
            super::recover_original_result(
                &result,
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
                scope,
            ),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }

    #[test]
    fn native_callback_view_does_not_materialize_original_scalar_storage() {
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let original = tcl_vm::Value::from_native_scalar_cache(
            tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Double(1.5),
            ),
            None,
            dialect,
        )
        .unwrap();
        let carrier = super::from_vm_object_value(&original).unwrap();
        assert!(
            matches!(carrier,tcl_engine_api::Value::NativeScalar(tcl_engine_api::NativeScalarCache::Double(value)) if value.to_bits() == 1.5_f64.to_bits())
        );
        assert!(original.resident_string_bytes().is_none());
        let copied = super::to_vm_value(&carrier, dialect).unwrap();
        assert!(copied.resident_string_bytes().is_none());
        assert!(
            matches!(copied.native_scalar_cache(),Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Double(value))) if value.to_bits() == 1.5_f64.to_bits())
        );
    }

    #[test]
    fn compiled_handle_does_not_invoke_a_replacement_binding() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(CompileUnit {
                name: "binding",
                parameters: &[],
                body: "return ORIGINAL",
            })
            .unwrap();
        assert_eq!(
            engine.invoke(&handle, &[]).unwrap().as_str(),
            Some("ORIGINAL")
        );
        engine
            .vm
            .try_define_procedure(handle.procedure(), &[], "return REPLACED")
            .unwrap();
        assert!(matches!(
            engine.invoke(&handle, &[]),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }

    struct Collector {
        emitted: RefCell<Vec<Vec<String>>>,
    }

    impl HostCommand for Collector {
        fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            self.emitted.borrow_mut().push(
                arguments
                    .iter()
                    .map(|argument| argument.as_str().unwrap_or_default().to_owned())
                    .collect(),
            );
            Ok(Value::Empty.into())
        }
    }

    fn unit(body: &str) -> CompileUnit<'_> {
        CompileUnit {
            name: "test",
            parameters: &["words", "ctx"],
            body,
        }
    }

    fn registry_with_custom_list_expr_hook() -> tcl_registry::CommandRegistry {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let mut custom = registry.get("list").expect("list spec").clone();
        custom.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
        custom.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
        registry.insert(custom);
        registry
    }

    struct RefusingHostCommand;

    impl HostCommand for RefusingHostCommand {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::ExecutionRefusal(
                "host provider unavailable".into(),
            ))
        }
    }

    #[test]
    fn reached_host_refusal_preserves_prior_effects_and_cannot_be_caught() {
        let mut engine = TclVmEngine::new();
        engine
            .define_command("host_refuse", std::rc::Rc::new(RefusingHostCommand))
            .unwrap();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine.define_command("record", collector.clone()).unwrap();
        let handle = engine
            .compile(unit(
                "record BEFORE; catch {host_refuse} result; record AFTER; return $result",
            ))
            .unwrap();
        let error = engine
            .invoke(&handle, &[Value::Empty, Value::Empty])
            .unwrap_err();
        assert_eq!(
            error,
            EngineError::ExecutionRefusal("host provider unavailable".into())
        );
        assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
        // Refusal cleanup permits a separate accepted operation without replaying
        // the first invocation or publishing its internal empty unwind carrier.
        let handle = engine.compile(unit("record NEXT; return OK")).unwrap();
        assert_eq!(
            engine
                .invoke(&handle, &[Value::Empty, Value::Empty])
                .unwrap()
                .as_str(),
            Some("OK")
        );
        assert_eq!(
            *collector.emitted.borrow(),
            [vec!["BEFORE".to_owned()], vec!["NEXT".to_owned()]]
        );
    }

    #[test]
    fn invocation_evaluation_keeps_host_refusal_outside_guest_completion() {
        let mut engine = TclVmEngine::new();
        engine
            .define_command("host_refuse", Rc::new(RefusingHostCommand))
            .unwrap();
        let collector = Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine.define_command("record", collector.clone()).unwrap();
        let error = engine
            .eval_in_invocation("record BEFORE; catch {host_refuse} result; record AFTER")
            .unwrap_err();
        assert_eq!(
            error,
            EngineError::ExecutionRefusal("host provider unavailable".into())
        );
        assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
        assert!(error.script_message_bytes().is_none());
        assert!(error.script_options_bytes().is_none());
    }

    #[test]
    fn checked_dictionary_import_materializes_original_keys_with_the_actual_issuer() {
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let protocol = dialect.native_string_protocol().unwrap();
        let key = tcl_vm::Value::from_native_byte_array(Rc::from(&b"\0\xff"[..]), dialect).unwrap();
        let identity = key.native_object_identity();
        let mut children = vec![key.clone(), tcl_vm::Value::string("member")];
        let dictionary = recover_dictionary_children(&mut children, 1, Some(4), protocol).unwrap();
        assert!(children.is_empty());
        assert_eq!(
            key.resident_string_bytes().unwrap().as_ref(),
            b"\xc0\x80\xc3\xbf"
        );
        let members = dictionary.cached_dictionary_representation().unwrap();
        assert_eq!(dictionary.cached_dictionary_bucket_count(), Some(4));
        assert_eq!(members[0].0.native_object_identity(), identity);
        assert_eq!(
            members[0].0.byte_array_representation().unwrap().as_ref(),
            b"\0\xff"
        );
    }

    #[test]
    fn native_dictionary_search_fatal_refusal_crosses_the_engine_boundary_outside_guest_handlers() {
        use tcl_syntax::raw_string::NativeFatalCondition;
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        for add in [false, true] {
            for body in [
                "record BEFORE;catch {advance} result;record AFTER;return $result",
                "record BEFORE;try {advance} on error {r o} {record CAUGHT};record AFTER",
            ] {
                let mut engine = TclVmEngine::new();
                let driver =
                    tcl_vm::native_conformance::dictionary_search_mutation_driver(dialect, add)
                        .unwrap();
                engine.vm.register_native_command("advance", driver);
                let collector = Rc::new(Collector {
                    emitted: RefCell::new(Vec::new()),
                });
                engine.define_command("record", collector.clone()).unwrap();
                let handle = engine.compile(unit(body)).unwrap();
                let error = engine
                    .invoke(&handle, &[Value::Empty, Value::Empty])
                    .unwrap_err();
                assert_eq!(
                    error,
                    EngineError::ExecutionRefusal(
                        NativeFatalCondition::DictionarySearchConcurrentMutation.to_string()
                    )
                );
                assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
                let next = engine.compile(unit("record NEXT;return OK")).unwrap();
                assert_eq!(
                    engine
                        .invoke(&next, &[Value::Empty, Value::Empty])
                        .unwrap()
                        .as_str(),
                    Some("OK")
                );
                assert_eq!(
                    *collector.emitted.borrow(),
                    [vec!["BEFORE".to_owned()], vec!["NEXT".to_owned()]]
                );
            }
        }
    }

    #[test]
    fn a_unit_compiles_once_and_invokes_with_structured_values() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("return [llength $words]"))
            .expect("the body compiles");
        let words = Value::list([Value::string("a"), Value::string("b")]);
        let ctx = Value::dict_of([("nwords", Value::Int(2))]);
        let result = engine
            .invoke(&handle, &[words, ctx])
            .expect("the body runs");
        assert_eq!(result.as_str(), Some("2"));
    }

    #[test]
    fn an_owned_registry_hook_survives_vm_profile_compilation() {
        let mut engine = TclVmEngine::with_registry(registry_with_custom_list_expr_hook());
        let handle = engine
            .compile(unit("list {1 + 2}"))
            .expect("the custom Expr hook compiles");
        let result = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("the custom hook's bytecode runs");

        // The normal `list` command returns the literal `1 + 2`; the owned
        // registry deliberately assigns it the Expr hook, which returns 3.
        assert_eq!(result.as_str(), Some("3"));
    }

    #[test]
    fn a_dict_crosses_as_a_dict() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("return [dict get $ctx command]"))
            .expect("compiles");
        let ctx = Value::dict_of([
            ("command", Value::string("mylib::sort")),
            ("nwords", Value::Int(0)),
        ]);
        let result = engine
            .invoke(&handle, &[Value::list([]), ctx])
            .expect("runs");
        assert_eq!(result.as_str(), Some("mylib::sort"));
    }

    #[test]
    fn a_host_command_receives_what_the_body_emits() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine
            .define_command("role", collector.clone())
            .expect("registers");
        let handle = engine
            .compile(unit("role 0 varwrite\nrole 1 body"))
            .expect("compiles");
        engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("runs");
        assert_eq!(
            *collector.emitted.borrow(),
            vec![
                vec!["0".to_string(), "varwrite".to_string()],
                vec!["1".to_string(), "body".to_string()],
            ],
        );
    }

    /// A host command that creates another through the door it is given.
    struct Factory(std::rc::Rc<Collector>);

    impl HostCommand for Factory {
        fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::Unsupported("a factory needs the door"))
        }

        fn invoke_with_registrar(
            &self,
            registrar: &mut dyn CommandRegistrar,
            _arguments: &[Value],
        ) -> Result<HostOutcome, EngineError> {
            registrar.define_command("made", self.0.clone())?;
            Ok(Value::string("built").into())
        }
    }

    #[test]
    fn a_host_command_registers_on_a_bare_vm_with_the_door_open() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        register_host_command(
            engine.vm_mut(),
            "factory",
            std::rc::Rc::new(Factory(collector.clone())),
        );
        let handle = engine
            .compile(unit("set built [factory]\nmade $built 2"))
            .expect("compiles");
        engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("a command registered on the VM can register another, and both run");
        assert_eq!(
            *collector.emitted.borrow(),
            vec![vec!["built".to_string(), "2".to_string()]],
        );
    }

    #[test]
    fn a_command_nobody_registered_on_the_vm_is_not_there() {
        let mut engine = TclVmEngine::new();
        let handle = engine.compile(unit("factory")).expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("no such command");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message == "invalid command name \"factory\""),
            "{error:?}"
        );
    }

    #[test]
    fn an_error_in_the_body_is_reported_not_propagated() {
        let mut engine = TclVmEngine::new();
        let handle = engine.compile(unit("error boom")).expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the body raises");
        assert!(
            error.script_message_bytes() == Some(b"boom".as_slice()),
            "{error:?}"
        );
    }

    #[test]
    fn the_command_budget_is_enforced_and_distinguishable() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(200).with_wall_clock(Duration::from_secs(5)))
            .expect("the VM enforces both");
        // Dispatch-driven: `[$cmd length abc]` resolves a computed command
        // name, so every iteration is a real dispatch and is charged.
        let handle = engine
            .compile(unit(
                "set cmd string\nset i 0\nwhile {$i < 100000} { set x [$cmd length abc]\n incr i }",
            ))
            .expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the budget stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::Commands));
    }

    /// The third budget, and the reason it exists: one `string repeat` spends
    /// a single command and no measurable time while asking the allocator for
    /// as much as it likes. With only the other two armed this reaches OOM —
    /// which kills the process rather than the hook, so the host can neither
    /// report nor quarantine it. Here it is an ordinary, catchable refusal.
    #[test]
    fn the_value_size_budget_stops_an_allocation_the_other_two_cannot_see() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(
                Budget::of_commands(1_000_000)
                    .with_wall_clock(Duration::from_secs(5))
                    .with_max_value_bytes(1024),
            )
            .expect("the VM enforces all three");
        let handle = engine
            .compile(unit("string repeat aaaaaaaa 1000000"))
            .expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the value-size budget stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::ValueSize));
    }

    /// The same call under the cap still works: the budget bounds a runaway,
    /// it does not ban the command.
    #[test]
    fn a_value_within_the_size_budget_is_built_normally() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(1_000).with_max_value_bytes(1024))
            .expect("the VM enforces it");
        let handle = engine
            .compile(unit("string repeat ab 8"))
            .expect("compiles");
        let value = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("well within the cap");
        assert_eq!(value.as_str(), Some("abababababababab"));
    }

    /// A loop the compiler inlines dispatches nothing, so the command budget
    /// never fires on it — the wall clock is what contains that case, and the
    /// two together are why the host sets both.
    #[test]
    fn the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(1_000_000).with_wall_clock(Duration::from_millis(50)))
            .expect("the VM enforces both");
        let handle = engine
            .compile(unit("set i 0\nwhile {1} { incr i }"))
            .expect("compiles");
        let started = std::time::Instant::now();
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the wall clock stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::WallClock));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the cap must fire promptly: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_restricted_engine_has_only_the_whitelist_and_the_host_commands() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine.define_command("fold", collector).expect("registers");
        let handle = engine
            .compile(unit("fold [string length [lindex $words 0]]"))
            .expect("compiles");
        engine
            .restrict_commands(&["set", "expr", "if", "return", "string", "lindex", "llength"])
            .expect("restricts");
        engine
            .invoke(
                &handle,
                &[
                    Value::list([Value::string("abcde")]),
                    Value::dict_of::<&str>([]),
                ],
            )
            .expect("the whitelisted body still runs");

        let opener = engine
            .compile(unit("open /etc/passwd"))
            .expect("a body naming a forbidden command still compiles");
        let error = engine
            .invoke(&opener, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("but cannot run it");
        assert!(
            error
                .script_message_bytes()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .is_some_and(|message| message.contains("invalid command name")),
            "{error:?}"
        );
    }

    /// The compiler lowers `string trim` — and every other subcommand of an
    /// ensemble the VM knows — to a direct call of `::tcl::string::trim`, so a
    /// whitelist that names `string` has to keep those, or the same body runs when
    /// the call is the argument of a host command and fails when it is anywhere
    /// else. An ensemble the whitelist does not name keeps none of its own.
    #[test]
    fn a_whitelisted_ensembles_subcommands_run_wherever_they_are_called() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let mut engine = TclVmEngine::new();
        engine
            .restrict_commands(&["set", "return", "string", "dict"])
            .expect("restricts");
        for (body, expected) in [
            ("set r [string cat a - b]\nreturn $r", "a-b"),
            ("return [string toupper [string trim { x }]]", "X"),
            ("set d [dict create k v]\nreturn [dict get $d k]", "v"),
            ("return [string map {a b} [string cat a c]]", "bc"),
        ] {
            let handle = engine.compile(unit(body)).expect("compiles");
            let answer = engine.invoke(&handle, &arguments);
            assert_eq!(
                answer.as_ref().map(|value| value.as_str()),
                Ok(Some(expected)),
                "{body}"
            );
        }

        // Naming `dict` does not name `string`'s subcommands, spelt out or not.
        let mut dicts_only = TclVmEngine::new();
        dicts_only
            .restrict_commands(&["set", "return", "dict"])
            .expect("restricts");
        for body in [
            "return [::tcl::string::trim { x }]",
            "return [string trim { x }]",
        ] {
            let handle = dicts_only.compile(unit(body)).expect("compiles");
            assert!(dicts_only.invoke(&handle, &arguments).is_err(), "{body}");
        }
    }

    #[test]
    fn return_is_an_ordinary_early_exit() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("if {[llength $words] == 0} { return }\nreturn folded"))
            .expect("compiles");
        let abstained = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("an early return is not an error");
        assert_eq!(abstained.as_str(), Some(""));
    }

    /// `set_release` pins the VM's release: a leading zero is octal up to
    /// 8.6 and decimal from 9.0 (`expr {010 + 0}` is 8 on tclsh 8.4 to 8.6
    /// and 10 on 9.0 and 9.1), and every operation leaves the thread's own
    /// numeral grammar as it found it.
    #[test]
    fn set_release_pins_the_numeral_grammar() {
        for (profile, want) in [("tcl8.6", "8"), ("tcl9.0", "10"), ("f5-irules", "8")] {
            let mut engine = TclVmEngine::new();
            let collector = std::rc::Rc::new(Collector {
                emitted: RefCell::new(Vec::new()),
            });
            engine
                .define_command("fold", collector.clone())
                .expect("registers");
            engine
                .set_release(profile)
                .expect("a catalogue profile pins");
            assert_eq!(engine.release(), Some(profile));
            let before = tcl_syntax::number::runtime_syntax();
            let handle = engine
                .compile(unit("fold [expr {010 + 0}]"))
                .expect("compiles");
            engine
                .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
                .expect("runs");
            assert_eq!(
                *collector.emitted.borrow(),
                vec![vec![want.to_owned()]],
                "{profile}"
            );
            assert_eq!(
                tcl_syntax::number::runtime_syntax(),
                before,
                "{profile}: the thread keeps its own grammar"
            );
            assert_eq!(
                engine.set_release(profile),
                Ok(()),
                "{profile}: pinning the same release again is a no-op"
            );
            let other = if profile == "tcl9.0" {
                "tcl8.6"
            } else {
                "tcl9.0"
            };
            assert_eq!(
                engine.set_release(other),
                Err(EngineError::Unsupported(
                    "pinning a release after a unit was compiled"
                )),
                "{profile}: a compiled engine keeps its release"
            );
        }
        for name in ["no-such-dialect", "", "tcl", "tk", "jim"] {
            let mut engine = TclVmEngine::new();
            assert_eq!(
                engine.set_release(name),
                Err(EngineError::Unsupported("pinning a release")),
                "{name:?} names no release the VM can pin"
            );
            assert_eq!(engine.release(), None);
        }
    }

    /// `confine_stores` keeps every write in the invocation's own frame. Each
    /// body below writes somewhere else — a qualified global, an element, a
    /// loop, destructuring or capture target, a `dict` update, a namespace
    /// variable, a linked local, the caller's frame — and raises `can't set
    /// …: stores are confined to the activation` without writing. A caught
    /// error publishes neither `::errorInfo` nor `::errorCode`, since a later
    /// invocation would read them. The same writes to locals succeed, and
    /// reading a global still reads.
    #[test]
    fn confine_stores_refuses_every_store_outside_the_activation() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        for (body, written) in [
            ("set ::g 1", "::g"),
            ("incr ::counter", "::counter"),
            ("lappend ::l x", "::l"),
            ("set ::a(k) 1", "::a"),
            ("foreach ::x {1 2} {}", "::x"),
            ("lassign {1 2} ::p q", "::p"),
            ("regexp {(a)} a ::m", "::m"),
            ("regsub a abc b ::rs", "::rs"),
            ("scan 5 %d ::n", "::n"),
            ("dict set ::d k v", "::d"),
            ("dict unset ::du k", "::du"),
            ("dict lappend ::dl k v", "::dl"),
            ("dict incr ::di k", "::di"),
            ("dict append ::da k v", "::da"),
            ("binary scan A a ::b", "::b"),
            ("namespace eval ::ns {variable v 1}", "::ns::v"),
            ("global g2; set g2 1", "::g2"),
            ("upvar 0 ::g3 alias; set alias 1", "::g3"),
            ("uplevel 1 {set up 1}", "::up"),
        ] {
            let mut engine = TclVmEngine::new();
            engine.confine_stores().expect("the VM confines its stores");
            let handle = engine.compile(unit(body)).expect("compiles");
            let answer = engine.invoke(&handle, &arguments);
            assert!(
                matches!(
                    &answer,
                    Err(EngineError::Script { message, .. })
                        if message.contains("stores are confined to the activation")
                ),
                "{body}: {answer:?}"
            );
            let probe = engine
                .compile(unit(&format!("return [info exists {written}]")))
                .expect("compiles");
            assert_eq!(
                engine.invoke(&probe, &arguments).expect("reads").as_str(),
                Some("0"),
                "{body}: nothing was written"
            );
        }
        let mut engine = TclVmEngine::new();
        engine.confine_stores().expect("the VM confines its stores");
        let published = "return [list [info exists ::errorInfo] [info exists ::errorCode]]";
        let caught = format!("catch {{lindex {{}} y}}; {published}");
        for body in [caught.as_str(), published] {
            let handle = engine.compile(unit(body)).expect("compiles");
            assert_eq!(
                engine.invoke(&handle, &arguments).expect("runs").as_str(),
                Some("0 0"),
                "{body}"
            );
        }
        let mut engine = TclVmEngine::new();
        engine
            .vm_mut()
            .set_var("::seen", tcl_vm::Value::string("yes"))
            .expect("seeds");
        engine.confine_stores().expect("the VM confines its stores");
        let handle = engine
            .compile(unit(
                "set acc {}; foreach x {a b} {lappend acc $x}; incr n; dict set d k v\n\
                 set arr(k) 1; lassign {1 2} p q; regexp {(a)} a whole m\n\
                 regsub a abc b rs; dict lappend dl k v; dict incr di k\n\
                 return [list $acc $n $d $arr(k) $p $m $rs $dl $di $::seen]",
            ))
            .expect("compiles");
        assert_eq!(
            engine.invoke(&handle, &arguments).expect("runs").as_str(),
            Some("{a b} 1 {k v} 1 1 a bbc {k v} {k 1} yes")
        );
        // The `rand()` generator's seed is interpreter state every
        // invocation shares: `srand` writes it and `rand` reads what an
        // earlier call left. Both raise while stores are confined, under the
        // VM's default release and under 8.4, whose math functions are
        // `expr` builtins no command restriction removes.
        for release in [None, Some("tcl8.4"), Some("f5-irules")] {
            for body in ["expr {srand(7)}", "expr {rand()}"] {
                let mut engine = TclVmEngine::new();
                if let Some(release) = release {
                    engine.set_release(release).expect("pins");
                }
                engine.confine_stores().expect("the VM confines its stores");
                let handle = engine.compile(unit(body)).expect("compiles");
                let answer = engine.invoke(&handle, &arguments);
                assert!(
                    matches!(
                        &answer,
                        Err(EngineError::Script { message, .. })
                            if message.contains("stores are confined to the activation")
                    ),
                    "{release:?} {body}: {answer:?}"
                );
            }
        }
    }

    /// `confine_stores` refuses an array's creation and an unset outside the
    /// activation as it refuses a store: `array set ::fresh {}` makes no
    /// global array, and no form of removal — a whole variable or an
    /// element, `-nocomplain`, `array unset` with or without a pattern, a
    /// linked local, a `dict update` over a missing key — takes away a
    /// global seeded before the confinement. The same forms on locals run.
    #[test]
    fn confine_stores_refuses_creation_and_unset_outside_the_activation() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let run = |engine: &mut TclVmEngine, body: &str| {
            let handle = engine.compile(unit(body)).expect("compiles");
            engine
                .invoke(&handle, &arguments)
                .map(|value| value.as_str().expect("a string").to_owned())
        };
        let left =
            "return [list [info exists ::fresh] [info exists ::seed] [info exists ::arr(k)]]";
        for body in [
            "array set ::fresh {}",
            "unset ::seed",
            "unset -nocomplain ::seed",
            "unset ::arr(k)",
            "array unset ::arr",
            "array unset ::arr k*",
            "global seed; unset seed",
            "global arr; unset arr(k)",
            "upvar #0 seed alias; unset alias",
            "set d {}; dict update d k ::seed {}",
        ] {
            let mut engine = TclVmEngine::new();
            run(&mut engine, "set ::seed old; set ::arr(k) v").expect("an open VM writes globals");
            engine.confine_stores().expect("the VM confines its stores");
            let answer = run(&mut engine, body);
            assert!(
                matches!(
                    &answer,
                    Err(EngineError::Script { message, .. })
                        if message.contains("stores are confined to the activation")
                ),
                "{body}: {answer:?}"
            );
            assert_eq!(run(&mut engine, left), Ok("0 1 1".to_owned()), "{body}");
        }
        let mut engine = TclVmEngine::new();
        engine.confine_stores().expect("the VM confines its stores");
        assert_eq!(
            run(
                &mut engine,
                "array set fresh {}; unset fresh; set x 1; unset x; set y 1; unset -nocomplain y z\n\
                 array set a {k 1 j 2}; unset a(k); array unset a j*; array unset a\n\
                 set d {}; dict update d k v {}\n\
                 return [list [info exists fresh] [info exists x] [info exists y] [info exists a]]",
            ),
            Ok("0 0 0 0".to_owned())
        );
    }

    /// An engine restricted to a whitelist that allows `expr` keeps its
    /// math functions, which from 8.5 are commands (`tcl::mathfunc::abs`),
    /// so a body answers `expr {abs(-1)}` under every pinned release rather
    /// than only under 8.4, where they are builtins; `rand` and `srand` go
    /// with every other command the whitelist does not name.
    #[test]
    fn a_restricted_engine_keeps_the_math_functions_but_the_generator() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        for release in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
        ] {
            let mut engine = TclVmEngine::new();
            engine.set_release(release).expect("pins");
            engine
                .restrict_commands(&["expr", "return"])
                .expect("restricts");
            engine.confine_stores().expect("confines");
            let handle = engine
                .compile(unit("return [expr {abs(-1) + int(2.5) + double(1)}]"))
                .expect("compiles");
            assert_eq!(
                engine.invoke(&handle, &arguments).expect("folds").as_str(),
                Some("4.0"),
                "{release}"
            );
            for body in ["return [expr {rand()}]", "return [expr {srand(7)}]"] {
                let handle = engine.compile(unit(body)).expect("compiles");
                assert!(
                    engine.invoke(&handle, &arguments).is_err(),
                    "{release}: {body}"
                );
            }
        }
    }

    /// A confined engine reads no host environment (`value-evaluation.md`
    /// § *The rest of the route contract*): before `confine_stores` the
    /// host has seeded `::env`, `::tcl_platform` and `::tcl_library`, and
    /// after it none exists, so reading one raises as a read of an unset
    /// variable does and a body cannot fold the analysing machine's user,
    /// platform or paths into an answer about a program that runs
    /// elsewhere.
    #[test]
    fn a_confined_engine_reads_no_host_environment() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let seeded = "return [list [info exists ::env] [info exists ::tcl_platform] \
                      [info exists ::tcl_library]]";
        let mut open = TclVmEngine::new();
        let handle = open.compile(unit(seeded)).expect("compiles");
        assert_eq!(
            open.invoke(&handle, &arguments).expect("reads").as_str(),
            Some("1 1 1"),
            "the host seeds its globals"
        );
        let mut confined = TclVmEngine::new();
        confined
            .confine_stores()
            .expect("the VM confines its stores");
        let handle = confined.compile(unit(seeded)).expect("compiles");
        assert_eq!(
            confined
                .invoke(&handle, &arguments)
                .expect("reads")
                .as_str(),
            Some("0 0 0"),
            "confining removes them"
        );
        for body in [
            "return $::tcl_platform(byteOrder)",
            "return $::tcl_library",
            "return $::env(PATH)",
        ] {
            let handle = confined.compile(unit(body)).expect("compiles");
            let answer = confined.invoke(&handle, &arguments);
            assert!(
                matches!(&answer, Err(EngineError::Script { .. })),
                "{body}: {answer:?}"
            );
        }
    }

    #[test]
    fn each_invocation_gets_its_own_locals() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit(
                "if {[info exists leak]} { return leaked }\nset leak 1\nreturn fresh",
            ))
            .expect("compiles");
        for _ in 0..3 {
            let result = engine
                .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
                .expect("runs");
            assert_eq!(result.as_str(), Some("fresh"));
        }
    }
    #[test]
    fn byte_bridge_keeps_resident_spelling_and_binary_purity() {
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        for bytes in [b"a\0z".as_slice(), b"a\xC0\x80z", b"a\xFFz"] {
            let value = super::to_vm_value(&Value::string_bytes(bytes), dialect).unwrap();
            assert_eq!(value.string_bytes().as_ref(), bytes);
            assert_eq!(
                super::from_vm_value(&value, dialect).unwrap().as_bytes(),
                Some(bytes)
            );
        }
        let pure = super::to_vm_value(&Value::byte_array(&b"\0\xFF"[..]), dialect).unwrap();
        let carrier = super::from_vm_value(&pure, dialect).unwrap();
        assert_eq!(carrier.as_byte_array(), Some(b"\0\xFF".as_slice()));
        assert_eq!(carrier.as_bytes(), None);
        assert!(pure.resident_string_bytes().is_none());
        let resident = Value::byte_array(&b"\0\xFF"[..]).with_resident_string(&b"custom\xFF"[..]);
        let value = super::to_vm_value(&resident, dialect).unwrap();
        assert_eq!(
            value
                .byte_array_representation()
                .expect("binary backing")
                .as_ref(),
            b"\0\xFF"
        );
        assert_eq!(
            value.resident_string_bytes().expect("resident").as_ref(),
            b"custom\xFF"
        );
    }

    #[test]
    fn c84_long_carrier_retains_original_descriptor_and_refuses_foreign_release() {
        use tcl_engine_api::{NativeScalarCache, NativeStringStorageIdentity};
        let original = Value::NativeScalar(NativeScalarCache::Tcl84Long(17))
            .with_resident_string_storage(
                &b"17\0suffix"[..],
                NativeStringStorageIdentity::Allocated,
            );
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        let native = super::to_vm_value(&original, dialect).unwrap();
        assert_eq!(
            native.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(17))
        );
        let exported = super::from_vm_value(&native, dialect).unwrap();
        assert_eq!(exported.as_bytes(), original.as_bytes());
        let Value::Resident { value, storage, .. } = exported else {
            panic!("resident native long")
        };
        assert_eq!(storage, NativeStringStorageIdentity::Allocated);
        assert!(matches!(
            value.as_ref(),
            Value::NativeScalar(NativeScalarCache::Tcl84Long(17))
        ));
        let foreign = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        assert!(super::to_vm_value(&original, foreign).is_err());
    }

    #[test]
    fn scalar_bridge_retains_full_cache_storage_and_descriptor_origin() {
        use tcl_engine_api::{
            NativeCVersion, NativeIntegerRadix, NativeScalarCache, NativeStringStorageIdentity,
        };
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let original = Value::NativeScalar(NativeScalarCache::BigInteger {
            negative: false,
            radix: NativeIntegerRadix::Decimal,
            digits: Rc::from("18446744073709551617"),
        })
        .with_resident_string_storage(
            &b"0x10000000000000001\0tail"[..],
            NativeStringStorageIdentity::Allocated,
        );
        let native = super::to_vm_value(&original, dialect).unwrap();
        let exported = super::from_vm_value(&native, dialect).unwrap();
        assert_eq!(exported.as_bytes(), original.as_bytes());
        let Value::Resident { value, storage, .. } = exported else {
            panic!("resident scalar")
        };
        assert_eq!(storage, NativeStringStorageIdentity::Allocated);
        assert!(
            matches!(value.as_ref(), Value::NativeScalar(NativeScalarCache::BigInteger { digits, .. }) if digits.as_ref()=="18446744073709551617")
        );

        let word = Value::NativeScalar(NativeScalarCache::WordBoolean {
            value: true,
            origin: NativeCVersion::V9_0,
        });
        assert!(super::to_vm_value(&word, dialect).is_err());
        let resident = word.with_resident_string(&b"YES\xFF"[..]);
        let native = super::to_vm_value(&resident, dialect).unwrap();
        assert_eq!(
            native.native_word_boolean_version(),
            Some(tcl_dialect::TclVersion::V9_0)
        );
        assert_eq!(native.resident_string_bytes().unwrap().as_ref(), b"YES\xFF");
        assert!(
            super::to_vm_value(
                &resident,
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            )
            .is_err()
        );

        let bits = 0xfff8_1234_0000_4321;
        let native = super::to_vm_value(
            &Value::NativeScalar(NativeScalarCache::Double(f64::from_bits(bits))),
            dialect,
        )
        .unwrap();
        let Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Double(value),
        )) = native.native_scalar_cache()
        else {
            panic!("Double cache")
        };
        assert_eq!(value.to_bits(), bits);
    }

    #[test]
    fn guest_byte_error_retains_complete_native_options() {
        let completion = tcl_vm::Completion::new(
            tcl_vm::Code::Error,
            tcl_vm::Value::from_string_bytes(b"prefix\0\xFF".as_slice()),
            tcl_vm::Value::list(vec![
                tcl_vm::Value::string("-errorcode"),
                tcl_vm::Value::from_string_bytes(b"CODE\0\xFF".as_slice()),
                tcl_vm::Value::string("-custom"),
                tcl_vm::Value::string("retained"),
            ]),
        );
        let expected_options = completion.options.string_bytes();
        let error = super::internal_error(
            tcl_vm::TclError::from_completion(completion),
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
        );
        let EngineError::ScriptBytes {
            message,
            code,
            options,
        } = error
        else {
            panic!("byte guest carrier")
        };
        assert_eq!(message, b"prefix\0\xFF");
        assert_eq!(code.as_deref(), Some(b"CODE\0\xFF".as_slice()));
        assert_eq!(options.as_deref(), Some(expected_options.as_ref()));
    }

    #[test]
    fn guest_error_export_preserves_original_stringless_dictionary_and_members() {
        use tcl_dialect::TclVersion;
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let key = tcl_vm::Value::new_native_string_bytes(b"-errorcode".as_slice());
            let code = tcl_vm::Value::new_native_string_bytes(b"CODE\0\xff".as_slice());
            let key_identity = key.native_object_identity();
            let code_identity = code.native_object_identity();
            let original = tcl_vm::Value::native_dictionary_constructor(
                vec![(key.clone(), code.clone())],
                Some(4),
                dialect.native_string_protocol().unwrap(),
            )
            .unwrap();
            let identity = original.native_object_identity();
            assert!(original.resident_string_bytes().is_none());
            let completion = tcl_vm::Completion::new(
                tcl_vm::Code::Error,
                tcl_vm::Value::new_native_string_bytes(b"FAILED\0\xff".as_slice()),
                original,
            );
            let EngineError::ScriptBytes {
                message,
                code: exported_code,
                options,
            } = TclVmEngine::completion_to_result(&completion, dialect).unwrap_err()
            else {
                panic!("original guest-error export")
            };
            assert_eq!(message, b"FAILED\0\xff");
            assert_eq!(exported_code.as_deref(), Some(b"CODE\0\xff".as_slice()));
            assert_eq!(
                options.as_deref(),
                completion.options.resident_string_bytes().as_deref()
            );
            assert_eq!(completion.options.native_object_identity(), identity);
            assert_eq!(completion.options.cached_dictionary_bucket_count(), Some(4));
            assert!(completion.options.cached_list_representation().is_none());
            drop(key);
            drop(code);
            completion
                .options
                .with_cached_dictionary_representation(|pairs, _| {
                    assert_eq!(pairs[0].0.native_object_identity(), key_identity);
                    assert_eq!(pairs[0].1.native_object_identity(), code_identity);
                    assert!(!pairs[0].0.native_object_is_shared());
                    assert!(!pairs[0].1.native_object_is_shared());
                })
                .unwrap();
        }
    }

    #[test]
    fn guest_dictionary_options_refuse_an_unknown_native_issuer_without_shimmer() {
        let mut dialect =
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let options = tcl_vm::Value::native_dictionary_constructor(
            vec![(
                tcl_vm::Value::string("-errorcode"),
                tcl_vm::Value::string("CODE"),
            )],
            Some(4),
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let identity = options.native_object_identity();
        let completion = tcl_vm::Completion::new(
            tcl_vm::Code::Error,
            tcl_vm::Value::string("FAILED"),
            options,
        );
        dialect.core_point = None;
        dialect.native_family = None;
        assert!(matches!(
            TclVmEngine::completion_to_result(&completion, dialect),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert_eq!(completion.options.native_object_identity(), identity);
        assert!(completion.options.resident_string_bytes().is_none());
        assert_eq!(completion.options.cached_dictionary_bucket_count(), Some(4));
        assert!(completion.options.cached_list_representation().is_none());
    }
}
