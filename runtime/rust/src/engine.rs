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

//! The runtime as an engine of the extension interface (`tcl-engine-api`):
//! [`RuntimeEngine`] runs hosted bodies on this interpreter, so the hook host
//! and anything else written on the interface can target it as they target the
//! bytecode VM.
//!
//! The interpreter enforces the limits and the confinement itself
//! ([`Interp::set_limits`], [`Interp::confine_stores`]); what this module adds
//! is the interface's bookkeeping over it: the host commands, each an
//! extension command whose procedure is a Rust trampoline; the compiled units,
//! each a procedure; the whitelist; and the pinned release.
//!
//! Host and unit retention follows actual command generations through rename.
//! Native publication receipts retain a weak interpreter and the original
//! namespace incarnation. They support immediate commits and explicitly refuse
//! deferred native overlays, guest retirement execution and unclassified delete
//! callbacks. Original-object host callbacks require a bridge this adapter does
//! not provide; exact strings and supported cache/storage snapshots are distinct
//! supported views. Host refusals remain outside Tcl completion and `catch`.
//! Public invocation returns exact result bytes; invocation evaluation additionally
//! retains the original completion code and complete options dictionary.

use std::any::Any;
use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

use tcl_engine_api::{
    Budget, BudgetKind, CommandPublicationPurpose, CommandPublicationService, CommandRegistrar,
    CompileUnit, CompletionCode, Engine, EngineError, HostArgumentView, HostCommand, HostOutcome,
    PreparedCommandPublication, Value,
};
use tcl_syntax::number::{runtime_syntax, set_runtime_syntax, NumberSyntax};

use crate::budget::LimitKind;
use crate::interp::native_host_publication::{
    self, HostCommands, HostRegistration, PublicationService,
};
use crate::interp::{new_string, Code, Command, Interp, ObjCommand, Param};
use crate::obj::{self, TclObj};

/// A panic a host command raised, held until the evaluation that called the
/// command has unwound, then resumed on the embedder's side of the interface.
type PanicSlot = Rc<RefCell<Option<Box<dyn Any + Send>>>>;

/// A compiled unit retains its original interpreter and installed command
/// generation. Its reported procedure spelling does not grant binding authority.
#[derive(Debug, Clone)]
pub struct RuntimeHandle {
    procedure: String,
    parameters: usize,
    generation: u64,
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
}

impl RuntimeHandle {
    /// The procedure the unit was defined as.
    #[must_use]
    pub fn procedure(&self) -> &str {
        &self.procedure
    }
}

/// The runtime engine.
pub struct RuntimeEngine {
    interp: Interp,
    /// The profile [`Engine::set_release`] pinned, by canonical name.
    release: Option<&'static str>,
    /// Mints the procedure name each compiled unit is defined as.
    units: u32,
    host_commands: HostCommands,
    /// The procedures compiled units were defined as, kept by a whitelist.
    unit_commands: Vec<u64>,
    panic: PanicSlot,
}

impl RuntimeEngine {
    /// A fresh engine over a new interpreter at the runtime's default release.
    #[must_use]
    pub fn new() -> Self {
        // Creating the interpreter installs its release's numeral grammar for
        // the whole thread; the caller's comes back when the guard drops.
        let _grammar = GrammarGuard::keep();
        Self {
            interp: Interp::new(),
            release: None,
            units: 0,
            host_commands: Rc::new(RefCell::new(std::collections::HashMap::new())),
            unit_commands: Vec::new(),
            panic: Rc::new(RefCell::new(None)),
        }
    }

    /// The interpreter, for an embedder that drives it beside the interface.
    pub fn interp_mut(&mut self) -> &mut Interp {
        &mut self.interp
    }

