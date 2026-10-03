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

//! `tcl-engine-wasm` — the runtime compiled to `wasm32-wasip1`
//! (`tcl_runtime.wasm`) as an engine of the extension interface, under
//! wasmtime, and the C extensions built for it evaluated on it.
//!
//! - [`WasmRuntime`] is the runtime module, compiled once and shared.
//! - [`Extension`] is a C extension built for the runtime as a side module
//!   (`clang --target=wasm32-wasip1 -fPIC` against the header's WASM leg, then
//!   `wasm-ld -shared`), named by its content hash.
//! - [`WasmEngine`] implements [`Engine`] over one instance of the runtime: a
//!   unit is a procedure, a host command is a host function in the runtime's
//!   table, and the budget is the interpreter's own count of commands and of
//!   value bytes. What no count sees — a C command's own loop, a loop with no
//!   command in it, memory the evaluation grows — is bounded by fuel, by the
//!   epoch and by a cap on the memory's growth.
//! - [`WasmExtensionHost`] is the host the registry's extension seam
//!   (`tcl_registry::extension_host`) binds: each evaluation a fresh instance
//!   with the extension loaded into it.
//!
//! Every WASI function a module imports is a stub that answers the same on
//! every run (`wasi`), so nothing of the machine reaches an evaluation. The
//! language server never links this crate: the registry's seam is all the
//! analysis depends on.

mod dylink;
mod extension_host;
mod host_call;
mod session;
mod wasi;

use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tcl_engine_api::{Budget, CompileUnit, Engine, EngineError, HostCommand, Value};
use wasmtime::{Config, Module};

pub use extension_host::WasmExtensionHost;

use crate::host_call::Defined;
use crate::session::{Completion, Failure, Limits, Session};

/// How often the epoch advances: the wall clock's resolution.
const EPOCH_TICK: Duration = Duration::from_millis(5);

/// Fuel for each command a command budget allows. What no command count sees
/// is bounded by fuel instead: a C command's own loop, a loop with no command
/// in it. Running out is the command budget, outrun. A command costs the
/// runtime built for debugging about 170 000 instructions, and one built for
/// release far fewer, so an ordinary body meets its count long before its
/// fuel.
pub const FUEL_PER_COMMAND: u64 = 2_000_000;

/// Fuel every evaluation has beside its commands' for what the runtime builds
/// the first time it is used in an instance: the command tables of the release
/// it runs, about 700 million instructions in a debug build. No count sees
/// that work, and it is not the body's.
pub const FIRST_USE_FUEL: u64 = 2_000_000_000;

/// The bytes the memory may grow by for each byte of the value-size budget,
/// and at least: room for the interpreter's own allocations beside the value
/// the budget bounds.
const GROWTH_PER_VALUE_BYTE: u64 = 4;
const LEAST_GROWTH: u64 = 32 * 1024 * 1024;

/// The runtime module, compiled once and shared by every engine and
/// evaluation made from it.
#[derive(Clone)]
pub struct WasmRuntime(Arc<Compiled>);

struct Compiled {
    engine: wasmtime::Engine,
    module: Module,
    ticking: Arc<AtomicBool>,
}

impl Drop for Compiled {
    fn drop(&mut self) {
        self.ticking.store(false, Ordering::Relaxed);
    }
}

impl WasmRuntime {
    /// Compile `runtime`, the runtime built for `wasm32-wasip1`, and start
    /// the clock its evaluations' wall-clock budgets are kept by.
    ///
    /// # Errors
    ///
    /// [`EngineError::Crashed`] when wasmtime cannot compile it.
    pub fn new(runtime: &[u8]) -> Result<Self, EngineError> {
        let mut config = Config::new();
        config.consume_fuel(true).epoch_interruption(true);
        let engine = wasmtime::Engine::new(&config).map_err(|error| crashed(&error))?;
        let module = Module::new(&engine, runtime).map_err(|error| crashed(&error))?;
        let ticking = Arc::new(AtomicBool::new(true));
        let (ticker, running) = (engine.clone(), Arc::clone(&ticking));
        std::thread::Builder::new()
            .name("tcl-engine-wasm epoch".to_owned())
            .spawn(move || {
                while running.load(Ordering::Relaxed) {
                    std::thread::sleep(EPOCH_TICK);
                    ticker.increment_epoch();
                }
            })
            .map_err(|error| EngineError::Crashed(error.to_string()))?;
        Ok(Self(Arc::new(Compiled {
            engine,
            module,
            ticking,
        })))
    }

