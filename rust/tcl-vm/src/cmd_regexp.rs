// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `regexp` / `regsub` — a thin adapter over the shared
//! [`tcl_cmd_core::regex`] plumbing, driven by the pure-Rust Tcl 9 ARE engine
//! ([`tcl_regex`]).
//!
//! The command logic (option parsing, the match/advance loop, `-indices`/
//! `-inline`/`-start`/`-all`, submatch assignment, the `regsub` spec expansion)
//! is shared with `runtime/rust`; only the **engine** is provided here. The
//! approximate `regex` crate has no full ARE syntax — `\m`/`\M`/
//! `[[:<:]]` word edges, POSIX longest-match submatches, etc. — so the VM
//! uses the faithful [`tcl_regex`] engine instead, matching `tclsh` 9.0
//! behaviour.

use tcl_cmd_core::regex::{
    self as core_re, RegexEngine, RegexFlags, RegexpResult, RegsubError, RegsubResult,
};
use tcl_runtime_api::{Code, Commands, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// The `errorInfo` frame C appends when a `regsub -command` prefix fails
/// (`Tcl_RegsubObjCmd`'s `Tcl_AppendObjToErrorInfo`). tclsh 9.0.4 / 9.1b0,
/// `regsub -command {.x.} {abcxdef} error`:
///
/// ```text
/// cxd
///     while executing
/// "error cxd"
///     (-command substitution computation script)
///     invoked from within
/// "regsub -command {.x.} {abcxdef} error"
/// ```
const COMMAND_SUBST_FRAME: &str = "\n    (-command substitution computation script)";

/// The ARE engine as the shared plumbing's provider. Reused by `lsearch
/// -regexp` (`cmd_list`) and `switch -regexp` (`cmd_switch`).
pub(crate) use tcl_regex::cmd_core::AreEngine as CrateEngine;

/// Does `pattern` match anywhere in `subject` (ARE, optional `-nocase`)? A small
/// boolean helper for the bytecode `MatchesRegex`-style opcode in `exec`. A
/// search that established neither answer is an error, never a `false`.
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
    match CrateEngine::exec(&mut re, &cps, 0, false) {
        core_re::RegexpPrecision::Exact { .. } => Ok(true),
        core_re::RegexpPrecision::NoMatch => Ok(false),
        core_re::RegexpPrecision::Declined(decline) => {
            Err(String::from_utf8_lossy(&decline.into_error().0).into_owned())
        }
    }
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
    // The core owns `-command` (option table, prefix split, per-match word
    // list); the VM supplies only the evaluator and the release it is pinned
    // to, which is what decides whether `-command` is an option at all.
    let version = vm.runtime_version();
    let outcome = core_re::regsub_eval::<CrateEngine, Completion<Value>>(&refs, version, |words| {
        regsub_command_call(vm, words)
    });
    let RegsubResult { text, count, var } = match outcome {
        Ok(r) => r,
        Err(RegsubError::Regex(e)) => return err(String::from_utf8_lossy(&e.0).into_owned()),
        Err(RegsubError::Eval(completion)) => {
            // C adds the context frame only for a genuine error; a
            // `break`/`continue`/custom code from the prefix propagates
            // untouched (tclsh 9.0.4: `proc q args {return -code continue}`,
            // `catch {regsub -command {.x.} abcxdef q}` → 4, `::errorInfo`
            // never set).
            if completion.code == Code::Error {
                vm.seed_error_info_frame(&completion.result.to_str(), COMMAND_SUBST_FRAME);
            }
            return completion;
        }
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

/// Evaluate one `regsub -command` substitution: `words` is the whole command —
/// the prefix's own words followed by the matched text and each submatch — and
/// its result is the replacement text. Argv-based (`Vm::dispatch`), so a word
/// containing `$`/`[` is passed literally, exactly as C's `Tcl_EvalObjv` does.
fn regsub_command_call(vm: &mut Vm, words: &[Vec<u8>]) -> Result<Vec<u8>, Completion<Value>> {
    let argv: Vec<Value> = words
        .iter()
        .map(|w| Value::string(String::from_utf8_lossy(w).into_owned()))
        .collect();
    let Some((name, rest)) = argv.split_first() else {
        // Unreachable: the core rejects an empty prefix before the match loop.
        return Err(err("command prefix must be a list of at least one element"));
    };
    let comp = vm.dispatch(&name.to_str(), rest);
    if comp.code.is_ok() {
        Ok(comp.result.to_str().as_bytes().to_vec())
    } else {
        Err(comp)
    }
}
