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

//! `tcl::prefix` — match a string against a table of valid prefixes.
//!
//! Implements the `match` / `all` / `longest` subcommands the Tcl ensemble
//! dispatch (and a few stdlib helpers) rely on. The `match` error messages and
//! `-message` / `-error` / `-exact` options mirror `tclIndexObj.c`.

use tcl_runtime_api::{Code, Completion};
use tcl_syntax::list::split_list;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("tcl::prefix", cmd_prefix);
    vm.register("::tcl::prefix", cmd_prefix);
}

fn cmd_prefix(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"tcl::prefix subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "all" => prefix_all(rest),
        "longest" => prefix_longest(rest),
        "match" => prefix_match(vm, rest),
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be all, longest, or match"
        )),
    }
}

/// Split a list value, surfacing a parse error as a completion.
fn entries(v: &Value) -> Result<Vec<String>, Completion<Value>> {
    split_list(&v.to_str())
        .map(|e| e.iter().map(ToString::to_string).collect())
        .map_err(|e| err(e.message().to_string()))
}

/// `tcl::prefix all table string` — every table entry with `string` as a prefix.
fn prefix_all(rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return err("wrong # args: should be \"tcl::prefix all table string\"");
    };
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = s.to_str();
    let out: Vec<Value> = table
        .into_iter()
        .filter(|e| e.starts_with(&*s))
        .map(Value::string)
        .collect();
    ok(Value::list(out))
}

/// `tcl::prefix longest table string` — the longest common prefix of the table
/// entries that have `string` as a prefix (empty when none match).
fn prefix_longest(rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return err("wrong # args: should be \"tcl::prefix longest table string\"");
    };
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = s.to_str();
    let matches: Vec<&String> = table.iter().filter(|e| e.starts_with(&*s)).collect();
    let Some((first, others)) = matches.split_first() else {
        return ok(Value::empty());
    };
    // Longest common prefix of all matching entries.
    let mut len = first.chars().count();
    for e in others {
        let common = first
            .chars()
            .zip(e.chars())
            .take_while(|(a, b)| a == b)
            .count();
        len = len.min(common);
    }
    ok(Value::string(first.chars().take(len).collect::<String>()))
}

/// `tcl::prefix match ?-exact? ?-message s? ?-error opts? table string`.
fn prefix_match(_vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let mut exact = false;
    let mut message = "option".to_string();
    let mut error_opts: Option<Value> = None;
    // The trailing two words are always `table string`; everything before them
    // is the option region (C parses `objv[2 .. objc-2]`). So a non-option there
    // is `bad option` (not `wrong # args`), and a `-message`/`-error` with no
    // following word *within* the region is `missing value` (string-26.x).
    if rest.len() < 2 {
        return err("wrong # args: should be \"tcl::prefix match ?options? table string\"");
    }
    let (opts, tail) = rest.split_at(rest.len() - 2);
    let [table, sv] = tail else {
        return err("wrong # args: should be \"tcl::prefix match ?options? table string\"");
    };
    let mut i = 0;
    while i < opts.len() {
        match &*opts[i].to_str() {
            "-exact" => {
                exact = true;
                i += 1;
            }
            "-message" => {
                let Some(v) = opts.get(i + 1) else {
                    return err("missing value for -message");
                };
                message = v.to_str().to_string();
                i += 2;
            }
            "-error" => {
                let Some(v) = opts.get(i + 1) else {
                    return err("missing value for -error");
                };
                error_opts = Some(v.clone());
                i += 2;
            }
            other => {
                return err(format!(
                    "bad option \"{other}\": must be -error, -exact, or -message"
                ));
            }
        }
    }
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = sv.to_str();

    // The shared `Tcl_GetIndexFromObjStruct` matcher over the runtime
    // `String` table — `TclPrefixMatchObjCmd` passes the caller's table,
    // `-message` noun, and `-exact` (as `TCL_EXACT`) straight through. An
    // exact entry always wins; otherwise a unique prefix (unless `-exact`);
    // the miss carries C's exact bad/ambiguous wording (including the
    // empty-string-never-matches rule, where the old local matcher wrongly
    // resolved `""` against a one-entry table).
    let options = if exact {
        tcl_cmd_core::prefix::OptionTable::exact_only(&message, &table)
    } else {
        tcl_cmd_core::prefix::OptionTable::abbreviating(&message, &table)
    };
    let msg = match options.index_of(s.as_bytes()) {
        Ok(i) => return ok(Value::string(table[i].clone())),
        Err(m) => String::from_utf8_lossy(&m).into_owned(),
    };
    match error_opts {
        // No `-error`: a normal error.
        None => err(msg),
        Some(opts) => match opts.as_list() {
            // The `-error` value must be a proper, even-length list (a
            // return-options dict): a malformed or odd one is reported as such
            // (string-26.3).
            Err(e) => err(e.message),
            Ok(list) if list.is_empty() => ok(Value::empty()),
            Ok(list) if list.len() % 2 != 0 => {
                err("error options must have an even number of elements")
            }
            // `-error <opts>`: report the message with the caller's return
            // options attached (the trampoline applies `-code`/`-level`).
            Ok(_) => Completion::new(Code::Error, Value::string(msg), opts),
        },
    }
}
