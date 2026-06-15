//! The interpreter state (`Vm`): the call-frame stack, the command table, the
//! compiled-proc registry, and the variable/command/eval surface.

use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, CompileService, Completion, FrameId, ROOT_NS, VarStore};
use tcl_syntax::expr::{eval, parse_expr};

use crate::command::{BuiltinFn, Command, ProcDef, register_builtins};
use crate::error::TclError;
use crate::expr::ExprEval;
use crate::frame::{CallFrame, Local};
use crate::value::Value;

/// Build an `OK` completion (empty options dict).
pub(crate) fn ok(result: Value) -> Completion<Value> {
    Completion::new(Code::Ok, result, Value::empty())
}

/// Build an `ERROR` completion from a message (empty options dict).
pub(crate) fn err(message: impl Into<String>) -> Completion<Value> {
    let m: String = message.into();
    Completion::new(Code::Error, Value::string(m), Value::empty())
}

/// The bytecode VM's interpreter state.
pub struct Vm {
    /// Call-frame stack; `frames[0]` is the global scope.
    frames: Vec<CallFrame>,
    /// Command table (builtins + user procs), keyed by simple name.
    commands: HashMap<String, Command>,
    /// Pre-compiled proc bodies from the module(s), keyed by qualified name.
    module_procs: HashMap<String, Rc<FunctionAsm>>,
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
            frames: vec![CallFrame::new(0, ROOT_NS, None, Vec::new())],
            commands: HashMap::new(),
            module_procs: HashMap::new(),
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

    pub(crate) fn register_command(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_owned(), cmd);
    }

    pub(crate) fn lookup_command(&self, name: &str) -> Option<Command> {
        self.commands
            .get(name)
            .or_else(|| self.commands.get(name.strip_prefix("::").unwrap_or(name)))
            .cloned()
    }

    pub(crate) fn module_proc(&self, qname: &str) -> Option<Rc<FunctionAsm>> {
        self.module_procs.get(qname).cloned()
    }

    /// Merge a module's pre-compiled proc bodies into the registry.
    pub(crate) fn merge_procs(&mut self, procs: &HashMap<String, FunctionAsm>) {
        for (qname, asm) in procs {
            self.module_procs
                .entry(qname.clone())
                .or_insert_with(|| Rc::new(asm.clone()));
        }
    }

    // -- frames --

    pub(crate) fn current_level(&self) -> usize {
        self.frames.len() - 1
    }

    pub(crate) fn push_call_frame(
        &mut self,
        proc_name: Option<String>,
        call_argv: Vec<Value>,
    ) -> usize {
        let level = self.frames.len();
        self.frames
            .push(CallFrame::new(level, ROOT_NS, proc_name, call_argv));
        level
    }

    pub(crate) fn pop_call_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    /// Set a local directly in the current frame (proc argument binding).
    pub(crate) fn set_local(&mut self, name: &str, value: Value) {
        if let Some(f) = self.frames.last_mut() {
            f.locals.insert(name.to_owned(), Local::Scalar(value));
        }
    }

    /// Install a cross-frame link in the current frame (`upvar`/`global`).
    pub(crate) fn add_link(&mut self, local: &str, level: usize, target: &str) {
        if let Some(f) = self.frames.last_mut() {
            f.locals.insert(
                local.to_owned(),
                Local::Link {
                    level,
                    name: target.to_owned(),
                },
            );
        }
    }

    /// Resolve `name` to the (frame level, simple name) that actually owns it,
    /// following `upvar`/`global` links. `::`-qualified names resolve at global.
    fn locate(&self, name: &str) -> (usize, String) {
        if let Some(stripped) = name.strip_prefix("::") {
            return (0, stripped.to_owned());
        }
        let mut level = self.frames.len() - 1;
        let mut nm = name.to_owned();
        for _ in 0..64 {
            match self.frames[level].locals.get(&nm) {
                Some(Local::Link {
                    level: tl,
                    name: tn,
                }) => {
                    level = *tl;
                    nm = tn.clone();
                }
                _ => break,
            }
        }
        (level, nm)
    }

    /// Read a scalar (following links).
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<Value> {
        let (lvl, nm) = self.locate(name);
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Scalar(v)) => Some(v.clone()),
            _ => None,
        }
    }

    /// Write a scalar (following links to the owning frame).
    pub fn set_var(&mut self, name: &str, value: Value) {
        let (lvl, nm) = self.locate(name);
        if let Some(f) = self.frames.get_mut(lvl) {
            f.locals.insert(nm, Local::Scalar(value));
        }
    }

    /// Remove a scalar; returns whether it existed.
    pub fn unset_var(&mut self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        self.frames
            .get_mut(lvl)
            .is_some_and(|f| f.locals.remove(&nm).is_some())
    }

    fn var_exists(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        self.frames
            .get(lvl)
            .is_some_and(|f| matches!(f.locals.get(&nm), Some(Local::Scalar(_))))
    }

    /// Publish `errorInfo` / `errorCode` into the global frame.
    pub(crate) fn publish_error(&mut self, info: &str, code: &Value) {
        if let Some(g) = self.frames.first_mut() {
            g.locals
                .insert("errorInfo".to_owned(), Local::Scalar(Value::string(info)));
            g.locals
                .insert("errorCode".to_owned(), Local::Scalar(code.clone()));
        }
    }

    pub(crate) fn write_output(&mut self, s: &str, newline: bool) {
        let _ = self.out.write_all(s.as_bytes());
        if newline {
            let _ = self.out.write_all(b"\n");
        }
        let _ = self.out.flush();
    }

    /// Register a user procedure.
    pub(crate) fn define_proc(&mut self, proc: ProcDef) {
        let simple = proc
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&proc.name)
            .to_owned();
        let cmd = Command::Proc(Rc::new(proc));
        self.register_command(&simple, cmd);
    }

    /// Parse and evaluate a Tcl expression string against this VM.
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, TclError> {
        let node = parse_expr(src, None);
        let mut ops = ExprEval { vm: self };
        eval(&node, &mut ops)
    }

    /// Compile and run a Tcl source string via the injected [`CompileService`]
    /// (the runtime-`eval` / command-substitution path) in the *current* frame.
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

/// The Family-B variable store over the call-frame stack. `FrameId` is the
/// absolute level; `get`/`set` follow links exactly like the by-name accessors.
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
        self.var_exists(name)
    }
}
