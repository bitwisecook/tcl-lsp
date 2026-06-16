//! The interpreter state (`Vm`): the call-frame stack, the command table, the
//! compiled-proc registry, and the variable/command/eval surface.

use std::collections::{BTreeMap, HashMap};
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

/// Quote a trace-callback argument as a single Tcl word: empty or
/// whitespace-bearing values are brace-wrapped, simple words are passed bare.
fn tcl_brace(s: &str) -> String {
    if s.is_empty() || s.contains(char::is_whitespace) || s.contains(['[', ']', '$', '{', '}']) {
        format!("{{{s}}}")
    } else {
        s.to_string()
    }
}

/// Split an `arr(key)` variable reference into `(base, key)`, or `None` for a
/// plain scalar/array name. The key may be empty; the base must not be.
fn elem_ref(name: &str) -> Option<(&str, &str)> {
    let open = name.find('(')?;
    if open > 0 && name.ends_with(')') {
        Some((&name[..open], &name[open + 1..name.len() - 1]))
    } else {
        None
    }
}

/// The bytecode VM's interpreter state.
pub struct Vm {
    /// Call-frame stack; `frames[0]` is the global scope.
    frames: Vec<CallFrame>,
    /// Command table (builtins + user procs), keyed by canonical name — a
    /// builtin's simple name, or a proc's namespace-qualified name without the
    /// leading `::` (e.g. `foo::bar`; a global proc is just `bar`).
    commands: HashMap<String, Command>,
    /// Pre-compiled proc bodies from the module(s), keyed by qualified name.
    module_procs: HashMap<String, Rc<FunctionAsm>>,
    /// Current-namespace stack (canonical, no leading `::`; `""` = global). The
    /// top governs `proc`/command/variable name resolution. `namespace eval`
    /// and proc activation push/pop it.
    ns_stack: Vec<String>,
    /// Existing namespaces (canonical names; `""` global is implicit).
    namespaces: std::collections::HashSet<String>,
    /// Export patterns per namespace (canonical name → glob patterns), set by
    /// `namespace export` and consulted by `namespace import`.
    ns_exports: HashMap<String, Vec<String>>,
    /// Provided packages → version (`package provide`/`require`).
    packages: HashMap<String, String>,
    /// Variable traces, keyed by a resolved-owner key (frame level + name) so a
    /// trace fires regardless of the access path (`upvar` alias, qualified
    /// name, …). Newest trace last; fired newest-first.
    var_traces: HashMap<String, Vec<VarTrace>>,
    /// Re-entrancy guard: `"<key>\0<op>"` entries for traces currently firing.
    active_traces: std::collections::HashSet<String>,
    /// Nesting depth of `namespace eval`/`inscope` bodies currently executing.
    /// While `> 0`, `upvar`/aliasing treats unqualified names as the current
    /// namespace's variables (stored in the global frame) rather than locals.
    ns_script_depth: u32,
    out: Box<dyn Write>,
    compiler: Option<Box<dyn CompileService>>,
}

