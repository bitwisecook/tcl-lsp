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

//! Behaviour tests moved from `tcl-vm`'s `cmd_dict::tests`.

use crate::prelude::*;

#[test]
fn jim_dict_with_compares_original_key_units_and_skips_empty_body_writeback() {
    let profile = tcl_vm::test_support::profile("jim");
    let mut vm = crate::fixture::distribution(profile);
    vm.set_var("raw", Value::new_native_string_bytes(vec![0xff]))
        .unwrap();
    vm.set_var("utf", Value::new_native_string_bytes(vec![0xc3, 0xbf]))
        .unwrap();
    for (source, expected) in [
        (
            "set $raw [list $utf before]; dict with $raw [list set $utf after]; list [dict get [set $raw] $utf] [set $utf]",
            "before after",
        ),
        (
            "set d {k before k final}; set s [dict with d {}]; list $s $d $k",
            "{} {k before k final} final",
        ),
        (
            "set d {d scalar}; set s [dict with d {set d hello}]; list $s $d",
            "hello hello",
        ),
    ] {
        let result = vm.try_eval_source(source).unwrap();
        assert_eq!(result.code, Code::Ok, "{source}: {result:?}");
        assert_eq!(result.result.string_bytes().as_ref(), expected.as_bytes());
    }
}

fn confine(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    vm.set_stores_confined(true);
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

fn release(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

/// Jim's scripted `dict update` wrapper over upvar aliases of a caller's
/// variable and dictionary member, with the stores of the inner procedure
/// confined to its activation or not.
#[test]
fn confined_jim_dictionary_missing_key_keeps_outer_alias_and_dictionary_member() {
    const SCRIPT: &str = r"
proc inner {target unset_target} {
    upvar 1 held alias dictionary dictionary_alias dictionary(k) member_alias
    set local_dictionary {k local}; set d {}; set wrapper_body NOT_ENTERED
    set members [list [set alias] [set member_alias] [info exists member_alias]]
    confine
    set update [catch {dict update d k $target {set wrapper_body REACHED}} m1]
    set after_update [list $wrapper_body [uplevel 1 {set held}] [uplevel 1 {set dictionary}] $d]
    set unset [catch {unset -nocomplain $unset_target} m2]
    set after_unset [uplevel 1 {list [info exists held] [set dictionary]}]
    set path [catch {set path_dictionary {k OLD}; dict set path_dictionary k NEW; set path_member [dict get $path_dictionary k]; dict unset path_dictionary k; list $path_member [dict size $path_dictionary]} m3]
    set locals {}
    foreach local {absent local_dictionary(k) local_dictionary(missing)} {
        lappend locals [catch {unset -nocomplain $local}]
    }
    list $members $update $m1 $after_update $unset $m2 $after_unset $path $m3 $locals $local_dictionary
}
proc outer {target unset_target} {
    set held old; set dictionary {k held}
    inner $target $unset_target
}
";
    for confined in [false, true] {
        for (target, unset_target) in [("alias", "alias"), ("member_alias", "dictionary_alias(k)")]
        {
            let mut vm = crate::fixture::distribution(tcl_vm::test_support::profile("jim"));
            vm.register("confine", if confined { confine } else { release });
            crate::fixture::eval(&mut vm, SCRIPT);
            let result = crate::fixture::eval(&mut vm, &format!("outer {target} {unset_target}"));
            assert_eq!(result.code, Code::Ok, "{target}: {result:?}");
            let fields = result.result.as_list().unwrap();
            let field = |index: usize| fields[index].to_str().to_string();
            assert_eq!(field(0), "old held 1", "{target}");
            assert_eq!(field(7), "0", "{target}");
            assert_eq!(field(8), "NEW 0", "{target}");
            assert_eq!(field(9), "0 0 0", "{target}");
            assert_eq!(field(10), "", "{target}");
            if confined {
                assert_eq!(field(1), "1", "{target}");
                assert!(
                    field(2).contains("stores are confined to the activation"),
                    "{target}"
                );
                assert_eq!(field(3), "REACHED old {k held} {}", "{target}");
                assert_eq!(field(4), "1", "{target}");
                assert!(
                    field(5).contains("stores are confined to the activation"),
                    "{target}"
                );
                assert_eq!(field(6), "1 {k held}", "{target}");
            } else {
                assert_eq!(field(1), "0", "{target}: {}", field(2));
                let expected = if target == "alias" { "k old" } else { "k held" };
                assert_eq!(
                    field(3),
                    format!("REACHED old {{k held}} {{{expected}}}"),
                    "{target}"
                );
                assert_eq!(field(4), "0", "{target}: {}", field(5));
                let expected = if target == "alias" {
                    "0 {k held}"
                } else {
                    "1 {}"
                };
                assert_eq!(field(6), expected, "{target}");
            }
        }
    }
}

#[test]
fn confined_dictionary_update_missing_key_keeps_foreign_target_and_unset_traces() {
    use std::cell::Cell;

    struct ObserveUnset(Rc<Cell<usize>>);

    impl tcl_vm::NativeCommand for ObserveUnset {
        fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
            self.0.set(self.0.get() + 1);
            Completion::new(Code::Ok, Value::empty(), Value::empty())
        }
    }

    const SCRIPT: &str = r#"
proc p {} {
    set d {}
    confine
    set r [list [catch {dict update d k ::seed {}} m] $m [info exists ::seed]]
    foreach target {::missing_ns::x local} {
        if {$target eq "local"} {set local old}
        lappend r [catch {dict update d k $target {}} m] [info exists $target]
    }
    set r
}
"#;
    for confined in [false, true] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
        let callbacks = Rc::new(Cell::new(0));
        vm.register_native_command(
            "observe_missing_key_unset",
            Rc::new(ObserveUnset(callbacks.clone())),
        );
        vm.register("confine", if confined { confine } else { release });
        crate::fixture::eval(
            &mut vm,
            "set ::seed old; trace add variable ::seed unset observe_missing_key_unset",
        );
        crate::fixture::eval(&mut vm, SCRIPT);
        let result = crate::fixture::eval(&mut vm, "p");
        assert_eq!(result.code, Code::Ok, "{result:?}");
        if confined {
            assert_eq!(
                result.result.to_str().as_ref(),
                "1 {can't unset \"::seed\": stores are confined to the activation} 1 0 0 0 0"
            );
            assert_eq!(callbacks.get(), 0);
        } else {
            assert_eq!(result.result.to_str().as_ref(), "0 {} 0 0 0 0 0");
            assert_eq!(callbacks.get(), 1);
        }
    }
}
