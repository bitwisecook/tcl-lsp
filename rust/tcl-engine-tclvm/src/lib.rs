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
#[cfg(test)]
use tcl_engine_api::CommandRegistrar;
use tcl_engine_api::{
    Budget, BudgetKind, CommandPublicationService, CompileUnit, CompletionCode, Engine,
    EngineError, HostCommand, HostOutcome, PreparedCommandPublication, Value,
};
use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_vm::NativeCommand;
use tcl_vm::{Code, Completion, LimitKind, Vm};

mod host_command;
mod original_object;
mod original_recovery;
mod registration;
mod value_conversion;

#[cfg(test)]
use original_recovery::recover_original_result;
#[cfg(test)]
use original_recovery::{recover_dictionary_children, recover_original_result_in};
pub use registration::register_host_command;
use registration::{
    define_host_command, define_prepared_host_command, open_publication_service,
    remove_host_command, remove_prepared_host_command,
};
#[cfg(test)]
use value_conversion::from_vm_object_value;
use value_conversion::{ActualScalarImport, from_vm_value_using, to_vm_value_using};
#[cfg(test)]
use value_conversion::{from_vm_value, to_vm_value};

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

struct VmRegistrar<'a> {
    vm: &'a mut Vm,
    host_commands: HostCommandNames,
    original: &'a tcl_vm::embed::OriginalOperandProjection,
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
    let evaluated = vm.eval_value_source(script);
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
        .eval_value_source(script)
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

#[cfg(test)]
tcl_registry::testing::install_shipped!();
