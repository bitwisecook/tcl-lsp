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

//! What the embedding host plugs in and observes: the host and compiler, the debug hook and line watch, a pending `exit`, channels, the system encoding and output.

use super::super::{DebugHookActivation, ROOT_INTERP, Vm};
use crate::vars::VarState as Local;
use std::io;
use std::io::Write;
use std::rc::Rc;
use tcl_bytecode::ModuleAsm;
use tcl_platform::Host;
use tcl_runtime_api::CompileService;

#[cfg(test)]
fn run_observer_probe(vm: &Vm) {
    if let Some(probe) = vm.debug_hooks.observer_probe {
        probe(vm);
    }
}

#[cfg(not(test))]
const fn run_observer_probe(_: &Vm) {}

impl DebugHookActivation {
    pub(in super::super) fn enter(active: &Rc<std::cell::Cell<bool>>) -> Self {
        let previous = active.replace(true);
        Self {
            active: Rc::clone(active),
            previous,
        }
    }
}

impl Drop for DebugHookActivation {
    fn drop(&mut self) {
        self.active.set(self.previous);
    }
}

impl Vm {
    /// Install a debug hook fired once per source command (the execution-control
    /// seam a step debugger drives). Pass `None` to detach.
    pub fn set_debug_hook(&mut self, hook: Option<crate::debug::DebugHook>) {
        self.native_compiler_pass_owner
            .invalidate_ordinary_entry_observation();
        self.debug_hooks.debug_hook = hook;
        self.debug_hooks.last_debug_key = None;
    }

    /// Install a shared cell the VM keeps set to the source line of the
    /// instruction being dispatched — the cheap sibling of
    /// [`Self::set_debug_hook`] for an embedder that only needs line
    /// attribution (a registered command reads the cell to learn which line
    /// invoked it). Pass `None` to detach.
    pub fn set_line_watch(&mut self, watch: Option<std::rc::Rc<std::cell::Cell<u32>>>) {
        self.debug_hooks.line_watch = watch;
    }

    /// Publish `line` to the line watch, when one is installed.
    #[inline]
    pub(crate) fn note_line(&self, line: u32) {
        if let Some(watch) = &self.debug_hooks.line_watch {
            watch.set(line);
        }
    }

    /// Record a pending `exit` with the given process code. The VM library does
    /// not terminate the process; the driver decides (see [`Self::take_exit`]).
    pub(crate) fn set_exit(&mut self, code: i32) {
        self.pending_exit = Some(code);
    }

    /// Whether an `exit` is pending (the unwinding completion should propagate
    /// uncatchably, like C Tcl's `Tcl_Exit`).
    #[must_use]
    pub fn exit_pending(&self) -> bool {
        self.pending_exit.is_some()
    }

    /// Take the pending `exit` code, if any. A standalone driver (the `tclvm`
    /// CLI) calls this after running and translates `Some(code)` into
    /// `std::process::exit`; an embedder may ignore it and keep running.
    pub fn take_exit(&mut self) -> Option<i32> {
        self.pending_exit.take()
    }

    /// Fire the debug hook for the command an instruction belongs to, if a hook
    /// is installed and this instruction begins a *new* source command (so the
    /// hook fires once per command, not per instruction). Returns `true` when
    /// the hook asked to stop. `span_start` is the instruction's source-span
    /// start (`None` for synthetic instructions, which never fire).
    pub(crate) fn debug_step(
        &mut self,
        line: u32,
        span_start: Option<u32>,
        cmd_text: impl AsRef<[u8]>,
    ) -> bool {
        if self.debug_hooks.debug_hook.is_none() {
            return false;
        }
        let Some(start) = span_start else {
            return false;
        };
        let key = (u64::from(line) << 32) | u64::from(start);
        if self.debug_hooks.last_debug_key == Some(key) {
            return false;
        }
        self.debug_hooks.last_debug_key = Some(key);
        let bytes = cmd_text.as_ref();
        let display = std::str::from_utf8(bytes).map_or_else(
            |_| tcl_bytecode::format::esc_bytes(bytes, bytes.len()),
            str::to_owned,
        );
        self.fire_debug_hook(line, &display)
    }

