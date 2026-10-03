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

//! `tcl-engine-api` — the **Tcl extension interface**.
//!
//! The bottom of the two layers `docs/design/registry/spec-packs.md` describes: the
//! surface that plays, for our engines, the role Tcl's C API plays for C Tcl —
//! in modern idiomatic Rust, with traits and owned structured values, no raw
//! interp pointers, and no C-shaped warts.
//!
//! ```text
//!   hook host  (tcl-spec-hooks)     <- consumer 1: written in Rust on top
//!   C-Tcl shim (tcl-cshim)          <- consumer 2: C extensions behind it
//!  --------------- this crate ---------------
//!   tcl-vm engine  (tcl-engine-tclvm)
//!   Tcl->WASM codegen runtime engine (later)
//! ```
//!
//! Three rules govern it, and they are the ones the design states:
//!
//! 1. **Common to the backends, not to every backend.** `tcl-vm` and the
//!    Tcl->WASM codegen runtime implement it; the BPF backend explicitly does
//!    not, because hosted extensions make no sense there. Selecting one engine
//!    must not require another to be present, which is why this crate has no
//!    dependencies at all and each engine implementation is its own crate.
//! 2. **Two consumers, one shape.** The hook host is written on the compile,
//!    invoke and host-command surface; what only the C-Tcl shim needs — the
//!    completion code a host command answers ([`HostOutcome`]), and the door a
//!    running command holds on commands, packages, variables and evaluation
//!    ([`CommandRegistrar`]) — is defaulted, so a host that does not need it is
//!    unaffected and an engine that cannot offer it declines.
//! 3. **All C-required mangling lives in the shim.** String lifetimes, interp
//!    pointers and `int` result codes stay on the far side of that shim: this
//!    interface speaks [`Value`], [`CompletionCode`] and `Result`.
//!
//! ## The shape
//!
//! A unit of Tcl ([`CompileUnit`]) compiles **once** to an engine handle
//! ([`Engine::Handle`]), and the handle is invoked many times with structured
//! values, returning structured values. Compilation and invocation are
//! separate because that separation is the whole performance story of a hosted
//! hook: the body is bytecode-compiled at pack load, never per call site.
//!
//! An engine also accepts host commands ([`HostCommand`]) — the emitter verbs
//! and sandbox builtins a host injects — and a [`Budget`], which it must
//! enforce: a body that outruns its budget fails with
//! [`EngineError::BudgetExceeded`] rather than running on.

pub mod value;

use std::rc::Rc;
use std::time::Duration;

pub use value::Value;

/// A unit of Tcl to compile: a proc-shaped body with named parameters.
///
/// Proc-shaped rather than script-shaped because every calling convention
/// above this layer is `{words ctx}`-shaped, and because proc semantics are
/// what make each invocation independent — its own locals, and `return` as an
/// ordinary early exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileUnit<'a> {
    /// A stable name for the unit, used in diagnostics and crash records
    /// (`mylib::sort const_fold`). Not a Tcl command name: the engine mangles
    /// it however it needs to.
    pub name: &'a str,
    /// The parameter names, bound positionally at invocation.
    pub parameters: &'a [&'a str],
    /// The body source.
    pub body: &'a str,
}

/// How a host command completed, when it did not fail.
///
/// Tcl's completion codes other than `error`, which is the `Err` of the call: a
/// command that fails is an [`EngineError::Script`] and carries its message and
/// `-errorcode` there. The rest are what a C command's `TCL_RETURN`,
/// `TCL_BREAK` and `TCL_CONTINUE` are, and an engine carries each as the code
/// Tcl does: the calling procedure returns with the command's value, the
/// enclosing loop ends, the enclosing loop goes on to its next iteration. Any
/// other integer is a code of the command's own, which propagates until a
/// `catch` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionCode {
    /// `TCL_OK`: the command's value is its result.
    #[default]
    Ok,
    /// `TCL_RETURN`: the calling procedure returns, with the command's value and
    /// the options of the `return` behind it ([`HostOutcome::options`]).
    Return,
    /// `TCL_BREAK`.
    Break,
    /// `TCL_CONTINUE`.
    Continue,
    /// A code of the command's own: any integer but `TCL_OK` to
    /// `TCL_CONTINUE`, which are the variants above. Never `1`, `TCL_ERROR`,
    /// which is [`EngineError::Script`].
    Other(i32),
}

