//! The `namespace` ensemble.
//!
//! `namespace eval ns body` runs `body` with the current namespace set to `ns`
//! so that `proc`/command/variable name resolution qualifies relative to it
//! (see [`Vm::qualify_name`]/[`Vm::lookup_command`]). The introspection
//! subcommands (`current`, `qualifiers`, `tail`, `parent`, `children`,
//! `exists`) operate on canonical names; `export`/`import` are accepted as
//! no-ops for now (the codegen already records export/import metadata).

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

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
        "current" => ok(Value::string(display_ns(vm.current_ns()))),
        "qualifiers" => ok(Value::string(qualifiers(&first(rest)))),
        "tail" => ok(Value::string(tail(&first(rest)))),
        "parent" => {
            let target = if rest.is_empty() {
                vm.current_ns().to_string()
            } else {
                canon_ns(vm, &rest[0].to_str())
            };
            let parent = target.rsplit_once("::").map_or("", |(p, _)| p);
            ok(Value::string(display_ns(parent)))
        }
        "children" => {
            let parent = if rest.is_empty() {
                vm.current_ns().to_string()
            } else {
                canon_ns(vm, &rest[0].to_str())
            };
            let mut kids: Vec<String> = vm
                .child_namespaces(&parent)
                .iter()
                .map(|c| display_ns(c))
                .collect();
            kids.sort();
            ok(Value::list(kids.into_iter().map(Value::string).collect()))
        }
        "exists" => {
            let ns = canon_ns(vm, &first(rest));
            ok(Value::bool(vm.namespace_exists(&ns)))
        }
        // Accepted no-ops (metadata only, for now).
        "export" | "import" | "forget" | "delete" | "ensemble" | "unknown" => ok(Value::empty()),
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

/// Everything before the last `::` of a (possibly absolute) name, preserving an
/// absolute leading `::`.
fn qualifiers(name: &str) -> String {
    match name.rsplit_once("::") {
        Some((q, _)) => q.to_string(),
        None => String::new(),
    }
}

/// The last `::`-separated component of a name.
fn tail(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
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
    vm.declare_namespace(&child);
    vm.push_ns(child);
    let result = vm.eval_source(&body);
    vm.pop_ns();
    match result {
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

#[cfg(test)]
mod tests {
    use super::{qualifiers, tail};

    #[test]
    fn name_splitting() {
        assert_eq!(qualifiers("foo::bar::baz"), "foo::bar");
        assert_eq!(qualifiers("bar"), "");
        assert_eq!(tail("foo::bar::baz"), "baz");
        assert_eq!(tail("bar"), "bar");
    }
}
