//! Commands and builtins.
//!
//! A [`Command`] is either a native builtin or a user procedure. Procs are
//! dispatched by the engine (not here) so a call pushes an activation rather
//! than recursing (steering §8b); builtins run synchronously and return a
//! completion.

use std::rc::Rc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::list::split_list;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// A native builtin: receives argv *without* the command name (Tcl's `objv[1..]`).
pub type BuiltinFn = fn(&mut Vm, &[Value]) -> Completion<Value>;

/// A procedure parameter: a name with an optional default.
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Default value, if the parameter is optional.
    pub default: Option<Value>,
}

/// A user procedure: parameters plus the pre-compiled body.
pub struct ProcDef {
    /// Canonical (namespace-qualified, no leading `::`) proc name.
    pub name: String,
    /// The namespace the body executes in (canonical; `""` = global).
    pub namespace: String,
    /// Formal parameters in order.
    pub params: Vec<Param>,
    /// Whether the last parameter is the `args` catch-all.
    pub has_args: bool,
    /// Pre-compiled body bytecode.
    pub body: Rc<FunctionAsm>,
    /// Original body source text — retained for `info body` (M3). Keeping it
    /// now avoids a frame/proc-model rework when `info` lands.
    #[allow(dead_code)]
    pub body_src: Value,
}

/// A registered command.
#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// A user procedure (dispatched by the engine, which pushes an activation).
    Proc(Rc<ProcDef>),
    /// An `interp alias` — invoking the command evaluates these target words
    /// (the target command plus any fixed prefix arguments) with the call's own
    /// arguments appended.
    Alias(Rc<Vec<Value>>),
}

/// Register the builtin set on `vm`.
pub(crate) fn register_builtins(vm: &mut Vm) {
    vm.register("set", cmd_set);
    vm.register("puts", cmd_puts);
    vm.register("source", cmd_source);
    vm.register("tcl::build-info", cmd_build_info);
    vm.register("interp", cmd_interp);
    vm.register("rename", cmd_rename);
    vm.register("eval", cmd_eval);
    vm.register("apply", cmd_apply);
    vm.register("incr", cmd_incr);
    vm.register("expr", cmd_expr);
    vm.register("proc", cmd_proc);
    vm.register("return", cmd_return);
    vm.register("error", cmd_error);
    vm.register("break", cmd_break);
    vm.register("continue", cmd_continue);
    vm.register("catch", cmd_catch);
    vm.register("global", cmd_global);
    vm.register("upvar", cmd_upvar);
    vm.register("uplevel", cmd_uplevel);
    vm.register("variable", cmd_variable);
    vm.register("unset", cmd_unset);
    vm.register("subst", cmd_subst);
    vm.register("auto_load", cmd_auto_load);
    vm.register("auto_import", |_, _| ok(Value::empty()));
    crate::cmd_array::register(vm);
    crate::cmd_chan::register(vm);
    crate::cmd_list::register(vm);
    crate::cmd_string::register(vm);
    crate::cmd_dict::register(vm);
    crate::cmd_file::register(vm);
    crate::cmd_format::register(vm);
    crate::cmd_info::register(vm);
    crate::cmd_math::register(vm);
    crate::cmd_binary::register(vm);
    crate::cmd_prefix::register(vm);
    crate::cmd_namespace::register(vm);
    crate::cmd_package::register(vm);
    crate::cmd_regexp::register(vm);
    crate::cmd_switch::register(vm);
    crate::cmd_trace::register(vm);
}

/// `set varName ?newValue?` — read or write a scalar.
fn cmd_set(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [name] => {
            let n = name.to_str();
            vm.var_get(&n)
                .map_or_else(|| err(format!("can't read \"{n}\": no such variable")), ok)
        }
        [name, value] => match vm.var_set(&name.to_str(), value.clone()) {
            Ok(()) => ok(value.clone()),
            Err(e) => e,
        },
        _ => err("wrong # args: should be \"set varName ?newValue?\""),
    }
}

