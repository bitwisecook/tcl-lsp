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
//! interface was built against: `docs/design/spec-packs.md` names `tcl-vm` the
//! canonical engine everywhere (server, CLI, and studio, which ships it to
//! wasm already).
//!
//! Everything the interface asks for maps onto an embedder API the VM now has
//! (issue #1373), rather than onto a shim around a gap:
//!
//! | interface | VM |
//! |---|---|
//! | [`Engine::compile`] | [`Vm::define_procedure`] — one compile, then bytecode |
//! | [`Engine::invoke`] | [`Vm::invoke_command`] — the public call path |
//! | [`Engine::define_command`] | [`Vm::register_native_command`] — stateful host commands |
//! | [`Engine::restrict_commands`] | [`Vm::retain_commands`] — a closed whitelist |
//! | [`Budget::commands`] | [`Vm::set_command_limit`] — enforced, not merely stored |
//! | [`Budget::wall_clock`] | [`Vm::set_wall_clock_budget`] |
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

use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::{
    first_fatal_parse_error_with_config, lower_to_ir_for_bytecode,
    lower_to_ir_for_bytecode_with_dialect, lower_to_ir_traced_with_dialect,
};
use tcl_dialect::DialectProfile;
use tcl_engine_api::{Budget, BudgetKind, CompileUnit, Engine, EngineError, HostCommand, Value};
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, CompileError, CompileService, Completion, NativeCommand, Vm};

/// The compile service the VM injects for runtime `eval` and body
/// compilation — `tcl-vm` itself never links `tcl-compiler`, so the engine
/// supplies it.
struct VmCompiler {
    registry: CommandRegistry,
}

impl CompileService for VmCompiler {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, source: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(message) = tcl_compiler::lowering::first_fatal_parse_error(source) {
            return Err(CompileError(message));
        }
        let ir = lower_to_ir_for_bytecode(source, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }

    fn compile_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::registry_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        if let Some(message) = first_fatal_parse_error_with_config(source, config) {
            return Err(CompileError(message));
        }
        let ir = lower_to_ir_for_bytecode_with_dialect(source, registry, config, profile.name);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }

    fn compile_traced_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::registry_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        if let Some(message) = first_fatal_parse_error_with_config(source, config) {
            return Err(CompileError(message));
        }
        let ir = lower_to_ir_traced_with_dialect(source, registry, config, profile.name);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }
}

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

/// Adapts a [`HostCommand`] to the VM's [`NativeCommand`], converting values
/// at the boundary in both directions.
struct HostCommandShim {
    command: Rc<dyn HostCommand>,
}

impl NativeCommand for HostCommandShim {
    fn invoke(&self, _vm: &mut Vm, arguments: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let arguments: Vec<Value> = arguments.iter().map(from_vm_value).collect();
        match self.command.invoke(&arguments) {
            Ok(result) => Completion::new(
                Code::Ok,
                to_vm_value(&result),
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

/// The `tcl-vm` engine.
pub struct TclVmEngine {
    vm: Vm,
    budget: Budget,
    /// Mints the internal procedure name each compiled unit is defined as.
    units: u32,
    /// Command names registered through [`Engine::define_command`], kept so a
    /// later [`Engine::restrict_commands`] does not remove them.
    host_commands: Vec<String>,
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
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(VmCompiler { registry }));
        Self {
            vm,
            budget: Budget::default(),
            units: 0,
            host_commands: Vec::new(),
            unit_commands: Vec::new(),
        }
    }

    /// The underlying VM, for an embedder that needs a VM-specific facility
    /// the interface does not expose. Using it makes that code engine-specific
    /// by construction, which is the point of it being a separate accessor.
    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
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
        match message.as_str() {
            "command count limit exceeded" => {
                Err(EngineError::BudgetExceeded(BudgetKind::Commands))
            }
            "time limit exceeded" => Err(EngineError::BudgetExceeded(BudgetKind::WallClock)),
            "value size limit exceeded" => Err(EngineError::BudgetExceeded(BudgetKind::ValueSize)),
            _ => Err(EngineError::Script {
                message,
                code: None,
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
        self.vm
            .register_native_command(name, Rc::new(HostCommandShim { command }));
        self.host_commands.push(name.to_owned());
        Ok(())
    }

    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        let host_commands = self.host_commands.clone();
        let unit_commands = self.unit_commands.clone();
        self.vm.retain_commands(&|name| {
            allowed.contains(&name)
                || host_commands.iter().any(|command| command == name)
                || unit_commands.iter().any(|command| command == name)
        });
        Ok(())
    }

    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        self.units += 1;
        // A name no Tcl source can spell, so a body cannot call (or shadow)
        // another unit even if the sandbox ever gained a way to try.
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
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use super::{Engine, TclVmEngine};
    use tcl_engine_api::{Budget, BudgetKind, CompileUnit, EngineError, HostCommand, Value};

    struct Collector {
        emitted: RefCell<Vec<Vec<String>>>,
    }

    impl HostCommand for Collector {
        fn invoke(&self, arguments: &[Value]) -> Result<Value, EngineError> {
            self.emitted.borrow_mut().push(
                arguments
                    .iter()
                    .map(|argument| argument.as_str().unwrap_or_default().to_owned())
                    .collect(),
            );
            Ok(Value::Empty)
        }
    }

    fn unit(body: &str) -> CompileUnit<'_> {
        CompileUnit {
            name: "test",
            parameters: &["words", "ctx"],
            body,
        }
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