    /// The profile [`Engine::set_release`] pinned, if one was.
    #[must_use]
    pub fn release(&self) -> Option<&'static str> {
        self.release
    }

    fn claim_grammar(&self) -> GrammarGuard {
        GrammarGuard::claim(self.interp.runtime_version().number_syntax())
    }

    /// Resume a panic a host command raised during the evaluation that just
    /// returned, so it reaches the embedder as if the command had been called
    /// from Rust.
    fn resume_panic(&self) {
        if let Some(payload) = self.panic.borrow_mut().take() {
            resume_unwind(payload);
        }
    }
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for RuntimeEngine {
    fn host_metadata_list_reader(&self) -> Option<tcl_engine_api::HostMetadataListReader> {
        let profile = self.interp.dialect_profile();
        if profile.is_fallback() {
            return None;
        }
        let syntax = profile.grammar.list_parse;
        let escapes = self.interp.lexer_config().escapes;
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

    type Handle = RuntimeHandle;

    fn name(&self) -> &'static str {
        "runtime"
    }

    fn command_publication_service(
        &mut self,
    ) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
        Ok(PublicationService::open(
            &self.interp,
            Rc::clone(&self.host_commands),
        )?)
    }

    fn define_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_prepared_host_command(
            &mut self.interp,
            &self.host_commands,
            &self.panic,
            &publication,
            command,
        )
    }

    fn remove_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        remove_prepared_host_command(
            &mut self.interp,
            &self.host_commands,
            &self.panic,
            &publication,
        )
    }

    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(
            &mut self.interp,
            &self.host_commands,
            &self.panic,
            name,
            command,
        )
    }

    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        remove_host_command(&mut self.interp, &self.host_commands, &self.panic, name)
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

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        let _grammar = self.claim_grammar();
        let answer = evaluate(&mut self.interp, |interp| {
            interp.eval_str(script.as_bytes())
        });
        self.resume_panic();
        answer
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(&mut self.interp, name, version)
    }

    /// Keep only the `allowed` commands, the host's and the compiled units',
    /// by the bytecode VM's engine's rule ([`Interp::restrict_to`]).
    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        let mut kept: Vec<_> = self.host_commands.borrow().keys().copied().collect();
        kept.extend(self.unit_commands.iter().copied());
        self.interp
            .restrict_to_tokens(allowed, &kept)
            .map_err(EngineError::execution_failure)
    }

    /// Define the unit as a procedure, `::spectcl::unit::N`. The runtime parses
    /// a body when it runs it, so a body that does not parse fails its first
    /// invocation rather than its compilation.
    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        let _grammar = self.claim_grammar();
        self.units += 1;
        let procedure = tcl_syntax::naming::compiled_unit_procedure_name(self.units);
        let parameters = unit
            .parameters
            .iter()
            .map(|name| Param {
                name: name.as_bytes().to_vec(),
                default: None,
            })
            .collect();
        let body = obj::Owned::fresh(new_string(unit.body.as_bytes()));
        let generation = self.interp.install_proc_original_storage(
            procedure.as_bytes(),
            parameters,
            None,
            body.as_ptr(),
            None,
            None,
        );
        if let Some(error) = host_error(&self.interp) {
            return Err(error);
        }
        let generation = generation.ok_or_else(|| {
            EngineError::ExecutionRefusal("compiled unit publication failed".into())
        })?;
        self.unit_commands.push(generation);
        Ok(RuntimeHandle {
            procedure,
            parameters: unit.parameters.len(),
            generation,
            interpreter: self.interp.native_callable_interpreter(),
        })
    }

    fn invoke(&mut self, handle: &Self::Handle, arguments: &[Value]) -> Result<Value, EngineError> {
        if handle.interpreter != self.interp.native_callable_interpreter()
            || self.interp.resolve_cmd_token(handle.procedure.as_bytes()) != Some(handle.generation)
        {
            return Err(EngineError::ExecutionRefusal(
                "compiled procedure binding was retired, replaced or belongs to another interpreter"
                    .into(),
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
        let _grammar = self.claim_grammar();
        let mut owned = Vec::with_capacity(arguments.len() + 1);
        owned.push(obj::Owned::fresh(new_string(handle.procedure.as_bytes())));
        for argument in arguments {
            owned.push(to_obj(&mut self.interp, argument)?);
        }
        let words: Vec<_> = owned.iter().map(obj::Owned::as_ptr).collect();
        let answer = evaluate(&mut self.interp, |interp| interp.dispatch(&words));
        self.resume_panic();
        let outcome = answer?;
        match outcome.code {
            CompletionCode::Ok | CompletionCode::Return => Ok(outcome.value),
            _ => Err(EngineError::ScriptBytes {
                message: value_bytes(&mut self.interp, &outcome.value)?,
                code: None,
                options: Some(value_bytes(&mut self.interp, &outcome.options)?),
            }),
        }
    }

    fn set_budget(&mut self, budget: Budget) -> Result<(), EngineError> {
        self.interp
            .set_limits(budget.commands, budget.wall_clock, budget.max_value_bytes);
        Ok(())
    }

    fn commands_spent(&self) -> Option<u64> {
        Some(self.interp.commands_since_begin())
    }

    /// Pin the interpreter to the profile `profile` names, resolved through
    /// the one dialect ingress, as the bytecode VM's engine pins it. A name
    /// that resolves to no catalogue profile, or to one with no Tcl release to
    /// run, is `Unsupported`; so is a second, different pin once a unit is
    /// compiled.
    fn set_release(&mut self, profile: &str) -> Result<(), EngineError> {
        let Some(resolved) = crate::sandbox::release_profile(profile) else {
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
        self.interp.set_dialect_profile(resolved);
        self.release = Some(resolved.name);
        Ok(())
    }

    fn confine_stores(&mut self) -> Result<(), EngineError> {
        self.interp.confine_stores();
        Ok(())
    }
}

/// Read the existing host-only channel before reaching result or option updaters.
fn host_error(interp: &Interp) -> Option<EngineError> {
    interp
        .native_execution_refusal()
        .map(EngineError::execution_failure)
}

/// Project the same original completion used by the Family-B embedding boundary.
fn capture_answer(interp: &mut Interp, code: Code) -> Result<HostOutcome, EngineError> {
    if let Some(error) = host_error(interp) {
        return Err(error);
    }
    if let Some(kind) = interp.limit_exceeded() {
        return Err(EngineError::BudgetExceeded(budget_kind(kind)));
    }
    let completion = crate::completion::capture_bytes(interp, code).map_err(value_error)?;
    let result = completion.result;
    let options = completion.options;
    if code == Code::Error {
        return Err(EngineError::ScriptBytes {
            message: result.to_vec(),
            code: Some(
                interp
                    .error_code_bytes_checked()
                    .map_err(EngineError::execution_failure)?,
            ),
            options: Some(options.to_vec()),
        });
    }
    // Shared Code::as_int widens its original Other(i32) without changing it.
    let code = i32::try_from(completion.code.as_int())
        .expect("shared completion codes retain their original i32");
    Ok(HostOutcome {
        code: CompletionCode::from_int(code).expect("non-error completion"),
        value: Value::string_bytes(result),
        options: Value::string_bytes(options),
    })
}

/// Run one public engine evaluation under its limits and original activation.
fn evaluate(
    interp: &mut Interp,
    body: impl FnOnce(&mut Interp) -> Code,
) -> Result<HostOutcome, EngineError> {
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter_engine_evaluation(
            interp,
        )
        .map_err(EngineError::execution_failure)?;
    interp.begin_evaluation();
    let entered = interp.codegen_activation_enter();
    let code = if entered { body(interp) } else { Code::Error };
    let answer = scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)
        .and_then(|()| capture_answer(interp, code));
    if entered {
        interp.codegen_activation_leave(code);
    }
    // Cleanup may reach a callback too; keep the original entered cause.
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    if let Some(error) = host_error(interp) {
        return Err(error);
    }
    answer
}

fn value_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::ExecutionRefusal(error.to_string())
}

/// Preserve the typed scalar cause until the engine's host-only error boundary.
fn scalar_access_error(
    interp: &Interp,
    error: crate::capi::NativeScalarObjectAccessError,
) -> EngineError {
    if let Some(first) = host_error(interp) {
        return first;
    }
    match error {
        crate::capi::NativeScalarObjectAccessError::Execution(error) => {
            EngineError::execution_failure(error)
        }
        crate::capi::NativeScalarObjectAccessError::Value(error) => value_error(error),
    }
}

/// The interface's name for a limit.
fn budget_kind(kind: LimitKind) -> BudgetKind {
    match kind {
        LimitKind::Commands => BudgetKind::Commands,
        LimitKind::WallClock => BudgetKind::WallClock,
        LimitKind::ValueSize => BudgetKind::ValueSize,
    }
}

/// The interpreter's name for a budget.
fn limit_kind(kind: BudgetKind) -> LimitKind {
    match kind {
        BudgetKind::Commands => LimitKind::Commands,
        BudgetKind::WallClock => LimitKind::WallClock,
        BudgetKind::ValueSize => LimitKind::ValueSize,
    }
}

/// A host command's state, owned by its extension command: the command, the
/// engine's list of host command names, and where its panics go.
struct HostEntry {
    command: Rc<dyn HostCommand>,
    host_commands: HostCommands,
    panic: PanicSlot,
}

fn define_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    name: &[u8],
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(EngineError::execution_failure)?;
    if interp
        .name_policy_protocol()
        .is_some_and(|policy| policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native)
    {
        let service = PublicationService::open(interp, Rc::clone(hosts))?;
        let publication = service.prepare(name, CommandPublicationPurpose::CreateCCommand)?;
        return define_prepared_host_command(interp, hosts, panic, &publication, command);
    }
    // Explicit compatibility registration uses the interpreter's selected
    // authored recipe. It cannot issue an authenticated native receipt.
    let (namespace, simple) = interp
        .namespaces_mut()
        .command_c_api_publication_at(interp.current_ns(), name)
        .ok_or_else(|| {
            EngineError::ExecutionRefusal(
                "logical command publication policy is unavailable".into(),
            )
        })?;
    let old = interp.namespaces().command_generation(namespace, &simple);
    let retirement = retire_host(interp, hosts, panic, old);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    retirement?;
    let installed = install_host(interp, hosts, panic, namespace, &simple, command);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    installed
}

