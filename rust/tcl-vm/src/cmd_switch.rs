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

//! The `switch` builtin. Option parsing and pattern selection (exact/glob/regexp,
//! and Tcl 9.1's `-integer`, incl. `default` and the TIP #75 `-matchvar`/`-indexvar` side-channel) are the
//! shared [`tcl_cmd_core::switch`] core, over `ValueOps` + the `regex`-crate
//! engine. The VM owns the per-target parts: extracting the pattern/body pairs
//! (inline or brace-list), resolving a `-` fall-through, the variable writes, and
//! evaluating the chosen body as a transparent script (`return`/`break`/
//! `continue` propagate).
//!
//! `-regexp` switches match through the engine; exact switches still use the
//! `JUMP_TABLE` opcode, so this runtime form
//! is invoked for `-glob`/`-regexp`/`-integer`/`-nocase`/dynamic cases.

mod native_jim;

use tcl_cmd_core::switch::{self as core_switch, Selection};
use tcl_runtime_api::Completion;
use tcl_runtime_api::completion_options::ControlOptionPolicy;

use crate::cmd_regexp::CrateEngine;
use crate::command::settle_control_options;
use crate::interp::{Vm, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("switch", cmd_switch);
}

fn original_switch_pairs(
    vm: &mut Vm,
    rest: &[Value],
    version: tcl_dialect::TclVersion,
) -> Result<Vec<(Value, Value)>, Completion<Value>> {
    let pairs = if rest.len() == 1 {
        let items = match rest[0].as_list() {
            Ok(i) => i,
            Err(e) => return Err(crate::command::completion_from_tcl_error(vm, e)),
        };
        if items.is_empty() {
            return Err(crate::command::native_wrong_arguments_message(
                vm,
                format!(
                    "wrong # args: should be \"{}\"",
                    core_switch::usage(version, true)
                ),
            ));
        }
        if !items.len().is_multiple_of(2) {
            // The "misplaced comment" heuristic: a pattern beginning with `#`.
            let hint = items.iter().step_by(2).any(|p| p.to_str().starts_with('#'));
            return Err(crate::command::completion_from_cmd_error(
                vm,
                core_switch::extra_pattern_error(hint),
            ));
        }
        items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect()
    } else {
        if !rest.len().is_multiple_of(2) {
            return Err(crate::command::completion_from_cmd_error(
                vm,
                core_switch::extra_pattern_error(false),
            ));
        }
        rest.as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect()
    };
    Ok(pairs)
}

fn cmd_switch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_jim_switch_protocol()
    {
        return native_jim::invoke(vm, args, protocol);
    }
    // Options + the `string` index are shared (the VM's argv is name-stripped).
    let version = vm
        .actual_native_invocation_dialect()
        .tcl_version
        .unwrap_or_else(|| vm.runtime_version());
    let opts = match core_switch::parse_options(vm, args, version) {
        Ok(o) => o,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let value = args[opts.value_index].clone();
    let rest = &args[opts.value_index + 1..];

    // Pattern/body pairs: a single trailing argument is the brace-list form.
    let pairs = match original_switch_pairs(vm, rest, version) {
        Ok(pairs) => pairs,
        Err(error) => return error,
    };

    // A trailing `-` fall-through body has nothing to fall through to.
    if let Some((pat, body)) = pairs.last() {
        match core_switch::body_is_fallthrough(vm, body) {
            Ok(true) => {
                let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(vm, pat) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
                return crate::command::completion_from_cmd_error(
                    vm,
                    core_switch::no_body_error(&bytes),
                );
            }
            Ok(false) => {}
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    }

    let patterns: Vec<Value> = pairs.iter().map(|(p, _)| p.clone()).collect();
    let sel = match core_switch::select_original_with_jim::<Vm, CrateEngine, Value, Completion<Value>>(
        vm,
        &opts,
        &value,
        &patterns,
        version,
        crate::cmd_regexp::invoke_jim_regexp,
    ) {
        Ok(s) => s,
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Command(error)) => {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Callback(completion)) => {
            return completion;
        }
    };
    let Selection::Matched { index, writes } = sel else {
        return settle_control_options(ok(Value::empty()), ControlOptionPolicy::FRESH_FORWARDED);
    };
    // TIP #75 `-matchvar`/`-indexvar` writes happen before the body runs.
    for (name, val) in writes {
        if let Err(e) = vm.store_original_named_variable(&name, val) {
            return e;
        }
    }
    // Resolve a `-` fall-through to the next real body (the trailing-`-` check
    // above guarantees one exists).
    let mut b = index;
    loop {
        match core_switch::body_is_fallthrough(vm, &pairs[b].1) {
            Ok(true) => b += 1,
            Ok(false) => break,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    }
    let completion = match vm.eval_original_script_value(
        &pairs[b].1,
        tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
        None,
    ) {
        Ok(c) => c,
        Err(e) => crate::command::completion_from_tcl_error(vm, e),
    };
    settle_control_options(completion, ControlOptionPolicy::FRESH_FORWARDED)
}