/// `source ?-encoding name? fileName` — read a file and evaluate it as a
/// script in the current context. The optional `-encoding` flag is accepted
/// and ignored (files are read as UTF-8).
fn cmd_source(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let path = match args {
        [file] => file.to_str(),
        [flag, _enc, file] if &*flag.to_str() == "-encoding" => file.to_str(),
        _ => return err("wrong # args: should be \"source ?-encoding name? fileName\""),
    };
    let contents = match std::fs::read_to_string(&*path) {
        Ok(c) => c,
        Err(e) => return err(format!("couldn't read file \"{path}\": {e}")),
    };
    // Track the path so `info script` (and callers like `[file dirname [info
    // script]]`) resolve relative to the file being sourced.
    vm.push_script(path.to_string());
    let result = match vm.eval_source(&contents) {
        Ok(c) => c,
        Err(e) => err(e.message),
    };
    vm.pop_script();
    result
}

/// `tcl::build-info ?option?` — report build-time configuration. With no
/// argument it returns the patchlevel/build string; with an option name it
/// returns 1 if that build option was set, else 0. This VM is a plain release
/// build, so every queryable flag (`debug`, `purify`, `memdebug`,
/// `no-deprecate`, …) is absent and reports 0.
fn cmd_build_info(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [] => ok(Value::string("9.0.0")),
        [_option] => ok(Value::int(0)),
        _ => err("wrong # args: should be \"tcl::build-info ?option?\""),
    }
}

/// `eval arg ?arg ...?` — concatenate the arguments with spaces and evaluate the
/// result as a script in the current frame.
fn cmd_eval(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"eval arg ?arg ...?\"");
    }
    let script = if let [single] = args {
        single.to_str().to_string()
    } else {
        args.iter()
            .map(|v| v.to_str().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    match vm.eval_source(&script) {
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

/// Monotonic counter minting unique temporary command names for `apply`.
fn fresh_apply_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("::tcl::apply::lambda{n}")
}

/// `apply lambda ?arg ...?` — invoke an anonymous function `{params body ?ns?}`.
/// Implemented by binding the lambda to a temporary command and evaluating a
/// call, so parameter binding and `return` semantics match a normal proc.
fn cmd_apply(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((lambda, call_args)) = args.split_first() else {
        return err("wrong # args: should be \"apply lambdaExpr ?arg ...?\"");
    };
    let parts = match lambda.as_list() {
        Ok(p) => p,
        Err(c) => return err(c.message),
    };
    if parts.len() < 2 || parts.len() > 3 {
        return err(format!(
            "can't interpret \"{}\" as a lambda expression",
            lambda.to_str()
        ));
    }
    let (params_vec, has_args) = match parse_params(&parts[0].to_str()) {
        Ok(p) => p,
        Err(e) => return err(e),
    };
    let body = parts[1].clone();
    let Some(body_asm) = vm.compile_dynamic_body(&body.to_str()) else {
        return err("apply: could not compile lambda body");
    };
    // The optional third element is the namespace the body runs in (default
    // global). Strip a leading `::` to the canonical form.
    let namespace = parts
        .get(2)
        .map(|v| v.to_str().trim_start_matches("::").to_string())
        .unwrap_or_default();

    let name = fresh_apply_name();
    vm.define_proc(ProcDef {
        name: name.clone(),
        namespace,
        params: params_vec,
        has_args,
        body: body_asm,
        body_src: body,
    });
    let mut words = Vec::with_capacity(call_args.len() + 1);
    words.push(Value::string(name.as_str()));
    words.extend_from_slice(call_args);
    let script = tcl_syntax::list::join_list(words.iter().map(Value::to_str));
    let result = vm.eval_source(&script);
    vm.take_command(&name);
    match result {
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

/// `rename oldName newName` — rename a command, or delete it when `newName` is
/// empty.
fn cmd_rename(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [old, new] = args else {
        return err("wrong # args: should be \"rename oldName newName\"");
    };
    let old_name = old.to_str();
    let Some(cmd) = vm.take_command(&old_name) else {
        return err(format!(
            "can't rename \"{old_name}\": command doesn't exist"
        ));
    };
    let new_name = new.to_str();
    if !new_name.is_empty() {
        // An unqualified target binds in the current namespace; a qualified one
        // is used as given. `register_command` canonicalises the key.
        let key = if new_name.contains("::") {
            new_name.to_string()
        } else {
            vm.qualify_name(&new_name)
        };
        vm.register_command(&key, cmd);
    }
    ok(Value::empty())
}

/// `interp` — the subset the stdlib/test suite needs in a single-interpreter
/// VM. Only the current interpreter (`{}` path) is modelled, so `slaves`/`exists`
/// answer for it and the unsupported sub-interpreter operations are rejected.
fn cmd_interp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"interp cmd ?arg ...?\"");
    };
    match &*sub.to_str() {
        // interp alias srcPath srcCmd targetPath targetCmd ?arg ...?
        "alias" => match rest {
            [src_path, src_cmd, target_path, target @ ..] if !target.is_empty() => {
                if !src_path.to_str().is_empty() || !target_path.to_str().is_empty() {
                    return err("only the current interpreter ({}) is supported");
                }
                let words: Vec<Value> = target.to_vec();
                vm.register_command(&src_cmd.to_str(), Command::Alias(Rc::new(words)));
                ok(src_cmd.clone())
            }
            _ => err(
                "wrong # args: should be \"interp alias srcPath srcCmd targetPath targetCmd ?arg ...?\"",
            ),
        },
        "exists" => match rest {
            [] => ok(Value::int(1)),
            [path] => ok(Value::bool(path.to_str().is_empty())),
            _ => err("wrong # args: should be \"interp exists ?path?\""),
        },
        "slaves" | "children" => ok(Value::empty()),
        other => err(format!(
            "bad option \"{other}\": only alias, exists, and slaves are supported"
        )),
    }
}

/// The standard library `parray` proc, defined on demand by `auto_load`.
const PARRAY_SRC: &str = r#"proc ::parray {a {pattern *}} {
    upvar 1 $a array
    set maxl 0
    foreach name [lsort [array names array $pattern]] {
        if {[string length $name] > $maxl} {
            set maxl [string length $name]
        }
    }
    set maxl [expr {$maxl + [string length $a] + 2}]
    foreach name [lsort [array names array $pattern]] {
        set nameString [format %s(%s) $a $name]
        puts stdout [format "%-*s = %s" $maxl $nameString $array($name)]
    }
}"#;

