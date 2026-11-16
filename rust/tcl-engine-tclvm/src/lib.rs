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

mod closed_native_execution;
pub use closed_native_execution::execute_optimised_closed_source;
#[doc(hidden)]
pub use closed_native_execution::{
    PrivateApplicationHookHost, PrivateApplicationVm, private_application_hook_engine,
};

use std::cell::RefCell;
use std::rc::Rc;

use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_engine_api::OriginalObject;
use tcl_engine_api::{
    Budget, BudgetKind, CommandPublicationKey, CommandPublicationPurpose,
    CommandPublicationService, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, PreparedCommandPublication, Value,
};
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Completion, LimitKind, NativeCommand, Vm};

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
    original: &'a tcl_vm::embed::OriginalOperandProjection,
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
        .map_err(|error| internal_error(vm, error))
}

fn read_variable(vm: &mut Vm, name: &str) -> Result<Value, EngineError> {
    with_engine_projection(vm, |vm, original| read_variable_using(vm, original, name))
}

fn read_variable_using(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    name: &str,
) -> Result<Value, EngineError> {
    check_projection(vm, original)?;
    let reached = vm
        .read_variable(name)
        .map_err(|error| projection_error(vm, original, error));
    check_projection(vm, original)?;
    let value = reached?;
    let reached = from_vm_value_using(vm, original, &value);
    if let Err(error) = &reached {
        retain_engine_failure(vm, original, error);
    }
    drop(value);
    check_projection(vm, original)?;
    reached
}

fn set_variable(vm: &mut Vm, name: &str, value: &Value) -> Result<(), EngineError> {
    let original = vm
        .capture_original_scalar_ingress()
        .map_err(EngineError::execution_failure)?;
    let dialect = vm.native_scalar_carrier_dialect();
    let reached = (|| {
        let imported = {
            let actual = ActualScalarImport {
                vm: RefCell::new(&mut *vm),
                original: &original,
            };
            to_vm_value_using(value, dialect, Some(&actual))?
        };
        let reached = vm
            .write_variable(name, imported)
            .map_err(|error| internal_error(vm, error));
        vm.check_original_scalar_ingress(&original)
            .map_err(EngineError::execution_failure)?;
        reached
    })();
    vm.finish_original_scalar_ingress(original)
        .map_err(EngineError::execution_failure)?;
    reached
}

fn unset_variable(vm: &mut Vm, name: &str) -> Result<(), EngineError> {
    vm.unset_variable(name)
        .map_err(|error| internal_error(vm, error))
}

/// Evaluate `script` in the VM's current frame and report how it completed: a
/// normal completion, `return`, `break` and `continue` as the code they are, and
/// an error as the error, a budget one as the budget it outran.
fn evaluate(vm: &mut Vm, script: &str) -> Result<HostOutcome, EngineError> {
    let evaluated = vm.eval_source(script);
    if let Some(kind) = vm.limit_exceeded() {
        return Err(EngineError::BudgetExceeded(budget_kind(kind)));
    }
    let completion = match evaluated {
        Ok(completion) => completion,
        Err(error) => error.into_completion().map_err(host_failure)?,
    };
    with_engine_projection(vm, |vm, original| {
        let reached = evaluate_completion_using(vm, original, &completion);
        if let Err(error) = &reached {
            retain_engine_failure(vm, original, error);
        }
        drop(completion);
        check_projection(vm, original)?;
        reached
    })
}

fn evaluate_using(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    script: &str,
) -> Result<HostOutcome, EngineError> {
    check_projection(vm, original)?;
    let reached = vm
        .eval_source(script)
        .map_err(|error| projection_error(vm, original, error));
    check_projection(vm, original)?;
    let completion = reached?;
    let reached = evaluate_completion_using(vm, original, &completion);
    if let Err(error) = &reached {
        retain_engine_failure(vm, original, error);
    }
    drop(completion);
    check_projection(vm, original)?;
    reached
}