    /// Compile `artefact`, an extension built for the runtime as a side
    /// module, whose entry point is `PREFIX_Init`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Crashed`], saying why, for an artefact that is not a side
    /// module this host can load, or that does not compile.
    pub fn extension(&self, artefact: &[u8], prefix: &str) -> Result<Extension, EngineError> {
        let layout = dylink::layout(artefact).map_err(EngineError::Crashed)?;
        let module = Module::new(&self.0.engine, artefact).map_err(|error| crashed(&error))?;
        Ok(Extension(Arc::new(Linked {
            module,
            layout,
            prefix: prefix.to_owned(),
            hash: tcl_registry::extension_host::artefact_hash(artefact),
        })))
    }

    fn engine(&self) -> &wasmtime::Engine {
        &self.0.engine
    }

    fn module(&self) -> &Module {
        &self.0.module
    }
}

/// A C extension compiled for the runtime: a side module, the prefix its
/// entry point is named by, and its content hash.
#[derive(Clone)]
pub struct Extension(Arc<Linked>);

struct Linked {
    module: Module,
    layout: dylink::Layout,
    prefix: String,
    hash: u64,
}

impl Extension {
    /// The content hash it is named by.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.0.hash
    }

    /// The prefix its entry point is named by.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.0.prefix
    }

    fn module(&self) -> &Module {
        &self.0.module
    }

    fn layout(&self) -> dylink::Layout {
        self.0.layout
    }
}

/// What has been set up on an engine, kept so that an instance a trap left
/// unusable can be rebuilt as it was.
enum Setup {
    Extension(Extension),
    Command(String, Rc<dyn HostCommand>),
    Removed(String),
    Package(String, String),
    Restrict(Vec<String>, Vec<String>),
    Confine,
    Release(&'static str),
    Unit(String, Vec<String>, String),
}

/// A compiled unit: the procedure it was defined as, and how many arguments
/// it takes.
#[derive(Debug, Clone)]
pub struct WasmHandle {
    procedure: String,
    parameters: usize,
}

/// The runtime compiled to `wasm32` as an engine.
pub struct WasmEngine {
    runtime: WasmRuntime,
    /// The instance, `None` once a trap has left it unusable, until the next
    /// operation rebuilds it.
    session: Option<Session>,
    setup: Vec<Setup>,
    budget: Budget,
    release: Option<&'static str>,
    units: u32,
    host_commands: Vec<String>,
    unit_commands: Vec<String>,
    spent: Option<u64>,
}

impl WasmEngine {
    /// A fresh engine over a new instance of `runtime`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Crashed`] when the runtime cannot be instantiated.
    pub fn new(runtime: &WasmRuntime) -> Result<Self, EngineError> {
        let session = Session::new(runtime).map_err(|error| crashed(&error))?;
        Ok(Self {
            runtime: runtime.clone(),
            session: Some(session),
            setup: Vec::new(),
            budget: Budget::default(),
            release: None,
            units: 0,
            host_commands: Vec::new(),
            unit_commands: Vec::new(),
            spent: None,
        })
    }

    /// The catalogue profile the engine's release is pinned to, if one is.
    #[must_use]
    pub fn release(&self) -> Option<&'static str> {
        self.release
    }

    /// Load `extension` into the engine and run its entry point, as `load`
    /// does: what it registers is callable from the units.
    ///
    /// # Errors
    ///
    /// The error its entry point reported, or why it could not be linked.
    pub fn load_extension(&mut self, extension: &Extension) -> Result<(), EngineError> {
        self.apply(Setup::Extension(extension.clone()))
    }

    /// The instance, rebuilt from the setup when a trap left it unusable.
    fn session(&mut self) -> Result<&mut Session, EngineError> {
        if self.session.is_none() {
            let mut session = Session::new(&self.runtime).map_err(|error| crashed(&error))?;
            for setup in &self.setup {
                replay(&mut session, setup).map_err(Failure::into_engine_error)?;
            }
            self.session = Some(session);
        }
        Ok(self.session.as_mut().expect("built above"))
    }