/// `auto_load command` — there is no on-disk autoloader, but a few standard
/// library procs are provided on demand (so e.g. `info body ::parray` works).
/// Returns 1 if the command was made available, 0 otherwise.
fn cmd_auto_load(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let name = args
        .first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default();
    let simple = name.rsplit("::").next().unwrap_or(&name);
    let src = match simple {
        "parray" => PARRAY_SRC,
        _ => return ok(Value::int(0)),
    };
    match vm.eval_source(src) {
        Ok(c) if c.code.is_ok() => ok(Value::int(1)),
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

/// `subst ?-nobackslashes? ?-nocommands? ?-novariables? string` — perform
/// backslash / command / variable substitution on a string.
fn cmd_subst(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (mut backslashes, mut commands, mut variables) = (true, true, true);
    let mut rest = args;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-nobackslashes" => backslashes = false,
            "-nocommands" => commands = false,
            "-novariables" => variables = false,
            s if s.starts_with('-') && s.len() > 1 => {
                return err(format!(
                    "bad switch \"{s}\": must be -nobackslashes, -nocommands, or -novariables"
                ));
            }
            _ => break,
        }
        rest = &rest[1..];
    }
    let [string] = rest else {
        return err(
            "wrong # args: should be \"subst ?-nobackslashes? ?-nocommands? ?-novariables? string\"",
        );
    };
    match crate::subst::subst_command(vm, &string.to_str(), backslashes, commands, variables) {
        Ok(s) => ok(Value::string(s)),
        Err(e) => err(e.message),
    }
}

/// `puts ?-nonewline? ?channelId? string` — write to the VM's output sink.
fn cmd_puts(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut newline = true;
    if let Some(first) = rest.first()
        && &*first.to_str() == "-nonewline"
    {
        newline = false;
        rest = &rest[1..];
    }
    let (channel, text) = match rest {
        [string] => ("stdout".to_string(), string.to_str().to_string()),
        // `puts ?-nonewline? channelId string`
        [channel, string] => (channel.to_str().to_string(), string.to_str().to_string()),
        _ => {
            return err("wrong # args: should be \"puts ?-nonewline? ?channelId? string\"");
        }
    };
    match crate::cmd_chan::chan_puts(vm, &channel, &text, newline) {
        Ok(()) => ok(Value::empty()),
        Err(e) => err(e),
    }
}

