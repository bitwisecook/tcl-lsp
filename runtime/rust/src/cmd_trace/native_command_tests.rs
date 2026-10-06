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

//! Original command/execution trace operands and native callback byte extents.

use super::*;
use crate::obj::{self, Owned};

fn interpreter(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn command_trace_reaches_original_prefix_before_a_refused_name() {
    for is_add in [false, true] {
        let mut interp = interpreter("tcl9.1");
        assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
        let name = Owned::retain(interp.result_obj());
        obj::invalidate_string(name.as_ptr());
        let member = Owned::fresh(obj::new_string_bytes(b"callback"));
        let prefix = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
        assert!(!obj::has_string_rep(prefix.as_ptr()));
        let operation = if is_add {
            b"add".as_slice()
        } else {
            b"remove".as_slice()
        };
        let words = [b"trace".as_slice(), operation, b"command", b"rename"]
            .map(|word| Owned::fresh(obj::new_string_bytes(word)));
        let argv = [
            words[0].as_ptr(),
            words[1].as_ptr(),
            words[2].as_ptr(),
            name.as_ptr(),
            words[3].as_ptr(),
            prefix.as_ptr(),
        ];
        assert_eq!(
            cmd_trace_add_remove(&mut interp, &argv, is_add, ops::CMD_ANY),
            Code::Error
        );
        assert!(interp.host_refusal_pending());
        assert!(
            obj::has_string_rep(prefix.as_ptr()),
            "prefix getter was reached first"
        );
        assert!(!obj::has_string_rep(name.as_ptr()));
        assert!(interp.traces.borrow().cmd_traces.is_empty());
    }
}

#[test]
fn command_trace_info_refuses_a_stringless_original_name() {
    let mut interp = interpreter("tcl9.1");
    assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
    let name = Owned::retain(interp.result_obj());
    obj::invalidate_string(name.as_ptr());
    let words = [b"trace".as_slice(), b"info", b"command"]
        .map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let argv = [
        words[0].as_ptr(),
        words[1].as_ptr(),
        words[2].as_ptr(),
        name.as_ptr(),
    ];
    assert_eq!(
        cmd_trace_info(&mut interp, &argv, ops::CMD_ANY),
        Code::Error
    );
    assert!(interp.host_refusal_pending());
    assert!(!obj::has_string_rep(name.as_ptr()));
}

#[test]
fn original_command_traces_match_ten_native_byte_windows() {
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for row in include_str!("../../tests/data/native_command_traces/windows.tsv").lines() {
        let fields = row.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(&format!("tcl{}", fields[0]));
        let source = decode(fields[2]);
        assert_eq!(
            interp.eval_str(&source).as_int(),
            fields[3].parse::<i64>().unwrap(),
            "{} {}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            interp.result_bytes(),
            decode(fields[4]),
            "{} {}",
            fields[0],
            fields[1]
        );
        assert!(!interp.host_refusal_pending());
        compared += 1;
    }
    assert_eq!(compared, 10);
}
