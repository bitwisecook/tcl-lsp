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

use std::rc::Rc;

use crate::return_options::NativeReturnOps;
use tcl_cmd_core::prefix::{NativePrefixProtocol, Resolution};
use tcl_cmd_core::return_options::{
    self, PreparedOptionPairs, ReturnOptionPair, ReturnOptionsOps, ReturnOptionsPurpose,
};
use tcl_runtime_api::Completion;
use tcl_syntax::value::ValueOps;

use crate::interp::{Vm, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("tcl::prefix", cmd_prefix);
    vm.register_stock_builtin("::tcl::prefix", cmd_prefix);
}

fn cmd_prefix(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tcl::prefix subcommand ?arg ...?\"",
        );
    };
    let word = match ValueOps::native_string_bytes(vm, sub) {
        Ok(word) => word,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    let Some(selected) = vm
        .name_policy_protocol()
        .and_then(NativePrefixProtocol::from_policy)
    else {
        return vm.refuse_host_command("native prefix protocol is unavailable".into());
    };
    let canon = if selected.policy().recipe().is_jim084() {
        let table: Vec<Rc<[u8]>> = [b"match".as_slice(), b"all", b"longest"]
            .into_iter()
            .map(Rc::from)
            .collect();
        match selected.resolve(&table, &word, false) {
            Resolution::Exact(index) | Resolution::UniquePrefix(index) => {
                ["match", "all", "longest"][index]
            }
            resolution => {
                return crate::command::completion_from_cmd_error(
                    vm,
                    selected.miss_error(
                        &table,
                        b"option",
                        &word,
                        resolution == Resolution::Ambiguous,
                    ),
                );
            }
        }
    } else {
        match tcl_cmd_core::ensemble::resolve_subcommand(PREFIX_SUBS, &word, true) {
            Some(index) => PREFIX_SUBS[index],
            None => {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::new_bytes(
                        tcl_cmd_core::ensemble::unknown_subcommand_message(
                            PREFIX_SUBS,
                            &word,
                            true,
                            b"::tcl::prefix",
                        ),
                    ),
                );
            }
        }
    };
    match canon {
        "all" => prefix_all(vm, rest),
        "longest" => prefix_longest(vm, rest),
        _ => prefix_match(vm, rest),
    }
}

/// `tcl::prefix`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
const PREFIX_SUBS: &[&str] = &["all", "longest", "match"];

