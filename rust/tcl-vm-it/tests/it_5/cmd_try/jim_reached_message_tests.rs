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

//! Jim `try` handler selection and binding against the jimsh capture.

use crate::prelude::*;

const ORIGINAL: &str = include_str!(
    "../../../../../experiments/name-handling/results/jim-try-reached-message-getters/capture07/execution.stdout"
);

fn observed(mode: usize) -> Vec<&'static str> {
    ORIGINAL
        .lines()
        .find(|row| row.starts_with(&format!("OBS|{mode}|")))
        .unwrap()
        .split('|')
        .collect()
}

fn word(value: &str) -> Value {
    Value::string(value)
}

#[test]
fn original_jim_unselected_script_and_later_trap_keep_reached_integer_getters_lazy() {
    for mode in [11, 12, 13] {
        let mut vm = crate::fixture::distribution(tcl_vm::test_support::profile("jim"));
        crate::fixture::eval(
            &mut vm,
            "proc body_error {} {return -code error -errorcode 777 BODY}",
        );
        let integer = Value::int(777);
        let variables = || Value::list(vec![word("message"), word("options")]);
        let args = match mode {
            11 => vec![
                word("body_error"),
                word("on"),
                word("error"),
                variables(),
                word("set ::handled $::message"),
                word("on"),
                word("error"),
                Value::list(vec![word("other"), word("options2")]),
                integer,
            ],
            12 => vec![
                word("body_error"),
                word("trap"),
                Value::list(vec![integer, word("888")]),
                variables(),
                word("set ::handled $::message"),
            ],
            13 => vec![
                word("body_error"),
                word("on"),
                word("error"),
                variables(),
                word("set ::handled $::message"),
                word("trap"),
                Value::list(vec![integer]),
                Value::list(vec![word("other"), word("options2")]),
                word("set ::handled WRONG"),
            ],
            _ => unreachable!(),
        };
        let completion = crate::fixture::call(&mut vm, "try", &args);
        let row = observed(mode);
        assert_eq!(completion.code.as_int(), row[2].parse::<i64>().unwrap(), "{mode}");
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            crate::fixture::unhex(row[4]),
            "{mode}"
        );
        for (name, expected) in [
            (b"message".as_slice(), row[5]),
            (b"handled".as_slice(), row[6]),
            (b"other".as_slice(), row[7]),
        ] {
            let value = vm.get_var_bytes(name);
            assert_eq!(value.is_some(), expected != "-", "{mode}");
            if let Some(value) = value {
                assert_eq!(
                    vm.native_string_bytes(&value).unwrap().as_ref(),
                    crate::fixture::unhex(expected),
                    "{mode}"
                );
            }
        }
    }
}