fn evaluate_completion_using(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    completion: &Completion<tcl_vm::Value>,
) -> Result<HostOutcome, EngineError> {
    check_projection(vm, original)?;
    let code = match completion.code {
        Code::Ok => CompletionCode::Ok,
        Code::Return => CompletionCode::Return,
        Code::Break => CompletionCode::Break,
        Code::Continue => CompletionCode::Continue,
        Code::Error => {
            let failure = TclVmEngine::completion_to_result_using(vm, original, completion)
                .expect_err("an error completion has no normal value");
            if matches!(
                failure,
                EngineError::Script { .. } | EngineError::ScriptBytes { .. }
            ) {
                vm.publish_caught_error(completion)
                    .map_err(|error| projection_error(vm, original, error))?;
                check_projection(vm, original)?;
            }
            return Err(failure);
        }
        Code::Other(other) => CompletionCode::Other(other),
    };
    let value = from_vm_value_using(vm, original, &completion.result)?;
    Ok(if code == CompletionCode::Return {
        let options = vm.host_return_options(completion);
        HostOutcome::returning(value, from_vm_value_using(vm, original, &options)?)
    } else {
        HostOutcome::completing(code, value)
    })
}

/// Export an internal VM failure without rebuilding its guest completion.
fn internal_error(vm: &mut Vm, error: tcl_vm::TclError) -> EngineError {
    match error.into_completion() {
        Ok(completion) => with_engine_projection(vm, |vm, original| {
            let reached = TclVmEngine::completion_to_result_using(vm, original, &completion);
            if let Err(error) = &reached {
                retain_engine_failure(vm, original, error);
            }
            let current = check_projection(vm, original);
            drop(completion);
            current?;
            check_projection(vm, original)?;
            reached
        })
        .expect_err("an internal Guest error has no ordinary Engine value"),
        Err(failure) => host_failure(failure),
    }
}

fn host_failure(failure: tcl_vm::TclHostFailure) -> EngineError {
    match failure {
        tcl_vm::TclHostFailure::Execution(first) => EngineError::execution_failure(first),
        tcl_vm::TclHostFailure::ValueAccess(first) => EngineError::execution_failure(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(first),
        ),
        tcl_vm::TclHostFailure::PrimitiveErrorStateRequired(error) => {
            EngineError::ExecutionRefusal(format!(
                "primitive error state is unavailable at this context-free conversion: {error:?}"
            ))
        }
    }
}

fn projection_error(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    error: tcl_vm::TclError,
) -> EngineError {
    match error.into_completion() {
        Err(failure) => host_failure(
            vm.retain_original_operand_failure(original, failure)
                .into_completion()
                .expect_err("retained Host cause has no Guest completion"),
        ),
        Ok(completion) => {
            let reached = TclVmEngine::completion_to_result_using(vm, original, &completion);
            drop(completion);
            match vm.check_original_operand_projection(original) {
                Ok(()) => reached.expect_err("Guest error has no ordinary Engine value"),
                Err(first) => host_failure(
                    first
                        .into_completion()
                        .expect_err("original check is operational"),
                ),
            }
        }
    }
}

fn check_projection(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
) -> Result<(), EngineError> {
    vm.check_original_operand_projection(original)
        .map_err(|error| projection_error(vm, original, error))
}

fn retain_engine_failure(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    error: &EngineError,
) {
    if let Some(first) = error.native_execution_failure() {
        let _ = vm.retain_original_operand_failure(
            original,
            tcl_vm::TclHostFailure::Execution(first.clone()),
        );
    } else if let EngineError::ExecutionRefusal(reason) = error {
        drop(vm.refuse_host_command(reason.clone()));
    }
}

fn with_engine_projection<T>(
    vm: &mut Vm,
    work: impl FnOnce(&mut Vm, &tcl_vm::embed::OriginalOperandProjection) -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    let original = vm
        .capture_original_operand_projection()
        .map_err(|error| internal_error(vm, error))?;
    let reached = work(vm, &original);
    if let Err(error) = &reached {
        retain_engine_failure(vm, &original, error);
    }
    if let Err(first) = check_projection(vm, &original) {
        drop(reached);
        let _ = vm.finish_original_operand_projection(original);
        return Err(first);
    }
    let retired = vm
        .finish_original_operand_projection(original)
        .map_err(|error| internal_error(vm, error));
    if let Err(first) = retired {
        drop(reached);
        return Err(first);
    }
    reached
}

