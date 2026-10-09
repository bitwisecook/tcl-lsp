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

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_engine_api::{
    Budget, BudgetKind, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, Value,
};
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Completion, NativeCommand, Vm};

/// A compiled unit: the VM procedure the body was defined as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmHandle {
    procedure: String,
    parameters: usize,
}

impl VmHandle {
    /// The VM procedure name this handle invokes. Exposed for diagnostics.
    #[must_use]
    pub fn procedure(&self) -> &str {
        &self.procedure
    }
}

/// The names registered through [`Engine::define_command`] — shared with
/// every [`HostCommandShim`] so a command registered from inside another's
/// invocation is kept by a later [`Engine::restrict_commands`] too.
type HostCommandNames = Rc<RefCell<Vec<String>>>;

/// Adapts a [`HostCommand`] to the VM's [`NativeCommand`], converting values
/// at the boundary in both directions.
struct HostCommandShim {
    command: Rc<dyn HostCommand>,
    host_commands: HostCommandNames,
}

/// The registration door a host command gets while it runs: the VM the
/// native-command seam already hands over, plus the engine's name list.
struct VmRegistrar<'a> {
    vm: &'a mut Vm,
    host_commands: HostCommandNames,
}

impl CommandRegistrar for VmRegistrar<'_> {
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(self.vm, &self.host_commands, name, command);
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        Ok(remove_host_command(self.vm, &self.host_commands, name))
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        provide_package(self.vm, name, version)
    }

    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        self.vm.library_loaded(file_name, prefix);
        Ok(())
    }

    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        read_variable(self.vm, name)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        set_variable(self.vm, name, &value)
    }

    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        unset_variable(self.vm, name)
    }

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        evaluate(self.vm, script)
    }
}

fn provide_package(vm: &mut Vm, name: &str, version: &str) -> Result<(), EngineError> {
    vm.package_provide(name, version)
        .map_err(|error| EngineError::Script {
            message: error.message,
            code: error.error_code,
        })
}

fn read_variable(vm: &mut Vm, name: &str) -> Result<Value, EngineError> {
    vm.read_variable(name)
        .map(|value| from_vm_value(&value))
        .map_err(|completion| script_error(&completion))
}

fn set_variable(vm: &mut Vm, name: &str, value: &Value) -> Result<(), EngineError> {
    vm.write_variable(name, to_vm_value(value))
        .map_err(|completion| script_error(&completion))
}

fn unset_variable(vm: &mut Vm, name: &str) -> Result<(), EngineError> {
    vm.unset_variable(name)
        .map_err(|completion| script_error(&completion))
}

/// Evaluate `script` in the VM's current frame and report how it completed: a
/// normal completion, `return`, `break` and `continue` as the code they are, and
/// an error as the error, a budget one as the budget it outran.
fn evaluate(vm: &mut Vm, script: &str) -> Result<HostOutcome, EngineError> {
    let completion = match vm.eval_source(script) {
        Ok(completion) => completion,
        Err(error) => {
            let options = error
                .error_code
                .as_deref()
                .map_or_else(tcl_vm::Value::empty, |code| {
                    tcl_vm::Value::list(vec![
                        tcl_vm::Value::string("-errorcode"),
                        tcl_vm::Value::string(code),
                    ])
                });
            vm.publish_caught_error(&Completion::new(
                Code::Error,
                tcl_vm::Value::string(error.message.as_str()),
                options,
            ));
            return Err(EngineError::Script {
                message: error.message,
                code: error.error_code,
            });
        }
    };
    let code = match completion.code {
        Code::Ok => CompletionCode::Ok,
        Code::Return => CompletionCode::Return,
        Code::Break => CompletionCode::Break,
        Code::Continue => CompletionCode::Continue,
        Code::Error => {
            let failure = script_error(&completion);
            if matches!(failure, EngineError::Script { .. }) {
                // The host command takes the error as its own, so `$errorCode`
                // and `$errorInfo` are what a `catch` of the script would leave.
                vm.publish_caught_error(&completion);
            }
            return Err(failure);
        }
        Code::Other(other) => CompletionCode::Other(other),
    };
    let value = from_vm_value(&completion.result);
    Ok(if code == CompletionCode::Return {
        HostOutcome::returning(value, from_vm_value(&completion.options))
    } else {
        HostOutcome::completing(code, value)
    })
}