/// `incr varName ?increment?` — add to an integer variable (default 1; missing
/// variable starts at 0).
fn cmd_incr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (name, amount) = match args {
        [name] => (name.to_str(), 1),
        [name, inc] => match inc.as_int() {
            Ok(n) => (name.to_str(), n),
            Err(e) => return err(e.message),
        },
        _ => return err("wrong # args: should be \"incr varName ?increment?\""),
    };
    let old = match vm.var_get(&name) {
        Some(v) => match v.as_int() {
            Ok(n) => n,
            Err(e) => return err(e.message),
        },
        None => 0,
    };
    let next = Value::int(old.wrapping_add(amount));
    if let Err(e) = vm.var_set(&name, next.clone()) {
        return e;
    }
    ok(next)
}

/// `expr arg ?arg ...?` — concatenate the args and evaluate as an expression.
fn cmd_expr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"expr arg ?arg ...?\"");
    }
    let joined = args
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    match vm.eval_expr(&joined) {
        Ok(v) => ok(v),
        Err(e) => err(e.message),
    }
}

/// Resolve a Tcl index spec (`N`, `end`, `end-N`, `end+N`) against a length.
/// Returns a possibly out-of-range signed index; callers clamp/empty as needed.
pub(crate) fn resolve_index(spec: &str, len: usize) -> Option<isize> {
    let s = spec.trim();
    let n = isize::try_from(len).unwrap_or(isize::MAX);
    if s == "end" {
        return Some(n - 1);
    }
    if let Some(rest) = s.strip_prefix("end-") {
        return rest.trim().parse::<isize>().ok().map(|k| n - 1 - k);
    }
    if let Some(rest) = s.strip_prefix("end+") {
        return rest.trim().parse::<isize>().ok().map(|k| n - 1 + k);
    }
    s.parse::<isize>().ok()
}

/// Parse a proc parameter spec (`"a b {c 1} args"`) into params + `has_args`.
fn parse_params(spec: &str) -> Result<(Vec<Param>, bool), String> {
    let elems = split_list(spec).map_err(|e| e.message().to_string())?;
    let mut params = Vec::with_capacity(elems.len());
    for e in &elems {
        let parts = split_list(e.as_ref()).map_err(|err| err.message().to_string())?;
        match parts.as_slice() {
            [n] => params.push(Param {
                name: n.to_string(),
                default: None,
            }),
            [n, d] => params.push(Param {
                name: n.to_string(),
                default: Some(Value::string(d.as_ref())),
            }),
            _ => return Err(format!("too many fields in argument specifier \"{e}\"")),
        }
    }
    let has_args = params.last().is_some_and(|p| p.name == "args");
    Ok((params, has_args))
}

/// `proc name params body` — define a procedure (body is taken pre-compiled
/// from the module; dynamic bodies are not yet supported).
fn cmd_proc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, params, body_text] = args else {
        return err("wrong # args: should be \"proc name args body\"");
    };
    let name_s = name.to_str();
    // The pre-compiled body is keyed by the name as the compiler saw it
    // (global-qualified with a leading `::`), independent of the namespace the
    // `proc` runs in.
    let body_key = if name_s.starts_with("::") {
        name_s.to_string()
    } else {
        format!("::{name_s}")
    };
    // The registration / activation name is qualified with the *current*
    // namespace (so `namespace eval foo { proc bar … }` defines `foo::bar`).
    let reg_name = vm.qualify_name(&name_s);
    let namespace = reg_name
        .rsplit_once("::")
        .map_or_else(String::new, |(ns, _)| ns.to_string());
    let (params_vec, has_args) = match parse_params(&params.to_str()) {
        Ok(p) => p,
        Err(e) => return err(e),
    };
    // Prefer the pre-compiled body; fall back to compiling a dynamically-built
    // body at runtime (e.g. `proc $name $params [subst {…}]`).
    let body = match vm.module_proc(&body_key) {
        Some(b) => b,
        None => match vm.compile_dynamic_body(&body_text.to_str()) {
            Some(b) => b,
            None => return err(format!("proc \"{name_s}\": could not compile body")),
        },
    };
    vm.define_proc(ProcDef {
        name: reg_name,
        namespace,
        params: params_vec,
        has_args,
        body,
        body_src: body_text.clone(),
    });
    ok(Value::empty())
}

