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

//! `try_eval_expr_bool_bytes` reports the expression's own completion and truth.

use crate::prelude::*;

const ENGINES: [&str; 6] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];

fn interpreter(engine: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(engine).analyser_profile();
    crate::fixture::interpreter(profile)
}

#[test]
fn original_expression_condition_retains_successful_result_and_truth() {
    for engine in ENGINES {
        let mut vm = interpreter(engine);
        for (source, truth, result) in [
            (b"0".as_slice(), false, b"0".as_slice()),
            (b"1", true, b"1"),
            (b"2 + 3", true, b"5"),
        ] {
            let (completion, actual) = vm.try_eval_expr_bool_bytes(source).unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}/{source:?}");
            assert_eq!(actual, Some(truth), "{engine}/{source:?}");
            assert_eq!(completion.result.string_bytes().as_ref(), result);
        }
    }
}

#[test]
fn original_expression_condition_preserves_full_abrupt_completion() {
    for engine in ENGINES {
        for source in [
            b"1 +".as_slice(),
            b"[return -code break -level 0 PAYLOAD]",
            b"[return -code 7 -level 0 PAYLOAD]",
        ] {
            let mut reference = interpreter(engine);
            let expected = reference.try_eval_expr_bytes(source).unwrap();
            assert_ne!(expected.code, Code::Ok, "{engine}/{source:?}");
            let mut vm = interpreter(engine);
            let (completion, truth) = vm.try_eval_expr_bool_bytes(source).unwrap();
            assert_eq!(truth, None, "{engine}/{source:?}");
            assert_eq!(completion.code, expected.code, "{engine}/{source:?}");
            assert_eq!(completion.option_origin, expected.option_origin);
            assert_eq!(
                completion.result.string_bytes(),
                expected.result.string_bytes()
            );
            assert_eq!(
                completion.options.string_bytes(),
                expected.options.string_bytes()
            );
        }
    }
}

#[test]
fn original_expression_condition_boolean_failure_is_guest_completion() {
    for engine in ENGINES {
        let mut vm = interpreter(engine);
        let (completion, truth) = vm.try_eval_expr_bool_bytes(b"{NOT_A_BOOLEAN}").unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
        assert_eq!(truth, None);
        assert_ne!(completion.result.string_bytes().len(), 0);
    }
}
