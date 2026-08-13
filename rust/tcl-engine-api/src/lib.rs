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
//! The bottom of the two layers `docs/design/spec-packs.md` describes: the
//! surface that plays, for our engines, the role Tcl's C API plays for C Tcl —
//! in modern idiomatic Rust, with traits and owned structured values, no raw
//! interp pointers, and no C-shaped warts.
//!
//! ```text
//!   hook host  (tcl-spec-hooks)     <- consumer 1: written in Rust on top
//!   C-Tcl shim (#1372, later)       <- consumer 2: in view, not yet built
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
//! 2. **Designed for two use cases, built for the first.** The shape is what
//!    the hook host needs today; the C-Tcl shim's only claim on it now is that
//!    nothing here precludes it.
//! 3. **All C-required mangling lives in the shim.** String lifetimes, interp
//!    pointers, and result codes stay on the far side of that shim: this
//!    interface speaks [`Value`] and `Result`.
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

/// A command implemented by the host and callable from Tcl.
///
/// The emitter verbs (`role`, `fold`, `reject`, …) and any host-supplied
/// builtin (the conservative `foldlist`) arrive this way. Deliberately
/// *engine-blind*: an implementation receives values and returns a value or an
/// error, and cannot reach the interpreter — which is what keeps a host
/// portable across engines, and a hook body free of ambient authority.
pub trait HostCommand {
    /// Run the command with the call's arguments (the command name excluded).
    fn invoke(&self, arguments: &[Value]) -> Result<Value, EngineError>;
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

    /// What the last invocation actually spent, when the engine can say —
    /// commands dispatched. `None` from an engine with no counter.
    fn commands_spent(&self) -> Option<u64>;
}