    /// Apply `setup` to the instance and keep it, so a rebuilt instance has it
    /// too.
    fn apply(&mut self, setup: Setup) -> Result<(), EngineError> {
        let session = self.session()?;
        match replay(session, &setup) {
            Ok(()) => {
                self.setup.push(setup);
                Ok(())
            }
            Err(failure) => Err(self.failed(failure)),
        }
    }

    /// The interface's error for `failure`, dropping the instance a trap left
    /// unusable.
    fn failed(&mut self, failure: Failure) -> EngineError {
        if matches!(failure, Failure::Trap(_)) {
            self.session = None;
        }
        failure.into_engine_error()
    }

    /// Keep what the running host commands set up through their doors.
    fn keep_defined(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        for defined in std::mem::take(&mut session.store.data_mut().defined) {
            match defined {
                Defined::Command(name, command) => {
                    remember(&mut self.host_commands, &name);
                    self.setup.push(Setup::Command(name, command));
                }
                Defined::Removed(name) => {
                    forget(&mut self.host_commands, &name);
                    self.setup.push(Setup::Removed(name));
                }
                Defined::Package(name, version) => {
                    self.setup.push(Setup::Package(name, version));
                }
            }
        }
    }
}

/// Apply `setup` to `session`.
fn replay(session: &mut Session, setup: &Setup) -> Result<(), Failure> {
    match setup {
        Setup::Extension(extension) => session.load(extension),
        Setup::Command(name, command) => session.define_command(name, Rc::clone(command)),
        Setup::Removed(name) => session.delete_command(name).map(drop),
        Setup::Package(name, version) => session.provide_package(name, version),
        Setup::Restrict(allowed, kept) => session.restrict(allowed, kept),
        Setup::Confine => session.confine(),
        Setup::Release(profile) => session.set_release(profile),
        Setup::Unit(procedure, parameters, body) => {
            session.define_unit(procedure, parameters, body)
        }
    }
}

/// Add `name`, unrooted, to `names` unless it is there.
fn remember(names: &mut Vec<String>, name: &str) {
    let unrooted = name.trim_start_matches("::").to_owned();
    if !names.contains(&unrooted) {
        names.push(unrooted);
    }
}

/// Take `name`, unrooted, out of `names`.
fn forget(names: &mut Vec<String>, name: &str) {
    let unrooted = name.trim_start_matches("::");
    names.retain(|known| known != unrooted);
}

/// The catalogue profile `profile` names when it is a release the runtime
/// runs, resolved by the rule the runtime resolves it by
/// (`runtime/rust/src/sandbox.rs`, `release_profile`).
fn release_name(profile: &str) -> Option<&'static str> {
    tcl_registry::model::resolve_known_environment(profile)
        .and_then(|environment| environment.catalogue_profile())
        .filter(|resolved| resolved.runtime_base.is_some())
        .map(|resolved| resolved.name)
}

impl Engine for WasmEngine {
    type Handle = WasmHandle;