fn define_prepared_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    publication: &PreparedCommandPublication,
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(EngineError::execution_failure)?;
    let selected = native_host_publication::consume(
        interp,
        Rc::clone(hosts),
        publication,
        CommandPublicationPurpose::CreateCCommand,
    )?;
    let (namespace, simple, old) = selected
        .ok_or_else(|| EngineError::ExecutionRefusal("creation receipt has no slot".into()))?;
    let retirement = retire_host(interp, hosts, panic, old);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    retirement?;
    if !interp.namespaces().namespace_is_live(namespace) {
        return Err(EngineError::ExecutionRefusal(
            "native publication namespace retired during callback".into(),
        ));
    }
    let installed = install_host(interp, hosts, panic, namespace, &simple, command);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    installed
}

fn retire_host(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    generation: Option<u64>,
) -> Result<(), EngineError> {
    let command = generation.and_then(|generation| hosts.borrow().get(&generation).cloned());
    if let Some(registration) = command {
        if registration.retiring.replace(true) {
            return Ok(());
        }
        struct RetirementGuard(Rc<std::cell::Cell<bool>>);
        impl Drop for RetirementGuard {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _retirement = RetirementGuard(Rc::clone(&registration.retiring));
        let mut registrar = RuntimeRegistrar {
            interp,
            host_commands: Rc::clone(hosts),
            panic: Rc::clone(panic),
        };
        registration.command.retire_with_registrar(&mut registrar)?;
        if let Some(error) = host_error(registrar.interp) {
            return Err(error);
        }
    }
    Ok(())
}

fn install_host(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    namespace: crate::namespace::NsId,
    simple: &[u8],
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    native_host_publication::ensure_retirement(
        interp,
        Rc::clone(hosts),
        interp.namespaces().command_generation(namespace, simple),
    )?;
    let entry = Box::new(HostEntry {
        command: Rc::clone(&command),
        host_commands: Rc::clone(hosts),
        panic: Rc::clone(panic),
    });
    let client_data = Box::into_raw(entry).cast::<c_void>();
    interp.bind_command_replacement(
        namespace,
        simple,
        Command::ObjCmd(Rc::new(ObjCommand::new_owned_runtime_shim(
            host_command_proc,
            client_data,
            Some(host_command_delete),
        ))),
    );
    if let Some(error) = host_error(interp) {
        return Err(error);
    }
    let generation = interp
        .namespaces()
        .command_generation(namespace, simple)
        .ok_or_else(|| {
            EngineError::ExecutionRefusal("host command publication did not create a token".into())
        })?;
    hosts.borrow_mut().insert(
        generation,
        HostRegistration {
            command,
            retiring: Rc::new(std::cell::Cell::new(false)),
        },
    );
    Ok(())
}

fn remove_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    name: &[u8],
) -> Result<bool, EngineError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(EngineError::execution_failure)?;
    if interp
        .name_policy_protocol()
        .is_some_and(|policy| policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native)
    {
        let service = PublicationService::open(interp, Rc::clone(hosts))?;
        let publication = service.prepare(name, CommandPublicationPurpose::DeleteCCommand)?;
        return remove_prepared_host_command(interp, hosts, panic, &publication);
    }
    let generation = interp.resolve_cmd_token(name);
    native_host_publication::ensure_retirement(interp, Rc::clone(hosts), generation)?;
    let retirement = retire_host(interp, hosts, panic, generation);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    retirement?;
    let removed = generation.is_some_and(|generation| {
        hosts.borrow_mut().remove(&generation);
        interp.delete_command_generation(generation)
    });
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    Ok(removed)
}

