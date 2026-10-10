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
            .map_err(|cause| EngineError::ExecutionRefusal(cause.to_string()))
    }

    /// Define the unit as a procedure, `::spectcl::unit::N`. The runtime parses
    /// a body when it runs it, so a body that does not parse fails its first
    /// invocation rather than its compilation.
    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        let _grammar = self.claim_grammar();
        self.units += 1;
        let procedure = format!("::spectcl::unit::{}", self.units);
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
        .map(|error| EngineError::ExecutionRefusal(error.to_string()))
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
            code: Some(interp.error_code()),
            options: Some(options.to_vec()),
        });
    }
    Ok(HostOutcome {
        code: CompletionCode::from_int(completion.code.as_int()).expect("non-error completion"),
        value: Value::string_bytes(result),
        options: Value::string_bytes(options),
    })
}

/// Run one public engine evaluation under its limits and original activation.
fn evaluate(
    interp: &mut Interp,
    body: impl FnOnce(&mut Interp) -> Code,
) -> Result<HostOutcome, EngineError> {
    interp.reset_native_compilation_admission();
    interp.begin_evaluation();
    let entered = interp.codegen_activation_enter();
    let code = if entered { body(interp) } else { Code::Error };
    let answer = capture_answer(interp, code);
    if entered {
        interp.codegen_activation_leave(code);
    }
    if let Some(error) = host_error(interp) {
        return Err(error);
    }
    answer
}

fn value_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::ExecutionRefusal(error.to_string())
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
    retire_host(interp, hosts, panic, old)?;
    install_host(interp, hosts, panic, namespace, &simple, command)
}