fn parse_code(s: &str) -> Code {
    match s {
        "error" | "1" => Code::Error,
        "return" | "2" => Code::Return,
        "break" | "3" => Code::Break,
        "continue" | "4" => Code::Continue,
        _ => Code::Ok,
    }
}

/// Build an options dict value `-code N -level L [-errorcode ..] [-errorinfo ..]`.
fn options_dict(code: Code, level: i64, extra: &[(&str, Value)]) -> Value {
    let mut items = vec![
        Value::string("-code"),
        Value::int(code.as_int()),
        Value::string("-level"),
        Value::int(level),
    ];
    for (k, v) in extra {
        items.push(Value::string(*k));
        items.push(v.clone());
    }
    Value::list(items)
}

/// Look up a key in an options-dict value, returning the following element.
fn opt_get(options: &Value, key: &str) -> Option<Value> {
    let items = options.as_list().ok()?;
    let mut i = 0;
    while i + 1 < items.len() {
        if &*items[i].to_str() == key {
            return Some(items[i + 1].clone());
        }
        i += 2;
    }
    None
}

/// `return ?-code c? ?-level l? ?-errorcode ec? ?-errorinfo ei? ?value?`.
fn cmd_return(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut value = Value::empty();
    let mut ret_code = Code::Ok;
    let mut level = 1i64;
    let mut extra: Vec<(&str, Value)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].to_str();
        match &*a {
            "-code" if i + 1 < args.len() => {
                ret_code = parse_code(&args[i + 1].to_str());
                i += 2;
            }
            "-level" if i + 1 < args.len() => {
                level = args[i + 1].as_int().unwrap_or(1);
                i += 2;
            }
            "-errorcode" if i + 1 < args.len() => {
                extra.push(("-errorcode", args[i + 1].clone()));
                i += 2;
            }
            "-errorinfo" if i + 1 < args.len() => {
                extra.push(("-errorinfo", args[i + 1].clone()));
                i += 2;
            }
            _ if i == args.len() - 1 => {
                value = args[i].clone();
                i += 1;
            }
            _ => return err(format!("bad option \"{a}\"")),
        }
    }
    let options = options_dict(ret_code, level, &extra);
    // Plain `return` raises TCL_RETURN (absorbed at the proc boundary); an
    // explicit non-OK -code takes effect immediately (M2 simplification: skip
    // the -level countdown).
    let final_code = if ret_code == Code::Ok {
        Code::Return
    } else {
        ret_code
    };
    Completion::new(final_code, value, options)
}

/// `error message ?info? ?code?`.
fn cmd_error(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((msg, rest)) = args.split_first() else {
        return err("wrong # args: should be \"error message ?errorInfo? ?errorCode?\"");
    };
    let einfo = rest
        .first()
        .map_or_else(|| msg.to_str().to_string(), |v| v.to_str().to_string());
    let ecode = rest
        .get(1)
        .cloned()
        .unwrap_or_else(|| Value::string("NONE"));
    let options = options_dict(
        Code::Error,
        0,
        &[("-errorcode", ecode), ("-errorinfo", Value::string(einfo))],
    );
    Completion::new(Code::Error, msg.clone(), options)
}

/// `break`.
fn cmd_break(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    Completion::new(Code::Break, Value::empty(), Value::empty())
}

/// `continue`.
fn cmd_continue(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    Completion::new(Code::Continue, Value::empty(), Value::empty())
}

/// `catch script ?resultVarName? ?optionsVarName?`.
fn cmd_catch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (script, resvar, optvar) = match args {
        [s] => (s, None, None),
        [s, r] => (s, Some(r), None),
        [s, r, o] => (s, Some(r), Some(o)),
        _ => {
            return err("wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"");
        }
    };
    let comp = match vm.eval_source(&script.to_str()) {
        Ok(c) => c,
        Err(e) => Completion::new(Code::Error, e.into_value(), Value::empty()),
    };
    if let Some(r) = resvar
        && let Err(e) = vm.set_var(&r.to_str(), comp.result.clone())
    {
        return e;
    }
    if let Some(o) = optvar
        && let Err(e) = vm.set_var(&o.to_str(), comp.options.clone())
    {
        return e;
    }
    if comp.code == Code::Error {
        let einfo = opt_get(&comp.options, "-errorinfo").map_or_else(
            || comp.result.to_str().to_string(),
            |v| v.to_str().to_string(),
        );
        let ecode = opt_get(&comp.options, "-errorcode").unwrap_or_else(|| Value::string("NONE"));
        vm.publish_error(&einfo, &ecode);
    }
    ok(Value::int(comp.code.as_int()))
}

