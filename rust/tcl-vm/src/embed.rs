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

//! The embedder surface: what a Rust host needs to run small Tcl bodies at
//! rate, register commands of its own, and bound what a body may do.
//!
//! Everything here is issue #1373: the gaps `SpecTcl` hook benchmarking found in
//! the VM's boundary, closed in the VM rather than shimmed around in the
//! embedder.
//!
//! - **Call a function without a driver script.** [`Vm::invoke_command`] is
//!   public, and [`Vm::compile_function`] / [`Vm::invoke_function`] pre-compile
//!   a body once into a [`FunctionHandle`] and run it with no per-call
//!   `FunctionAsm` clone — the deep copy [`Vm::run_function`] performs.
//! - **Register a stateful command.** [`Vm::register_native_command`] takes an
//!   `Rc<dyn NativeCommand>`, so an embedder's command can carry state (the
//!   emitter verbs of a `SpecTcl` hook family collect what the body emitted).
//! - **Restrict the command table.** [`Vm::retain_commands`] reduces a fresh VM
//!   to a closed whitelist, which is how a sandbox is built out of a normal
//!   interpreter rather than a second one.
//! - **Bound the work.** [`Vm::set_command_limit`] arms the `commands` limit
//!   the VM now enforces, and [`Vm::commands_run`] reports the fuel spent.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_runtime_api::{Completion, ScriptCompileTarget};

use crate::command::{Command, NativeCommand};
use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// A pre-compiled Tcl body, ready to run any number of times.
///
/// Holds the compiled activation behind an `Rc`, so invoking it is a pointer
/// clone rather than a deep copy of the bytecode — the difference between a
/// hook body that costs its own execution and one that pays for its
/// compilation shape on every call site. It also retains its source to refresh
/// the activation when the owning VM changes dialect profile or namespace, or
/// a specialised command binding no longer resolves to the implementation it
/// assumed.
#[derive(Clone)]
pub struct FunctionHandle {
    source: String,
    state: Rc<RefCell<FunctionHandleState>>,
}

struct FunctionHandleState {
    unit: crate::compiled::CompiledUnit,
    owner_nonce: u64,
}

impl std::fmt::Debug for FunctionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionHandle")
            .field("source", &self.source)
            .field("owner_nonce", &self.state.borrow().owner_nonce)
            .field(
                "instructions",
                &self.state.borrow().unit.asm.instructions.len(),
            )
            .field(
                "profile_generation",
                &self.state.borrow().unit.profile_generation,
            )
            .field("command_epoch", &self.state.borrow().unit.command_epoch)
            .field(
                "source_namespace",
                &self.state.borrow().unit.source_namespace,
            )
            .field(
                "compiler_generation",
                &self.state.borrow().unit.compiler.generation(),
            )
            .finish()
    }
}

impl Vm {
    /// Compile `src` to a reusable [`FunctionHandle`], registering any procs it
    /// defines. Needs an injected
    /// [`CompileService`](tcl_runtime_api::CompileService), like every other
    /// runtime-compilation path.
    pub fn compile_function(&mut self, src: &str) -> Result<FunctionHandle, TclError> {
        Ok(FunctionHandle {
            source: src.to_owned(),
            state: Rc::new(RefCell::new(FunctionHandleState {
                unit: self.compile_script_cached(src)?,
                owner_nonce: self.owner_nonce,
            })),
        })
    }

