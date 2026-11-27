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

//! `catch` and `info exists` compile in every position of a procedure body,
//! so the body's local-variable table holds the slots C allocates (#2207).
//!
//! Expected slots are `tcl::unsupported::disassemble proc` on tclsh 8.5.19,
//! 8.6.18, 9.0.4 and 9.1.0. Tcl 8.4.20 compiles `catch` (`TclCompileCatchCmd`
//! allocates the result variable before compiling the body) but has no
//! `info exists` compiler, so `pub` gets no slot there.

use crate::prelude::*;
use tcl_bytecode::Op;

struct Case {
    name: &'static str,
    params: &'static str,
    body: &'static str,
    slots: &'static [&'static str],
    /// The slots Tcl 8.4.20 allocates, where they differ.
    slots_8_4: Option<&'static [&'static str]>,
    op: Op,
}

const CASES: &[Case] = &[
    Case {
        name: "catch_statement",
        params: "x",
        body: " catch {set q $x} ",
        slots: &["x", "q"],
        slots_8_4: None,
        op: Op::BEGIN_CATCH4,
    },
    Case {
        name: "catch_assigned",
        params: "x",
        body: " set rc [catch {set q $x}] ",
        slots: &["x", "rc", "q"],
        slots_8_4: None,
        op: Op::BEGIN_CATCH4,
    },
    Case {
        name: "catch_result_variable",
        params: "x",
        body: " catch {set q $x} m ",
        slots: &["x", "m", "q"],
        slots_8_4: None,
        op: Op::BEGIN_CATCH4,
    },
    Case {
        name: "catch_condition",
        params: "x",
        body: " if {[catch {set q $x}]} {} ",
        slots: &["x", "q"],
        slots_8_4: None,
        op: Op::BEGIN_CATCH4,
    },
    Case {
        name: "info_exists_statement",
        params: "",
        body: " info exists pub ",
        slots: &["pub"],
        slots_8_4: Some(&[]),
        op: Op::EXIST_SCALAR,
    },
    Case {
        name: "info_exists_assigned",
        params: "",
        body: " set q [info exists pub] ",
        slots: &["q", "pub"],
        slots_8_4: Some(&["q"]),
        op: Op::EXIST_SCALAR,
    },
    Case {
        name: "info_exists_argument",
        params: "",
        body: " list [info exists pub] ",
        slots: &["pub"],
        slots_8_4: Some(&[]),
        op: Op::EXIST_SCALAR,
    },
    Case {
        name: "info_exists_condition",
        params: "",
        body: " if {[info exists pub]} {return 1}; return 0 ",
        slots: &["pub"],
        slots_8_4: Some(&[]),
        op: Op::EXIST_SCALAR,
    },
];

fn compiled_body(profile: &str, case: &Case) -> Rc<tcl_bytecode::FunctionAsm> {
    Rc::new(crate::fixture::procedure_body(
        tcl_dialect::DialectProfile::find(profile).expect("release profile"),
        case.name,
        case.params,
        case.body,
    ))
}

fn slot_names(asm: &tcl_bytecode::FunctionAsm) -> Vec<String> {
    asm.lvt
        .native_slot_names()
        .into_iter()
        .map(|name| String::from_utf8_lossy(name.expect("named slot").as_bytes()).into_owned())
        .collect()
}

#[test]
fn catch_and_info_exists_allocate_c_slots_in_every_position() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in CASES {
            let asm = compiled_body(profile, case);
            assert_eq!(slot_names(&asm), case.slots, "{profile}/{}", case.name);
            assert!(
                asm.instructions
                    .iter()
                    .any(|instruction| instruction.op == case.op),
                "{profile}/{}: compiled inline",
                case.name
            );
        }
    }
}

#[test]
fn tcl_8_4_allocates_catch_slots_but_not_info_exists_slots() {
    for case in CASES {
        let asm = compiled_body("tcl8.4", case);
        let expected = case.slots_8_4.unwrap_or(case.slots);
        assert_eq!(slot_names(&asm), expected, "tcl8.4/{}", case.name);
        assert_eq!(
            asm.instructions
                .iter()
                .any(|instruction| instruction.op == case.op),
            case.op == Op::BEGIN_CATCH4,
            "tcl8.4/{}",
            case.name
        );
    }
}