fn remove_prepared_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    publication: &PreparedCommandPublication,
) -> Result<bool, EngineError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(EngineError::execution_failure)?;
    let selected = native_host_publication::consume(
        interp,
        Rc::clone(hosts),
        publication,
        CommandPublicationPurpose::DeleteCCommand,
    )?;
    let Some((namespace, simple, generation)) = selected else {
        return Ok(false);
    };
    let retirement = retire_host(interp, hosts, panic, generation);
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    retirement?;
    // A callback-created replacement has another generation and survives.
    if interp.namespaces().command_generation(namespace, &simple) != generation {
        return Ok(false);
    }
    let removed = generation.is_some_and(|generation| {
        hosts.borrow_mut().remove(&generation);
        interp.delete_command_generation(generation)
    });
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    Ok(removed)
}

fn provide_package(interp: &mut Interp, name: &str, version: &str) -> Result<(), EngineError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(EngineError::execution_failure)?;
    let code = crate::cmd_package::provide_package(interp, name.as_bytes(), version.as_bytes());
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    let answer = capture_answer(interp, code).map(|_| ());
    scope
        .currency()
        .ensure_current_or_refuse()
        .map_err(EngineError::execution_failure)?;
    answer
}

/// The procedure every host command is registered with: the words become the
/// interface's values, the command runs with the door open, and its answer
/// becomes the interpreter's result and completion code, as a C command's
/// would.
///
/// A panic stops at this frame, which C conventions say nothing may unwind
/// through: it is held for the engine to resume once the evaluation has
/// unwound, and the command fails meanwhile.
unsafe extern "C" fn host_command_proc(
    client_data: *mut c_void,
    interp: *mut Interp,
    objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: the actual registered box is live at dispatch entry. Preserve
    // the invocation's existing Rust roles before any getter or callback; a
    // selected C self-delete may retire the registration box during this call.
    let (command, host_commands, panic) = {
        let entry = unsafe { &*client_data.cast::<HostEntry>() };
        (
            Rc::clone(&entry.command),
            Rc::clone(&entry.host_commands),
            Rc::clone(&entry.panic),
        )
    };
    // SAFETY: dispatch passes its own live interpreter for the call.
    let interp = unsafe { &mut *interp };
    let count = usize::try_from(objc).unwrap_or(0);
    // SAFETY: dispatch passes `objc` live words.
    let words = unsafe { std::slice::from_raw_parts(objv, count) };
    if command.argument_view() == HostArgumentView::OriginalObjects {
        return c_int::try_from(
            interp
                .refuse_host_command("runtime original-object callback bridge is unavailable")
                .as_int(),
        )
        .unwrap_or(1);
    }
    let arguments = words
        .iter()
        .skip(1)
        .map(|&word| match command.argument_view() {
            HostArgumentView::MaterializedStrings => interp
                .native_object_string_bytes(word)
                .map(Value::string_bytes)
                .map_err(value_error),
            HostArgumentView::NativeObjectSnapshots => snapshot_value(interp, word),
            HostArgumentView::OriginalObjects => unreachable!("refused before reading arguments"),
        })
        .collect::<Result<Vec<_>, EngineError>>();
    let arguments = match arguments {
        Ok(arguments) => arguments,
        Err(error) => {
            return c_int::try_from(fail(interp, error).as_int()).unwrap_or(1);
        }
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut registrar = RuntimeRegistrar {
            interp: &mut *interp,
            host_commands: Rc::clone(&host_commands),
            panic: Rc::clone(&panic),
        };
        command.invoke_with_registrar(&mut registrar, &arguments)
    }));
    let code = match outcome {
        Ok(Ok(outcome)) => answer(interp, &outcome),
        Ok(Err(error)) => fail(interp, error),
        Err(payload) => {
            *panic.borrow_mut() = Some(payload);
            interp.refuse_host_command("a host command panicked")
        }
    };
    c_int::try_from(code.as_int()).unwrap_or(1)
}

