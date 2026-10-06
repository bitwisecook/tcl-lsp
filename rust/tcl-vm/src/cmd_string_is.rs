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

//! `string is class ?-strict? ?-failindex var? str` — the per-runtime option
//! wrapper over the shared `tcl_cmd_core::string_is` classifier. The
//! classification logic (character/value classes, fail-index semantics) lives in
//! the core; this parses the options and writes the `-failindex` variable.

use tcl_cmd_core::prefix::OptionTable;
use tcl_cmd_core::string_is::{CLASSES, class_check};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// C's `StringIsCmd` resolves the option words with `Tcl_GetIndexFromObj`
/// (flags 0), so `-s` / `-f` abbreviate but a lone `-` (or the empty word) is
/// *ambiguous*, not bad (verified against tclsh 8.6.16).
const OPTIONS: OptionTable<'static> =
    OptionTable::abbreviating("option", &["-strict", "-failindex"]);

/// `string is …`. `rest` is everything after the `is` subcommand.
pub(crate) fn string_is(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const WRONG: &str =
        "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"";
    if rest.len() < 2 {
        return err(WRONG);
    }
    let class = match vm.native_static_option_index(&rest[0], CLASSES, false, "class") {
        Ok(index) => CLASSES[index],
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let mut strict = false;
    let mut fail_var: Option<Value> = None;
    let mut i = 1;
    while i < rest.len() - 1 {
        // The loop only visits non-final arguments (the last is the string),
        // so an unrecognised token here is in option position: a bad option,
        // not a wrong count (`string is alpha a b` → `bad option "a"`).
        match vm.native_static_option_index(&rest[i], OPTIONS.names(), false, "option") {
            Ok(0) => {
                strict = true;
                i += 1;
            }
            Ok(1) => {
                let Some(v) = rest.get(i + 1) else {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        format!(
                            "wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""
                        ),
                    );
                };
                fail_var = Some(v.clone());
                i += 2;
            }
            Ok(_) => unreachable!("the option table is closed"),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    }
    if rest.len() - i > 1 {
        // Too many arguments — Tcl uses the generic "class" form here.
        return err(WRONG);
    }
    if rest.len() - i < 1 {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!("wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""),
        );
    }
    let s = rest[i].to_str();
    // The emulated release's numeral grammar: the numeric classes inherit
    // every version difference in it (`string is integer 08` is false up to
    // 8.6, true from 9.0).
    let numbers = vm.runtime_version().number_syntax();
    let (member, fail) = class_check(class, &s, strict, numbers);
    if !member
        && let Some(var) = fail_var
        && let Err(e) = vm.set_var(&var.to_str(), Value::int(fail))
    {
        return e;
    }
    ok(Value::bool(member))
}