    fn name(&self) -> &'static str {
        "wasm"
    }

    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.apply(Setup::Command(name.to_owned(), command))?;
        remember(&mut self.host_commands, name);
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        let session = self.session()?;
        let existed = match session.delete_command(name) {
            Ok(existed) => existed,
            Err(failure) => return Err(self.failed(failure)),
        };
        forget(&mut self.host_commands, name);
        self.setup.push(Setup::Removed(name.to_owned()));
        Ok(existed)
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        self.apply(Setup::Package(name.to_owned(), version.to_owned()))
    }

    /// Keep only the `allowed` commands, the host's and the compiled units',
    /// by the rule the runtime's native engine keeps them (an allowed `expr`
    /// keeps the math functions but the generator, an allowed ensemble its
    /// subcommands).
    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        let allowed: Vec<String> = allowed.iter().map(|name| (*name).to_owned()).collect();
        let kept: Vec<String> = self
            .host_commands
            .iter()
            .chain(&self.unit_commands)
            .cloned()
            .collect();
        self.apply(Setup::Restrict(allowed, kept))
    }

    /// Define the unit as a procedure, `::spectcl::unit::N`. The runtime parses
    /// a body when it runs it, so a body that does not parse fails its first
    /// invocation rather than its compilation.
    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        let procedure = format!("::spectcl::unit::{}", self.units + 1);
        let parameters: Vec<String> = unit
            .parameters
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        self.apply(Setup::Unit(
            procedure.clone(),
            parameters,
            unit.body.to_owned(),
        ))?;
        self.units += 1;
        self.unit_commands
            .push(procedure.trim_start_matches("::").to_owned());
        Ok(WasmHandle {
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
        let limits = limits_of(self.budget);
        let texts: Vec<String> = arguments.iter().map(host_call::text_of).collect();
        let mut words: Vec<&[u8]> = vec![handle.procedure.as_bytes()];
        words.extend(texts.iter().map(String::as_bytes));
        let session = self.session()?;
        let answer = session
            .arm(limits)
            .and_then(|()| session.evaluate(&words))
            .and_then(|completion| Ok((completion, session.exceeded()?)));
        self.spent = self.session.as_mut().and_then(Session::commands_spent);
        let panic = self
            .session
            .as_mut()
            .and_then(|session| session.store.data_mut().panic.take());
        // What a door set up stays set up however the evaluation ended, as it
        // does natively, so an instance rebuilt after a trap has it too.
        self.keep_defined();
        let result = match answer {
            Ok((_, Some(kind))) => Err(EngineError::BudgetExceeded(kind)),
            Ok((completion, None)) => completion_value(&completion),
            Err(failure) => Err(self.failed(failure)),
        };
        if let Some(payload) = panic {
            std::panic::resume_unwind(payload);
        }
        result
    }

    fn set_budget(&mut self, budget: Budget) -> Result<(), EngineError> {
        self.budget = budget;
        Ok(())
    }

    fn commands_spent(&self) -> Option<u64> {
        self.spent
    }

    /// Pin the interpreter to the profile `profile` names, as the runtime's
    /// native engine pins it: a name that is no release the runtime runs is
    /// `Unsupported`, and so is a second, different pin once a unit is
    /// compiled.
    fn set_release(&mut self, profile: &str) -> Result<(), EngineError> {
        let Some(name) = release_name(profile) else {
            return Err(EngineError::Unsupported("pinning a release"));
        };
        if self.release == Some(name) {
            return Ok(());
        }
        if self.units > 0 {
            return Err(EngineError::Unsupported(
                "pinning a release after a unit was compiled",
            ));
        }
        self.apply(Setup::Release(name))?;
        self.release = Some(name);
        Ok(())
    }

    fn confine_stores(&mut self) -> Result<(), EngineError> {
        self.apply(Setup::Confine)
    }
}

/// The interface's answer for a completion: its value, or the error it was.
fn completion_value(completion: &Completion) -> Result<Value, EngineError> {
    let text = String::from_utf8_lossy(&completion.result).into_owned();
    match completion.code {
        0 | 2 => Ok(Value::string(text)),
        1 => Err(EngineError::Script {
            message: text,
            code: error_code(&completion.options),
        }),
        _ => Err(EngineError::Script {
            message: text,
            code: None,
        }),
    }
}

/// The `-errorcode` in a completion's options.
fn error_code(options: &[u8]) -> Option<String> {
    let options = String::from_utf8_lossy(options);
    let items = tcl_syntax::list::split_list(&options).ok()?;
    items
        .as_chunks::<2>()
        .0
        .iter()
        .find(|[name, _]| name == "-errorcode")
        .map(|[_, code]| code.to_string())
}

/// The limits an evaluation under `budget` runs with.
fn limits_of(budget: Budget) -> Limits {
    Limits {
        fuel: budget.commands.map(|commands| {
            commands
                .saturating_add(1)
                .saturating_mul(FUEL_PER_COMMAND)
                .saturating_add(FIRST_USE_FUEL)
        }),
        ticks: budget.wall_clock.map(ticks),
        growth: budget.max_value_bytes.map(|bytes| {
            bytes
                .saturating_mul(GROWTH_PER_VALUE_BYTE)
                .max(LEAST_GROWTH)
        }),
        commands: budget.commands,
        value_bytes: budget.max_value_bytes,
    }
}

/// Epoch ticks for `wall_clock`, at least one.
fn ticks(wall_clock: Duration) -> u64 {
    let tick = EPOCH_TICK.as_millis().max(1);
    u64::try_from(wall_clock.as_millis() / tick + 1).unwrap_or(u64::MAX)
}

fn crashed(error: &wasmtime::Error) -> EngineError {
    EngineError::Crashed(format!("{error:#}"))
}
