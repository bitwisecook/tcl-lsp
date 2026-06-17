//! `string is class ?-strict? ?-failindex var? str` — the per-runtime option
//! wrapper over the shared `tcl_cmd_core::string_is` classifier. The
//! classification logic (character/value classes, fail-index semantics) lives in
//! the core; this parses the options and writes the `-failindex` variable.

use tcl_cmd_core::string_is::{class_check, resolve_class};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// `string is …`. `rest` is everything after the `is` subcommand.
pub(crate) fn string_is(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const WRONG: &str =
        "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"";
    if rest.len() < 2 {
        return err(WRONG);
    }
    let class = match resolve_class(&rest[0].to_str()) {
        Ok(c) => c,
        Err(e) => return err(e.into_message()),
    };
    let mut strict = false;
    let mut fail_var: Option<Value> = None;
    let mut i = 1;
    while i < rest.len() - 1 {
        let opt = rest[i].to_str();
        if "-strict".starts_with(&*opt) && opt.len() >= 2 {
            strict = true;
            i += 1;
        } else if "-failindex".starts_with(&*opt) && opt.len() >= 2 {
            let Some(v) = rest.get(i + 1) else {
                return err(format!(
                    "wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""
                ));
            };
            fail_var = Some(v.clone());
            i += 2;
        } else {
            break;
        }
    }
    if rest.len() - i > 1 {
        // Too many arguments — Tcl uses the generic "class" form here.
        return err(WRONG);
    }
    if rest.len() - i < 1 {
        return err(format!(
            "wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""
        ));
    }
    let s = rest[i].to_str();
    let (member, fail) = class_check(class, &s, strict);
    if !member
        && let Some(var) = fail_var
        && let Err(e) = vm.set_var(&var.to_str(), Value::int(fail))
    {
        return e;
    }
    ok(Value::bool(member))
}