    /// Define a Tcl procedure from the host, without going through the `proc`
    /// command.
    ///
    /// The body compiles once, here, and every later call runs that bytecode.
    /// Host-defined rather than `proc`-defined because an embedder building a
    /// sandbox removes `proc` from the command table
    /// ([`Self::retain_commands`]) — the procedure it wants to *call* must not
    /// depend on the command it wants to *forbid*, and ordering the two would
    /// be a trap.
    ///
    /// Proc semantics are what make each invocation independent: its own
    /// locals, and `return` as an ordinary early exit rather than an error.
    pub fn define_procedure(
        &mut self,
        name: &str,
        parameters: &[&str],
        body: &str,
    ) -> Result<(), TclError> {
        let registered = self.qualify_name(name);
        let namespace = crate::interp::key_holder_and_tail_unrooted(&registered).0;
        let (parameters, has_args) =
            crate::command::parse_params(&parameters.join(" ")).map_err(TclError::new)?;
        let compiled = self
            .prepare_procedure_body(None, &parameters, &namespace, body)
            .map_err(|mut error| {
                error.message = format!("procedure \"{name}\": {}", error.message);
                error
            })?;
        let ns_id = self.definition_namespace_token(&namespace);
        self.define_proc(crate::command::ProcDef {
            name: registered,
            namespace,
            ns_id,
            params: parameters,
            has_args,
            body: compiled,
            body_src: Value::string(body),
            usage_name: None,
        });
        Ok(())
    }

    /// Arm a wall-clock cap: `budget` from now, after which the VM's poll
    /// ([`interp limit time`](Vm::limit_check_tick)) fails the running body
    /// with `time limit exceeded`. `None` disarms it.
    ///
    /// Polled rather than pre-emptive — a body between polls can overrun by
    /// the poll interval, which is why the command budget, not this, is the
    /// containment guarantee. This is the belt to its braces: it catches a
    /// body that spends its time inside *one* command.
    pub fn set_wall_clock_budget(&mut self, budget: Option<std::time::Duration>) {
        let deadline = budget.map(|budget| {
            let now = self.host_rc().clock().now_millis();
            let millis = i128::try_from(budget.as_millis()).unwrap_or(i128::from(i64::MAX));
            now.saturating_add(millis)
        });
        self.set_time_limit_deadline(deadline);
    }

    /// Run a pre-compiled body to completion in the current frame.
    ///
    /// The invoke-by-handle path: no compilation, no `FunctionAsm` clone, and
    /// no driver script — the three costs an embedder previously paid per
    /// call.
    #[must_use]
    pub fn invoke_function(&mut self, handle: &FunctionHandle) -> Completion<Value> {
        if handle.state.borrow().owner_nonce != self.owner_nonce {
            return crate::interp::err("FunctionHandle belongs to a different Vm");
        }
        let profile_changed =
            handle.state.borrow().unit.profile_generation != self.profile_generation();
        let compiler_changed = !handle
            .state
            .borrow()
            .unit
            .compiler
            .is_current_service(self.compiler_generation());
        let epoch_changed = handle.state.borrow().unit.command_epoch != self.trace_deopt_epoch();
        let namespace = self.current_ns().to_owned();
        let namespace_changed = handle.state.borrow().unit.source_namespace != namespace;
        let bindings_match = self.function_command_bindings_match(&handle.state.borrow().unit.asm);
        if profile_changed
            || compiler_changed
            || epoch_changed
            || namespace_changed
            || !bindings_match
        {
            let current_is_plain = handle.state.borrow().unit.asm.plain_command_dispatch;
            let plain_target = || ScriptCompileTarget {
                source: &handle.source,
                namespace: &namespace,
            };
            let asm = match if self.step_trace_active() {
                self.compile_plain_function_cached(plain_target()).map(Some)
            } else if profile_changed || compiler_changed || namespace_changed {
                self.compile_fast_function_for_namespace(&handle.source, &namespace)
            } else if !current_is_plain && !bindings_match {
                self.compile_plain_function_cached(plain_target()).map(Some)
            } else if epoch_changed && current_is_plain {
                self.compile_fast_function_for_namespace(&handle.source, &namespace)
            } else {
                Ok(Some(Rc::clone(&handle.state.borrow().unit.asm)))
            } {
                Ok(Some(asm)) => asm,
                Ok(None) => match self.compile_plain_function_cached(plain_target()) {
                    Ok(asm) => asm,
                    Err(error) => return crate::command::completion_from_tcl_error(error),
                },
                Err(error) => return crate::command::completion_from_tcl_error(error),
            };
            *handle.state.borrow_mut() = FunctionHandleState {
                unit: self.compiled_unit(asm, namespace),
                owner_nonce: self.owner_nonce,
            };
        }
        self.run_compiled_unit(handle.state.borrow().unit.clone())
    }

