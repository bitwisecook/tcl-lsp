//! The `namespace` ensemble.
//!
//! `namespace eval ns body` runs `body` with the current namespace set to `ns`
//! so that `proc`/command/variable name resolution qualifies relative to it
//! (see [`Vm::qualify_name`]/[`Vm::lookup_command`]). The introspection
//! subcommands (`current`, `qualifiers`, `tail`, `parent`, `children`,
//! `exists`) operate on canonical names; `export`/`import` are accepted as
//! no-ops for now (the codegen already records export/import metadata).

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// Run `body` as a script in namespace `target`, absorbing a top-level
/// `return` at the boundary (a namespace body completes like a proc body).
///
/// The body runs in its own call frame (like a proc) so `info level` counts it
/// and `uplevel`/`upvar` from a proc called within reach it (and its namespace
/// variables). `call_argv` is the invoking command (e.g. `namespace eval ::ns
/// {…}`) for `info level N`.
fn eval_in_ns(vm: &mut Vm, target: String, body: &str, call_argv: Vec<Value>) -> Completion<Value> {
    vm.declare_namespace(&target);
    vm.push_ns_eval_frame(&target, call_argv);
    vm.push_ns(target);
    vm.enter_ns_script();
    let result = vm.eval_source(body);
    vm.leave_ns_script();
    vm.pop_ns();
    vm.pop_call_frame();
    match result {
        Ok(c) if c.code == Code::Return => ok(c.result),
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("namespace", cmd_namespace);
}

/// Display form of a canonical namespace (`""` → `::`, `foo` → `::foo`).
fn display_ns(canonical: &str) -> String {
    if canonical.is_empty() {
        "::".to_string()
    } else {
        format!("::{canonical}")
    }
}

/// Canonicalise a possibly-absolute namespace reference (drop leading `::`),
/// relative names are resolved against the current namespace.
fn canon_ns(vm: &Vm, name: &str) -> String {
    if name == "::" {
        return String::new();
    }
    vm.qualify_name(name)
}

fn cmd_namespace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"namespace subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "eval" => ns_eval(vm, rest),
        "current" => ok(tcl_cmd_core::namespace::current(vm)),
        "qualifiers" => ns_text_op(rest, tcl_cmd_core::namespace::qualifiers),
        "tail" => ns_text_op(rest, tcl_cmd_core::namespace::tail),
        // exists/parent/children route through the shared core over `Namespaces`
        // (the VM's String model honours the `NsId` handles via its arena). This
        // also gave `children` its missing `?pattern?` filter and made
        // parent/children on a missing namespace error, both matching tclsh.
        "exists" => ok(tcl_cmd_core::namespace::exists(vm, &first(rest))),
        "parent" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::parent(vm, name.as_deref()) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            }
        }
        "children" => {
            let name = rest.first().map(|v| v.to_str().to_string());
            let pattern = rest.get(1).map(|v| v.to_str().to_string());
            match tcl_cmd_core::namespace::children(vm, name.as_deref(), pattern.as_deref()) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            }
        }
        // `namespace code script` captures the current namespace as a callback
        // command prefix: `::namespace inscope <ns> <script>`.
        "code" => {
            let script = first(rest);
            let ns = display_ns(vm.current_ns());
            ok(Value::list(vec![
                Value::string("::namespace"),
                Value::string("inscope"),
                Value::string(ns),
                Value::string(script),
            ]))
        }
        // `namespace inscope ns script ?arg ...?` runs `script` (with any extra
        // args appended as list elements) in namespace `ns`.
        "inscope" => ns_inscope(vm, rest),
        "which" => {
            // `namespace which ?-command|-variable? name` → the resolved FQN, via
            // the shared `Namespaces` resolution core (flag handling stays here).
            let name = rest
                .last()
                .map(|v| v.to_str().to_string())
                .unwrap_or_default();
            ok(tcl_cmd_core::namespace::which_command(vm, &name))
        }
        "origin" => {
            // `namespace origin command` → the original command's fully-qualified
            // name (following imports). We do not track import provenance, so the
            // resolved qualified name is returned; an unknown command errors.
            let name = first(rest);
            if vm.lookup_command(&name).is_some() {
                ok(Value::string(display_ns(&vm.qualify_name(&name))))
            } else {
                err(format!("invalid command name \"{name}\""))
            }
        }
        "export" => {
            // `namespace export ?-clear? pattern ...` — record export patterns.
            let pats: Vec<String> = rest
                .iter()
                .map(|v| v.to_str().to_string())
                .filter(|p| p != "-clear")
                .collect();
            vm.add_exports(&pats);
            ok(Value::empty())
        }
        "import" => {
            // `namespace import ?-force? pattern ...`
            for p in rest {
                let pat = p.to_str();
                if &*pat == "-force" {
                    continue;
                }
                vm.import_commands(&pat);
            }
            ok(Value::empty())
        }
        // `namespace delete ?ns ...?` — destroy each namespace (and its
        // descendants, commands, and variables). An unknown namespace errors,
        // after deleting any that preceded it (matching tclsh).
        "delete" => {
            for n in rest {
                let canon = canon_ns(vm, &n.to_str());
                if !vm.delete_namespace(&canon) {
                    return err(format!(
                        "unknown namespace \"{}\" in namespace delete command",
                        n.to_str()
                    ));
                }
            }
            ok(Value::empty())
        }
        // Accepted no-ops (metadata only, for now).
        "forget" | "ensemble" | "unknown" => ok(Value::empty()),
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be \
             children, current, eval, exists, export, parent, qualifiers, or tail"
        )),
    }
}

fn first(rest: &[Value]) -> String {
    rest.first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default()
}

/// `namespace qualifiers`/`tail`: run the first argument (lenient — defaults to
/// empty) through a shared `tcl_cmd_core::namespace` text op, as a `Value`. The
/// shared core handles `::`-runs the way C does (the VM's old `rsplit("::")`
/// diverged for 3+ colons, e.g. `tail foo:::`).
fn ns_text_op(rest: &[Value], op: fn(&[u8]) -> &[u8]) -> Completion<Value> {
    let name = first(rest);
    ok(Value::string(
        std::str::from_utf8(op(name.as_bytes())).unwrap_or(""),
    ))
}

fn ns_inscope(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, parts)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"");
    };
    if parts.is_empty() {
        return err("wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"");
    }
    let target = canon_ns(vm, &ns.to_str());
    let body = parts
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut call_argv = vec![Value::string("namespace"), Value::string("inscope")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, target, &body, call_argv)
}

fn ns_eval(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, body_parts)) = rest.split_first() else {
        return err("wrong # args: should be \"namespace eval name arg ?arg ...?\"");
    };
    if body_parts.is_empty() {
        return err("wrong # args: should be \"namespace eval name arg ?arg ...?\"");
    }
    let child = canon_ns(vm, &ns.to_str());
    // Multiple body args are concatenated with spaces, as a script.
    let body = body_parts
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut call_argv = vec![Value::string("namespace"), Value::string("eval")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(vm, child, &body, call_argv)
}
