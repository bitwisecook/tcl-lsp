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

//! Jim `catch` and `try` over a quiet `::errorCode`, against the jimsh 0.84
//! captures.

use crate::prelude::*;

fn jim() -> Vm {
    crate::fixture::distribution(tcl_vm::test_support::profile("jim"))
}

#[test]
fn catch_try_quiet_global_capture_matches_original_reached_message_and_options() {
    const CAPTURE: &str = include_str!(
        "../../../../../../experiments/name-handling/results/jim-catch-try-quiet-global-capture/capture03/execution.stdout"
    );
    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    for (input, observed) in CAPTURE
        .lines()
        .collect::<Vec<_>>()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|rows| (rows[0], rows[1]))
    {
        let input = input.split('|').collect::<Vec<_>>();
        let observed = observed.split('|').collect::<Vec<_>>();
        let mut vm = jim();
        assert_eq!(
            vm.try_eval_source_bytes(b"set keep SAVED; set handler NONE; set finally NONE")
                .unwrap()
                .code,
            Code::Ok
        );
        let completion = vm.try_eval_source_bytes(&unhex(input[3])).unwrap();
        assert_eq!(
            completion.code.as_int(),
            observed[3].parse::<i64>().unwrap()
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            unhex(observed[4])
        );
        let message = vm.get_var_bytes(b"message");
        assert_eq!(message.is_some(), observed[5] != "-");
        if let Some(message) = message {
            assert_eq!(crate::fixture::bytes(&vm, &message), unhex(observed[5]));
        }
        let options = vm.get_var_bytes(b"options");
        assert_eq!(options.is_some(), observed[6] != "-");
        let errorcode = options
            .as_ref()
            .and_then(|options| crate::fixture::option(options, "-errorcode"));
        assert_eq!(errorcode.is_some(), observed[8] != "-");
        for (name, expected) in [
            (b"handler".as_slice(), observed[9]),
            (b"finally".as_slice(), observed[10]),
        ] {
            let value = vm.get_var_bytes(name).unwrap();
            assert_eq!(crate::fixture::bytes(&vm, &value), unhex(expected));
        }
    }
}
