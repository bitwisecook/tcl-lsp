//! The interpreter state (`Vm`): the call-frame stack, the command table, the
//! compiled-proc registry, and the variable/command/eval surface.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};
use std::rc::Rc;

use tcl_bytecode::{FunctionAsm, ModuleAsm};
use tcl_platform::Host;
use tcl_runtime_api::{
    Code, CommandId, Commands, CompileService, Completion, FrameId, Frames, Introspect, Namespaces,
    NsId, ProcInfo, ProcParam, Procs, ROOT_NS, Traces, VarStore,
};
use tcl_syntax::expr::{eval, parse_expr};

use crate::command::{BuiltinFn, Command, ProcDef, register_builtins};
use crate::error::TclError;
use crate::expr::ExprEval;
use crate::frame::{CallFrame, Local};
use crate::host_native::NativeHost;
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
    /// Namespace-name ⇆ opaque `NsId` arena for the Family-B `Frames`/`Namespaces`
    /// contract. The VM resolves namespaces by their canonical `String` name; this
    /// side-table mints stable `NsId` handles for them (`ns_arena[id]` is the name,
    /// `ROOT_NS` = 0 = `""`), bridging the handle-based trait to the string model.
    ns_arena: Vec<String>,
    ns_intern: HashMap<String, NsId>,
    /// Command-FQN ⇆ dense raw `CommandId` arena for `Namespaces::find_command`
    /// and `Commands::dispatch_id`. Interior-mutable because `find_command` is
    /// `&self` but mints a handle on first sight. Bidirectional: `find_command`
    /// interns an absolute FQN, `dispatch_id` reverses the id to that FQN and
    /// invokes it.
    cmd_arena: RefCell<CmdArena>,
    /// Provided packages → version (`package provide`/`require`).
    packages: HashMap<String, String>,
    /// Variable traces, keyed by a resolved-owner key (frame level + name) so a
    /// trace fires regardless of the access path (`upvar` alias, qualified
    /// name, …). Newest trace last; fired newest-first.
    var_traces: HashMap<String, Vec<VarTrace>>,
    /// Re-entrancy guard: `"<key>\0<op>"` entries for traces currently firing.
    active_traces: std::collections::HashSet<String>,
    /// Frame depths at which the currently-executing `namespace eval`/`inscope`
    /// bodies started (innermost last). A namespace body runs in the frame that
    /// invoked it, so when the current frame depth matches the innermost entry
    /// we are directly in a namespace script (unqualified names alias namespace
    /// variables); inside a proc called from one, the depths differ and
    /// unqualified names are proc locals.
    ns_script_frames: Vec<usize>,
    out: Box<dyn Write>,
    compiler: Option<Box<dyn CompileService<Module = ModuleAsm>>>,
    /// Open I/O channels (file handles), keyed by channel id (`file3`, …). The
    /// predefined `stdin`/`stdout`/`stderr` are not stored here; commands
    /// special-case those names.
    channels: HashMap<String, crate::cmd_chan::Channel>,
    /// Monotonic counter for minting fresh channel ids.
    chan_counter: u32,
    /// Stack of file paths currently being evaluated by `source`. The top is
    /// what `info script` returns; empty when not inside a `source`.
    script_stack: Vec<String>,
    /// The host environment: the capability seam (`tcl-platform`) through which
    /// every command reaches the filesystem, clock, env, stdio, subprocess, and
    /// sockets. The bytecode VM is a native target, so this defaults to a
    /// full-capability [`NativeHost`]; [`Vm::set_host`] swaps it (e.g. for a
    /// sandboxed, WASM-posture host in capability tests). An `Rc` (not `Box`) so
    /// a command can clone a handle and pass `&dyn Host` *alongside* a `&mut Vm`
    /// borrow (the VM is itself the `ValueOps` a shared helper takes).
    host: Rc<dyn Host>,
}

