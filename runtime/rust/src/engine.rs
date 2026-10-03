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

use std::any::Any;
use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

use tcl_engine_api::{
    Budget, BudgetKind, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, Value,
};
use tcl_syntax::number::{runtime_syntax, set_runtime_syntax, NumberSyntax};

use crate::budget::LimitKind;
use crate::interp::{new_string, obj_bytes, Code, Interp, ObjCommand, Param};
use crate::obj::{self, TclObj};

/// The names of the host commands an engine registered, in the unrooted
/// spelling [`Interp::retain_commands`] compares, kept so a later whitelist
/// keeps them.
type HostCommandNames = Rc<RefCell<Vec<String>>>;

/// A panic a host command raised, held until the evaluation that called the
/// command has unwound, then resumed on the embedder's side of the interface.
type PanicSlot = Rc<RefCell<Option<Box<dyn Any + Send>>>>;

/// A compiled unit: the procedure it was defined as, and how many arguments it
/// takes.
#[derive(Debug, Clone)]
pub struct RuntimeHandle {
    procedure: String,
    parameters: usize,
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
    host_commands: HostCommandNames,
    /// The procedures compiled units were defined as, kept by a whitelist.
    unit_commands: Vec<String>,
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
            host_commands: Rc::new(RefCell::new(Vec::new())),
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

    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(
            &mut self.interp,
            &self.host_commands,
            &self.panic,
            name,
            command,
        );
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        Ok(remove_host_command(
            &mut self.interp,
            &self.host_commands,
            name,
        ))
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(&mut self.interp, name, version)
    }

    /// Keep only the `allowed` commands, the host's and the compiled units',
    /// by the bytecode VM's engine's rule ([`Interp::restrict_to`]).
    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        let mut kept = self.host_commands.borrow().clone();
        kept.extend(self.unit_commands.iter().cloned());
        self.interp.restrict_to(allowed, &kept);
        Ok(())
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
        let body = new_string(unit.body.as_bytes());
        // SAFETY: a fresh object, held across the definition, which copies it.
        unsafe { obj::incr_ref_count(body) };
        self.interp
            .define_proc(procedure.as_bytes(), parameters, body);
        // SAFETY: balances the hold above.
        unsafe { obj::decr_ref_count(body) };
        self.unit_commands
            .push(procedure.trim_start_matches("::").to_owned());
        Ok(RuntimeHandle {
            procedure,
            parameters: unit.parameters.len(),
        })
    }

    fn invoke(&mut self, handle: &Self::Handle, arguments: &[Value]) -> Result<Value, EngineError> {
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
        let mut words = Vec::with_capacity(arguments.len() + 1);
        words.push(new_string(handle.procedure.as_bytes()));
        words.extend(arguments.iter().map(to_obj));
        for &word in &words {
            // SAFETY: each word is fresh; the call holds it for the dispatch.
            unsafe { obj::incr_ref_count(word) };
        }
        let answer = evaluate(&mut self.interp, |interp| interp.dispatch(&words));
        for &word in &words {
            // SAFETY: balances the hold above.
            unsafe { obj::decr_ref_count(word) };
        }
        self.resume_panic();
        match answer {
            Answer::Completed(Code::Ok | Code::Return, value) => Ok(Value::string(value)),
            Answer::Completed(_, message) => Err(EngineError::Script {
                message,
                code: None,
            }),
            Answer::Failed(error) => Err(error),
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

/// How an evaluation ended.
enum Answer {
    /// It completed with a code and a value (an error's message).
    Completed(Code, String),
    /// It failed as the interface reports a failure: a script error with its
    /// `-errorcode`, or a limit outrun.
    Failed(EngineError),
}

/// Run `body` as one evaluation under the interpreter's limits: an activation
/// of its own (so a `catch` inside behaves as at any script's top level, and an
/// uncaught error is published as at the end of one), its limits begun, and
/// its answer read before the activation's end resets the error state.
fn evaluate(interp: &mut Interp, body: impl FnOnce(&mut Interp) -> Code) -> Answer {
    interp.begin_evaluation();
    let entered = interp.codegen_activation_enter();
    let code = if entered { body(interp) } else { Code::Error };
    let text = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    let error_code =
        (code == Code::Error).then(|| String::from_utf8_lossy(&interp.error_code()).into_owned());
    if entered {
        interp.codegen_activation_leave(code);
    }
    if let Some(kind) = interp.limit_exceeded() {
        return Answer::Failed(EngineError::BudgetExceeded(budget_kind(kind)));
    }
    match error_code {
        Some(code) => Answer::Failed(EngineError::Script {
            message: text,
            code: Some(code),
        }),
        None => Answer::Completed(code, text),
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
    host_commands: HostCommandNames,
    panic: PanicSlot,
}

fn define_host_command(
    interp: &mut Interp,
    host_commands: &HostCommandNames,
    panic: &PanicSlot,
    name: &str,
    command: Rc<dyn HostCommand>,
) {
    let entry = Box::new(HostEntry {
        command,
        host_commands: Rc::clone(host_commands),
        panic: Rc::clone(panic),
    });
    let client_data = Box::into_raw(entry).cast::<c_void>();
    interp.create_obj_command(
        name.as_bytes(),
        ObjCommand::new(host_command_proc, client_data, Some(host_command_delete)),
    );
    let unrooted = name.trim_start_matches("::").to_owned();
    let mut names = host_commands.borrow_mut();
    if !names.contains(&unrooted) {
        names.push(unrooted);
    }
}

fn remove_host_command(interp: &mut Interp, host_commands: &HostCommandNames, name: &str) -> bool {
    let unrooted = name.trim_start_matches("::");
    host_commands
        .borrow_mut()
        .retain(|command| command != unrooted);
    interp.delete_command(name.as_bytes())
}

fn provide_package(interp: &mut Interp, name: &str, version: &str) -> Result<(), EngineError> {
    match crate::cmd_package::provide_package(interp, name.as_bytes(), version.as_bytes()) {
        Code::Ok => Ok(()),
        _ => Err(EngineError::Script {
            message: String::from_utf8_lossy(&interp.result_bytes()).into_owned(),
            code: Some(String::from_utf8_lossy(&interp.error_code()).into_owned()),
        }),
    }
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
    let arguments: Vec<Value> = words
        .iter()
        .skip(1)
        .map(|&word| Value::string(String::from_utf8_lossy(&obj_bytes(word))))
        .collect();
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
            interp.c_api_error(b"a host command panicked", None);
            Code::Error
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
    match outcome.code {
        CompletionCode::Return => {
            let words = [
                new_string(b"return"),
                new_string(b"-options"),
                to_obj(&outcome.options),
                to_obj(&outcome.value),
            ];
            for &word in &words {
                // SAFETY: each word is fresh; the call holds it.
                unsafe { obj::incr_ref_count(word) };
            }
            let code = crate::builtins::ret(interp, &words);
            for &word in &words {
                // SAFETY: balances the hold above.
                unsafe { obj::decr_ref_count(word) };
            }
            code
        }
        code => {
            let value = to_obj(&outcome.value);
            interp.set_result(value);
            Code::from_int(code.as_int())
        }
    }
}

/// Leave a host command's failure as the interpreter's error and answer
/// `TCL_ERROR`. A limit the command's own evaluation outran stays that limit.
fn fail(interp: &mut Interp, error: EngineError) -> Code {
    match error {
        EngineError::Script { message, code } => {
            interp.c_api_error(message.as_bytes(), code.as_deref().map(str::as_bytes));
            Code::Error
        }
        EngineError::BudgetExceeded(kind) => interp.exceed_limit(limit_kind(kind)),
        other => {
            interp.c_api_error(other.to_string().as_bytes(), None);
            Code::Error
        }
    }
}

/// The door a running host command holds on this engine.
struct RuntimeRegistrar<'a> {
    interp: &'a mut Interp,
    host_commands: HostCommandNames,
    panic: PanicSlot,
}

impl CommandRegistrar for RuntimeRegistrar<'_> {
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(self.interp, &self.host_commands, &self.panic, name, command);
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        Ok(remove_host_command(self.interp, &self.host_commands, name))
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(self.interp, name, version)
    }
}

/// An interface value as a fresh object: a dict is the flat key/value list
/// that is a Tcl dict, and a list keeps its elements' representations.
fn to_obj(value: &Value) -> *mut TclObj {
    match value {
        Value::Empty => new_string(b""),
        Value::Str(text) => new_string(text.as_bytes()),
        Value::Int(number) => obj::new_wide_int_obj(*number),
        Value::Double(number) => obj::new_double_obj(*number),
        Value::List(items) => {
            let elements: Vec<*mut TclObj> = items.iter().map(to_obj).collect();
            list_of(&elements)
        }
        Value::Dict(entries) => {
            let elements: Vec<*mut TclObj> = entries
                .iter()
                .flat_map(|(key, item)| [to_obj(key), to_obj(item)])
                .collect();
            list_of(&elements)
        }
    }
}

/// A list of fresh `elements`: the list's hold on each is the only one.
fn list_of(elements: &[*mut TclObj]) -> *mut TclObj {
    crate::list::new_list_obj(elements)
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