/// The error a failed completion is, with the `-errorcode` its options carry and
/// a budget the VM reported as the budget it outran.
fn script_error(completion: &Completion<tcl_vm::Value>) -> EngineError {
    let message = completion.result.to_str().to_string();
    if let Some(kind) = budget_kind(&message) {
        return EngineError::BudgetExceeded(kind);
    }
    let code = error_code_of(&completion.options);
    EngineError::Script { message, code }
}

/// The `-errorcode` in a completion's options dict, as list text.
fn error_code_of(options: &tcl_vm::Value) -> Option<String> {
    let items = options.as_list().ok()?;
    items
        .as_chunks::<2>()
        .0
        .iter()
        .find(|pair| &*pair[0].to_str() == "-errorcode")
        .map(|pair| pair[1].to_str().to_string())
}

/// Which budget a VM error message says the body outran, if it says one.
fn budget_kind(message: &str) -> Option<BudgetKind> {
    match message {
        "command count limit exceeded" => Some(BudgetKind::Commands),
        "time limit exceeded" => Some(BudgetKind::WallClock),
        "value size limit exceeded" => Some(BudgetKind::ValueSize),
        _ => None,
    }
}

/// The message the VM reports when a body outruns `kind` of budget.
fn budget_message(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Commands => "command count limit exceeded",
        BudgetKind::WallClock => "time limit exceeded",
        BudgetKind::ValueSize => "value size limit exceeded",
    }
}

/// The VM's code for a host command's [`CompletionCode`].
fn to_vm_code(code: CompletionCode) -> Code {
    Code::from_int(code.as_int())
}

/// The options a host command's completion carries to the VM: those of the
/// `return` behind a `Return`, which the calling procedure's boundary reads, and
/// none for any other code.
fn completion_options(outcome: &HostOutcome) -> tcl_vm::Value {
    if outcome.code == CompletionCode::Return {
        to_vm_value(&outcome.options)
    } else {
        tcl_vm::Value::string(String::new())
    }
}

fn define_host_command(
    vm: &mut Vm,
    host_commands: &HostCommandNames,
    name: &str,
    command: Rc<dyn HostCommand>,
) {
    vm.register_native_command(
        name,
        Rc::new(HostCommandShim {
            command,
            host_commands: Rc::clone(host_commands),
        }),
    );
    host_commands.borrow_mut().push(name.to_owned());
}

/// Register `command` as the host command `name` on a bare [`Vm`].
///
/// What [`Engine::define_command`] does, for an embedder that drives a VM
/// itself and not through the interface (the `tclvm` binary): the command
/// converts values at the boundary the same way, and runs with the
/// registration door open, so a command that creates commands publishes them
/// before the calling script's next statement.
pub fn register_host_command(vm: &mut Vm, name: &str, command: Rc<dyn HostCommand>) {
    define_host_command(vm, &Rc::new(RefCell::new(Vec::new())), name, command);
}

/// Whether `name` is the direct form of a subcommand of an ensemble `allowed`
/// names: `tcl::ENSEMBLE::SUBCOMMAND`, in the VM's canonical unrooted spelling.
fn is_subcommand_of_allowed(name: &str, allowed: &[&str]) -> bool {
    name.strip_prefix("tcl::")
        .and_then(|rest| rest.split_once("::"))
        .is_some_and(|(ensemble, _)| allowed.contains(&ensemble))
}

fn remove_host_command(vm: &mut Vm, host_commands: &HostCommandNames, name: &str) -> bool {
    host_commands.borrow_mut().retain(|command| command != name);
    vm.remove_command(name)
}