/// The interface's name for a VM limit.
fn budget_kind(kind: LimitKind) -> BudgetKind {
    match kind {
        LimitKind::Commands => BudgetKind::Commands,
        LimitKind::WallClock => BudgetKind::WallClock,
        LimitKind::ValueSize => BudgetKind::ValueSize,
    }
}

/// The VM's name for a budget.
fn limit_kind(kind: BudgetKind) -> LimitKind {
    match kind {
        BudgetKind::Commands => LimitKind::Commands,
        BudgetKind::WallClock => LimitKind::WallClock,
        BudgetKind::ValueSize => LimitKind::ValueSize,
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

/// A host outcome as the completion the VM receives. Only a `Return` has a
/// use for options, as in C Tcl, where `catch` of any other code reports its
/// own.
fn host_outcome_completion(
    outcome: HostOutcome,
) -> Completion<tcl_engine_api::OriginalObjectResult> {
    let options = if outcome.code == tcl_engine_api::CompletionCode::Return {
        outcome.options
    } else {
        Value::Empty
    };
    Completion::new(
        to_vm_code(outcome.code),
        tcl_engine_api::OriginalObjectResult::Value(outcome.value),
        tcl_engine_api::OriginalObjectResult::Value(options),
    )
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

fn host_error_completion(vm: &mut Vm, error: EngineError) -> Completion<tcl_vm::Value> {
    match error {
        EngineError::Script { message, code } => {
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
        EngineError::ScriptBytes {
            message,
            code,
            options,
        } => {
            let options =
                options.map_or_else(|| error_options(code), tcl_vm::Value::from_string_bytes);
            Completion::new(
                Code::Error,
                tcl_vm::Value::from_string_bytes(message),
                options,
            )
        }
        EngineError::ExecutionFailure(first) => {
            vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(*first))
        }
        EngineError::ExecutionRefusal(reason) => vm.refuse_host_command(reason),
        // A budget the host command outran is recorded as the VM's own
        // breach, so the invocation reports it even past a `catch`.
        EngineError::BudgetExceeded(kind) => vm.exceed_limit(limit_kind(kind)),
        error => vm.refuse_host_command(error.to_string()),
    }
}

fn retirement_error_completion(vm: &mut Vm, error: EngineError) -> Completion<tcl_vm::Value> {
    match error {
        EngineError::ExecutionFailure(first) => {
            vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(*first))
        }
        error => vm.refuse_host_command(error.to_string()),
    }
}

fn projection_unwind(vm: &mut Vm, error: tcl_vm::TclError) -> Completion<tcl_vm::Value> {
    match error.into_completion() {
        Ok(completion) => completion,
        Err(first) => vm.refuse_tcl_host_failure(first),
    }
}

impl HostCommandShim {
    fn invoke_projected(
        &self,
        vm: &mut Vm,
        original: &tcl_vm::embed::OriginalOperandProjection,
        scalar: &tcl_vm::NativeScalarIngress,
        source: &[tcl_vm::Value],
    ) -> Result<Completion<tcl_vm::Value>, EngineError> {
        let dialect = vm.native_scalar_carrier_dialect();
        let interpreter = vm.native_object_bridge_identity();
        let view = self.command.argument_view();
        let mut originals: Vec<Rc<dyn tcl_engine_api::OriginalObject>> = Vec::new();
        let mut arguments = Vec::new();
        let reached = (|| {
            if view == tcl_engine_api::HostArgumentView::OriginalObjects {
                for value in source {
                    vm.check_original_projected_value(original, value)
                        .map_err(|error| projection_error(vm, original, error))?;
                    originals.push(Rc::new(VmOriginalObject {
                        value: VmOriginalValue::Header(value.native_lifetime_lease()),
                        dialect,
                        interpreter,
                    }));
                }
            }
            for value in source {
                vm.check_original_projected_value(original, value)
                    .map_err(|error| projection_error(vm, original, error))?;
                let reached = match view {
                    tcl_engine_api::HostArgumentView::MaterializedStrings => {
                        from_vm_value_using(vm, original, value)
                    }
                    tcl_engine_api::HostArgumentView::NativeObjectSnapshots => {
                        from_vm_object_value(value)
                    }
                    tcl_engine_api::HostArgumentView::OriginalObjects => Ok(Value::Empty),
                };
                if let Err(error) = &reached {
                    retain_engine_failure(vm, original, error);
                }
                check_projection(vm, original)?;
                arguments.push(reached?);
            }
            vm.check_original_scalar_ingress(scalar)
                .map_err(EngineError::execution_failure)?;
            let mut registrar = VmRegistrar {
                vm,
                host_commands: Rc::clone(&self.host_commands),
                original,
            };
            let outcome = if view == tcl_engine_api::HostArgumentView::OriginalObjects {
                self.command
                    .invoke_original_completion_with_registrar(&mut registrar, &originals)
            } else {
                self.command
                    .invoke_with_registrar(&mut registrar, &arguments)
                    .map(host_outcome_completion)
            };
            if let Err(error) = &outcome {
                retain_engine_failure(registrar.vm, original, error);
            }
            // Retain the original callback cause before its returned ownership retires.
            let current = check_projection(registrar.vm, original);
            let scalar_current = registrar
                .vm
                .check_original_scalar_ingress(scalar)
                .map_err(EngineError::execution_failure);
            if let Err(first) = current.and(scalar_current) {
                drop(outcome);
                check_projection(registrar.vm, original)?;
                return Err(first);
            }
            match outcome {
                Ok(result) => {
                    let reached = {
                        let actual = ActualScalarImport {
                            vm: RefCell::new(&mut *registrar.vm),
                            original: scalar,
                        };
                        recover_original_completion(&result, dialect, interpreter, &actual)
                    };
                    if let Err(error) = &reached {
                        retain_engine_failure(registrar.vm, original, error);
                    }
                    let current = check_projection(registrar.vm, original);
                    drop(result);
                    check_projection(registrar.vm, original)?;
                    current?;
                    reached
                }
                Err(error) => Ok(host_error_completion(registrar.vm, error)),
            }
        })();
        if let Err(error) = &reached {
            retain_engine_failure(vm, original, error);
        }
        drop(arguments);
        drop(originals);
        check_projection(vm, original)?;
        reached
    }
}

impl NativeCommand for HostCommandShim {
    fn retire(&self, vm: &mut Vm) -> Completion<tcl_vm::Value> {
        let original = match vm.capture_original_operand_projection() {
            Ok(original) => original,
            Err(error) => return projection_unwind(vm, error),
        };
        let reached = {
            let mut registrar = VmRegistrar {
                vm,
                host_commands: Rc::clone(&self.host_commands),
                original: &original,
            };
            self.command.retire_with_registrar(&mut registrar)
        };
        if let Err(error) = &reached {
            // Every retirement failure remains Host, including Script/Budget;
            // latch it before mandatory original-owner settlement.
            drop(retirement_error_completion(vm, error.clone()));
        }
        let current = check_projection(vm, &original);
        let retired = vm.finish_original_operand_projection(original);
        match current {
            Err(first) => retirement_error_completion(vm, first),
            Ok(()) => match retired {
                Err(first) => projection_unwind(vm, first),
                Ok(()) => match reached {
                    Ok(()) => {
                        Completion::new(Code::Ok, tcl_vm::Value::empty(), tcl_vm::Value::empty())
                    }
                    Err(error) => retirement_error_completion(vm, error),
                },
            },
        }
    }

    fn invoke(&self, vm: &mut Vm, arguments: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let scalar = match vm.capture_original_scalar_ingress() {
            Ok(original) => original,
            Err(first) => {
                return vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(first));
            }
        };
        let original = match vm.capture_original_operand_projection() {
            Ok(original) => original,
            Err(error) => {
                let completion = projection_unwind(vm, error);
                let _ = vm.finish_original_scalar_ingress(scalar);
                return completion;
            }
        };
        let reached = self.invoke_projected(vm, &original, &scalar, arguments);
        if let Err(error) = &reached {
            retain_engine_failure(vm, &original, error);
        }
        if let Err(first) = check_projection(vm, &original) {
            drop(reached);
            let _ = check_projection(vm, &original);
            let _ = vm.finish_original_operand_projection(original);
            let _ = vm.finish_original_scalar_ingress(scalar);
            return host_error_completion(vm, first);
        }
        if let Err(first) = vm.finish_original_scalar_ingress(scalar) {
            drop(reached);
            let _ = check_projection(vm, &original);
            let _ = vm.finish_original_operand_projection(original);
            return vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(first));
        }
        if let Err(first) = vm.finish_original_operand_projection(original) {
            drop(reached);
            return projection_unwind(vm, first);
        }
        reached.unwrap_or_else(|error| host_error_completion(vm, error))
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
        None,
    )
}

