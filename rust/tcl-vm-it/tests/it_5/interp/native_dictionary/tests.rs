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

//! `dict update` of a missing key over a linked variable, with stores confined or not.

use crate::prelude::*;

fn confine(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    vm.set_stores_confined(true);
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

fn release(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

#[test]
fn confined_compiled_dictionary_missing_key_checks_linked_slot_before_unset() {
    // tclsh 8.6.18 and 9.0.4 give `0 0 1` unconfined: `dict update` of a
    // missing key unsets the linked variable once, then the local.
    let script = r"set ::seed old
        set ::unsets 0
        trace add variable ::seed unset {apply {args {incr ::unsets}}}
        proc p {} {
            set d {}; upvar #0 ::seed alias; set local old
            confine
            set update [catch {dict update d k alias {}} message]
            dict update d k local {}
            list $update $message [info exists local] [info exists ::seed] $::unsets
        }
        p";
    for confined in [false, true] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
        vm.register("confine", if confined { confine } else { release });
        let completion = crate::fixture::eval(&mut vm, script);
        assert_eq!(completion.code, Code::Ok, "{:?}", completion.result);
        let fields = completion.result.as_list().unwrap();
        let field = |index: usize| fields[index].to_str().to_string();
        if confined {
            assert_eq!(field(0), "1");
            assert_eq!(
                field(1),
                "can't unset \"alias\": stores are confined to the activation"
            );
            assert_eq!(field(3), "1");
            assert_eq!(field(4), "0");
        } else {
            assert_eq!(field(0), "0");
            assert_eq!(
                (field(2), field(3), field(4)),
                ("0".into(), "0".into(), "1".into())
            );
        }
    }
}
