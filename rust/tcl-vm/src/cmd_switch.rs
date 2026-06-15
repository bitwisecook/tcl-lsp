//! The `switch` builtin — the runtime form the codegen invokes for
//! `-glob`/`-regexp`/`-nocase` (exact switches use the `JUMP_TABLE` opcode).
//! M3: exact/glob/nocase; regexp falls back to exact (no engine yet).

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_case_match;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("switch", cmd_switch);
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Exact,
    Glob,
    Regexp,
}

fn cmd_switch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut mode = Mode::Exact;
    let mut nocase = false;
    let mut rest = args;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-exact" => mode = Mode::Exact,
            "-glob" => mode = Mode::Glob,
            "-regexp" => mode = Mode::Regexp,
            "-nocase" => nocase = true,
            "--" => {
                rest = &rest[1..];
                break;
            }
            s if s.starts_with('-') => {} // ignore unknown options (M3)
            _ => break,
        }
        rest = &rest[1..];
    }
    let Some((value, body_args)) = rest.split_first() else {
        return err("wrong # args: should be \"switch ?options? string ?pattern body ...?\"");
    };
    let v = value.to_str();

    // Pattern/body pairs: either a single brace-list arg or inline args.
    let pairs: Vec<(String, String)> = if body_args.len() == 1 {
        let items = match body_args[0].as_list() {
            Ok(i) => i,
            Err(e) => return err(e.message),
        };
        if items.len() % 2 != 0 {
            return err("extra switch pattern with no body");
        }
        items
            .chunks_exact(2)
            .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
            .collect()
    } else {
        if body_args.len() % 2 != 0 {
            return err("extra switch pattern with no body");
        }
        body_args
            .chunks_exact(2)
            .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
            .collect()
    };

    let last = pairs.len().saturating_sub(1);
    let matched = pairs.iter().position(|(pat, _)| {
        if pat == "default" {
            return true; // `default` matches anything (conventionally last)
        }
        let _ = last;
        match mode {
            Mode::Glob => string_case_match(pat, &v, nocase),
            Mode::Exact | Mode::Regexp if nocase => pat.eq_ignore_ascii_case(&v),
            Mode::Exact | Mode::Regexp => *pat == *v,
        }
    });

    let Some(mut idx) = matched else {
        return ok(Value::empty());
    };
    // Fall-through: a body of "-" reuses the next pattern's body.
    while pairs[idx].1 == "-" && idx + 1 < pairs.len() {
        idx += 1;
    }
    match vm.eval_source(&pairs[idx].1) {
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}