impl NativeCommand for HostCommandShim {
    fn invoke(&self, vm: &mut Vm, arguments: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let arguments: Vec<Value> = arguments.iter().map(from_vm_value).collect();
        let mut registrar = VmRegistrar {
            vm,
            host_commands: Rc::clone(&self.host_commands),
        };
        match self
            .command
            .invoke_with_registrar(&mut registrar, &arguments)
        {
            Ok(outcome) => Completion::new(
                to_vm_code(outcome.code),
                to_vm_value(&outcome.value),
                completion_options(&outcome),
            ),
            // A script error crosses as the Tcl error it is: the message
            // verbatim, and its `-errorcode` in the completion's options so
            // `catch` and `$errorCode` see what the host command set.
            Err(EngineError::Script { message, code }) => {
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
            // A budget the host command's own evaluation outran stays the VM's
            // message, so the invocation that called it reports the budget.
            Err(EngineError::BudgetExceeded(kind)) => Completion::new(
                Code::Error,
                tcl_vm::Value::string(budget_message(kind)),
                tcl_vm::Value::string(String::new()),
            ),
            Err(error) => Completion::new(
                Code::Error,
                tcl_vm::Value::string(error.to_string()),
                tcl_vm::Value::string(String::new()),
            ),
        }
    }
}

/// Convert an interface value to the VM's representation.
///
/// A dict becomes the flat key/value list that *is* a Tcl dict — no string
/// round-trip, and `dict get` works on it directly.
fn to_vm_value(value: &Value) -> tcl_vm::Value {
    match value {
        Value::Empty => tcl_vm::Value::string(String::new()),
        Value::Str(text) => tcl_vm::Value::string(text.to_string()),
        Value::Int(number) => tcl_vm::Value::int(*number),
        Value::Double(number) => tcl_vm::Value::double(*number),
        Value::List(items) => tcl_vm::Value::list(items.iter().map(to_vm_value).collect()),
        Value::Dict(entries) => {
            let mut flat = Vec::with_capacity(entries.len() * 2);
            for (key, item) in entries.iter() {
                flat.push(to_vm_value(key));
                flat.push(to_vm_value(item));
            }
            tcl_vm::Value::list(flat)
        }
    }
}

/// Convert a VM value to the interface's representation.
///
/// Deliberately string-shaped. The VM's value is dual-rep, and asking it "are
/// you *really* a list" is not a question — it is a *conversion*, which
/// installs a list intrep on every string that happens to parse as one and
/// changes what the body's own later use of that value costs. The host parses
/// what its protocol expects; the boundary reports what the body produced.
fn from_vm_value(value: &tcl_vm::Value) -> Value {
    Value::string(&*value.to_str())
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
    /// Command names registered through [`Engine::define_command`] or a
    /// running command's registrar, kept so a later
    /// [`Engine::restrict_commands`] does not remove them.
    host_commands: HostCommandNames,
    /// The procedures compiled units were defined as — likewise kept, since a
    /// unit compiled before the whitelist was applied must still be callable
    /// after it.
    unit_commands: Vec<String>,
}

impl TclVmEngine {
    /// A fresh engine with the default command registry driving its compiler.
    #[must_use]
    pub fn new() -> Self {
        Self::with_registry(CommandRegistry::build_default())
    }

    /// A fresh engine whose compiler resolves against `registry`.
    #[must_use]
    pub fn with_registry(registry: CommandRegistry) -> Self {
        // Building the VM pins its default release, which installs that
        // release's grammar for the whole thread.
        let _grammar = GrammarGuard::keep();
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(BytecodeCompileService::new(registry)));
        Self {
            vm,
            budget: Budget::default(),
            release: None,
            units: 0,
            host_commands: Rc::new(RefCell::new(Vec::new())),
            unit_commands: Vec::new(),
        }
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
    fn completion_to_result(completion: &Completion<tcl_vm::Value>) -> Result<Value, EngineError> {
        if completion.code.is_ok() || completion.code == Code::Return {
            return Ok(from_vm_value(&completion.result));
        }
        let message = completion.result.to_str().to_string();
        match budget_kind(&message) {
            Some(kind) => Err(EngineError::BudgetExceeded(kind)),
            None => Err(EngineError::Script {
                message,
                code: error_code_of(&completion.options),
            }),
        }
    }
}

impl Default for TclVmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for TclVmEngine {
    type Handle = VmHandle;

    fn name(&self) -> &'static str {
        "tclvm"
    }

    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        define_host_command(&mut self.vm, &self.host_commands, name, command);
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        Ok(remove_host_command(&mut self.vm, &self.host_commands, name))
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
        let host_commands = self.host_commands.borrow().clone();
        let unit_commands = self.unit_commands.clone();
        let math = allowed.contains(&"expr");
        self.vm.retain_commands(&|name| {
            allowed.contains(&name)
                || host_commands.iter().any(|command| command == name)
                || unit_commands.iter().any(|command| command == name)
                || (math
                    && name
                        .strip_prefix("tcl::mathfunc::")
                        .is_some_and(|function| !matches!(function, "rand" | "srand")))
                || is_subcommand_of_allowed(name, allowed)
        });
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
        let procedure = format!("::spectcl::unit::{}", self.units);
        self.vm
            .define_procedure(&procedure, unit.parameters, unit.body)
            .map_err(|error| EngineError::Compile(error.message))?;
        // The VM's command table is keyed by the canonical *unrooted* name, so
        // that is what the whitelist sweep compares against.
        self.unit_commands
            .push(procedure.trim_start_matches("::").to_owned());
        Ok(VmHandle {
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
        let arguments: Vec<tcl_vm::Value> = arguments.iter().map(to_vm_value).collect();
        let _grammar = self.claim_grammar();
        // Fuel is per invocation, so refill before every call rather than
        // letting a long-lived engine starve its own later hooks.
        self.vm.reset_command_count();
        if let Some(wall_clock) = self.budget.wall_clock {
            self.vm.set_wall_clock_budget(Some(wall_clock));
        }
        let completion = self.vm.invoke_command(&handle.procedure, &arguments);
        Self::completion_to_result(&completion)
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
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use super::{Engine, TclVmEngine, register_host_command};
    use tcl_engine_api::{
        Budget, BudgetKind, CommandRegistrar, CompileUnit, EngineError, HostCommand, HostOutcome,
        Value,
    };

    struct Collector {
        emitted: RefCell<Vec<Vec<String>>>,
    }

    impl HostCommand for Collector {
        fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            self.emitted.borrow_mut().push(
                arguments
                    .iter()
                    .map(|argument| argument.as_str().unwrap_or_default().to_owned())
                    .collect(),
            );
            Ok(Value::Empty.into())
        }
    }

    fn unit(body: &str) -> CompileUnit<'_> {
        CompileUnit {
            name: "test",
            parameters: &["words", "ctx"],
            body,
        }
    }