/// A single registered variable trace.
#[derive(Clone)]
struct VarTrace {
    /// Operations this trace fires on (`read`/`write`/`unset`/`array`).
    ops: Vec<String>,
    /// The command prefix invoked as `command name1 name2 op`.
    command: String,
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
            ns_stack: vec![String::new()],
            namespaces: std::collections::HashSet::new(),
            ns_exports: HashMap::new(),
            packages: HashMap::new(),
            var_traces: HashMap::new(),
            active_traces: std::collections::HashSet::new(),
            ns_script_depth: 0,
            out,
            compiler: None,
        };
        register_builtins(&mut vm);
        vm.bootstrap_globals();
        vm
    }

    /// Populate the predefined global variables a fresh interpreter exposes:
    /// the `tcl_platform`/`env` arrays and the `argv`/`argv0`/`argc` scalars,
    /// so library scripts (tcltest) that read them at load time work.
    fn bootstrap_globals(&mut self) {
        let plat = [
            ("platform", "unix"),
            ("os", "Linux"),
            ("osVersion", ""),
            ("machine", std::env::consts::ARCH),
            ("byteOrder", "littleEndian"),
            ("wordSize", "8"),
            ("pointerSize", "8"),
            ("pathSeparator", ":"),
            ("engine", "Tcl"),
            ("threaded", "1"),
            ("user", ""),
        ];
        for (k, v) in plat {
            let _ = self.write_array_raw("tcl_platform", k, Value::string(v));
        }
        for (k, v) in std::env::vars() {
            let _ = self.write_array_raw("env", &k, Value::string(v));
        }
        self.write_scalar_raw("argv", Value::list(Vec::new()));
        self.write_scalar_raw("argv0", Value::string("tcltest"));
        self.write_scalar_raw("argc", Value::int(0));
        self.write_scalar_raw("tcl_version", Value::string("9.0"));
        self.write_scalar_raw("tcl_patchLevel", Value::string("9.0.0"));
    }

    /// Inject the compiler used for runtime `eval` / command substitution.
    pub fn set_compiler(&mut self, compiler: Box<dyn CompileService>) {
        self.compiler = Some(compiler);
    }

    pub(crate) fn register(&mut self, name: &str, f: BuiltinFn) {
        self.register_command(name, Command::Builtin(f));
    }

    pub(crate) fn register_command(&mut self, name: &str, cmd: Command) {
        // The table is keyed by canonical names (no leading `::`).
        let key = name.strip_prefix("::").unwrap_or(name);
        self.commands.insert(key.to_owned(), cmd);
    }

    /// Resolve a command name to its definition, honouring the current
    /// namespace: an absolute `::a::b` name resolves exactly; an unqualified /
    /// relatively-qualified name is tried in the current namespace, then the
    /// global namespace (where builtins live).
    pub(crate) fn lookup_command(&self, name: &str) -> Option<Command> {
        if let Some(abs) = name.strip_prefix("::") {
            return self.commands.get(abs).cloned();
        }
        let cur = self.current_ns();
        if !cur.is_empty()
            && let Some(c) = self.commands.get(&format!("{cur}::{name}"))
        {
            return Some(c.clone());
        }
        self.commands.get(name).cloned()
    }

    /// The current namespace (canonical, no leading `::`; `""` = global).
    pub(crate) fn current_ns(&self) -> &str {
        self.ns_stack.last().map_or("", String::as_str)
    }

    /// Push a namespace onto the resolution stack (created if new).
    pub(crate) fn push_ns(&mut self, ns: String) {
        if !ns.is_empty() {
            self.namespaces.insert(ns.clone());
        }
        self.ns_stack.push(ns);
    }

    /// Pop the current namespace (the global base is never popped).
    pub(crate) fn pop_ns(&mut self) {
        if self.ns_stack.len() > 1 {
            self.ns_stack.pop();
        }
    }

    /// Canonicalise a name (no leading `::`) relative to the current namespace:
    /// an absolute `::a::b` drops the leading `::`; anything else is qualified
    /// with the current namespace.
    pub(crate) fn qualify_name(&self, name: &str) -> String {
        if let Some(abs) = name.strip_prefix("::") {
            return abs.to_string();
        }
        let cur = self.current_ns();
        if cur.is_empty() {
            name.to_string()
        } else {
            format!("{cur}::{name}")
        }
    }

    /// Register an existing namespace (and its ancestors).
    pub(crate) fn declare_namespace(&mut self, ns: &str) {
        if ns.is_empty() {
            return;
        }
        self.namespaces.insert(ns.to_string());
        if let Some((parent, _)) = ns.rsplit_once("::") {
            self.declare_namespace(parent);
        }
    }

    /// Whether a canonical namespace name exists.
    pub(crate) fn namespace_exists(&self, ns: &str) -> bool {
        ns.is_empty() || self.namespaces.contains(ns)
    }

    /// Record `namespace export` patterns for the current namespace.
    pub(crate) fn add_exports(&mut self, patterns: &[String]) {
        let ns = self.current_ns().to_string();
        self.ns_exports
            .entry(ns)
            .or_default()
            .extend_from_slice(patterns);
    }

    /// `namespace import` for `pattern` (e.g. `::tcltest::*`): alias every
    /// exported command of the source namespace matching the glob into the
    /// current namespace under its tail name. Returns the imported tail names.
    pub(crate) fn import_commands(&mut self, pattern: &str) -> Vec<String> {
        let abs = pattern.strip_prefix("::").unwrap_or(pattern);
        let (src_ns, glob) = match abs.rsplit_once("::") {
            Some((ns, g)) => (ns.to_string(), g.to_string()),
            None => (String::new(), abs.to_string()),
        };
        let exports = self.ns_exports.get(&src_ns).cloned().unwrap_or_default();
        let prefix = if src_ns.is_empty() {
            String::new()
        } else {
            format!("{src_ns}::")
        };
        // Candidate commands: those in the source namespace whose tail matches
        // the import glob and an export pattern.
        let mut to_import: Vec<(String, Command)> = Vec::new();
        for (cmd_name, cmd) in &self.commands {
            let Some(tail) = cmd_name.strip_prefix(&prefix) else {
                continue;
            };
            if tail.is_empty() || tail.contains("::") {
                continue;
            }
            if tcl_syntax::glob::string_match(&glob, tail)
                && exports
                    .iter()
                    .any(|p| tcl_syntax::glob::string_match(p, tail))
            {
                to_import.push((tail.to_string(), cmd.clone()));
            }
        }
        let mut imported = Vec::new();
        for (tail, cmd) in to_import {
            let alias = self.qualify_name(&tail);
            self.register_command(&alias, cmd);
            imported.push(tail);
        }
        imported
    }

    /// Immediate child namespaces of `parent` (canonical names).
    pub(crate) fn child_namespaces(&self, parent: &str) -> Vec<String> {
        let prefix = if parent.is_empty() {
            String::new()
        } else {
            format!("{parent}::")
        };
        self.namespaces
            .iter()
            .filter(|ns| {
                ns.strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.is_empty() && !rest.contains("::"))
            })
            .cloned()
            .collect()
    }

    /// Record a provided package version.
    pub(crate) fn provide_package(&mut self, name: &str, version: &str) {
        self.packages.insert(name.to_string(), version.to_string());
    }

    /// The provided version of a package, if any.
    pub(crate) fn package_version(&self, name: &str) -> Option<&str> {
        self.packages.get(name).map(String::as_str)
    }

    /// Names of all provided packages.
    pub(crate) fn package_names(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }

    // -- variable traces (`trace add|remove|info variable`) --

    /// The resolved-owner key a variable name's traces are stored under, so a
    /// trace fires regardless of access path (alias / qualified name). An array
    /// *element* reference (`arr(key)`) keys on the resolved element so element
    /// traces are distinct from each other and from whole-array traces.
    fn trace_key(&self, name: &str) -> String {
        if let Some((base, key)) = elem_ref(name) {
            let (lvl, nm) = self.locate(base);
            format!("{lvl}\u{0}{nm}({key})")
        } else {
            let (lvl, nm) = self.locate(name);
            format!("{lvl}\u{0}{nm}")
        }
    }

    /// Register a `trace add variable` callback.
    pub(crate) fn add_var_trace(&mut self, name: &str, ops: Vec<String>, command: String) {
        let key = self.trace_key(name);
        self.var_traces
            .entry(key)
            .or_default()
            .push(VarTrace { ops, command });
    }

    /// Remove a `trace remove variable` callback matching `ops` + `command`.
    pub(crate) fn remove_var_trace(&mut self, name: &str, ops: &[String], command: &str) {
        let key = self.trace_key(name);
        if let Some(list) = self.var_traces.get_mut(&key) {
            list.retain(|t| !(t.ops == ops && t.command == command));
            if list.is_empty() {
                self.var_traces.remove(&key);
            }
        }
    }

    /// The traces on `name` as `(ops, command)` pairs (newest first), for
    /// `trace info variable`.
    pub(crate) fn var_trace_info(&self, name: &str) -> Vec<(Vec<String>, String)> {
        let key = self.trace_key(name);
        self.var_traces.get(&key).map_or_else(Vec::new, |list| {
            list.iter()
                .rev()
                .map(|t| (t.ops.clone(), t.command.clone()))
                .collect()
        })
    }

    /// Fire the variable traces for `name` on operation `op`, running each
    /// callback as `command name1 name2 op`. A read/write callback error aborts
    /// the access (`can't read`/`can't set "name": …`); unset errors are
    /// ignored. Re-entrant firing of the same variable+op is suppressed.
    pub(crate) fn fire_var_traces(
        &mut self,
        name: &str,
        op: &str,
    ) -> Result<(), Completion<Value>> {
        if self.var_traces.is_empty() {
            return Ok(());
        }
        let (base, elem) = elem_ref(name).map_or_else(
            || (name.to_string(), String::new()),
            |(b, k)| (b.to_string(), k.to_string()),
        );
        // Fire the element-specific traces, then the whole-array traces (for an
        // element write a trace on the base array also fires).
        let mut keys = vec![self.trace_key(name)];
        if elem_ref(name).is_some() {
            let (lvl, nm) = self.locate(&base);
            keys.push(format!("{lvl}\u{0}{nm}"));
        }
        for key in keys {
            let guard = format!("{key}\u{0}{op}");
            if self.active_traces.contains(&guard) {
                continue;
            }
            let Some(traces) = self.var_traces.get(&key).cloned() else {
                continue;
            };
            for tr in traces.iter().rev() {
                if !tr.ops.iter().any(|o| o == op) {
                    continue;
                }
                let script = format!(
                    "{} {} {} {}",
                    tr.command,
                    tcl_brace(&base),
                    tcl_brace(&elem),
                    op
                );
                self.active_traces.insert(guard.clone());
                let r = self.eval_source(&script);
                self.active_traces.remove(&guard);
                let failed = match r {
                    Ok(c) if c.code.is_ok() => None,
                    Ok(c) => Some(c.result.to_str().to_string()),
                    Err(e) => Some(e.message),
                };
                if let Some(msg) = failed {
                    match op {
                        "write" => return Err(err(format!("can't set \"{name}\": {msg}"))),
                        "read" => return Err(err(format!("can't read \"{name}\": {msg}"))),
                        _ => {} // unset trace errors are ignored
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn module_proc(&self, qname: &str) -> Option<Rc<FunctionAsm>> {
        self.module_procs.get(qname).cloned()
    }

    /// Compile a proc body at runtime (for `proc` with a dynamically-built
    /// body that wasn't pre-compiled into a module). The body is compiled as a
    /// script; its parameters resolve through the call frame (`loadStk`), so a
    /// top-level compilation runs correctly as a proc body. Any procs the body
    /// itself defines are merged into the registry.
    pub(crate) fn compile_dynamic_body(&mut self, src: &str) -> Option<Rc<FunctionAsm>> {
        let module = self.compiler.as_ref()?.compile(src).ok()?;
        self.merge_procs(&module.procedures);
        Some(Rc::new(module.top_level))
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

    /// All registered command names (for `info commands`).
    pub(crate) fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// The `ProcDef` for a user proc, if `name` resolves to one (`info body`/`args`).
    pub(crate) fn proc_def(&self, name: &str) -> Option<Rc<crate::command::ProcDef>> {
        match self.lookup_command(name) {
            Some(Command::Proc(p)) => Some(p),
            _ => None,
        }
    }

    /// User-proc names (for `info procs`).
    pub(crate) fn proc_names(&self) -> Vec<String> {
        self.commands
            .iter()
            .filter(|(_, c)| matches!(c, Command::Proc(_)))
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// The invocation argv of the frame at absolute `level` (`info level N`).
    pub(crate) fn frame_argv(&self, level: usize) -> Option<Vec<Value>> {
        self.frames.get(level).map(|f| f.call_argv.clone())
    }

    /// Scalar variable names visible in the current frame (`info vars`/`locals`).
    pub(crate) fn local_scalar_names(&self) -> Vec<String> {
        self.frames.last().map_or_else(Vec::new, |f| {
            f.locals
                .iter()
                .filter(|(_, l)| matches!(l, Local::Scalar(_) | Local::Array(_)))
                .map(|(n, _)| n.clone())
                .collect()
        })
    }

    /// Global variable names (`info globals`).
    pub(crate) fn global_names(&self) -> Vec<String> {
        self.frames
            .first()
            .map_or_else(Vec::new, |f| f.locals.keys().cloned().collect())
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

    /// Install a link in the global frame keyed by `alias` (a canonical,
    /// namespace-qualified name). Used for namespace-level `upvar` aliases so
    /// they coincide with the `variable`-resolved namespace variable.
    pub(crate) fn add_global_link(&mut self, alias: &str, level: usize, target: &str) {
        if let Some(f) = self.frames.first_mut() {
            f.locals.insert(
                alias.to_owned(),
                Local::Link {
                    level,
                    name: target.to_owned(),
                },
            );
        }
    }

    /// Whether a `namespace eval`/`inscope` body is currently executing.
    pub(crate) fn in_ns_script(&self) -> bool {
        self.ns_script_depth > 0
    }

    /// Enter/leave a `namespace eval`/`inscope` body (around its evaluation).
    pub(crate) fn enter_ns_script(&mut self) {
        self.ns_script_depth += 1;
    }
    pub(crate) fn leave_ns_script(&mut self) {
        self.ns_script_depth = self.ns_script_depth.saturating_sub(1);
    }

    /// Resolve `name` to the (frame level, owning name) that actually owns it,
    /// following `upvar`/`global`/`variable` links. Any namespace-qualified name
    /// (containing `::`, including a plain `::global`) lives in the global frame
    /// keyed by its canonical name (leading `::` stripped) — this is where
    /// namespace variables (`tcltest::numTests`) are stored.
    fn locate(&self, name: &str) -> (usize, String) {
        self.locate_from(name, self.frames.len().saturating_sub(1))
    }

    /// Like [`Self::locate`] but begins link resolution at frame `start` (used
    /// to resolve an array base that an `upvar`/`variable` link landed on at a
    /// non-top frame level).
    fn locate_from(&self, name: &str, start: usize) -> (usize, String) {
        let stripped = name.strip_prefix("::").unwrap_or(name);
        if stripped.contains("::") || name.starts_with("::") {
            return (0, stripped.to_owned());
        }
        let mut level = start;
        let mut nm = name.to_owned();
        for _ in 0..64 {
            match self.frames.get(level).and_then(|f| f.locals.get(&nm)) {
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

    /// Read a scalar (following links). A link may resolve to an array element
    /// name (`upvar 0 arr(key) alias`), in which case the element is read.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<Value> {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        if let Some((base, key)) = elem_ref(&nm) {
            // The base may itself be a link (`variable`/`upvar` to a namespace
            // array), so resolve it onward from the frame it landed on.
            let (blvl, bnm) = self.locate_from(base, lvl);
            return match self.frames.get(blvl)?.locals.get(&bnm) {
                Some(Local::Array(m)) => m.get(key).cloned(),
                _ => None,
            };
        }
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Scalar(v)) => Some(v.clone()),
            _ => None,
        }
    }

    /// Write a scalar with no trace firing (frame argument binding, rollback).
    /// A link resolving to an array element name writes that element.
    fn write_scalar_raw(&mut self, name: &str, value: Value) {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        if let Some((base, key)) = elem_ref(&nm) {
            // Resolve the array base onward (it may be a link to a namespace
            // array) before writing the element.
            let key = key.to_owned();
            let (blvl, bnm) = self.locate_from(base, lvl);
            if let Some(f) = self.frames.get_mut(blvl) {
                match f.locals.get_mut(&bnm) {
                    Some(Local::Array(m)) => {
                        m.insert(key, value);
                    }
                    Some(_) => {}
                    None => {
                        let mut m = BTreeMap::new();
                        m.insert(key, value);
                        f.locals.insert(bnm, Local::Array(m));
                    }
                }
            }
            return;
        }
        if let Some(f) = self.frames.get_mut(lvl) {
            f.locals.insert(nm, Local::Scalar(value));
        }
    }

    /// Write a scalar, firing `write` traces afterwards (the old value is
    /// restored if a trace callback aborts the write).
    pub fn set_var(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        if self.var_traces.is_empty() {
            self.write_scalar_raw(name, value);
            return Ok(());
        }
        let old = self.get_var(name);
        self.write_scalar_raw(name, value);
        if let Err(e) = self.fire_var_traces(name, "write") {
            if let Some(o) = old {
                self.write_scalar_raw(name, o);
            } else {
                let (lvl, nm) = self.locate(name);
                if let Some(f) = self.frames.get_mut(lvl) {
                    f.locals.remove(&nm);
                }
            }
            return Err(e);
        }
        Ok(())
    }

    /// Remove a scalar; returns whether it existed.
    pub fn unset_var(&mut self, name: &str) -> bool {
        // Unset traces fire before removal; their errors are ignored.
        let _ = self.fire_var_traces(name, "unset");
        let (lvl, nm) = self.locate(name);
        let existed = self
            .frames
            .get_mut(lvl)
            .is_some_and(|f| f.locals.remove(&nm).is_some());
        // A variable's traces are dropped when it is unset.
        if existed {
            let key = self.trace_key(name);
            self.var_traces.remove(&key);
        }
        existed
    }

    fn var_exists(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        self.frames
            .get(lvl)
            .is_some_and(|f| matches!(f.locals.get(&nm), Some(Local::Scalar(_))))
    }

    /// Whether a scalar or array variable named `name` exists (`info exists`).
    pub(crate) fn has_var(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Scalar(_) | Local::Array(_))
        )
    }

    /// Whether `name` exists, resolving an `arr(key)` element reference to the
    /// element — the `info exists` / `existStk` semantic.
    pub(crate) fn exists_var(&self, name: &str) -> bool {
        if let Some((base, key)) = elem_ref(name) {
            return self.get_array_elem(base, key).is_some();
        }
        self.has_var(name)
    }

    /// Read `name`, resolving an `arr(key)` reference to the array element —
    /// the runtime-name analogue used by `set`/`incr`/`append`/`lappend`.
    pub(crate) fn var_get(&self, name: &str) -> Option<Value> {
        if let Some((base, key)) = elem_ref(name) {
            return self.get_array_elem(base, key);
        }
        self.get_var(name)
    }

    /// Write `name`, resolving an `arr(key)` reference to the array element.
    pub(crate) fn var_set(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        if let Some((base, key)) = elem_ref(name) {
            return self.set_array_elem(base, key, value);
        }
        self.set_var(name, value)
    }

    // -- arrays (link-aware via `locate`) --

    pub(crate) fn get_array_elem(&self, name: &str, key: &str) -> Option<Value> {
        let resolved = self.ns_var_fallback(name);
        let lookup = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(lookup);
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Array(m)) => m.get(key).cloned(),
            _ => None,
        }
    }

    /// When an unqualified `name` is not a local in the current frame but the
    /// current namespace has a variable `ns::name`, resolve to that qualified
    /// name. This is the namespace-variable fallback (a namespace script or an
    /// undeclared access reaching an existing namespace variable). Only
    /// resolves to variables that already exist, so frame locals are unaffected.
    fn ns_var_fallback(&self, name: &str) -> Option<String> {
        if name.contains("::") {
            return None;
        }
        let cur = self.current_ns();
        if cur.is_empty() {
            return None;
        }
        let top = self.frames.last()?;
        if top.locals.contains_key(name) {
            return None;
        }
        let q = format!("{cur}::{name}");
        if self
            .frames
            .first()
            .is_some_and(|g| g.locals.contains_key(&q))
        {
            Some(q)
        } else {
            None
        }
    }

    /// Write an array element with no trace firing.
    fn write_array_raw(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        let frame = self
            .frames
            .get_mut(lvl)
            .expect("locate returns a valid level");
        match frame.locals.get_mut(&nm) {
            Some(Local::Array(m)) => {
                m.insert(key.to_owned(), value);
                Ok(())
            }
            Some(Local::Scalar(_)) => Err(err(format!(
                "can't set \"{name}({key})\": variable isn't array"
            ))),
            Some(Local::Link { .. }) => unreachable!("locate resolves links"),
            None => {
                let mut m = BTreeMap::new();
                m.insert(key.to_owned(), value);
                frame.locals.insert(nm, Local::Array(m));
                Ok(())
            }
        }
    }

    pub(crate) fn set_array_elem(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if self.var_traces.is_empty() {
            return self.write_array_raw(name, key, value);
        }
        let old = self.get_array_elem(name, key);
        self.write_array_raw(name, key, value)?;
        let full = format!("{name}({key})");
        if let Err(e) = self.fire_var_traces(&full, "write") {
            match old {
                Some(o) => {
                    let _ = self.write_array_raw(name, key, o);
                }
                None => self.array_unset_elem(name, key),
            }
            return Err(e);
        }
        Ok(())
    }

    pub(crate) fn array_is(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Array(_))
        )
    }

    pub(crate) fn array_pairs(&self, name: &str) -> Vec<(String, Value)> {
        let (lvl, nm) = self.locate(name);
        match self.frames.get(lvl).and_then(|f| f.locals.get(&nm)) {
            Some(Local::Array(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn array_unset_elem(&mut self, name: &str, key: &str) {
        if !self.var_traces.is_empty() {
            let _ = self.fire_var_traces(&format!("{name}({key})"), "unset");
        }
        let (lvl, nm) = self.locate(name);
        if let Some(Local::Array(m)) = self.frames.get_mut(lvl).and_then(|f| f.locals.get_mut(&nm))
        {
            m.remove(key);
        }
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

    /// Register a user procedure under its canonical (namespace-qualified)
    /// name, and ensure its namespace exists.
    pub(crate) fn define_proc(&mut self, proc: ProcDef) {
        let key = proc.name.clone();
        if let Some((ns, _)) = key.rsplit_once("::") {
            self.declare_namespace(ns);
        }
        let cmd = Command::Proc(Rc::new(proc));
        self.register_command(&key, cmd);
    }

    /// Dispatch a *builtin* command by name (no proc activation). Returns `None`
    /// for a proc or unknown command. Used by `expr` math-function calls.
    pub(crate) fn dispatch_builtin(
        &mut self,
        name: &str,
        argv: &[Value],
    ) -> Option<Completion<Value>> {
        match self.lookup_command(name) {
            Some(Command::Builtin(f)) => Some(f(self, argv)),
            _ => None,
        }
    }

    /// Parse and evaluate a Tcl expression string against this VM.
    ///
    /// A whole expression that doesn't parse (e.g. a plain string subject the
    /// `switch` codegen feeds through `exprStk`) yields the string itself,
    /// matching the reference VM's lenient `exprStk`.
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, TclError> {
        let node = parse_expr(src, None);
        if matches!(node, tcl_syntax::expr::ExprNode::Raw { .. }) {
            return Ok(Value::string(src));
        }
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
        let _ = self.set_var(name, value);
    }

    fn unset(&mut self, _frame: FrameId, name: &str) -> bool {
        self.unset_var(name)
    }

    fn exists(&self, _frame: FrameId, name: &str) -> bool {
        self.var_exists(name)
    }
}