impl CompletionCode {
    /// The code an integer names, as a C command returns it, or `None` for
    /// `TCL_ERROR` (1), which is the `Err` of a call and not a completion.
    #[must_use]
    pub fn from_int(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => None,
            2 => Some(Self::Return),
            3 => Some(Self::Break),
            4 => Some(Self::Continue),
            other => Some(Self::Other(other)),
        }
    }

    /// The integer a C command returns for this code: `TCL_OK` is 0,
    /// `TCL_RETURN` 2, `TCL_BREAK` 3 and `TCL_CONTINUE` 4.
    #[must_use]
    pub fn as_int(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::Return => 2,
            Self::Break => 3,
            Self::Continue => 4,
            Self::Other(code) => code,
        }
    }
}

/// What a host command answers: its value, how it completed and, for a
/// `Return`, the options of the `return` behind it.
#[derive(Debug, Clone)]
pub struct HostOutcome {
    /// The result.
    pub value: Value,
    /// How the command completed.
    pub code: CompletionCode,
    /// The options of the `return` that raised a [`CompletionCode::Return`], as a
    /// Tcl dictionary: `-code`, `-level`, and what a `return -code error` carries
    /// (`-errorcode`, `-errorinfo`). Empty for every other code, and for a
    /// `Return` that is the command's own and plain. The calling procedure ends as
    /// the options say, as it does in C Tcl: an error for `-code error`, a break
    /// for `-code break`, a return from its caller too for `-level 2`.
    pub options: Value,
}

impl HostOutcome {
    /// A normal completion with `value`.
    #[must_use]
    pub fn ok(value: Value) -> Self {
        Self::completing(CompletionCode::Ok, value)
    }

    /// A completion with `code` and `value`, and no options.
    #[must_use]
    pub fn completing(code: CompletionCode, value: Value) -> Self {
        Self {
            value,
            code,
            options: Value::Empty,
        }
    }

    /// A `Return` with `value` and the `options` of the `return` that raised it.
    #[must_use]
    pub fn returning(value: Value, options: Value) -> Self {
        Self {
            value,
            code: CompletionCode::Return,
            options,
        }
    }
}

impl From<Value> for HostOutcome {
    fn from(value: Value) -> Self {
        Self::ok(value)
    }
}

/// A command implemented by the host and callable from Tcl.
///
/// The emitter verbs (`role`, `fold`, `reject`, …) and any host-supplied
/// builtin (the conservative `foldlist`) arrive this way. An implementation
/// receives values and answers a value and a completion code, or an error, and
/// cannot reach the interpreter unless an engine opens its door to it
/// ([`Self::invoke_with_registrar`]) — which is what keeps a host portable
/// across engines, and a hook body free of ambient authority.
pub trait HostCommand {
    /// Run the command with the call's arguments (the command name excluded).
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError>;

    /// Run the command with the engine's door open.
    ///
    /// An engine calls this rather than [`Self::invoke`], passing a
    /// [`CommandRegistrar`] that is live for the duration of the call, so a
    /// host command that *creates* commands (a factory, a C extension's
    /// `Tcl_CreateObjCommand` from inside a command procedure), reads or writes
    /// the calling frame's variables, or evaluates a script can do so before
    /// the calling script's next statement runs. The default ignores the door
    /// and runs [`Self::invoke`], so a command that only answers is unaffected.
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        let _ = registrar;
        self.invoke(arguments)
    }
}

/// The door of an engine, opened to a host command while it runs
/// ([`HostCommand::invoke_with_registrar`]).
///
/// What a running command may do beyond answering: change what is callable next
/// ([`Self::define_command`], [`Self::remove_command`]), say what a package
/// provides ([`Self::provide_package`]) and what library it loaded
/// ([`Self::library_loaded`]), read, write and unset a variable of the frame it
/// was called from, and evaluate a script there ([`Self::eval_in_invocation`]).
/// Each of the last five is declined, as [`EngineError::Unsupported`], by an
/// engine that has no such door.
pub trait CommandRegistrar {
    /// Register a host command under `name`, replacing any existing command
    /// of that name.
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError>;