fn recover_original_completion(
    completion: &tcl_engine_api::OriginalObjectCompletion,
    dialect: tcl_registry::InvocationDialect,
    interpreter: (u64, u64),
    actual: &ActualScalarImport<'_>,
) -> Result<Completion<tcl_vm::Value>, EngineError> {
    let mut memo = std::collections::BTreeMap::new();
    let result = recover_original_result_in(
        &completion.result,
        dialect,
        interpreter,
        &mut memo,
        Some(actual),
    )?;
    let options = recover_original_result_in(
        &completion.options,
        dialect,
        interpreter,
        &mut memo,
        Some(actual),
    )?;
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
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    Ok(Some(match result {
        R::Value(value) => to_vm_value_using(value, dialect, actual)?,
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
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<Option<tcl_vm::Value>, EngineError> {
    use tcl_engine_api::OriginalObjectResult as R;
    let value = match value {
        R::Value(Value::NativeScalar(cache)) => import_vm_scalar(
            cache,
            dialect,
            Some((Rc::clone(string), import_storage(storage))),
            actual,
        )?,
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
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    use OriginalRecoveryWork as Work;
    use tcl_engine_api::OriginalObjectResult as R;
    if let Some(actual) = actual {
        actual.check()?;
    }
    let reached = (|| {
        let mut work = vec![Work::Visit(result)];
        let mut values = Vec::new();
        let mut active = std::collections::BTreeSet::new();
        while let Some(next) = work.pop() {
            if let Some(actual) = actual {
                actual.check()?;
            }
            if let Work::Visit(result) = &next
                && let Some(value) = recover_original_leaf(result, dialect, interpreter, actual)?
            {
                if let Some(actual) = actual {
                    actual.check()?;
                }
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
                    if let Some(recovered) = recover_original_resident(
                        value,
                        string,
                        *storage,
                        dialect,
                        interpreter,
                        actual,
                    )? {
                        values.push(recovered);
                    } else {
                        work.push(Work::Resident(string, *storage));
                        work.push(Work::Visit(value));
                    }
                }
                Work::Visit(_) => unreachable!("original leaf already recovered"),
                Work::List(count, canonical) => {
                    recover_original_list_children(&mut values, count, canonical, dialect)?;
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
                    recover_original_resident_child(&mut values, string, storage)?;
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
    })();
    if let Some(actual) = actual {
        actual.check()?;
    }
    reached
}

fn recover_original_list_children(
    values: &mut Vec<tcl_vm::Value>,
    count: usize,
    canonical: bool,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), EngineError> {
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
    Ok(())
}

fn recover_original_resident_child(
    values: &mut Vec<tcl_vm::Value>,
    string: &Rc<[u8]>,
    storage: tcl_engine_api::NativeStringStorageIdentity,
) -> Result<(), EngineError> {
    let value = values
        .pop()
        .expect("visited resident callback representation");
    values.push(
        value
            .with_resident_string_bytes_and_storage(Rc::clone(string), import_storage(storage))
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
    );
    Ok(())
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
// The actual VM receipt spans recursive transport and mandatory cleanup.
struct ActualScalarImport<'a> {
    vm: RefCell<&'a mut Vm>,
    original: &'a tcl_vm::NativeScalarIngress,
}
impl ActualScalarImport<'_> {
    fn check(&self) -> Result<(), EngineError> {
        self.vm
            .borrow_mut()
            .check_original_scalar_ingress(self.original)
            .map_err(EngineError::execution_failure)
    }
    fn scalar(
        &self,
        facts: &tcl_engine_api::NativeScalarCache,
        resident: Option<(
            Rc<[u8]>,
            tcl_syntax::native_string::NativeStringStorageIdentity,
        )>,
    ) -> Result<tcl_vm::Value, EngineError> {
        self.vm
            .borrow_mut()
            .import_original_native_scalar(self.original, facts, resident)
            .map_err(EngineError::execution_failure)
    }
}

fn import_vm_scalar(
    facts: &tcl_engine_api::NativeScalarCache,
    dialect: tcl_registry::InvocationDialect,
    resident: Option<(
        Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    if let Some(actual) = actual {
        return actual.scalar(facts, resident);
    }
    let (cache, origin) = import_scalar_cache(facts);
    tcl_vm::Value::from_native_scalar_cache_with_storage(cache, origin, dialect, resident)
        .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
}

#[cfg(test)]
fn to_vm_value(
    value: &Value,
    dialect: tcl_registry::InvocationDialect,
) -> Result<tcl_vm::Value, EngineError> {
    to_vm_value_using(value, dialect, None)
}

fn to_vm_value_using(
    value: &Value,
    dialect: tcl_registry::InvocationDialect,
    actual: Option<&ActualScalarImport<'_>>,
) -> Result<tcl_vm::Value, EngineError> {
    if let Some(actual) = actual {
        actual.check()?;
    }
    let refusal = |error: &dyn std::fmt::Display| EngineError::ExecutionRefusal(error.to_string());
    let reached = (|| {
        Ok(match value {
            Value::Empty => tcl_vm::Value::string(String::new()),
            Value::Str(text) => tcl_vm::Value::from_string_bytes(text.as_bytes()),
            Value::StringBytes(bytes) => tcl_vm::Value::from_string_bytes(bytes.as_ref()),
            Value::ByteArray(bytes) => {
                tcl_vm::Value::from_native_byte_array(Rc::clone(bytes), dialect)
                    .map_err(|error| refusal(&error))?
            }
            Value::NativeScalar(cache) => import_vm_scalar(cache, dialect, None, actual)?,
            Value::Resident {
                value,
                string,
                storage,
            } => {
                let storage =
                    tcl_syntax::scalar_getter::carrier::checked_storage(*storage, string.len())
                        .map_err(|error| refusal(&error))?;
                if let Value::NativeScalar(cache) = value.as_ref() {
                    import_vm_scalar(cache, dialect, Some((Rc::clone(string), storage)), actual)?
                } else {
                    to_vm_value_using(value, dialect, actual)?
                        .with_resident_string_bytes_and_storage(Rc::clone(string), storage)
                        .map_err(|error| refusal(&error))?
                }
            }
            Value::Int(number) => tcl_vm::Value::int(*number),
            Value::Double(number) => tcl_vm::Value::double(*number),
            Value::List(items) => tcl_vm::Value::native_list_constructor(
                items
                    .iter()
                    .map(|value| to_vm_value_using(value, dialect, actual))
                    .collect::<Result<_, _>>()?,
                dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal("List import updater issuer unavailable".into())
                })?,
            ),
            Value::Dict(entries) => tcl_vm::Value::native_dictionary_constructor(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            to_vm_value_using(key, dialect, actual)?,
                            to_vm_value_using(value, dialect, actual)?,
                        ))
                    })
                    .collect::<Result<_, EngineError>>()?,
                None,
                dialect.native_string_protocol().ok_or_else(|| {
                    EngineError::ExecutionRefusal(
                        "Dictionary import updater issuer unavailable".into(),
                    )
                })?,
            )
            .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?,
        })
    })();
    if let Some(actual) = actual {
        actual.check()?;
    }
    reached
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
            value.native_scalar_descriptor_origin(),
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
        // A parsed variable name is a lookup cache over the word's own string
        // (a literal `a` an earlier `array set a` named), so the string is all
        // there is to carry.
        if !matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
                | tcl_syntax::native_object::NativeObjectCacheSnapshot::ParsedVariableName { .. }
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

fn from_vm_value_using(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    value: &tcl_vm::Value,
) -> Result<Value, EngineError> {
    vm.check_original_projected_value(original, value)
        .map_err(|error| projection_error(vm, original, error))?;
    if let Some(bytes) = value.byte_array_representation() {
        let payload = Value::byte_array(bytes);
        let result = if let Some(string) = value.resident_string_bytes() {
            payload.with_resident_string_storage(string, export_storage(value))
        } else {
            payload
        };
        check_projection(vm, original)?;
        return Ok(result);
    }
    if let Some(cache) = value.native_scalar_cache() {
        let payload = Value::NativeScalar(export_scalar_cache(
            cache,
            value.native_scalar_descriptor_origin(),
        )?);
        let string = vm
            .original_operand_native_raw_string(original, value)
            .map_err(|error| projection_error(vm, original, error))?;
        return Ok(payload.with_resident_string_storage(string, export_storage(value)));
    }
    vm.original_operand_native_raw_string(original, value)
        .map(Value::string_bytes)
        .map_err(|error| projection_error(vm, original, error))
}

#[cfg(test)]
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
            value.native_scalar_descriptor_origin(),
        )?);
        // Retain the bridge's string-valued host view while carrying the full
        // primary cache. Numeric string generation does not narrow that cache.
        let string = native_string_bytes(value, dialect)?;
        return Ok(payload.with_resident_string_storage(string, export_storage(value)));
    }
    Ok(Value::string_bytes(native_string_bytes(value, dialect)?))
}