    fn registry_with_custom_list_expr_hook() -> tcl_registry::CommandRegistry {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let mut custom = registry.get("list").expect("list spec").clone();
        custom.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
        custom.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
        registry.insert(custom);
        registry
    }

    #[test]
    fn a_unit_compiles_once_and_invokes_with_structured_values() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("return [llength $words]"))
            .expect("the body compiles");
        let words = Value::list([Value::string("a"), Value::string("b")]);
        let ctx = Value::dict_of([("nwords", Value::Int(2))]);
        let result = engine
            .invoke(&handle, &[words, ctx])
            .expect("the body runs");
        assert_eq!(result.as_str(), Some("2"));
    }

    #[test]
    fn an_owned_registry_hook_survives_vm_profile_compilation() {
        let mut engine = TclVmEngine::with_registry(registry_with_custom_list_expr_hook());
        let handle = engine
            .compile(unit("list {1 + 2}"))
            .expect("the custom Expr hook compiles");
        let result = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("the custom hook's bytecode runs");

        // The normal `list` command returns the literal `1 + 2`; the owned
        // registry deliberately assigns it the Expr hook, which returns 3.
        assert_eq!(result.as_str(), Some("3"));
    }

    #[test]
    fn a_dict_crosses_as_a_dict() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("return [dict get $ctx command]"))
            .expect("compiles");
        let ctx = Value::dict_of([
            ("command", Value::string("mylib::sort")),
            ("nwords", Value::Int(0)),
        ]);
        let result = engine
            .invoke(&handle, &[Value::list([]), ctx])
            .expect("runs");
        assert_eq!(result.as_str(), Some("mylib::sort"));
    }

    #[test]
    fn a_host_command_receives_what_the_body_emits() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine
            .define_command("role", collector.clone())
            .expect("registers");
        let handle = engine
            .compile(unit("role 0 varwrite\nrole 1 body"))
            .expect("compiles");
        engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("runs");
        assert_eq!(
            *collector.emitted.borrow(),
            vec![
                vec!["0".to_string(), "varwrite".to_string()],
                vec!["1".to_string(), "body".to_string()],
            ],
        );
    }

    /// A host command that creates another through the door it is given.
    struct Factory(std::rc::Rc<Collector>);

    impl HostCommand for Factory {
        fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::Unsupported("a factory needs the door"))
        }

        fn invoke_with_registrar(
            &self,
            registrar: &mut dyn CommandRegistrar,
            _arguments: &[Value],
        ) -> Result<HostOutcome, EngineError> {
            registrar.define_command("made", self.0.clone())?;
            Ok(Value::string("built").into())
        }
    }

    #[test]
    fn a_host_command_registers_on_a_bare_vm_with_the_door_open() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        register_host_command(
            engine.vm_mut(),
            "factory",
            std::rc::Rc::new(Factory(collector.clone())),
        );
        let handle = engine
            .compile(unit("set built [factory]\nmade $built 2"))
            .expect("compiles");
        engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("a command registered on the VM can register another, and both run");
        assert_eq!(
            *collector.emitted.borrow(),
            vec![vec!["built".to_string(), "2".to_string()]],
        );
    }

    #[test]
    fn a_command_nobody_registered_on_the_vm_is_not_there() {
        let mut engine = TclVmEngine::new();
        let handle = engine.compile(unit("factory")).expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("no such command");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message == "invalid command name \"factory\""),
            "{error:?}"
        );
    }

    #[test]
    fn an_error_in_the_body_is_reported_not_propagated() {
        let mut engine = TclVmEngine::new();
        let handle = engine.compile(unit("error boom")).expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the body raises");
        assert!(
            matches!(&error, EngineError::Script { message, .. } if message == "boom"),
            "{error:?}"
        );
    }

    #[test]
    fn the_command_budget_is_enforced_and_distinguishable() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(200).with_wall_clock(Duration::from_secs(5)))
            .expect("the VM enforces both");
        // Dispatch-driven: `[$cmd length abc]` resolves a computed command
        // name, so every iteration is a real dispatch and is charged.
        let handle = engine
            .compile(unit(
                "set cmd string\nset i 0\nwhile {$i < 100000} { set x [$cmd length abc]\n incr i }",
            ))
            .expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the budget stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::Commands));
    }

    /// The third budget, and the reason it exists: one `string repeat` spends
    /// a single command and no measurable time while asking the allocator for
    /// as much as it likes. With only the other two armed this reaches OOM —
    /// which kills the process rather than the hook, so the host can neither
    /// report nor quarantine it. Here it is an ordinary, catchable refusal.
    #[test]
    fn the_value_size_budget_stops_an_allocation_the_other_two_cannot_see() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(
                Budget::of_commands(1_000_000)
                    .with_wall_clock(Duration::from_secs(5))
                    .with_max_value_bytes(1024),
            )
            .expect("the VM enforces all three");
        let handle = engine
            .compile(unit("string repeat aaaaaaaa 1000000"))
            .expect("compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the value-size budget stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::ValueSize));
    }

    /// The same call under the cap still works: the budget bounds a runaway,
    /// it does not ban the command.
    #[test]
    fn a_value_within_the_size_budget_is_built_normally() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(1_000).with_max_value_bytes(1024))
            .expect("the VM enforces it");
        let handle = engine
            .compile(unit("string repeat ab 8"))
            .expect("compiles");
        let value = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("well within the cap");
        assert_eq!(value.as_str(), Some("abababababababab"));
    }

    /// A loop the compiler inlines dispatches nothing, so the command budget
    /// never fires on it — the wall clock is what contains that case, and the
    /// two together are why the host sets both.
    #[test]
    fn the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see() {
        let mut engine = TclVmEngine::new();
        engine
            .set_budget(Budget::of_commands(1_000_000).with_wall_clock(Duration::from_millis(50)))
            .expect("the VM enforces both");
        let handle = engine
            .compile(unit("set i 0\nwhile {1} { incr i }"))
            .expect("compiles");
        let started = std::time::Instant::now();
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("the wall clock stops it");
        assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::WallClock));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the cap must fire promptly: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_restricted_engine_has_only_the_whitelist_and_the_host_commands() {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine.define_command("fold", collector).expect("registers");
        let handle = engine
            .compile(unit("fold [string length [lindex $words 0]]"))
            .expect("compiles");
        engine
            .restrict_commands(&["set", "expr", "if", "return", "string", "lindex", "llength"])
            .expect("restricts");
        engine
            .invoke(
                &handle,
                &[
                    Value::list([Value::string("abcde")]),
                    Value::dict_of::<&str>([]),
                ],
            )
            .expect("the whitelisted body still runs");

        let opener = engine
            .compile(unit("open /etc/passwd"))
            .expect("a body naming a forbidden command still compiles");
        let error = engine
            .invoke(&opener, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("but cannot run it");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message.contains("invalid command name")),
            "{error:?}"
        );
    }

    /// The compiler lowers `string trim` — and every other subcommand of an
    /// ensemble the VM knows — to a direct call of `::tcl::string::trim`, so a
    /// whitelist that names `string` has to keep those, or the same body runs when
    /// the call is the argument of a host command and fails when it is anywhere
    /// else. An ensemble the whitelist does not name keeps none of its own.
    #[test]
    fn a_whitelisted_ensembles_subcommands_run_wherever_they_are_called() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let mut engine = TclVmEngine::new();
        engine
            .restrict_commands(&["set", "return", "string", "dict"])
            .expect("restricts");
        for (body, expected) in [
            ("set r [string cat a - b]\nreturn $r", "a-b"),
            ("return [string toupper [string trim { x }]]", "X"),
            ("set d [dict create k v]\nreturn [dict get $d k]", "v"),
            ("return [string map {a b} [string cat a c]]", "bc"),
        ] {
            let handle = engine.compile(unit(body)).expect("compiles");
            let answer = engine.invoke(&handle, &arguments);
            assert_eq!(
                answer.as_ref().map(|value| value.as_str()),
                Ok(Some(expected)),
                "{body}"
            );
        }

        // Naming `dict` does not name `string`'s subcommands, spelt out or not.
        let mut dicts_only = TclVmEngine::new();
        dicts_only
            .restrict_commands(&["set", "return", "dict"])
            .expect("restricts");
        for body in [
            "return [::tcl::string::trim { x }]",
            "return [string trim { x }]",
        ] {
            let handle = dicts_only.compile(unit(body)).expect("compiles");
            assert!(dicts_only.invoke(&handle, &arguments).is_err(), "{body}");
        }
    }

    #[test]
    fn return_is_an_ordinary_early_exit() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit("if {[llength $words] == 0} { return }\nreturn folded"))
            .expect("compiles");
        let abstained = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("an early return is not an error");
        assert_eq!(abstained.as_str(), Some(""));
    }

    /// `set_release` pins the VM's release: a leading zero is octal up to
    /// 8.6 and decimal from 9.0 (`expr {010 + 0}` is 8 on tclsh 8.4 to 8.6
    /// and 10 on 9.0 and 9.1), and every operation leaves the thread's own
    /// numeral grammar as it found it.
    #[test]
    fn set_release_pins_the_numeral_grammar() {
        for (profile, want) in [("tcl8.6", "8"), ("tcl9.0", "10"), ("f5-irules", "8")] {
            let mut engine = TclVmEngine::new();
            let collector = std::rc::Rc::new(Collector {
                emitted: RefCell::new(Vec::new()),
            });
            engine
                .define_command("fold", collector.clone())
                .expect("registers");
            engine
                .set_release(profile)
                .expect("a catalogue profile pins");
            assert_eq!(engine.release(), Some(profile));
            let before = tcl_syntax::number::runtime_syntax();
            let handle = engine
                .compile(unit("fold [expr {010 + 0}]"))
                .expect("compiles");
            engine
                .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
                .expect("runs");
            assert_eq!(
                *collector.emitted.borrow(),
                vec![vec![want.to_owned()]],
                "{profile}"
            );
            assert_eq!(
                tcl_syntax::number::runtime_syntax(),
                before,
                "{profile}: the thread keeps its own grammar"
            );
            assert_eq!(
                engine.set_release(profile),
                Ok(()),
                "{profile}: pinning the same release again is a no-op"
            );
            let other = if profile == "tcl9.0" {
                "tcl8.6"
            } else {
                "tcl9.0"
            };
            assert_eq!(
                engine.set_release(other),
                Err(EngineError::Unsupported(
                    "pinning a release after a unit was compiled"
                )),
                "{profile}: a compiled engine keeps its release"
            );
        }
        for name in ["no-such-dialect", "", "tcl", "tk", "jim"] {
            let mut engine = TclVmEngine::new();
            assert_eq!(
                engine.set_release(name),
                Err(EngineError::Unsupported("pinning a release")),
                "{name:?} names no release the VM can pin"
            );
            assert_eq!(engine.release(), None);
        }
    }

    /// `confine_stores` keeps every write in the invocation's own frame. Each
    /// body below writes somewhere else — a qualified global, an element, a
    /// loop, destructuring or capture target, a `dict` update, a namespace
    /// variable, a linked local, the caller's frame — and raises `can't set
    /// …: stores are confined to the activation` without writing. A caught
    /// error publishes neither `::errorInfo` nor `::errorCode`, since a later
    /// invocation would read them. The same writes to locals succeed, and
    /// reading a global still reads.
    #[test]
    fn confine_stores_refuses_every_store_outside_the_activation() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        for (body, written) in [
            ("set ::g 1", "::g"),
            ("incr ::counter", "::counter"),
            ("lappend ::l x", "::l"),
            ("set ::a(k) 1", "::a"),
            ("foreach ::x {1 2} {}", "::x"),
            ("lassign {1 2} ::p q", "::p"),
            ("regexp {(a)} a ::m", "::m"),
            ("regsub a abc b ::rs", "::rs"),
            ("scan 5 %d ::n", "::n"),
            ("dict set ::d k v", "::d"),
            ("dict unset ::du k", "::du"),
            ("dict lappend ::dl k v", "::dl"),
            ("dict incr ::di k", "::di"),
            ("dict append ::da k v", "::da"),
            ("binary scan A a ::b", "::b"),
            ("namespace eval ::ns {variable v 1}", "::ns::v"),
            ("global g2; set g2 1", "::g2"),
            ("upvar 0 ::g3 alias; set alias 1", "::g3"),
            ("uplevel 1 {set up 1}", "::up"),
        ] {
            let mut engine = TclVmEngine::new();
            engine.confine_stores().expect("the VM confines its stores");
            let handle = engine.compile(unit(body)).expect("compiles");
            let answer = engine.invoke(&handle, &arguments);
            assert!(
                matches!(
                    &answer,
                    Err(EngineError::Script { message, .. })
                        if message.contains("stores are confined to the activation")
                ),
                "{body}: {answer:?}"
            );
            let probe = engine
                .compile(unit(&format!("return [info exists {written}]")))
                .expect("compiles");
            assert_eq!(
                engine.invoke(&probe, &arguments).expect("reads").as_str(),
                Some("0"),
                "{body}: nothing was written"
            );
        }
        let mut engine = TclVmEngine::new();
        engine.confine_stores().expect("the VM confines its stores");
        let published = "return [list [info exists ::errorInfo] [info exists ::errorCode]]";
        let caught = format!("catch {{lindex {{}} y}}; {published}");
        for body in [caught.as_str(), published] {
            let handle = engine.compile(unit(body)).expect("compiles");
            assert_eq!(
                engine.invoke(&handle, &arguments).expect("runs").as_str(),
                Some("0 0"),
                "{body}"
            );
        }
        let mut engine = TclVmEngine::new();
        engine
            .vm_mut()
            .set_var("::seen", tcl_vm::Value::string("yes"))
            .expect("seeds");
        engine.confine_stores().expect("the VM confines its stores");
        let handle = engine
            .compile(unit(
                "set acc {}; foreach x {a b} {lappend acc $x}; incr n; dict set d k v\n\
                 set arr(k) 1; lassign {1 2} p q; regexp {(a)} a whole m\n\
                 regsub a abc b rs; dict lappend dl k v; dict incr di k\n\
                 return [list $acc $n $d $arr(k) $p $m $rs $dl $di $::seen]",
            ))
            .expect("compiles");
        assert_eq!(
            engine.invoke(&handle, &arguments).expect("runs").as_str(),
            Some("{a b} 1 {k v} 1 1 a bbc {k v} {k 1} yes")
        );
        // The `rand()` generator's seed is interpreter state every
        // invocation shares: `srand` writes it and `rand` reads what an
        // earlier call left. Both raise while stores are confined, under the
        // VM's default release and under 8.4, whose math functions are
        // `expr` builtins no command restriction removes.
        for release in [None, Some("tcl8.4"), Some("f5-irules")] {
            for body in ["expr {srand(7)}", "expr {rand()}"] {
                let mut engine = TclVmEngine::new();
                if let Some(release) = release {
                    engine.set_release(release).expect("pins");
                }
                engine.confine_stores().expect("the VM confines its stores");
                let handle = engine.compile(unit(body)).expect("compiles");
                let answer = engine.invoke(&handle, &arguments);
                assert!(
                    matches!(
                        &answer,
                        Err(EngineError::Script { message, .. })
                            if message.contains("stores are confined to the activation")
                    ),
                    "{release:?} {body}: {answer:?}"
                );
            }
        }
    }

    /// `confine_stores` refuses an array's creation and an unset outside the
    /// activation as it refuses a store: `array set ::fresh {}` makes no
    /// global array, and no form of removal — a whole variable or an
    /// element, `-nocomplain`, `array unset` with or without a pattern, a
    /// linked local, a `dict update` over a missing key — takes away a
    /// global seeded before the confinement. The same forms on locals run.
    #[test]
    fn confine_stores_refuses_creation_and_unset_outside_the_activation() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let run = |engine: &mut TclVmEngine, body: &str| {
            let handle = engine.compile(unit(body)).expect("compiles");
            engine
                .invoke(&handle, &arguments)
                .map(|value| value.as_str().expect("a string").to_owned())
        };
        let left =
            "return [list [info exists ::fresh] [info exists ::seed] [info exists ::arr(k)]]";
        for body in [
            "array set ::fresh {}",
            "unset ::seed",
            "unset -nocomplain ::seed",
            "unset ::arr(k)",
            "array unset ::arr",
            "array unset ::arr k*",
            "global seed; unset seed",
            "global arr; unset arr(k)",
            "upvar #0 seed alias; unset alias",
            "set d {}; dict update d k ::seed {}",
        ] {
            let mut engine = TclVmEngine::new();
            run(&mut engine, "set ::seed old; set ::arr(k) v").expect("an open VM writes globals");
            engine.confine_stores().expect("the VM confines its stores");
            let answer = run(&mut engine, body);
            assert!(
                matches!(
                    &answer,
                    Err(EngineError::Script { message, .. })
                        if message.contains("stores are confined to the activation")
                ),
                "{body}: {answer:?}"
            );
            assert_eq!(run(&mut engine, left), Ok("0 1 1".to_owned()), "{body}");
        }
        let mut engine = TclVmEngine::new();
        engine.confine_stores().expect("the VM confines its stores");
        assert_eq!(
            run(
                &mut engine,
                "array set fresh {}; unset fresh; set x 1; unset x; set y 1; unset -nocomplain y z\n\
                 array set a {k 1 j 2}; unset a(k); array unset a j*; array unset a\n\
                 set d {}; dict update d k v {}\n\
                 return [list [info exists fresh] [info exists x] [info exists y] [info exists a]]",
            ),
            Ok("0 0 0 0".to_owned())
        );
    }

    /// An engine restricted to a whitelist that allows `expr` keeps its
    /// math functions, which from 8.5 are commands (`tcl::mathfunc::abs`),
    /// so a body answers `expr {abs(-1)}` under every pinned release rather
    /// than only under 8.4, where they are builtins; `rand` and `srand` go
    /// with every other command the whitelist does not name.
    #[test]
    fn a_restricted_engine_keeps_the_math_functions_but_the_generator() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        for release in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
        ] {
            let mut engine = TclVmEngine::new();
            engine.set_release(release).expect("pins");
            engine
                .restrict_commands(&["expr", "return"])
                .expect("restricts");
            engine.confine_stores().expect("confines");
            let handle = engine
                .compile(unit("return [expr {abs(-1) + int(2.5) + double(1)}]"))
                .expect("compiles");
            assert_eq!(
                engine.invoke(&handle, &arguments).expect("folds").as_str(),
                Some("4.0"),
                "{release}"
            );
            for body in ["return [expr {rand()}]", "return [expr {srand(7)}]"] {
                let handle = engine.compile(unit(body)).expect("compiles");
                assert!(
                    engine.invoke(&handle, &arguments).is_err(),
                    "{release}: {body}"
                );
            }
        }
    }

    /// A confined engine reads no host environment (`value-evaluation.md`
    /// § *The rest of the route contract*): before `confine_stores` the
    /// host has seeded `::env`, `::tcl_platform` and `::tcl_library`, and
    /// after it none exists, so reading one raises as a read of an unset
    /// variable does and a body cannot fold the analysing machine's user,
    /// platform or paths into an answer about a program that runs
    /// elsewhere.
    #[test]
    fn a_confined_engine_reads_no_host_environment() {
        let arguments = [Value::list([]), Value::dict_of::<&str>([])];
        let seeded = "return [list [info exists ::env] [info exists ::tcl_platform] \
                      [info exists ::tcl_library]]";
        let mut open = TclVmEngine::new();
        let handle = open.compile(unit(seeded)).expect("compiles");
        assert_eq!(
            open.invoke(&handle, &arguments).expect("reads").as_str(),
            Some("1 1 1"),
            "the host seeds its globals"
        );
        let mut confined = TclVmEngine::new();
        confined
            .confine_stores()
            .expect("the VM confines its stores");
        let handle = confined.compile(unit(seeded)).expect("compiles");
        assert_eq!(
            confined
                .invoke(&handle, &arguments)
                .expect("reads")
                .as_str(),
            Some("0 0 0"),
            "confining removes them"
        );
        for body in [
            "return $::tcl_platform(byteOrder)",
            "return $::tcl_library",
            "return $::env(PATH)",
        ] {
            let handle = confined.compile(unit(body)).expect("compiles");
            let answer = confined.invoke(&handle, &arguments);
            assert!(
                matches!(&answer, Err(EngineError::Script { .. })),
                "{body}: {answer:?}"
            );
        }
    }

    #[test]
    fn each_invocation_gets_its_own_locals() {
        let mut engine = TclVmEngine::new();
        let handle = engine
            .compile(unit(
                "if {[info exists leak]} { return leaked }\nset leak 1\nreturn fresh",
            ))
            .expect("compiles");
        for _ in 0..3 {
            let result = engine
                .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
                .expect("runs");
            assert_eq!(result.as_str(), Some("fresh"));
        }
    }
}
