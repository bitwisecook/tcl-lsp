//! `regexp` / `regsub` — a thin adapter over the shared
//! [`tcl_cmd_core::regex`] plumbing, driven by the pure-Rust Tcl 9 ARE engine
//! ([`tcl_regex`]).
//!
//! The command logic (option parsing, the match/advance loop, `-indices`/
//! `-inline`/`-start`/`-all`, submatch assignment, the `regsub` spec expansion)
//! is shared with `runtime/rust`; only the **engine** is provided here. The VM
//! used to drive the approximate `regex` crate (no full ARE syntax — `\m`/`\M`/
//! `[[:<:]]` word edges, POSIX longest-match submatches, etc.); it now uses the
//! faithful [`tcl_regex`] engine, so the VM matches `tclsh` 9.0 behaviour.

use tcl_cmd_core::regex::{self as core_re, RegexEngine, RegexFlags, RegexpResult, RegsubResult};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// The ARE engine as the shared plumbing's provider. Reused by `lsearch
/// -regexp` (`cmd_list`) and `switch -regexp` (`cmd_switch`).
pub(crate) use tcl_regex::cmd_core::AreEngine as CrateEngine;

/// Does `pattern` match anywhere in `subject` (ARE, optional `-nocase`)? A small
/// boolean helper for the bytecode `MatchesRegex`-style opcode in `exec`.
pub(crate) fn regexp_matches(pattern: &str, subject: &str, nocase: bool) -> Result<bool, String> {
    let flags = RegexFlags {
        nocase,
        expanded: false,
        linestop: false,
        lineanchor: false,
    };
    let mut re = CrateEngine::compile(pattern.as_bytes(), flags)
        .map_err(|e| String::from_utf8_lossy(&e).into_owned())?;
    let cps: Vec<i32> = subject.chars().map(|c| c as i32).collect();
    Ok(CrateEngine::exec(&mut re, &cps, 0, false).is_some())
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("regexp", cmd_regexp);
    vm.register("regsub", cmd_regsub);
}

fn cmd_regexp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
    match core_re::regexp::<Vm, CrateEngine>(vm, &refs) {
        Ok(RegexpResult::Inline(v)) => ok(v),
        Ok(RegexpResult::Count { assign, count }) => {
            if let Some(pairs) = assign {
                for (name, val) in pairs {
                    if let Err(c) = vm.var_set(&String::from_utf8_lossy(&name), val) {
                        return c;
                    }
                }
            }
            ok(Value::int(count))
        }
        Err(e) => err(String::from_utf8_lossy(&e.0).into_owned()),
    }
}

fn cmd_regsub(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
    let RegsubResult { text, count, var } = match core_re::regsub::<CrateEngine>(&refs) {
        Ok(r) => r,
        Err(e) => return err(String::from_utf8_lossy(&e.0).into_owned()),
    };
    let result = Value::string(String::from_utf8_lossy(&text).into_owned());
    match var {
        Some(name) => {
            if let Err(c) = vm.var_set(&String::from_utf8_lossy(&name), result) {
                return c;
            }
            ok(Value::int(count))
        }
        None => ok(result),
    }
}