    /// Register an embedder command carrying its own state.
    ///
    /// The name is a plain command name (`fold`, `role`) or a qualified one;
    /// it replaces any existing command of that name, exactly as a `proc`
    /// redefinition does.
    pub fn register_native_command(&mut self, name: &str, command: Rc<dyn NativeCommand>) {
        let canonical = name.strip_prefix("::").unwrap_or(name);
        self.register_command(canonical, Command::Native(command));
    }

    /// Every command name currently registered, sorted.
    #[must_use]
    pub fn command_names(&self) -> Vec<String> {
        let mut names = self.registered_command_names();
        names.sort_unstable();
        names
    }

    /// Delete `name` from the command table, reporting whether it was there.
    pub fn remove_command(&mut self, name: &str) -> bool {
        // This is an embedder-owned teardown API, not Tcl's `rename`/`interp
        // hide` surface.  Keep it able to remove a command even when the
        // command is hidden by the emulated release.
        self.take_command_unchecked(name).is_some()
    }

    /// Reduce the command table to the commands `keep` accepts, returning how
    /// many were removed.
    ///
    /// This is the sandbox constructor: a fresh [`Vm`] with the whole builtin
    /// surface is narrowed to a closed whitelist, so a body cannot reach a
    /// command the embedder did not choose — including one the VM gains in a
    /// later release, which is why a whitelist and not a blacklist.
    pub fn retain_commands(&mut self, keep: &dyn Fn(&str) -> bool) -> usize {
        let doomed: Vec<String> = self
            .registered_command_names()
            .into_iter()
            .filter(|name| !keep(name))
            .collect();
        let removed = doomed.len();
        for name in doomed {
            self.remove_registered_command(&name);
        }
        removed
    }

    /// Arm the `commands` limit: the body may dispatch at most `limit`
    /// commands before every further dispatch fails with `command count limit
    /// exceeded`. `None` disarms it.
    ///
    /// The count is the interp's own ([`Self::commands_run`]), so refill fuel
    /// per invocation with [`Self::reset_command_count`].
    pub fn set_command_limit(&mut self, limit: Option<u64>) {
        let value = limit.map(|limit| i64::try_from(limit).unwrap_or(i64::MAX));
        self.set_command_limit_value(value);
    }

    /// Arm the value-size limit: any single value the body builds may be at
    /// most `bytes` long, and an attempt to exceed it fails with `value size
    /// limit exceeded`. `None` disarms it.
    ///
    /// The `commands` and `time` limits cannot express this. One `string
    /// repeat` opcode dispatches a single command and returns in microseconds
    /// while asking the allocator for an arbitrary amount, so both budgets are
    /// still nearly full when the process dies of OOM — the one sandbox
    /// failure that cannot be contained and reported, because it takes the
    /// server down with the hook.
    pub fn set_value_size_limit(&mut self, bytes: Option<u64>) {
        self.set_value_size_limit_value(bytes);
    }

    /// The armed value-size limit, if any.
    #[must_use]
    pub fn value_size_limit(&self) -> Option<u64> {
        self.value_size_limit_value()
    }

    /// The armed `commands` limit, if any.
    #[must_use]
    pub fn command_limit(&self) -> Option<u64> {
        self.command_limit_value()
            .map(|value| u64::try_from(value).unwrap_or(0))
    }

    /// Commands dispatched since the last [`Self::reset_command_count`].
    #[must_use]
    pub fn commands_run(&self) -> u64 {
        self.command_count()
    }

    /// Zero the command counter — refill the fuel before an invocation.
    pub fn reset_command_count(&mut self) {
        self.reset_command_count_inner();
    }
}