/// `tcl::prefix all table string` — every table entry with `string` as a prefix.
fn prefix_all(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tcl::prefix all table string\"",
        );
    };
    match tcl_cmd_core::prefix::native_all(vm, table, s) {
        Ok(value) => ok(value),
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

/// `tcl::prefix longest table string` — the longest common prefix of the table
/// entries that have `string` as a prefix (empty when none match).
fn prefix_longest(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tcl::prefix longest table string\"",
        );
    };
    match tcl_cmd_core::prefix::native_longest(vm, table, s) {
        Ok(value) => ok(value),
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

/// `tcl::prefix match ?-exact? ?-message s? ?-error opts? table string`.
fn prefix_match(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match prepare_match(vm, rest) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

fn prepare_match(vm: &mut Vm, rest: &[Value]) -> Result<Completion<Value>, tcl_cmd_core::CmdError> {
    if rest.len() < 2 {
        return Ok(crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tcl::prefix match ?options? table string\"",
        ));
    }
    let selected = vm
        .name_policy_protocol()
        .and_then(NativePrefixProtocol::from_policy)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native prefix",
        ))?;
    let jim = selected.policy().recipe().is_jim084();
    let (mut return_ops, return_protocol) = NativeReturnOps::selected(vm)?;
    let options: Vec<Rc<[u8]>> = [b"-error".as_slice(), b"-exact", b"-message"]
        .into_iter()
        .map(Rc::from)
        .collect();
    let (opts, tail) = rest.split_at(rest.len() - 2);
    let table_value = &tail[0];
    let key_value = &tail[1];
    let mut exact = false;
    let mut noun: Rc<[u8]> = Rc::from(b"option".as_slice());
    let mut error_options = None;
    let mut cursor = 0;
    while cursor < opts.len() {
        let key = ValueOps::native_string_bytes(vm, &opts[cursor])?;
        let index = match selected.resolve(&options, &key, false) {
            Resolution::Exact(index) | Resolution::UniquePrefix(index) => index,
            resolution => {
                return Err(selected.miss_error(
                    &options,
                    b"option",
                    &key,
                    resolution == Resolution::Ambiguous,
                ));
            }
        };
        if index == 1 {
            exact = true;
            cursor += 1;
            continue;
        }
        let value = opts
            .get(cursor + 1)
            .ok_or_else(|| selected.missing_value(index == 0))?;
        if index == 0 {
            error_options = Some(return_options::prepare_prefix_error_options(
                &mut return_ops,
                return_protocol,
                value,
            )?);
        } else {
            noun = ValueOps::native_string_bytes(vm, value)?;
        }
        cursor += 2;
    }
    let table = ValueOps::list_elements(vm, table_value)?;
    let matched = tcl_cmd_core::prefix::native_table_match(vm, &table, key_value, exact)?;
    match matched.resolution {
        Resolution::Exact(index) | Resolution::UniquePrefix(index) => {
            return Ok(ok(table[index].clone()));
        }
        _ => {}
    }
    let key =
        matched
            .key
            .as_deref()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native prefix miss key",
            ))?;
    let ambiguous = matched.resolution == Resolution::Ambiguous;
    let miss = selected.miss_error(&matched.entries, &noun, key, ambiguous);
    // Jim's empty-table failure is returned before its -error branch.
    let Some(options) = error_options.filter(|_| !jim || !table.is_empty()) else {
        return Err(miss);
    };
    if options.suppresses_error {
        return Ok(ok(Value::empty()));
    }
    let message = Value::new_native_string_bytes(selected.miss_message(
        &matched.entries,
        &noun,
        key,
        ambiguous,
    ));
    let prepared = if jim {
        let mut argv = vec![
            Value::string("-level"),
            Value::int(0),
            Value::string("-code"),
            Value::string("error"),
        ];
        argv.extend(return_ops.list(&options.original)?);
        argv.push(message);
        return_options::prepare_return(
            &mut return_ops,
            return_protocol,
            &argv,
            ReturnOptionsPurpose::User,
        )?
    } else {
        let mut pairs = Vec::new();
        for (key, value) in
            options
                .c_pairs
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native prefix options receipt",
                ))?
        {
            let key_bytes = return_ops.bytes(&key)?;
            pairs.push(ReturnOptionPair {
                key,
                value,
                key_bytes,
            });
        }
        pairs.push(ReturnOptionPair {
            key: Value::string("-code"),
            value: Value::int(1),
            key_bytes: b"-code".to_vec(),
        });
        return_options::prepare_return_pairs(
            &mut return_ops,
            return_protocol,
            PreparedOptionPairs { pairs },
            Some(message),
            ReturnOptionsPurpose::InternalDictionary,
        )?
    };
    Ok(crate::return_options::publish(
        vm,
        &mut return_ops,
        prepared,
    ))
}

#[cfg(test)]
mod native_object_tests {
    use super::*;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot;

    #[test]
    fn temporary_c86_table_retires_only_a_nonidentity_key_cache() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let key = Value::int(5);
        let table = Value::list(vec![Value::string("5")]);
        let result = prefix_match(&mut vm, &[table, key.clone()]);
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert!(matches!(
            key.native_object_snapshot().cache,
            NativeObjectCacheSnapshot::None
        ));
        assert_eq!(
            key.resident_string_bytes().as_deref(),
            Some(b"5".as_slice())
        );

        let original = Value::int(5);
        let table = Value::list(vec![original.clone()]);
        let result = prefix_match(&mut vm, &[table, original.clone()]);
        assert!(result.result.is_same_object(&original));
        assert!(original.resident_string_bytes().is_none());
        assert!(matches!(
            original.native_object_snapshot().cache,
            NativeObjectCacheSnapshot::Numeric(_)
        ));
    }

    #[test]
    fn error_options_are_checked_before_a_successful_match() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let completion = prefix_match(
            &mut vm,
            &[
                Value::string("-error"),
                Value::string("-x"),
                Value::list(vec![Value::string("a")]),
                Value::string("a"),
            ],
        );
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"error options must have an even number of elements"
        );
    }
}