    /// Build a [`crate::debug::DebugSnapshot`] of the current interpreter state
    /// for a command at `line` with text `cmd_text`. Reads the call stack (top
    /// first) and the current frame's variables.
    pub(crate) fn debug_snapshot(&self, line: u32, cmd_text: &str) -> crate::debug::DebugSnapshot {
        use crate::debug::{DebugFrame, DebugSnapshot, DebugVar};
        let stack: Vec<DebugFrame> = self
            .frames
            .iter()
            .rev()
            .map(|fr| {
                let token = fr.ns.resolved_token();
                let bytes = fr.ns_eval.clone().or_else(|| {
                    let token = token?;
                    if self.uses_native_jim_lookup() {
                        self.name_world
                            .borrow()
                            .jim_namespace_objects
                            .get(&token)
                            .cloned()
                    } else {
                        Some(Self::namespace_path_reporting_bytes(&self.ns_path(token)))
                    }
                });
                let namespace = bytes.as_ref().map_or_else(
                    || "<namespace unavailable>".to_owned(),
                    |bytes| {
                        bytes.try_utf8().map_or_else(
                            |_| {
                                token.map_or_else(
                                    || "<opaque namespace>".to_owned(),
                                    |token| format!("<namespace token {}>", token.0),
                                )
                            },
                            str::to_owned,
                        )
                    },
                );
                DebugFrame {
                    level: u32::try_from(fr.level).unwrap_or(0),
                    name: fr.proc_name.clone().unwrap_or_else(|| "global".to_owned()),
                    namespace,
                }
            })
            .collect();
        let mut variables: Vec<DebugVar> = self.frames.last().map_or_else(Vec::new, |frame| {
            frame
                .local_bindings()
                .filter_map(|(name, id)| {
                    let value = match self.var_arena.get(*id)?.state() {
                        Local::Scalar(value) => value.to_str().to_string(),
                        Local::Array(_) => String::new(),
                        Local::Undefined | Local::Link(_) | Local::NameLink(_) => return None,
                    };
                    let bytes = name.as_bytes();
                    let name = std::str::from_utf8(bytes).map_or_else(
                        |_| tcl_bytecode::format::esc_bytes(bytes, usize::MAX),
                        str::to_owned,
                    );
                    Some(DebugVar { name, value })
                })
                .collect()
        });
        variables.sort_by(|a, b| a.name.cmp(&b.name));
        DebugSnapshot {
            line,
            command_text: cmd_text.to_owned(),
            level: u32::try_from(self.current_level()).unwrap_or(0),
            stack,
            variables,
        }
    }

    /// Fire the debug hook (if installed) for the command at `line` / `cmd_text`.
    /// Returns `true` when the hook asked to stop the run.
    pub(crate) fn fire_debug_hook(&mut self, line: u32, cmd_text: &str) -> bool {
        if self.debug_hooks.debug_hook.is_none() {
            return false;
        }
        let _debug_activation = DebugHookActivation::enter(&self.debug_hooks.debug_hook_active);
        let snapshot = self.debug_snapshot(line, cmd_text);
        // Take the hook out so the closure can borrow `self`-derived data
        // (already captured in `snapshot`) without aliasing the field.
        let mut hook = self.debug_hooks.debug_hook.take();
        run_observer_probe(self);
        let action = hook
            .as_mut()
            .map_or(crate::debug::DebugAction::Continue, |h| h(&snapshot));
        self.debug_hooks.debug_hook = hook;
        action == crate::debug::DebugAction::Stop
    }

    /// Inject the compiler used for runtime compilation, invalidating every
    /// compiler-derived cache and lazily reusable source-bearing body.
    pub fn set_compiler(&mut self, compiler: Box<dyn CompileService<Module = ModuleAsm>>) {
        self.advance_native_operation_context();
        self.compiler = Some(Rc::from(compiler));
        self.compiler_generation = self.compiler_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
    }

    /// The host environment (capability seam) backing the platform commands.
    pub(crate) fn host(&self) -> &dyn Host {
        &*self.host
    }

    /// A cloned handle to the host, so a command can hold `&dyn Host` while also
    /// taking `&mut self` as the `ValueOps` a shared `tcl-cmd-core` helper needs.
    pub(crate) fn host_rc(&self) -> Rc<dyn Host> {
        Rc::clone(&self.host)
    }