/// `unset ?-nocomplain? ?--? name ...` — remove variables / array elements.
fn cmd_unset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut nocomplain = false;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-nocomplain" => {
                nocomplain = true;
                rest = &rest[1..];
            }
            "--" => {
                rest = &rest[1..];
                break;
            }
            s if s.starts_with('-') => rest = &rest[1..],
            _ => break,
        }
    }
    for n in rest {
        let name = n.to_str();
        if let Some(open) = name.find('(')
            && name.ends_with(')')
            && open > 0
        {
            vm.array_unset_elem(&name[..open], &name[open + 1..name.len() - 1]);
            continue;
        }
        let existed = vm.unset_var(&name);
        if !existed && !nocomplain {
            return err(format!("can't unset \"{name}\": no such variable"));
        }
    }
    ok(Value::empty())
}

/// `global name ?name ...?` — link names to the global frame.
fn cmd_global(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    for n in args {
        let nm = n.to_str();
        vm.add_link(&nm, 0, &nm);
    }
    ok(Value::empty())
}

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
fn cmd_upvar(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    // Default level 1 (the caller).
    let mut target = vm.current_level().saturating_sub(1);
    if let Some(first) = rest.first() {
        let s = first.to_str();
        if let Some(abs) = s.strip_prefix('#')
            && let Ok(n) = abs.parse::<usize>()
        {
            target = n;
            rest = &rest[1..];
        } else if !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = s.parse::<usize>()
        {
            target = vm.current_level().saturating_sub(n);
            rest = &rest[1..];
        }
    }
    if rest.is_empty() || !rest.len().is_multiple_of(2) {
        return err(
            "wrong # args: should be \"upvar ?level? otherVar localVar ?otherVar localVar ...?\"",
        );
    }
    let mut i = 0;
    while i + 1 < rest.len() {
        let other = rest[i].to_str();
        let local = rest[i + 1].to_str();
        // Inside a `namespace eval` body, an unqualified alias names a namespace
        // variable: store the link in the global frame under the qualified name
        // (and resolve the target to its namespace-qualified location), so a
        // proc's `variable <name>` finds the same cell.
        if vm.in_ns_script() && !local.contains("::") && !vm.current_ns().is_empty() {
            let alias = vm.qualify_name(&local);
            let target_name = vm.qualify_name(&other);
            vm.add_global_link(&alias, 0, &target_name);
        } else {
            vm.add_link(&local, target, &other);
        }
        i += 2;
    }
    ok(Value::empty())
}

/// `uplevel ?level? arg ?arg ...?` — evaluate the concatenated args as a script
/// in the call frame `level` up (default 1, the caller). `#N` selects an
/// absolute level.
fn cmd_uplevel(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut target = vm.current_level().saturating_sub(1);
    if let Some(first) = rest.first() {
        let s = first.to_str();
        if let Some(abs) = s.strip_prefix('#')
            && let Ok(n) = abs.parse::<usize>()
        {
            target = n;
            rest = &rest[1..];
        } else if !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = s.parse::<usize>()
        {
            target = vm.current_level().saturating_sub(n);
            rest = &rest[1..];
        }
    }
    if rest.is_empty() {
        return err("wrong # args: should be \"uplevel ?level? command ?arg ...?\"");
    }
    let script = rest
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    vm.eval_at_level(target, &script)
}

/// `variable ?name value ...? name ?value?` — namespace variables (global for M2).
fn cmd_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"variable ?name value ...? name ?value?\"");
    }
    let mut i = 0;
    while i < args.len() {
        let name = args[i].to_str();
        // Namespace variables live in the global frame keyed by their canonical
        // qualified name; alias the unqualified local the body uses to it.
        let qual = vm.qualify_name(&name);
        let local = name.rsplit("::").next().unwrap_or(&name).to_owned();
        vm.add_link(&local, 0, &qual);
        if i + 1 < args.len() {
            // `set_var` follows the alias just installed to the real cell.
            if let Err(e) = vm.set_var(&local, args[i + 1].clone()) {
                return e;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    ok(Value::empty())
}