/// The command-identity arena backing `Namespaces::find_command` /
/// `Commands::dispatch_id`: a bijection between a command's absolute FQN and a
/// dense raw `CommandId` (the index into `fqns`). Minted on first `find_command`.
#[derive(Default)]
struct CmdArena {
    ids: HashMap<String, u32>,
    fqns: Vec<String>,
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
            ns_arena: vec![String::new()],
            ns_intern: HashMap::from([(String::new(), ROOT_NS)]),
            cmd_arena: RefCell::new(CmdArena::default()),
            packages: HashMap::new(),
            var_traces: HashMap::new(),
            active_traces: std::collections::HashSet::new(),
            ns_script_frames: Vec::new(),
            out,
            compiler: None,
            channels: HashMap::new(),
            chan_counter: 2,
            script_stack: Vec::new(),
            host: Rc::new(NativeHost::new()),
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
    pub fn set_compiler(&mut self, compiler: Box<dyn CompileService<Module = ModuleAsm>>) {
        self.compiler = Some(compiler);
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

    /// Swap the host environment — e.g. a [`NativeHost::sandboxed`] to exercise
    /// the WASM-posture "unsupported" paths natively.
    pub fn set_host(&mut self, host: Rc<dyn Host>) {
        self.host = host;
    }

    pub(crate) fn register(&mut self, name: &str, f: BuiltinFn) {
        self.register_command(name, Command::Builtin(f));
    }

    pub(crate) fn register_command(&mut self, name: &str, cmd: Command) {
        // The table is keyed by canonical names (no leading `::`).
        let key = name.strip_prefix("::").unwrap_or(name);
        self.commands.insert(key.to_owned(), cmd);
    }

    /// The canonical table key `name` resolves to (honouring the current
    /// namespace, like [`Self::lookup_command`]), if such a command exists.
    fn command_key(&self, name: &str) -> Option<String> {
        if let Some(abs) = name.strip_prefix("::") {
            return self.commands.contains_key(abs).then(|| abs.to_owned());
        }
        let cur = self.current_ns();
        if !cur.is_empty() {
            let q = format!("{cur}::{name}");
            if self.commands.contains_key(&q) {
                return Some(q);
            }
        }
        self.commands.contains_key(name).then(|| name.to_owned())
    }

    /// Resolve and remove the command `name`, returning it (for `rename`).
    pub(crate) fn take_command(&mut self, name: &str) -> Option<Command> {
        let key = self.command_key(name)?;
        self.commands.remove(&key)
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

    /// Intern a canonical namespace name to its stable `NsId`, minting one on
    /// first sight (`ROOT_NS` = 0 = `""`). The handle a `Frames::push` caller
    /// passes — and what `Namespaces::current` will return — round-trips through
    /// [`ns_name`](Self::ns_name).
    pub fn intern_ns(&mut self, name: &str) -> NsId {
        if let Some(&id) = self.ns_intern.get(name) {
            return id;
        }
        let id = NsId(u32::try_from(self.ns_arena.len()).expect("namespace count fits u32"));
        self.ns_arena.push(name.to_string());
        self.ns_intern.insert(name.to_string(), id);
        id
    }

    /// The canonical namespace name for an interned `NsId` (`""` for `ROOT_NS`
    /// or any unknown id — a `Frames::push` into the global namespace).
    fn ns_name(&self, id: NsId) -> String {
        self.ns_arena
            .get(id.0 as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Resolve `name` from namespace `cxt` to its command's canonical key (the
    /// `commands` map key — a qualified name without the leading `::`), mirroring
    /// [`lookup_command`](Self::lookup_command)'s order: an absolute `::name`
    /// directly, else `cxt::name`, else the global `name`. `None` if unresolved.
    fn resolve_command_fqn(&self, cxt: &str, name: &str) -> Option<String> {
        if let Some(abs) = name.strip_prefix("::") {
            return self.commands.contains_key(abs).then(|| abs.to_string());
        }
        if !cxt.is_empty() {
            let qualified = format!("{cxt}::{name}");
            if self.commands.contains_key(&qualified) {
                return Some(qualified);
            }
        }
        self.commands.contains_key(name).then(|| name.to_string())
    }

    /// Intern an absolute command FQN to a stable, dense raw `CommandId`, minting
    /// one on first sight. Backs `Namespaces::find_command`.
    fn intern_cmd(&self, fqn: &str) -> u32 {
        let mut a = self.cmd_arena.borrow_mut();
        if let Some(&id) = a.ids.get(fqn) {
            return id;
        }
        let id = u32::try_from(a.fqns.len()).expect("command count fits u32");
        a.fqns.push(fqn.to_string());
        a.ids.insert(fqn.to_string(), id);
        id
    }

    /// The absolute FQN an interned raw `CommandId` was minted from, or `None`
    /// for a fabricated/out-of-range id. Backs `Commands::dispatch_id`'s reverse.
    fn command_fqn(&self, id: u32) -> Option<String> {
        self.cmd_arena.borrow().fqns.get(id as usize).cloned()
    }

    /// Push a namespace onto the resolution stack (created if new).
    pub(crate) fn push_ns(&mut self, ns: String) {
        if !ns.is_empty() {
            self.namespaces.insert(ns.clone());
        }
        // Ensure it has an `NsId` so `Namespaces::current` (a `&self` lookup) can
        // resolve it without minting.
        self.intern_ns(&ns);
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
        // Mint a stable `NsId` (handle) so the `Namespaces` nav methods are pure
        // `&self` lookups — every namespace, however created, has an id.
        self.intern_ns(ns);
        if let Some((parent, _)) = ns.rsplit_once("::") {
            self.declare_namespace(parent);
        }
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
            let base = self.trace_qualify(base);
            let (lvl, nm) = self.locate(&base);
            format!("{lvl}\u{0}{nm}({key})")
        } else {
            let name = self.trace_qualify(name);
            let (lvl, nm) = self.locate(&name);
            format!("{lvl}\u{0}{nm}")
        }
    }

    /// Resolve a bare variable name to its namespace-qualified form when the
    /// current scope is a `namespace eval` body, matching how `set`/`variable`
    /// bind a namespace variable. Unlike [`Self::ns_var_fallback`] this does not
    /// require the variable to already exist — `trace add variable foo …` at
    /// namespace-script level targets `::ns::foo` even before it is created, so
    /// the key matches the resolved name a later read inside a proc produces.
    fn trace_qualify(&self, name: &str) -> String {
        if name.contains("::") || !self.in_ns_script() {
            return name.to_string();
        }
        let cur = self.current_ns();
        if cur.is_empty() {
            return name.to_string();
        }
        // A genuine local in the namespace-eval frame shadows the namespace var.
        if self
            .frames
            .last()
            .is_some_and(|f| f.locals.contains_key(name))
        {
            return name.to_string();
        }
        format!("{cur}::{name}")
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
        // Tcl suppresses *all* of a variable's traces while any one of them is
        // being handled (the documented "traces are disabled during the
        // handling of other traces" behaviour — see tcltest's outputChannel
        // notes). The unit of suppression is the whole variable (every array
        // element, every operation), so a read trace that writes the same
        // variable won't re-enter its write trace, and a whole-array read trace
        // fires only once per top-level access rather than per element.
        let (base_lvl, base_nm) = self.locate(&base);
        let active_key = format!("{base_lvl}\u{0}{base_nm}");
        if self.active_traces.contains(&active_key) {
            return Ok(());
        }
        self.active_traces.insert(active_key.clone());
        let r = self.fire_var_traces_inner(name, op, &base, &elem);
        self.active_traces.remove(&active_key);
        r
    }

    /// Inner firing loop for [`Self::fire_var_traces`]: run the element-specific
    /// traces, then the whole-array traces, with the variable already marked
    /// active by the caller.
    fn fire_var_traces_inner(
        &mut self,
        name: &str,
        op: &str,
        base: &str,
        elem: &str,
    ) -> Result<(), Completion<Value>> {
        // Fire the element-specific traces, then the whole-array traces (for an
        // element write a trace on the base array also fires).
        let mut keys = vec![self.trace_key(name)];
        if elem_ref(name).is_some() {
            let (lvl, nm) = self.locate(base);
            keys.push(format!("{lvl}\u{0}{nm}"));
        }
        for key in keys {
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
                    tcl_brace(base),
                    tcl_brace(elem),
                    op
                );
                let r = self.eval_source(&script);
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

    /// Evaluate `src` as if `target` were the current call frame (`uplevel`).
    /// The frames above `target` are set aside for the duration and restored
    /// afterwards, so the script's variable references and `info level` resolve
    /// against the target activation. A `Return` completion is passed through to
    /// the calling frame (as the reference `uplevel` does).
    /// Push the path of a file being `source`d; the matching [`Self::pop_script`]
    /// restores the previous one. Drives `info script`.
    pub(crate) fn push_script(&mut self, path: String) {
        self.script_stack.push(path);
    }

    /// Pop the current `source` path (see [`Self::push_script`]).
    pub(crate) fn pop_script(&mut self) {
        self.script_stack.pop();
    }

    /// The path of the file currently being `source`d (`info script`); empty
    /// when evaluation is not inside a `source`.
    pub(crate) fn current_script(&self) -> &str {
        self.script_stack.last().map_or("", String::as_str)
    }

    pub(crate) fn eval_at_level(&mut self, target: usize, src: &str) -> Completion<Value> {
        if target >= self.frames.len() {
            return err(format!("bad level \"{target}\""));
        }
        let saved = self.frames.split_off(target + 1);
        let result = self.eval_source(src);
        // Restore any frames the script left in place, then re-attach the ones
        // we set aside (the script's own proc activations are already balanced).
        self.frames.truncate(target + 1);
        self.frames.extend(saved);
        match result {
            Ok(c) => c,
            Err(e) => err(e.message),
        }
    }

    /// The unqualified names of commands (or, with `procs_only`, just user
    /// procedures) defined **directly** in namespace `canonical` (unrooted; `""`
    /// = global) — the `Namespaces::commands_in`/`procs_in` enumeration, filtering
    /// the flat command map. Direct members only (`foo::sub::x` is not in `foo`).
    pub(crate) fn names_directly_in(&self, canonical: &str, procs_only: bool) -> Vec<String> {
        self.commands
            .iter()
            .filter(|(_, c)| !procs_only || matches!(c, Command::Proc(_)))
            .filter_map(|(key, _)| direct_member_tail(key, canonical).map(str::to_owned))
            .collect()
    }

    /// The `ProcDef` for a user proc, if `name` resolves to one (`info body`/`args`).
    pub(crate) fn proc_def(&self, name: &str) -> Option<Rc<crate::command::ProcDef>> {
        match self.lookup_command(name) {
            Some(Command::Proc(p)) => Some(p),
            _ => None,
        }
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

    /// Whether a `namespace eval`/`inscope` body is *directly* executing in the
    /// current frame. Returns `false` inside a proc called from such a body —
    /// a proc activation has its own scope where unqualified names are locals,
    /// not namespace variables. We test this by recording the frame depth at
    /// which each namespace script started and checking the innermost against
    /// the current depth.
    pub(crate) fn in_ns_script(&self) -> bool {
        self.ns_script_frames.last() == Some(&self.frames.len())
    }

    /// Enter/leave a `namespace eval`/`inscope` body (around its evaluation).
    /// The body runs in the current frame, so we remember that frame depth.
    pub(crate) fn enter_ns_script(&mut self) {
        self.ns_script_frames.push(self.frames.len());
    }
    pub(crate) fn leave_ns_script(&mut self) {
        self.ns_script_frames.pop();
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

    // -- frame-addressed storage (the `VarStore` `FrameId`-honouring path) ------
    //
    // These resolve `name` starting from an *explicit* frame (following links),
    // touching only storage: no current-eval-context namespace fallback and no
    // trace firing (both are current-frame concerns). The `VarStore` impl uses
    // them only for a non-current `FrameId`; a `FrameId` equal to the current
    // frame delegates to the full inherent accessors above, so the common case
    // keeps its exact behaviour (fallback + traces).

    /// Frame-addressed scalar read (the storage half of [`get_var`](Self::get_var)).
    pub(crate) fn get_var_from(&self, start: usize, name: &str) -> Option<Value> {
        let (lvl, nm) = self.locate_from(name, start);
        if let Some((base, key)) = elem_ref(&nm) {
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

    /// Frame-addressed scalar write (the storage half of
    /// [`write_scalar_raw`](Self::write_scalar_raw)).
    pub(crate) fn write_scalar_from(&mut self, start: usize, name: &str, value: Value) {
        let (lvl, nm) = self.locate_from(name, start);
        if let Some((base, key)) = elem_ref(&nm) {
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

    /// Frame-addressed scalar existence (the storage half of
    /// [`var_exists`](Self::var_exists)).
    pub(crate) fn exists_from(&self, start: usize, name: &str) -> bool {
        let (lvl, nm) = self.locate_from(name, start);
        self.frames
            .get(lvl)
            .is_some_and(|f| matches!(f.locals.get(&nm), Some(Local::Scalar(_))))
    }

    /// Frame-addressed unset (storage only — no unset-trace firing).
    pub(crate) fn unset_from(&mut self, start: usize, name: &str) -> bool {
        let (lvl, nm) = self.locate_from(name, start);
        self.frames
            .get_mut(lvl)
            .is_some_and(|f| f.locals.remove(&nm).is_some())
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
        // Only a namespace-eval body resolves a bare name to a namespace
        // variable. Inside a proc (even one defined in the namespace), an
        // unqualified name is a local unless declared via `variable`/`global`.
        if !self.in_ns_script() {
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

    /// Register a freshly opened channel, returning its minted id (`file3`, …).
    pub(crate) fn add_channel(&mut self, chan: crate::cmd_chan::Channel) -> String {
        let id = format!("file{}", self.chan_counter);
        self.chan_counter += 1;
        self.channels.insert(id.clone(), chan);
        id
    }

    /// Borrow an open channel by id (`None` for unknown ids and the predefined
    /// std channels, which callers handle by name).
    pub(crate) fn channel_mut(&mut self, id: &str) -> Option<&mut crate::cmd_chan::Channel> {
        self.channels.get_mut(id)
    }

    /// Close and drop a channel by id, returning `true` if it existed.
    pub(crate) fn remove_channel(&mut self, id: &str) -> bool {
        self.channels.remove(id).is_some()
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
/// absolute frame level (`GLOBAL_FRAME` = 0); access resolves from that frame,
/// following links. A `FrameId` naming the *current* frame delegates to the full
/// by-name accessors (namespace fallback + traces); any other frame uses the
/// frame-addressed storage helpers (no current-eval-context fallback/traces).
impl VarStore for Vm {
    type Value = Value;

    fn get(&self, frame: FrameId, name: &str) -> Option<Value> {
        if frame.0 == self.current_level() {
            self.get_var(name)
        } else {
            self.get_var_from(frame.0, name)
        }
    }

    fn set(&mut self, frame: FrameId, name: &str, value: Value) {
        if frame.0 == self.current_level() {
            let _ = self.set_var(name, value);
        } else {
            self.write_scalar_from(frame.0, name, value);
        }
    }

    fn unset(&mut self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            self.unset_var(name)
        } else {
            self.unset_from(frame.0, name)
        }
    }

    fn exists(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            // The *complete* existence check: a scalar, an array, or an array
            // element (`exists_var`) — not the scalar-only `var_exists`, which
            // would miss arrays like `::env` (a `VarStore` contract bug surfaced
            // by routing `info exists` through this method).
            self.exists_var(name)
        } else {
            self.exists_from(frame.0, name)
        }
    }

    // Element access: the VM's by-name accessors already parse `base(key)`, so
    // get/set/exists reconstruct the name and delegate (honouring `FrameId`).
    // `unset` is the exception — `unset_var` is element-blind — so it removes the
    // element directly (active frame; the cores always pass the current frame).

    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Value> {
        self.get(frame, &format!("{name}({key})"))
    }

    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Value) {
        self.set(frame, &format!("{name}({key})"), value);
    }

    fn unset_elem(&mut self, _frame: FrameId, name: &str, key: &str) -> bool {
        let existed = self.get_array_elem(name, key).is_some();
        self.array_unset_elem(name, key);
        existed
    }

    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool {
        self.exists(frame, &format!("{name}({key})"))
    }

    fn array_keys(&self, _frame: FrameId, name: &str) -> Option<Vec<String>> {
        // `array_is` distinguishes an (empty-or-not) array from a scalar/unset;
        // `array_pairs` yields the keys (active frame — the cores pass current).
        if self.array_is(name) {
            Some(self.array_pairs(name).into_iter().map(|(k, _)| k).collect())
        } else {
            None
        }
    }
}

/// Runtime introspection backing the `info` family (`info level`/`info level N`).
///
/// Handle-free — the reconciliation finding is that `Introspect` fits *both*
/// runtime models as-drafted (no `FrameId`/`NsId` reshape needed), so it is the
/// first Family-B role trait both the VM and `runtime/rust` satisfy with shared
/// semantics: [`level`](Introspect::level) is the current stack depth and
/// [`level_argv`](Introspect::level_argv) the retained invoking words at an
/// absolute level, `None` for a level with no call (the global frame).
impl Introspect for Vm {
    type Value = Value;

    fn level(&self) -> usize {
        self.current_level()
    }

    fn level_argv(&self, level: usize) -> Option<Value> {
        self.frame_argv(level)
            .filter(|av| !av.is_empty())
            .map(Value::list)
    }
}

/// Proc introspection (`info body`/`args`/`default`) over the VM's retained
/// [`ProcDef`](crate::command::ProcDef). The body (`body_src`) and any defaults
/// are flattened to owned bytes so the shared `info` core can rebuild the result
/// values through `ValueOps` — a string round-trip that is observably identical
/// (only the value's string content is significant to these subcommands).
impl Procs for Vm {
    fn proc_info(&self, name: &str) -> Option<ProcInfo> {
        let p = self.proc_def(name)?;
        Some(ProcInfo {
            body: p.body_src.to_str().as_bytes().to_vec(),
            params: p
                .params
                .iter()
                .map(|pp| ProcParam {
                    name: pp.name.as_bytes().to_vec(),
                    default: pp.default.as_ref().map(|d| d.to_str().as_bytes().to_vec()),
                })
                .collect(),
        })
    }
}

/// Command dispatch: resolve `name` in the current namespace context and run it
/// with `argv` (name-stripped) to a [`Completion`]. Builtins run inline, procs
/// run to completion in a nested activation, aliases re-evaluate their target,
/// and an unknown name yields an error completion. The owned-`Value` model keeps
/// the refcount discipline implicit (`Rc` clones), unlike the runtime's
/// `*mut TclObj` impl.
impl Commands for Vm {
    type Value = Value;

    fn dispatch(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        self.invoke_command(name, argv)
    }

    fn dispatch_id(&mut self, cmd: CommandId, argv: &[Value]) -> Completion<Value> {
        // Reverse the handle to its absolute FQN, then invoke that — the
        // resolve-then-invoke pairing with `Namespaces::find_command`.
        match self.command_fqn(cmd.0) {
            Some(fqn) => self.invoke_command(&fqn, argv),
            None => err("invalid command id"),
        }
    }
}

/// Variable traces: fire `var`'s `op` (`read`/`write`/`unset`) traces, aborting
/// the access if a callback errors. The VM's [`fire_var_traces`](Vm::fire_var_traces)
/// already produces the user-facing `can't read/set "var": <msg>` completion
/// (and swallows `unset`/`array` errors, matching C); the trait keeps only its
/// error result value (`options` is irrelevant to an aborted access).
impl Traces for Vm {
    type Value = Value;

    fn fire(&mut self, var: &str, op: &str) -> Result<(), Value> {
        self.fire_var_traces(var, op).map_err(|c| c.result)
    }
}

/// The call-frame stack. The VM tracks namespace context by `String`, so
/// [`push`](Frames::push) resolves the `NsId` to its name (via the intern arena)
/// and pushes a bare call frame plus that namespace context; [`pop`](Frames::pop)
/// unwinds both. [`link`](Frames::link) installs an `upvar`-style alias in the
/// current frame (the only frame `upvar` targets) — the VM stores variables
/// (globals included) in their frame's locals, so a plain level-addressed link
/// suffices.
impl Frames for Vm {
    fn push(&mut self, ns: NsId) -> FrameId {
        let name = self.ns_name(ns);
        let level = self.push_call_frame(None, Vec::new());
        self.push_ns(name);
        FrameId(level)
    }

    fn pop(&mut self) {
        self.pop_ns();
        self.pop_call_frame();
    }

    fn current(&self) -> FrameId {
        FrameId(self.current_level())
    }

    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str) {
        debug_assert_eq!(
            here.0,
            self.current_level(),
            "upvar installs in the current frame"
        );
        self.add_link(local, target.0, target_name);
    }
}

/// Namespace name resolution over the VM's String-based namespace model, bridged
/// to opaque `NsId`/`CommandId` handles via the intern arenas.
/// [`current`](Namespaces::current) returns the interned id of the current
/// namespace (interned when pushed). [`find_command`](Namespaces::find_command)
/// resolves `name` from `cxt` to its command key and interns that to a stable
/// `CommandId`. Note: the handle is produced for command *identity* only —
/// nothing dispatches by it yet (the `Commands` trait dispatches by name), the
/// open `find_command`/`CommandId` consumer question.
impl Namespaces for Vm {
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId> {
        let cxt_name = self.ns_name(cxt);
        // Intern the *absolute* FQN (the `commands` key is unrooted) so
        // `dispatch_id` can re-dispatch it unambiguously regardless of context.
        let key = self.resolve_command_fqn(&cxt_name, name)?;
        Some(CommandId(self.intern_cmd(&format!("::{key}"))))
    }

    fn current(&self) -> NsId {
        self.ns_intern
            .get(self.current_ns())
            .copied()
            .unwrap_or(ROOT_NS)
    }

    fn name(&self, ns: NsId) -> String {
        // The arena holds the canonical (unrooted) name; `namespace current`
        // reports the absolute form (`""` → `"::"`).
        let canonical = self.ns_name(ns);
        if canonical.is_empty() {
            "::".to_string()
        } else {
            format!("::{canonical}")
        }
    }

    fn command_name(&self, cmd: CommandId) -> Option<String> {
        self.command_fqn(cmd.0)
    }

    // Namespace-tree navigation over the arena. Every namespace is interned on
    // creation (`push_ns`/`declare_namespace`), so these are pure `&self` lookups
    // — the String model honouring the `NsId` handle contract.
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId> {
        // Resolve `name` (absolute, or relative to `cxt`) to a canonical name.
        let canonical = if let Some(abs) = name.strip_prefix("::") {
            abs.to_string()
        } else {
            let cxt_name = self.ns_name(cxt);
            if cxt_name.is_empty() {
                name.to_string()
            } else {
                format!("{cxt_name}::{name}")
            }
        };
        self.ns_intern.get(&canonical).copied()
    }

    fn parent(&self, ns: NsId) -> Option<NsId> {
        let name = self.ns_name(ns);
        if name.is_empty() {
            return None; // the global root has no parent
        }
        let parent = name.rsplit_once("::").map_or("", |(p, _)| p);
        self.ns_intern.get(parent).copied()
    }

    fn children(&self, ns: NsId) -> Vec<NsId> {
        self.child_namespaces(&self.ns_name(ns))
            .iter()
            .filter_map(|c| self.ns_intern.get(c).copied())
            .collect()
    }

    // Command enumeration over the flat command map (keyed by canonical unrooted
    // name): the direct members of namespace `ns`, as unqualified tails.
    fn commands_in(&self, ns: NsId) -> Vec<String> {
        self.names_directly_in(&self.ns_name(ns), false)
    }

    fn procs_in(&self, ns: NsId) -> Vec<String> {
        self.names_directly_in(&self.ns_name(ns), true)
    }
}

/// The unqualified tail of `key` if it names a command **directly** in namespace
/// `canonical` (unrooted; `""` = global), else `None`. A direct member's key is
/// `canonical::tail` (or a bare `tail` at the global level) with no further `::`
/// in the tail — so descendants (`foo::sub::x` for `foo`) and the namespace
/// itself are excluded.
fn direct_member_tail<'a>(key: &'a str, canonical: &str) -> Option<&'a str> {
    let tail = if canonical.is_empty() {
        key
    } else {
        key.strip_prefix(canonical)?.strip_prefix("::")?
    };
    if tail.is_empty() || tail.contains("::") {
        None
    } else {
        Some(tail)
    }
}

#[cfg(test)]
mod family_b_tests {
    use super::*;
    use tcl_runtime_api::GLOBAL_FRAME;

    #[test]
    fn introspect_level_and_argv() {
        let mut vm = Vm::new();
        // Top level: depth 0, the global frame has no invoking call.
        assert_eq!(Introspect::level(&vm), 0);
        assert!(Introspect::level_argv(&vm, 0).is_none());
        // A proc-call frame with its invoking words.
        vm.push_call_frame(
            Some("p".to_string()),
            vec![Value::string("p"), Value::string("x")],
        );
        assert_eq!(Introspect::level(&vm), 1);
        assert_eq!(&*Introspect::level_argv(&vm, 1).unwrap().to_str(), "p x");
        vm.pop_call_frame();
        assert_eq!(Introspect::level(&vm), 0);
    }

    #[test]
    fn varstore_honours_frame_id() {
        let mut vm = Vm::new();
        // Write into the global frame while it is current.
        vm.set(GLOBAL_FRAME, "g", Value::string("global"));
        // Enter a proc-call frame; the global var is not in it.
        vm.push_call_frame(Some("p".to_string()), vec![Value::string("p")]);
        let here = FrameId(vm.current_level());
        assert_ne!(here, GLOBAL_FRAME);
        vm.set(here, "loc", Value::string("local"));
        // FrameId is honoured: each frame sees only its own var.
        assert_eq!(
            vm.get(GLOBAL_FRAME, "g").map(|v| v.to_str().to_string()),
            Some("global".to_string())
        );
        assert!(vm.get(here, "g").is_none());
        assert!(vm.exists(here, "loc"));
        assert!(!vm.exists(GLOBAL_FRAME, "loc"));
        // Reach back into the global frame from the child frame.
        vm.set(GLOBAL_FRAME, "g2", Value::string("two"));
        vm.pop_call_frame();
        assert_eq!(
            vm.get(GLOBAL_FRAME, "g2").map(|v| v.to_str().to_string()),
            Some("two".to_string())
        );
        assert!(vm.unset(GLOBAL_FRAME, "g2"));
        assert!(!vm.exists(GLOBAL_FRAME, "g2"));
    }

    #[test]
    fn varstore_array_elements() {
        let mut vm = Vm::new();
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
        vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("v"));
        assert!(vm.exists_elem(GLOBAL_FRAME, "a", "k"));
        assert_eq!(
            vm.get_elem(GLOBAL_FRAME, "a", "k")
                .map(|v| v.to_str().to_string()),
            Some("v".to_string())
        );
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "nope"));
        assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    }

    #[test]
    fn commands_dispatch_builtin_and_unknown() {
        let mut vm = Vm::new();
        // A builtin runs inline and yields its result.
        let c = vm.dispatch("list", &[Value::string("a"), Value::string("b c")]);
        assert_eq!(c.code, Code::Ok);
        assert_eq!(&*c.result.to_str(), "a {b c}");
        // An unknown command name is an error completion.
        let c = vm.dispatch("no_such_command", &[]);
        assert_eq!(c.code, Code::Error);
        assert_eq!(
            &*c.result.to_str(),
            "invalid command name \"no_such_command\""
        );
    }

    #[test]
    fn frames_push_pop_current_link() {
        let mut vm = Vm::new();
        let to_s = |v: Option<Value>| v.map(|v| v.to_str().to_string());
        // A global, set while the global frame is current.
        vm.set(GLOBAL_FRAME, "g", Value::string("orig"));
        let outer = Frames::current(&vm);
        assert_eq!(outer, GLOBAL_FRAME);
        // Push a proc-call frame in a fresh namespace (interned to an NsId, which
        // round-trips through the arena back to its name on push).
        let ns = vm.intern_ns("foo");
        let inner = Frames::push(&mut vm, ns);
        assert_ne!(inner, outer);
        assert_eq!(Frames::current(&vm), inner);
        // `upvar`: link `gg` (inner) to the outer frame's global `g`; reads and
        // writes through `gg` reach `g`.
        Frames::link(&mut vm, inner, outer, "gg", "g");
        assert_eq!(to_s(vm.get(inner, "gg")), Some("orig".to_string()));
        vm.set(inner, "gg", Value::string("changed"));
        // Pop back to the global frame: the link is gone, the global updated.
        Frames::pop(&mut vm);
        assert_eq!(Frames::current(&vm), outer);
        assert_eq!(to_s(vm.get(GLOBAL_FRAME, "g")), Some("changed".to_string()));
    }

    #[test]
    fn namespaces_current_and_find_command() {
        let mut vm = Vm::new();
        // At the top level the current namespace is the global root.
        assert_eq!(Namespaces::current(&vm), ROOT_NS);
        // Builtins resolve from the global namespace to stable, distinct ids.
        let a = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
        assert_eq!(a, Namespaces::find_command(&vm, ROOT_NS, "list").unwrap());
        assert_ne!(
            a,
            Namespaces::find_command(&vm, ROOT_NS, "llength").unwrap()
        );
        assert!(Namespaces::find_command(&vm, ROOT_NS, "no_such_command").is_none());
        // `current` tracks the namespace pushed by `Frames::push`.
        let foo = vm.intern_ns("foo");
        Frames::push(&mut vm, foo);
        assert_eq!(Namespaces::current(&vm), foo);
        Frames::pop(&mut vm);
        assert_eq!(Namespaces::current(&vm), ROOT_NS);
    }

    #[test]
    fn commands_dispatch_id_composes() {
        let mut vm = Vm::new();
        // Resolve a command to a handle, then invoke it *by that handle* — the
        // find_command -> dispatch_id composition.
        let id = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
        let c = vm.dispatch_id(id, &[Value::string("a"), Value::string("b")]);
        assert_eq!(c.code, Code::Ok);
        assert_eq!(&*c.result.to_str(), "a b");
        // A fabricated id yields an error completion (no such command).
        let c = vm.dispatch_id(CommandId(9999), &[]);
        assert_eq!(c.code, Code::Error);
        assert_eq!(&*c.result.to_str(), "invalid command id");
    }
}