    /// Swap the host environment — e.g. a
    /// [`NativeHost::sandboxed`](crate::host_native::NativeHost::sandboxed) to exercise
    /// the WASM-posture "unsupported" paths natively.
    pub fn set_host(&mut self, host: Rc<dyn Host>) {
        if !Rc::ptr_eq(&self.host, &host) {
            self.advance_native_operation_context();
        }
        let system_encoding = host.system_encoding();
        self.host = host;
        // A child has its own host capability object but shares the VM's
        // standard channel handles. Replacing a child's host therefore must
        // not reset configuration established by the root or another child.
        if self.cur == ROOT_INTERP {
            self.system_encoding = system_encoding;
            self.default_system_encoding = system_encoding;
            self.reset_standard_channel_configs();
        }
        self.rebootstrap_host_globals();
        if self.is_safe {
            self.scrub_host_globals_for_safe();
        }
        if self.limits.confined_stores {
            self.scrub_host_globals_for_confinement();
        }
    }
}

impl Vm {
    pub(super) fn reset_standard_channel_configs(&mut self) {
        *self.standard_channels.borrow_mut() = tcl_cmd_core::channel::StandardChannelConfigs::new(
            self.runtime_version,
            self.system_encoding,
        );
    }

    /// The current `encoding system` value shared by this VM's interpreter
    /// tree.
    #[must_use]
    pub(crate) const fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.system_encoding
    }

    pub(crate) const fn default_system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.default_system_encoding
    }

    /// Set the system encoding used by channels opened in the future. Existing
    /// standard and file channel handles retain their current configuration.
    pub(crate) fn set_system_encoding(&mut self, encoding: tcl_platform::SystemEncoding) {
        self.system_encoding = encoding;
    }

    pub(crate) fn write_output_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.execution_refusal.is_some() {
            return Ok(());
        }
        let mut out = self.out.borrow_mut();
        out.write_all(bytes)?;
        out.flush()
    }

    /// Write Tcl-originated text through the mutable `stdout` channel.
    /// Embedders and CLI adapters use this instead of bypassing `fconfigure`.
    ///
    /// # Errors
    /// A channel conversion or raw sink failure.
    pub fn write_stdout_text(
        &mut self,
        text: &str,
        newline: bool,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        crate::cmd_chan::chan_puts(self, "stdout", text, newline)
    }

    /// Write Tcl-originated diagnostics through the mutable `stderr` channel.
    ///
    /// # Errors
    /// A channel conversion failure.
    pub fn write_stderr_text(
        &mut self,
        text: &str,
        newline: bool,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        crate::cmd_chan::chan_puts(self, "stderr", text, newline)
    }

    /// Report a diagnostic through `stderr`, including Tcl's fallback marker
    /// when strict channel conversion can write only a representable prefix.
    /// The marker itself takes the configured newline translation, just like
    /// Tcl's `Tcl_Main` and background-error reporter.
    pub fn report_stderr_text(&mut self, text: &str) {
        if self.write_stderr_text(text, true).is_err() {
            let _ = self.write_stderr_text("\n\t(encoding error in stderr)", true);
        }
    }

    /// Register a freshly opened channel, returning its minted id (`file3`, …).
    pub(crate) fn add_channel(&mut self, chan: crate::cmd_chan::Channel) -> String {
        let id = format!("file{}", self.counters.chan_counter);
        self.counters.chan_counter += 1;
        self.channels.insert(id.clone(), chan);
        id
    }

    /// Borrow an open channel by id (`None` for unknown ids and the predefined
    /// std channels, which callers handle by name).
    pub(crate) fn channel_mut(&mut self, id: &str) -> Option<&mut crate::cmd_chan::Channel> {
        self.channels.get_mut(id)
    }

    /// Borrow an open channel by id without mutating its handle.
    pub(crate) fn channel(&self, id: &str) -> Option<&crate::cmd_chan::Channel> {
        self.channels.get(id)
    }

    /// Close and drop a channel by id, returning `true` if it existed.
    pub(crate) fn remove_channel(&mut self, id: &str) -> bool {
        self.channels.remove(id).is_some()
    }
}