/// The delete procedure every host command is registered with.
unsafe extern "C" fn host_command_delete(client_data: *mut c_void) {
    // SAFETY: the client data is the box `define_host_command` leaked, and the
    // delete procedure runs once.
    drop(unsafe { Box::from_raw(client_data.cast::<HostEntry>()) });
}

/// Leave a host command's outcome as the interpreter's result and answer its
/// code. A `Return` with options takes effect as `return -options` would.
fn answer(interp: &mut Interp, outcome: &HostOutcome) -> Code {
    answer_values(
        interp,
        &outcome.value,
        &outcome.options,
        Code::from_int(outcome.code.as_int()),
    )
}

fn answer_values(interp: &mut Interp, value: &Value, options: &Value, code: Code) -> Code {
    let value = match to_obj(interp, value) {
        Ok(value) => value,
        Err(error) => return fail(interp, error),
    };
    let options = if code == Code::Return || !matches!(options, Value::Empty) {
        match to_obj(interp, options) {
            Ok(options) => Some(options),
            Err(error) => return fail(interp, error),
        }
    } else {
        None
    };
    crate::engine_abi::value_carriers::complete(
        interp,
        value.as_ptr(),
        options.as_ref().map(obj::Owned::as_ptr),
        code,
    )
}

fn fail(interp: &mut Interp, error: EngineError) -> Code {
    match error {
        EngineError::Script { message, code } => {
            interp.c_api_error(message.as_bytes(), code.as_deref().map(str::as_bytes));
            Code::Error
        }
        EngineError::ScriptBytes {
            message,
            code,
            options,
        } => {
            if let Some(options) = options {
                return answer_values(
                    interp,
                    &Value::string_bytes(message),
                    &Value::string_bytes(options),
                    Code::Error,
                );
            }
            interp.c_api_error(&message, code.as_deref());
            Code::Error
        }
        EngineError::BudgetExceeded(kind) => interp.exceed_limit(limit_kind(kind)),
        EngineError::ExecutionRefusal(reason) => interp.refuse_host_command(reason),
        EngineError::ExecutionFailure(cause) => interp.refuse_native_execution(*cause),
        other => interp.refuse_host_command(other.to_string()),
    }
}