    /// Remove a host command, reporting whether one of that name existed.
    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError>;

    /// Record that the package `name` is provided at `version`, as `package
    /// provide name version` does: the version is validated, and a package
    /// already provided at a different version is a [`EngineError::Script`] with
    /// the error `package provide` gives.
    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        let _ = (name, version);
        Err(EngineError::Unsupported("providing a package"))
    }

    /// Record that the library `prefix` has been loaded, from `file_name` (empty
    /// for one linked in), so that `info loaded` lists it.
    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        let _ = (file_name, prefix);
        Err(EngineError::Unsupported("recording a loaded library"))
    }

    /// The value of the variable `name` — a scalar, or an array element spelt
    /// `a(k)` — as the calling frame sees it, read as `set name` reads it, read
    /// traces included. A variable that is not there is an
    /// [`EngineError::Script`] with Tcl's own message (`can't read "x": no such
    /// variable`), so a host that reports the miss reports what Tcl does.
    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        let _ = name;
        Err(EngineError::Unsupported("reading a variable"))
    }

    /// Set the variable `name` in the calling frame as `set name value` does,
    /// write traces included; a store the engine refuses is an
    /// [`EngineError::Script`] with Tcl's message.
    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        let _ = (name, value);
        Err(EngineError::Unsupported("writing a variable"))
    }

    /// Unset the variable `name` in the calling frame as `unset name` does; one
    /// that is not there is an [`EngineError::Script`] with Tcl's message.
    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        let _ = name;
        Err(EngineError::Unsupported("unsetting a variable"))
    }

    /// Evaluate `script` in the calling frame, answering its value and the code
    /// it completed with: what `Tcl_EvalObjEx` does for a C command. An error
    /// the script raised is an [`EngineError::Script`], and a script that
    /// outruns the budget the invocation is under fails as the budget does.
    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        let _ = script;
        Err(EngineError::Unsupported(
            "evaluating a script in an invocation",
        ))
    }
}

/// What an engine must not let a body exceed.
///
/// Both are optional, and an engine that cannot enforce one must say so at
/// registration time ([`Engine::set_budget`] returning
/// [`EngineError::Unsupported`]) rather than silently running unbounded — a
/// budget nobody enforces is worse than no budget, because the layer above
/// stops guarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Budget {
    /// Maximum commands one invocation may dispatch.
    pub commands: Option<u64>,
    /// Maximum wall-clock time one invocation may take.
    pub wall_clock: Option<Duration>,
    /// Maximum byte length of any single value the body may build.
    ///
    /// Separate from [`Self::commands`] because neither of the other two can
    /// bound it: one opcode can allocate without dispatching a command or
    /// spending measurable time, so an unbounded body reaches OOM with both
    /// budgets nearly untouched. An OOM is the one failure the host cannot
    /// convert into an abstention — it ends the process, not the hook.
    pub max_value_bytes: Option<u64>,
}

impl Budget {
    /// A budget of `commands` commands and nothing else.
    #[must_use]
    pub const fn of_commands(commands: u64) -> Self {
        Self {
            commands: Some(commands),
            wall_clock: None,
            max_value_bytes: None,
        }
    }

    /// This budget with a value-size cap added.
    #[must_use]
    pub const fn with_max_value_bytes(self, bytes: u64) -> Self {
        Self {
            max_value_bytes: Some(bytes),
            ..self
        }
    }

    /// This budget with a wall-clock cap added.
    #[must_use]
    pub const fn with_wall_clock(self, wall_clock: Duration) -> Self {
        Self {
            wall_clock: Some(wall_clock),
            ..self
        }
    }
}

/// Which budget a body outran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetKind {
    /// The command count.
    Commands,
    /// The wall clock.
    WallClock,
    /// The size of a single allocated value.
    ValueSize,
}

