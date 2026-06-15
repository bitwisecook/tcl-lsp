//! The interpreter state (`Vm`) and the variable/command/eval surface.

use std::collections::HashMap;
use std::io::{self, Write};

use tcl_runtime_api::{Code, CompileService, Completion, FrameId, VarStore};
use tcl_syntax::expr::{eval, parse_expr};

use crate::command::{BuiltinFn, Command, register_builtins};
use crate::error::TclError;
use crate::expr::ExprEval;
use crate::value::Value;

/// Build an `OK` completion (empty options dict).
pub(crate) fn ok(result: Value) -> Completion<Value> {
    Completion::new(Code::Ok, result, Value::empty())
}

/// Build an `ERROR` completion from a message (empty options dict for M1).
pub(crate) fn err(message: impl Into<String>) -> Completion<Value> {
    let m: String = message.into();
    Completion::new(Code::Error, Value::string(m), Value::empty())
}

/// The bytecode VM's interpreter state.
///
/// M1 holds a flat global scalar table (the degenerate [`VarStore`]), a command
/// table, an output sink, and an optional [`CompileService`] for runtime `eval`.
/// Frames/namespaces/traces (the rest of Family B) land in M2.
pub struct Vm {
    globals: HashMap<String, Value>,
    commands: HashMap<String, Command>,
    out: Box<dyn Write>,
    compiler: Option<Box<dyn CompileService>>,
}

impl Vm {
    /// A VM writing `puts` output to stdout.
    #[must_use]
    pub fn new() -> Self {
        Self::with_output(Box::new(io::stdout()))
    }

    /// A VM writing `puts` output to `out` (tests pass a capture buffer).
    #[must_use]
    pub fn with_output(out: Box<dyn Write>) -> Self {
        let mut vm = Self {
            globals: HashMap::new(),
            commands: HashMap::new(),
            out,
            compiler: None,
        };
        register_builtins(&mut vm);
        vm
    }

    /// Inject the compiler used for runtime `eval` / command substitution.
    pub fn set_compiler(&mut self, compiler: Box<dyn CompileService>) {
        self.compiler = Some(compiler);
    }

    pub(crate) fn register(&mut self, name: &str, f: BuiltinFn) {
        self.commands.insert(name.to_owned(), Command::Builtin(f));
    }

    /// Read a global scalar.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.globals.get(name).cloned()
    }

    /// Write a global scalar.
    pub fn set_var(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_owned(), value);
    }

    /// Remove a global scalar; returns whether it existed.
    pub fn unset_var(&mut self, name: &str) -> bool {
        self.globals.remove(name).is_some()
    }

    pub(crate) fn write_output(&mut self, s: &str, newline: bool) {
        let _ = self.out.write_all(s.as_bytes());
        if newline {
            let _ = self.out.write_all(b"\n");
        }
        let _ = self.out.flush();
    }

    /// Dispatch a command by name with its argv (excluding the command name).
    pub fn invoke(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        match self.commands.get(name).copied() {
            Some(Command::Builtin(f)) => f(self, argv),
            None => err(format!("invalid command name \"{name}\"")),
        }
    }

    /// Parse and evaluate a Tcl expression string against this VM.
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, TclError> {
        let node = parse_expr(src, None);
        let mut ops = ExprEval { vm: self };
        eval(&node, &mut ops)
    }

    /// Compile and run a Tcl source string via the injected [`CompileService`]
    /// (the runtime-`eval` / command-substitution path). Errors cleanly when no
    /// compiler is wired.
    pub fn eval_source(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        let module = match self.compiler.as_ref() {
            Some(c) => c.compile(src).map_err(|e| TclError::new(e.0))?,
            None => {
                return Err(TclError::new(
                    "eval / command substitution requires a CompileService",
                ));
            }
        };
        Ok(self.run_module(&module))
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

/// The degenerate M1 [`VarStore`]: a flat global table (the frame is ignored
/// until M2 adds the frame stack). Exercises the Family-B contract.
impl VarStore for Vm {
    type Value = Value;

    fn get(&self, _frame: FrameId, name: &str) -> Option<Value> {
        self.get_var(name)
    }

    fn set(&mut self, _frame: FrameId, name: &str, value: Value) {
        self.set_var(name, value);
    }

    fn unset(&mut self, _frame: FrameId, name: &str) -> bool {
        self.unset_var(name)
    }

    fn exists(&self, _frame: FrameId, name: &str) -> bool {
        self.globals.contains_key(name)
    }
}
