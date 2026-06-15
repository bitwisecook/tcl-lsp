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
}

/// Register the builtin set on `vm`.
pub(crate) fn register_builtins(vm: &mut Vm) {
    vm.register("set", cmd_set);
    vm.register("puts", cmd_puts);
    vm.register("source", cmd_source);
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
    vm.register("variable", cmd_variable);
    vm.register("unset", cmd_unset);
    crate::cmd_array::register(vm);
    crate::cmd_list::register(vm);
    crate::cmd_string::register(vm);
    crate::cmd_dict::register(vm);
    crate::cmd_format::register(vm);
    crate::cmd_info::register(vm);
    crate::cmd_math::register(vm);
    crate::cmd_namespace::register(vm);
    crate::cmd_package::register(vm);
    crate::cmd_switch::register(vm);
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
            Err(e) => err(e),
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
    match vm.eval_source(&contents) {
        Ok(c) => c,
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
    let text = match rest {
        [string] => string.to_str(),
        // `puts channelId string` — M1/M2 ignore the channel and write stdout.
        [_channel, string] => string.to_str(),
        _ => {
            return err("wrong # args: should be \"puts ?-nonewline? ?channelId? string\"");
        }
    };
    vm.write_output(&text, newline);
    ok(Value::empty())
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
        return err(e);
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
    let Some(body) = vm.module_proc(&body_key) else {
        return err(format!(
            "proc \"{name_s}\": no pre-compiled body (dynamic proc bodies unsupported)"
        ));
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
    if let Some(r) = resvar {
        vm.set_var(&r.to_str(), comp.result.clone());
    }
    if let Some(o) = optvar {
        vm.set_var(&o.to_str(), comp.options.clone());
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
        vm.add_link(&local, target, &other);
        i += 2;
    }
    ok(Value::empty())
}

/// `variable ?name value ...? name ?value?` — namespace variables (global for M2).
fn cmd_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"variable ?name value ...? name ?value?\"");
    }
    let mut i = 0;
    while i < args.len() {
        let name = args[i].to_str();
        // The full (namespace-qualified) variable, and the unqualified alias
        // the surrounding body refers to it by.
        let qual = vm.qualify_name(&name);
        let local = name.rsplit("::").next().unwrap_or(&name).to_owned();
        if qual.contains("::") {
            // A genuine namespace variable: alias the local to it.
            vm.add_ns_link(&local, &qual);
        } else {
            // Global namespace: behaves like a global scalar.
            vm.add_link(&local, 0, &qual);
        }
        if i + 1 < args.len() {
            // `set_var` follows the alias just installed to the real cell.
            vm.set_var(&local, args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    ok(Value::empty())
}