fn define_prepared_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    publication: &PreparedCommandPublication,
    command: Rc<dyn HostCommand>,
) -> Result<(), EngineError> {
    let selected = native_host_publication::consume(
        interp,
        Rc::clone(hosts),
        publication,
        CommandPublicationPurpose::CreateCCommand,
    )?;
    let (namespace, simple, old) = selected
        .ok_or_else(|| EngineError::ExecutionRefusal("creation receipt has no slot".into()))?;
    retire_host(interp, hosts, panic, old)?;
    if !interp.namespaces().namespace_is_live(namespace) {
        return Err(EngineError::ExecutionRefusal(
            "native publication namespace retired during callback".into(),
        ));
    }
    install_host(interp, hosts, panic, namespace, &simple, command)
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
        Command::ObjCmd(Rc::new(ObjCommand::new(
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
    retire_host(interp, hosts, panic, generation)?;
    Ok(generation.is_some_and(|generation| {
        hosts.borrow_mut().remove(&generation);
        interp.delete_command_generation(generation)
    }))
}

fn remove_prepared_host_command(
    interp: &mut Interp,
    hosts: &HostCommands,
    panic: &PanicSlot,
    publication: &PreparedCommandPublication,
) -> Result<bool, EngineError> {
    let selected = native_host_publication::consume(
        interp,
        Rc::clone(hosts),
        publication,
        CommandPublicationPurpose::DeleteCCommand,
    )?;
    let Some((namespace, simple, generation)) = selected else {
        return Ok(false);
    };
    retire_host(interp, hosts, panic, generation)?;
    // A callback-created replacement has another generation and survives.
    if interp.namespaces().command_generation(namespace, &simple) != generation {
        return Ok(false);
    }
    Ok(generation.is_some_and(|generation| {
        hosts.borrow_mut().remove(&generation);
        interp.delete_command_generation(generation)
    }))
}

fn provide_package(interp: &mut Interp, name: &str, version: &str) -> Result<(), EngineError> {
    let code = crate::cmd_package::provide_package(interp, name.as_bytes(), version.as_bytes());
    capture_answer(interp, code).map(|_| ())
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
    // SAFETY: the client data is the `HostEntry` this command was registered
    // with, alive until its delete procedure runs, which waits for the call.
    let entry = unsafe { &*client_data.cast::<HostEntry>() };
    // SAFETY: dispatch passes its own live interpreter for the call.
    let interp = unsafe { &mut *interp };
    let count = usize::try_from(objc).unwrap_or(0);
    // SAFETY: dispatch passes `objc` live words.
    let words = unsafe { std::slice::from_raw_parts(objv, count) };
    if entry.command.argument_view() == HostArgumentView::OriginalObjects {
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
        .map(|&word| match entry.command.argument_view() {
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
            host_commands: Rc::clone(&entry.host_commands),
            panic: Rc::clone(&entry.panic),
        };
        entry
            .command
            .invoke_with_registrar(&mut registrar, &arguments)
    }));
    let code = match outcome {
        Ok(Ok(outcome)) => answer(interp, &outcome),
        Ok(Err(error)) => fail(interp, error),
        Err(payload) => {
            *entry.panic.borrow_mut() = Some(payload);
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
        Value::NativeScalar(cache) => {
            crate::engine_abi::value_carriers::scalar(interp, cache).map_err(value_error)?
        }
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
                tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation))
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
            carrier::export_scalar(cache, obj::native_word_boolean_version(original))
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
            let protocol = interp
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or_else(|| {
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
            let protocol = interp
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or_else(|| {
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
            ))
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
            ))
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
mod tests {
    use super::*;
    use tcl_engine_api::{NativeCVersion, NativeScalarCache, NativeStringStorageIdentity};

    struct Constant;
    impl HostCommand for Constant {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Ok(Value::string("host").into())
        }
    }

    fn engine(profile: &str) -> RuntimeEngine {
        let mut engine = RuntimeEngine::new();
        engine.set_release(profile).unwrap();
        engine
    }

    // Software embedding controls: the audited name recipes live in the shared
    // naming owner. These assertions exercise this adapter's actual tables,
    // receipt authority and callbacks, rather than claiming a fresh C run.
    #[test]
    fn publication_receipts_keep_actual_slots_and_refuse_forged_stale_or_foreign_authority() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut engine = engine(profile);
            let service = engine.command_publication_service().unwrap();
            let publication = service
                .prepare(
                    b"::scope::command",
                    CommandPublicationPurpose::CreateCCommand,
                )
                .unwrap();
            assert!(!service.observed_presence(&publication).unwrap());
            let mut forged = publication.clone();
            forged.key.simple = Rc::from(&b"other"[..]);
            assert!(matches!(
                engine.define_prepared_command(forged, Rc::new(Constant)),
                Err(EngineError::ExecutionRefusal(_))
            ));
            engine
                .define_prepared_command(publication.clone(), Rc::new(Constant))
                .unwrap();
            assert!(matches!(
                engine.define_prepared_command(publication, Rc::new(Constant)),
                Err(EngineError::ExecutionRefusal(_))
            ));
            let deletion = service
                .prepare(
                    b"::scope::command",
                    CommandPublicationPurpose::DeleteCCommand,
                )
                .unwrap();
            assert!(service.observed_presence(&deletion).unwrap());
            engine
                .eval_in_invocation("rename ::scope::command ::scope::moved")
                .unwrap();
            assert!(matches!(
                engine.remove_prepared_command(deletion),
                Err(EngineError::ExecutionRefusal(_))
            ));
            assert_eq!(
                engine
                    .eval_in_invocation("::scope::moved")
                    .unwrap()
                    .value
                    .as_str(),
                Some("host")
            );
            let publication = service
                .prepare(
                    b"::scope::another",
                    CommandPublicationPurpose::CreateCCommand,
                )
                .unwrap();
            let mut foreign = self::engine(profile);
            assert!(matches!(
                foreign.define_prepared_command(publication, Rc::new(Constant)),
                Err(EngineError::ExecutionRefusal(_))
            ));
            let publication = service
                .prepare(
                    b"::scope::replacement",
                    CommandPublicationPurpose::CreateCCommand,
                )
                .unwrap();
            engine
                .eval_in_invocation("namespace delete ::scope; namespace eval ::scope {}")
                .unwrap();
            assert!(matches!(
                engine.define_prepared_command(publication, Rc::new(Constant)),
                Err(EngineError::ExecutionRefusal(_))
            ));
            let missing = service
                .prepare(b"absent", CommandPublicationPurpose::DeleteCCommand)
                .unwrap();
            assert!(!engine.remove_prepared_command(missing).unwrap());
            drop(engine);
            assert!(matches!(
                service.prepare(b"expired", CommandPublicationPurpose::CreateCCommand),
                Err(EngineError::ExecutionRefusal(_))
            ));
        }
    }

    struct CaptureScope(Rc<RefCell<Option<Rc<dyn CommandPublicationService>>>>);
    impl HostCommand for CaptureScope {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Ok(Value::Empty.into())
        }
        fn invoke_with_registrar(
            &self,
            registrar: &mut dyn CommandRegistrar,
            _: &[Value],
        ) -> Result<HostOutcome, EngineError> {
            *self.0.borrow_mut() = Some(registrar.command_publication_service()?);
            Ok(Value::Empty.into())
        }
    }

    #[test]
    fn callback_scope_is_owned_and_c_creation_keeps_unqualified_global_rule() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut engine = engine("tcl8.6");
        let captured = Rc::new(RefCell::new(None));
        engine
            .define_command("capture", Rc::new(CaptureScope(Rc::clone(&captured))))
            .unwrap();
        engine
            .eval_in_invocation("namespace eval scope {capture}")
            .unwrap();
        let service = captured.borrow_mut().take().unwrap();
        let global = service
            .prepare(b"global_host", CommandPublicationPurpose::CreateCCommand)
            .unwrap();
        let nested = service
            .prepare(b"child::nested", CommandPublicationPurpose::CreateCCommand)
            .unwrap();
        assert_eq!(global.key.namespace, 0);
        assert_ne!(nested.key.namespace, 0);
        engine
            .define_prepared_command(global, Rc::new(Constant))
            .unwrap();
        engine
            .define_prepared_command(nested, Rc::new(Constant))
            .unwrap();
        assert_eq!(
            engine
                .eval_in_invocation("list [global_host] [::scope::child::nested]")
                .unwrap()
                .value
                .as_str(),
            Some("host host")
        );
        assert!(matches!(
            service.note_publication(
                &service
                    .prepare(b"pending", CommandPublicationPurpose::CreateCCommand)
                    .unwrap(),
                true
            ),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }

    #[test]
    fn whitelist_keeps_renamed_opaque_host_token_and_exact_unit_tokens() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut engine = engine("tcl8.6");
        engine
            .define_command_bytes(b"opaque\xff", Rc::new(Constant))
            .unwrap();
        engine.define_command("host", Rc::new(Constant)).unwrap();
        let handle = engine
            .compile(CompileUnit {
                parameters: &[],
                body: "moved",
            })
            .unwrap();
        engine.eval_in_invocation("rename host moved").unwrap();
        engine.restrict_commands(&["list"]).unwrap();
        assert_eq!(engine.invoke(&handle, &[]).unwrap().as_str(), Some("host"));
        let opaque = obj::Owned::fresh(new_string(b"opaque\xff"));
        assert_eq!(engine.interp.dispatch(&[opaque.as_ptr()]), Code::Ok);
        assert_eq!(engine.interp.result_bytes(), b"host");
        assert!(engine.interp.resolve_cmd_token(b"host").is_none());
    }

    struct Observe {
        view: HostArgumentView,
        arguments: Rc<RefCell<Vec<Value>>>,
    }
    impl HostCommand for Observe {
        fn argument_view(&self) -> HostArgumentView {
            self.view
        }
        fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            *self.arguments.borrow_mut() = arguments.to_vec();
            Ok(arguments.first().cloned().unwrap_or(Value::Empty).into())
        }
    }

    #[test]
    fn callbacks_and_results_keep_exact_counted_bytes_and_scalar_storage() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut engine = engine("tcl8.6");
        let observed = Rc::new(RefCell::new(Vec::new()));
        engine
            .define_command(
                "observe",
                Rc::new(Observe {
                    view: HostArgumentView::MaterializedStrings,
                    arguments: Rc::clone(&observed),
                }),
            )
            .unwrap();
        let handle = engine
            .compile(CompileUnit {
                parameters: &["argument"],
                body: "observe $argument",
            })
            .unwrap();
        let bytes = &b"A\0\xffB"[..];
        let returned = engine
            .invoke(&handle, &[Value::string_bytes(bytes)])
            .unwrap();
        assert!(matches!(returned, Value::StringBytes(ref actual) if actual.as_ref() == bytes));
        assert!(
            matches!(&observed.borrow()[0], Value::StringBytes(actual) if actual.as_ref() == bytes)
        );
        engine
            .define_command(
                "observe",
                Rc::new(Observe {
                    view: HostArgumentView::NativeObjectSnapshots,
                    arguments: Rc::clone(&observed),
                }),
            )
            .unwrap();
        let input = Value::NativeScalar(NativeScalarCache::Integer(16))
            .with_resident_string_storage(
                Rc::from(&b"0x10"[..]),
                NativeStringStorageIdentity::Allocated,
            );
        engine.invoke(&handle, &[input]).unwrap();
        assert!(
            matches!(&observed.borrow()[0], Value::Resident { value, string, storage: NativeStringStorageIdentity::Allocated }
            if matches!(value.as_ref(), Value::NativeScalar(NativeScalarCache::Integer(16))) && string.as_ref() == b"0x10")
        );
        let byte_array = Value::byte_array(Rc::from(&b"\0\xff"[..]));
        engine.invoke(&handle, &[byte_array]).unwrap();
        assert!(
            matches!(&observed.borrow()[0], Value::ByteArray(bytes) if bytes.as_ref() == b"\0\xff")
        );
        let foreign = Value::NativeScalar(NativeScalarCache::WordBoolean {
            value: true,
            origin: NativeCVersion::V9_1,
        });
        assert!(matches!(
            engine.invoke(&handle, &[foreign]),
            Err(EngineError::ExecutionRefusal(_))
        ));
        let unknown = Value::Int(16).with_resident_string(Rc::from(&b"0x10"[..]));
        assert!(matches!(
            engine.invoke(&handle, &[unknown]),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }

    struct Refuse;
    impl HostCommand for Refuse {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::ExecutionRefusal(
                "callback capability unavailable".into(),
            ))
        }
    }

    #[test]
    fn host_refusal_is_uncatchable_and_retains_prior_effects_and_completion_state() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Existing child host-failure source/API contract:
        // naming.interpreter.original-child-host-refusal-transport
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md
        // This callback control adds a host reason without claiming C supplied it.
        let mut engine = engine("tcl8.6");
        engine.define_command("refuse", Rc::new(Refuse)).unwrap();
        assert!(matches!(
            engine.eval_in_invocation("set before 1; catch {refuse}; set after 1"),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(engine.interp.var_get(b"before").is_ok());
        assert!(engine.interp.var_get(b"after").is_err());
        assert_eq!(
            engine
                .eval_in_invocation("set before")
                .unwrap()
                .value
                .as_str(),
            Some("1")
        );
        let mut origin = Interp::new();
        origin.set_result_bytes(b"retained\0\xff");
        origin.set_return_state(3, Code::Other(7));
        origin.set_return_options(vec![(b"-custom".to_vec(), b"owned".to_vec())]);
        assert_eq!(
            fail(
                &mut origin,
                EngineError::ExecutionRefusal("first cause".into())
            ),
            Code::Error
        );
        assert_eq!(origin.result_bytes(), b"retained\0\xff");
        assert_eq!(origin.pending_return_level(), 3);
        assert_eq!(origin.pending_return_code(), Code::Other(7));
        assert_eq!(origin.pending_return_option_objects().len(), 1);
        let mut parent = Interp::new();
        parent.set_result_bytes(b"parent");
        parent.transport_host_refusal_from(&origin);
        assert_eq!(
            parent.host_command_refusal().as_deref(),
            Some("first cause")
        );
        assert_eq!(parent.result_bytes(), b"parent");
    }

    struct GuestError;
    impl HostCommand for GuestError {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::ScriptBytes {
                message: b"message".to_vec(),
                code: None,
                options: Some(
                    b"-code 1 -level 0 -errorcode {CUSTOM DETAIL} -tag retained".to_vec(),
                ),
            })
        }
    }
    #[test]
    fn guest_failure_options_and_nonstandard_completions_use_shared_original_owner() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut engine = engine("tcl8.6");
        engine.define_command("guest", Rc::new(GuestError)).unwrap();
        let result = engine.eval_in_invocation("catch {guest} message options; list $message [dict get $options -tag] [dict get $options -errorcode]").unwrap();
        assert_eq!(
            result.value.as_str(),
            Some("message retained {CUSTOM DETAIL}")
        );
        let completion = engine
            .eval_in_invocation("return -level 0 -code 7 -tag retained value")
            .unwrap();
        assert_eq!(completion.code, CompletionCode::Other(7));
        assert_eq!(completion.value.as_str(), Some("value"));
        assert!(value_bytes(&mut engine.interp, &completion.options)
            .unwrap()
            .windows(b"-tag retained".len())
            .any(|bytes| bytes == b"-tag retained"));
        let error = engine.eval_in_invocation("guest").unwrap_err();
        assert_eq!(error.script_message_bytes(), Some(&b"message"[..]));
        assert!(error
            .script_options_bytes()
            .unwrap()
            .windows(b"-tag retained".len())
            .any(|bytes| bytes == b"-tag retained"));
    }

    #[test]
    fn compiled_handles_retain_installed_generation_and_original_interpreter() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut original = engine(profile);
            let unit = CompileUnit {
                name: "handle control",
                parameters: &[],
                body: "set ::entered ORIGINAL; return ORIGINAL",
            };
            let handle = original.compile(unit).unwrap();
            assert_eq!(
                original.invoke(&handle, &[]).unwrap().as_str(),
                Some("ORIGINAL")
            );
            let mut foreign = engine(profile);
            let foreign_handle = foreign.compile(unit).unwrap();
            assert_eq!(foreign_handle.procedure(), handle.procedure());
            assert!(matches!(
                foreign.invoke(&handle, &[]),
                Err(EngineError::ExecutionRefusal(_))
            ));
            assert!(foreign.interp.var_get(b"entered").is_err());
            original.eval_in_invocation("unset ::entered").unwrap();
            original
                .eval_in_invocation(&format!(
                "rename {} {{}}; proc {} {{}} {{set ::entered REPLACEMENT; return REPLACEMENT}}",
                handle.procedure(), handle.procedure(),
            ))
                .unwrap();
            assert!(matches!(
                original.invoke(&handle, &[]),
                Err(EngineError::ExecutionRefusal(_))
            ));
            assert!(original.interp.var_get(b"entered").is_err());
            assert_eq!(
                original
                    .eval_in_invocation(handle.procedure())
                    .unwrap()
                    .value
                    .as_str(),
                Some("REPLACEMENT")
            );
        }
    }

    struct RetireReplacing(Rc<RefCell<usize>>);
    impl HostCommand for RetireReplacing {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Ok(Value::Empty.into())
        }
        fn retire_with_registrar(
            &self,
            registrar: &mut dyn CommandRegistrar,
        ) -> Result<(), EngineError> {
            *self.0.borrow_mut() += 1;
            registrar.define_command("retiring", Rc::new(Constant))
        }
    }
    #[test]
    fn retirement_callback_reentry_protects_new_generation_and_guest_traces_refuse_before_effects()
    {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut engine = engine("tcl8.6");
        let calls = Rc::new(RefCell::new(0));
        engine
            .define_command("retiring", Rc::new(RetireReplacing(Rc::clone(&calls))))
            .unwrap();
        assert!(!engine.remove_command("retiring").unwrap());
        assert_eq!(*calls.borrow(), 1);
        assert_eq!(
            engine
                .eval_in_invocation("retiring")
                .unwrap()
                .value
                .as_str(),
            Some("host")
        );
        engine.eval_in_invocation("proc traced {} {}; proc observer args {set ::fired 1}; trace add command traced delete observer").unwrap();
        let service = engine.command_publication_service().unwrap();
        assert!(matches!(
            service.prepare(b"traced", CommandPublicationPurpose::DeleteCCommand),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(engine.interp.var_get(b"::fired").is_err());
        assert!(engine.interp.resolve_cmd_token(b"traced").is_some());
    }

    #[test]
    fn original_object_view_and_logical_native_receipts_refuse_explicitly() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut logical = RuntimeEngine::new();
        assert!(matches!(
            logical.command_publication_service(),
            Err(EngineError::ExecutionRefusal(_))
        ));
        logical
            .define_command("compatibility", Rc::new(Constant))
            .unwrap();
        let mut engine = engine("tcl8.6");
        engine
            .define_command(
                "original",
                Rc::new(Observe {
                    view: HostArgumentView::OriginalObjects,
                    arguments: Rc::new(RefCell::new(Vec::new())),
                }),
            )
            .unwrap();
        assert!(matches!(
            engine.eval_in_invocation("catch {original anything}; set after 1"),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(engine.interp.var_get(b"after").is_err());
    }
}