#[cfg(test)]
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
        let native = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        Self::with_native_registry(registry, native).expect("the C9.0 native core")
    }

    /// A fresh engine whose actual native core is the requested measured release.
    ///
    /// # Errors
    /// Refuses profiles without an original native execution point or bootstrap.
    pub fn for_native_release(profile: &str) -> Result<Self, EngineError> {
        let _grammar = GrammarGuard::keep();
        let native = tcl_registry::model::ingress::resolve_known_environment(profile)
            .and_then(|environment| environment.catalogue_profile())
            .filter(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile)
                    .execution_point()
                    .is_some()
            })
            .ok_or(EngineError::Unsupported("creating a native release engine"))?;
        let mut engine = Self::with_native_registry(CommandRegistry::build_default(), native)
            .ok_or(EngineError::Unsupported("creating a native release engine"))?;
        engine.release = Some(native.name);
        Ok(engine)
    }

    fn with_native_registry(
        registry: CommandRegistry,
        native: &'static tcl_dialect::DialectProfile,
    ) -> Option<Self> {
        let host = tcl_vm::standard_host();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::stdout()),
            Rc::clone(&host),
            native,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )?;
        // The core bootstraps only the core's own roots; the embedding host
        // seeds its globals (`tcl_library`, `auto_path`, `env`, `tcl_platform`).
        vm.set_host(host);
        vm.set_compiler(Box::new(BytecodeCompileService::new(registry)));
        Some(Self {
            vm,
            budget: Budget::default(),
            release: None,
            units: 0,
            host_commands: Rc::new(RefCell::new(Vec::new())),
            unit_commands: Vec::new(),
        })
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
    #[cfg(test)]
    fn completion_to_result(
        vm: &mut Vm,
        completion: &Completion<tcl_vm::Value>,
    ) -> Result<Value, EngineError> {
        with_engine_projection(vm, |vm, original| {
            Self::completion_to_result_using(vm, original, completion)
        })
    }

    fn completion_to_result_using(
        vm: &mut Vm,
        original: &tcl_vm::embed::OriginalOperandProjection,
        completion: &Completion<tcl_vm::Value>,
    ) -> Result<Value, EngineError> {
        check_projection(vm, original)?;
        // A budget the invocation outran is the failure however the body
        // completed: a `catch` of the limit error does not turn an exhausted
        // hook into an answer.
        if let Some(kind) = vm.limit_exceeded() {
            return Err(EngineError::BudgetExceeded(budget_kind(kind)));
        }
        if completion.code.is_ok() || completion.code == Code::Return {
            return from_vm_value_using(vm, original, &completion.result);
        }
        // An error raised with no code has `-errorcode NONE`: tclsh 8.4.20 to
        // 9.0.4 and jimsh 0.84 all report `NONE` after `catch {error plain}`.
        let code = vm
            .original_completion_option_bytes(original, completion, b"-errorcode")
            .map_err(|error| projection_error(vm, original, error))?
            .or_else(|| (completion.code == Code::Error).then(|| b"NONE".to_vec()));
        let message = vm
            .original_operand_native_raw_string(original, &completion.result)
            .map_err(|error| projection_error(vm, original, error))?;
        // The budget a body outran is the VM's own record above; an error
        // that only spells a limit's message is the script's error.
        let options = vm
            .original_operand_native_raw_string(original, &completion.options)
            .map_err(|error| projection_error(vm, original, error))?;
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

impl Default for TclVmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for TclVmEngine {
    fn host_metadata_list_reader(&self) -> Option<tcl_engine_api::HostMetadataListReader> {
        let profile = self.vm.dialect_profile();
        if profile.is_fallback() {
            return None;
        }
        let syntax = profile.grammar.list_parse;
        let escapes = self.vm.escape_syntax();
        Some(Rc::new(move |source| {
            tcl_syntax::list::split_list_with_syntax(source, syntax, escapes)
                .map(|items| {
                    items
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                })
                .map_err(|error| EngineError::Script {
                    message: format!("invalid host metadata list: {}", error.full_message(source)),
                    code: None,
                })
        }))
    }

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
        let procedure = tcl_syntax::naming::compiled_unit_procedure_name(self.units);
        self.vm
            .try_define_procedure(&procedure, unit.parameters, unit.body)
            .map_err(|error| match error {
                tcl_vm::VmCompilationError::Host(first) => EngineError::execution_failure(first),
                tcl_vm::VmCompilationError::Tcl(error) => internal_error(&mut self.vm, error),
            })?;
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
        let original = self
            .vm
            .capture_original_scalar_ingress()
            .map_err(EngineError::execution_failure)?;
        let dialect = self.vm.native_scalar_carrier_dialect();
        let wall_clock = self.budget.wall_clock;
        let reached = with_engine_projection(&mut self.vm, |vm, projected| {
            let arguments: Vec<tcl_vm::Value> = {
                let actual = ActualScalarImport {
                    vm: RefCell::new(&mut *vm),
                    original: &original,
                };
                let imported = arguments
                    .iter()
                    .map(|value| to_vm_value_using(value, dialect, Some(&actual)))
                    .collect::<Result<_, _>>();
                actual.check()?;
                imported?
            };
            let _grammar = GrammarGuard::claim(vm.runtime_version().number_syntax());
            // Fuel is per invocation, so refill before every call rather than
            // letting a long-lived engine starve its own later hooks.
            vm.reset_command_count();
            if let Some(wall_clock) = wall_clock {
                vm.set_wall_clock_budget(Some(wall_clock));
            }
            let invoked = vm.try_invoke_command(&handle.procedure, &arguments);
            // A result about to leave the VM answers to the value-size budget
            // however it was built, a folded constant included.
            if let Ok(completion) = &invoked {
                vm.charge_value_size(&completion.result);
            }
            let current = vm.check_original_scalar_ingress(&original);
            let projected_current = check_projection(vm, projected);
            drop(arguments);
            let retired = vm.check_original_scalar_ingress(&original);
            let completed = (|| {
                projected_current?;
                current.map_err(EngineError::execution_failure)?;
                retired.map_err(EngineError::execution_failure)?;
                let completion = invoked
                    .as_ref()
                    .map_err(|error| EngineError::execution_failure(error.clone()))?;
                Self::completion_to_result_using(vm, projected, completion)
            })();
            if let Err(error) = &completed {
                retain_engine_failure(vm, projected, error);
            }
            let current = check_projection(vm, projected);
            drop(invoked);
            current?;
            vm.check_original_scalar_ingress(&original)
                .map_err(|error| EngineError::execution_failure(error.clone()))?;
            check_projection(vm, projected)?;
            completed
        });
        self.vm
            .finish_original_scalar_ingress(original)
            .map_err(EngineError::execution_failure)?;
        reached
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
mod tests;

#[cfg(test)]
#[path = "c84_wide_transport_tests.rs"]
mod c84_wide_transport_tests;

#[cfg(test)]
#[path = "original_projection_tests.rs"]
mod original_projection_tests;

#[cfg(test)]
mod metadata_reader_unknown_profile_controls;