/// The door a running host command holds on this engine.
struct RuntimeRegistrar<'a> {
    interp: &'a mut Interp,
    host_commands: HostCommands,
    panic: PanicSlot,
}

impl CommandRegistrar for RuntimeRegistrar<'_> {
    fn command_publication_service(
        &mut self,
    ) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
        Ok(PublicationService::open(
            self.interp,
            Rc::clone(&self.host_commands),
        )?)
    }

    fn define_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_prepared_host_command(
            self.interp,
            &self.host_commands,
            &self.panic,
            &publication,
            command,
        )
    }

    fn remove_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        remove_prepared_host_command(self.interp, &self.host_commands, &self.panic, &publication)
    }

    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(self.interp, &self.host_commands, &self.panic, name, command)
    }

    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        remove_host_command(self.interp, &self.host_commands, &self.panic, name)
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

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        let code = self.interp.eval_str(script.as_bytes());
        capture_answer(self.interp, code)
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(self.interp, name, version)
    }
}

/// Import structured storage through its selected producer without text round trips.
fn to_obj(interp: &mut Interp, value: &Value) -> Result<obj::Owned, EngineError> {
    Ok(match value {
        Value::Empty => obj::Owned::fresh(new_string(b"")),
        Value::Str(text) => obj::Owned::fresh(new_string(text.as_bytes())),
        Value::StringBytes(bytes) => obj::Owned::fresh(new_string(bytes)),
        Value::ByteArray(bytes) => crate::engine_abi::value_carriers::byte_array(interp, bytes)
            .map_err(|error| host_error(interp).unwrap_or_else(|| value_error(error)))?,
        Value::NativeScalar(cache) => crate::engine_abi::value_carriers::scalar(interp, cache)
            .map_err(|error| scalar_access_error(interp, error))?,
        Value::Resident {
            value,
            string,
            storage,
        } => {
            tcl_syntax::scalar_getter::carrier::checked_storage(*storage, string.len())
                .map_err(value_error)?;
            let original = to_obj(interp, value)?;
            crate::engine_abi::value_carriers::resident(&original, string, *storage)
                .map_err(value_error)?;
            original
        }
        Value::Int(number) => obj::Owned::fresh(obj::new_wide_int_obj(*number)),
        Value::Double(number) => obj::Owned::fresh(obj::new_double_obj(*number)),
        Value::List(items) => {
            let items = items
                .iter()
                .map(|value| to_obj(interp, value))
                .collect::<Result<Vec<_>, _>>()?;
            let pointers: Vec<_> = items.iter().map(obj::Owned::as_ptr).collect();
            obj::Owned::fresh(interp.new_list_object(&pointers))
        }
        Value::Dict(entries) => {
            let items = entries
                .iter()
                .map(|(key, value)| Ok((to_obj(interp, key)?, to_obj(interp, value)?)))
                .collect::<Result<Vec<_>, EngineError>>()?;
            let pairs: Vec<_> = items
                .iter()
                .map(|(key, value)| (key.as_ptr(), value.as_ptr()))
                .collect();
            let recipe = interp.native_invocation_dialect().native_string_materialization(Some(
                tcl_native_model::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation))
                .ok_or_else(|| EngineError::ExecutionRefusal("dictionary import updater issuer unavailable".into()))?;
            obj::Owned::fresh(
                crate::dict::new_dict_obj_native(&pairs, None, recipe.protocol())
                    .map_err(value_error)?,
            )
        }
    })
}