/// Everything that can go wrong at the interface.
///
/// A host converts every variant to its own abstention; the variants exist so
/// it can *report* the difference (a crash record names the panic, a
/// performance badge names the budget) without parsing message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// The unit did not compile.
    Compile(String),
    /// The body raised a Tcl error.
    Script {
        /// The error message.
        message: String,
        /// The `-errorcode`, when the engine supplies one.
        code: Option<String>,
    },
    /// The body outran its budget.
    BudgetExceeded(BudgetKind),
    /// The body panicked, or the engine did on its behalf. The payload is
    /// whatever the boundary recovered.
    Crashed(String),
    /// The engine cannot do what was asked (an unenforceable budget, a
    /// registration it does not support).
    Unsupported(&'static str),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(message) => write!(f, "compile error: {message}"),
            Self::Script { message, code } => match code {
                Some(code) => write!(f, "error: {message} ({code})"),
                None => write!(f, "error: {message}"),
            },
            Self::BudgetExceeded(BudgetKind::Commands) => write!(f, "command budget exceeded"),
            Self::BudgetExceeded(BudgetKind::WallClock) => write!(f, "wall-clock budget exceeded"),
            Self::BudgetExceeded(BudgetKind::ValueSize) => write!(f, "value-size budget exceeded"),
            Self::Crashed(payload) => write!(f, "engine crashed: {payload}"),
            Self::Unsupported(what) => write!(f, "unsupported by this engine: {what}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// An execution engine that can host Tcl on behalf of a Rust embedder.
///
/// Single-threaded by construction (`Rc`, `&mut self`): an engine owns
/// interpreter state, and every engine we have is thread-confined. A host that
/// serves several threads owns one engine per thread, which is also what
/// per-pack isolation wants.
pub trait Engine {
    /// A compiled unit, ready to invoke. Opaque to the host: it is produced by
    /// [`Self::compile`] and consumed by [`Self::invoke`], never inspected.
    type Handle;

    /// A short, stable name for this engine — `"tclvm"`, `"wasm"`. Reported in
    /// crash records and studio badges so a divergence between engines is
    /// attributable.
    fn name(&self) -> &'static str;

    /// Register a host command under `name`, replacing any existing command of
    /// that name.
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError>;

    /// Remove a command registered through [`Self::define_command`], reporting
    /// whether one of that name existed.
    ///
    /// The other half of the registration door: an embedder whose commands
    /// come and go (a C extension's `Tcl_DeleteCommand`) needs the engine to
    /// forget one, not merely to shadow it. The default declines, so an engine
    /// that cannot unregister says so rather than leaving the command callable.
    fn remove_command(&mut self, _name: &str) -> Result<bool, EngineError> {
        Err(EngineError::Unsupported("removing a command"))
    }

    /// Record that the package `name` is provided at `version`, as `package
    /// provide` does: [`CommandRegistrar::provide_package`] for an embedder that
    /// drives the engine itself, outside an invocation. The default declines.
    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        let _ = (name, version);
        Err(EngineError::Unsupported("providing a package"))
    }

    /// Record that the library `prefix` has been loaded, from `file_name`:
    /// [`CommandRegistrar::library_loaded`] outside an invocation. The default
    /// declines.
    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        let _ = (file_name, prefix);
        Err(EngineError::Unsupported("recording a loaded library"))
    }

    /// The value of the variable `name` in the frame the engine is in (the
    /// global frame between invocations, the calling frame inside one):
    /// [`CommandRegistrar::variable`] for an embedder that drives the engine
    /// itself. The default declines.
    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        let _ = name;
        Err(EngineError::Unsupported("reading a variable"))
    }

    /// Set the variable `name` in that frame. The default declines.
    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        let _ = (name, value);
        Err(EngineError::Unsupported("writing a variable"))
    }

    /// Unset the variable `name` in that frame. The default declines.
    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        let _ = name;
        Err(EngineError::Unsupported("unsetting a variable"))
    }

    /// Evaluate `script` in that frame, answering its value and the code it
    /// completed with ([`CommandRegistrar::eval_in_invocation`] is the form a
    /// running host command uses). The default declines.
    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        let _ = script;
        Err(EngineError::Unsupported(
            "evaluating a script in an invocation",
        ))
    }

    /// Reduce the engine's command surface to exactly `allowed` plus whatever
    /// [`Self::define_command`] has registered.
    ///
    /// A whitelist, never a blacklist: a command the engine gains in a later
    /// release must not silently become reachable from a body.
    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError>;

    /// Compile `unit` to a reusable handle. Called once per hook, at load.
    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError>;

    /// Invoke a compiled handle, binding `arguments` to the unit's parameters
    /// positionally.
    ///
    /// The returned value is the body's result. A host whose protocol ignores
    /// the result (`SpecTcl`'s emitter protocol does) still calls this and reads
    /// what its host commands collected.
    fn invoke(&mut self, handle: &Self::Handle, arguments: &[Value]) -> Result<Value, EngineError>;

    /// Set the budget every subsequent invocation runs under.
    fn set_budget(&mut self, budget: Budget) -> Result<(), EngineError>;

    /// Pin every later compilation and invocation to the named dialect
    /// profile. Called once per (pack, profile), after the engine is built
    /// and before `compile`: a second profile is a second engine, never a
    /// per-call setter.
    ///
    /// The argument is the profile's canonical name, not a profile value,
    /// because this crate depends on nothing; the engine resolves it. The
    /// default declines, so an engine that cannot pin a release says so
    /// rather than running at its own default while the caller believes
    /// otherwise — the contract [`Self::set_budget`] has for a budget it
    /// cannot enforce.
    fn set_release(&mut self, profile: &str) -> Result<(), EngineError> {
        let _ = profile;
        Err(EngineError::Unsupported("pinning a release"))
    }

    /// Refuse, as a Tcl error, every store whose name resolves outside the
    /// running procedure's own frame: a `::`-qualified name, a namespace
    /// variable, a linked variable. Reads are unaffected, except that an
    /// engine which seeds globals from its host — an environment, platform
    /// facts, library paths — removes them.
    ///
    /// What keeps a hosted body's answer a function of its arguments: with
    /// its writes confined to its own activation, no invocation leaves state
    /// behind that a later one reads, and no global tells it about the
    /// machine it happens to run on. Called once per engine, after
    /// [`Self::restrict_commands`]. The default declines, so an engine that
    /// cannot confine its stores says so and the host builds no sandbox on
    /// it.
    fn confine_stores(&mut self) -> Result<(), EngineError> {
        Err(EngineError::Unsupported(
            "confining stores to the activation",
        ))
    }

    /// What the last invocation actually spent, when the engine can say —
    /// commands dispatched. `None` from an engine with no counter.
    fn commands_spent(&self) -> Option<u64>;
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// A door that registers and removes commands and nothing else.
    struct Bare;

    impl CommandRegistrar for Bare {
        fn define_command(
            &mut self,
            _name: &str,
            _command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            Ok(())
        }

        fn remove_command(&mut self, _name: &str) -> Result<bool, EngineError> {
            Ok(false)
        }
    }

    /// An engine that implements only what the trait requires.
    struct Minimal;

    impl Engine for Minimal {
        type Handle = ();

        fn name(&self) -> &'static str {
            "minimal"
        }

        fn define_command(
            &mut self,
            _name: &str,
            _command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            Ok(())
        }

        fn restrict_commands(&mut self, _allowed: &[&str]) -> Result<(), EngineError> {
            Ok(())
        }

        fn compile(&mut self, _unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
            Ok(())
        }

        fn invoke(
            &mut self,
            _handle: &Self::Handle,
            _arguments: &[Value],
        ) -> Result<Value, EngineError> {
            Ok(Value::Empty)
        }

        fn set_budget(&mut self, _budget: Budget) -> Result<(), EngineError> {
            Ok(())
        }

        fn commands_spent(&self) -> Option<u64> {
            None
        }
    }

    /// A command that only answers, counting how it was called.
    struct Answers {
        plain: Cell<u32>,
    }

    impl HostCommand for Answers {
        fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            self.plain.set(self.plain.get() + 1);
            Ok(Value::string("answer").into())
        }
    }

    fn declined<T: std::fmt::Debug>(result: Result<T, EngineError>, what: &'static str) {
        assert_eq!(
            result.map(|_| ()).unwrap_err(),
            EngineError::Unsupported(what)
        );
    }

    #[test]
    fn a_door_without_the_extra_doors_declines_each_of_them() {
        let mut door = Bare;
        declined(door.provide_package("p", "1.0"), "providing a package");
        declined(door.library_loaded("f", "P"), "recording a loaded library");
        declined(door.variable("x"), "reading a variable");
        declined(door.set_variable("x", Value::Empty), "writing a variable");
        declined(door.unset_variable("x"), "unsetting a variable");
        declined(
            door.eval_in_invocation("set x 1"),
            "evaluating a script in an invocation",
        );
    }

    #[test]
    fn an_engine_without_the_extra_doors_declines_each_of_them() {
        let mut engine = Minimal;
        declined(engine.provide_package("p", "1.0"), "providing a package");
        declined(
            engine.library_loaded("f", "P"),
            "recording a loaded library",
        );
        declined(engine.variable("x"), "reading a variable");
        declined(engine.set_variable("x", Value::Empty), "writing a variable");
        declined(engine.unset_variable("x"), "unsetting a variable");
        declined(
            engine.eval_in_invocation("set x 1"),
            "evaluating a script in an invocation",
        );
        declined(engine.remove_command("c"), "removing a command");
    }

    #[test]
    fn a_command_that_only_answers_is_run_by_the_door_entry_point_too() {
        let command = Answers {
            plain: Cell::new(0),
        };
        let outcome = command
            .invoke_with_registrar(&mut Bare, &[])
            .expect("answers");
        assert_eq!(
            command.plain.get(),
            1,
            "the default entry point calls invoke"
        );
        assert_eq!(outcome.value.as_str(), Some("answer"));
        assert_eq!(outcome.code, CompletionCode::Ok, "and completes normally");
    }

    #[test]
    fn a_code_is_the_integer_a_c_command_returns_for_it() {
        for (int, code) in [
            (0, CompletionCode::Ok),
            (2, CompletionCode::Return),
            (3, CompletionCode::Break),
            (4, CompletionCode::Continue),
            (5, CompletionCode::Other(5)),
            (-1, CompletionCode::Other(-1)),
            (1000, CompletionCode::Other(1000)),
        ] {
            assert_eq!(CompletionCode::from_int(int), Some(code), "{int}");
            assert_eq!(code.as_int(), int, "{code:?}");
        }
        assert_eq!(
            CompletionCode::from_int(1),
            None,
            "an error is the Err of a call, not a completion"
        );
    }

    #[test]
    fn only_a_return_carries_options() {
        let plain = HostOutcome::ok(Value::string("v"));
        assert!(plain.options.is_empty(), "{:?}", plain.options);
        let broke = HostOutcome::completing(CompletionCode::Break, Value::Empty);
        assert!(broke.options.is_empty());
        let options = Value::string("-code 1 -level 1 -errorcode {X Y}");
        let returned = HostOutcome::returning(Value::string("msg"), options.clone());
        assert_eq!(returned.code, CompletionCode::Return);
        assert_eq!(returned.value.as_str(), Some("msg"));
        assert_eq!(returned.options.as_str(), options.as_str());
        let converted = HostOutcome::from(Value::Int(3));
        assert!(converted.options.is_empty());
    }

    #[test]
    fn an_outcome_carries_the_code_it_was_given() {
        assert_eq!(CompletionCode::default(), CompletionCode::Ok);
        let outcome = HostOutcome::completing(CompletionCode::Break, Value::string("x"));
        assert_eq!(outcome.code, CompletionCode::Break);
        assert_eq!(outcome.value.as_str(), Some("x"));
        assert_eq!(HostOutcome::from(Value::Int(3)).code, CompletionCode::Ok);
    }
}