fn value_bytes(interp: &mut Interp, value: &Value) -> Result<Vec<u8>, EngineError> {
    let value = to_obj(interp, value)?;
    interp
        .native_object_string_bytes(value.as_ptr())
        .map(|bytes| bytes.to_vec())
        .map_err(value_error)
}

/// Inspect original cache/storage without materialising a convenient replacement.
fn snapshot_value(interp: &Interp, original: *mut TclObj) -> Result<Value, EngineError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::scalar_getter::carrier;
    let snapshot = obj::native_object_snapshot(original).map_err(value_error)?;
    let payload = match snapshot.cache {
        Cache::None => Value::Empty,
        Cache::ByteArray {
            bytes,
            proper: true,
        } => Value::byte_array(bytes),
        Cache::Numeric(cache) => Value::NativeScalar(
            carrier::export_scalar(cache, obj::native_scalar_descriptor_origin(original))
                .map_err(value_error)?,
        ),
        Cache::WordBoolean { value, version } => Value::NativeScalar(
            carrier::export_scalar(
                tcl_syntax::scalar_getter::NativeScalarCache::WordBoolean(value),
                Some(version),
            )
            .map_err(value_error)?,
        ),
        Cache::List {
            canonical: true, ..
        } => {
            let protocol = interp.invocation_string_protocol().ok_or_else(|| {
                EngineError::ExecutionRefusal("native List snapshot issuer unavailable".into())
            })?;
            if crate::list::native_string_protocol(original) != Some(protocol) {
                return Err(EngineError::ExecutionRefusal(
                    "foreign native List snapshot issuer".into(),
                ));
            }
            let items = crate::list::list_elements_native_checked(original, protocol)
                .map_err(value_error)?;
            Value::list(
                items
                    .iter()
                    .map(|&item| snapshot_value(interp, item))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        Cache::Dictionary { .. } => {
            let protocol = interp.invocation_string_protocol().ok_or_else(|| {
                EngineError::ExecutionRefusal(
                    "native Dictionary snapshot issuer unavailable".into(),
                )
            })?;
            if crate::dict::native_string_protocol(original) != Some(protocol) {
                return Err(EngineError::ExecutionRefusal(
                    "foreign native Dictionary snapshot issuer".into(),
                ));
            }
            let pairs = crate::dict::native_dict_pairs(original, protocol).map_err(value_error)?;
            Value::dict(
                pairs
                    .iter()
                    .map(|&(key, value)| {
                        Ok((snapshot_value(interp, key)?, snapshot_value(interp, value)?))
                    })
                    .collect::<Result<Vec<_>, EngineError>>()?,
            )
        }
        _ => {
            return Err(EngineError::ExecutionRefusal(
                "native primary cache has no supported snapshot carrier".into(),
            ));
        }
    };
    Ok(match snapshot.resident {
        Some(bytes) => payload.with_resident_string_storage(
            bytes,
            carrier::export_storage(snapshot.storage.ok_or_else(|| {
                EngineError::ExecutionRefusal(
                    "native resident snapshot storage is unavailable".into(),
                )
            })?),
        ),
        None if matches!(payload, Value::Empty) => {
            return Err(EngineError::ExecutionRefusal(
                "stringless untyped object snapshot is unavailable".into(),
            ));
        }
        None => payload,
    })
}

/// Hands the thread back the numeral grammar it had when dropped: the
/// interpreter installs its release's grammar for the whole thread, so an
/// engine operation claims the engine's for its duration and restores the
/// caller's on every exit path, a resumed panic included.
struct GrammarGuard(NumberSyntax);

impl GrammarGuard {
    /// Install `syntax` for the guard's lifetime.
    fn claim(syntax: NumberSyntax) -> Self {
        let saved = runtime_syntax();
        set_runtime_syntax(syntax);
        Self(saved)
    }

    /// Keep whatever the guarded operation installs, restoring the caller's
    /// grammar afterwards.
    fn keep() -> Self {
        Self(runtime_syntax())
    }
}

impl Drop for GrammarGuard {
    fn drop(&mut self) {
        set_runtime_syntax(self.0);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_active_host_deletion_tests;
