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

use super::*;

fn jim_dictionary_scope_frames() -> (Vm, usize, usize) {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::interpreter_with_dictionary_library(profile);
    vm.push_call_frame(Some("outer".into()), Vec::new());
    let outer = vm.current_level();
    assert_eq!(
        vm.try_eval_source("set held old; set dictionary {k held}")
            .unwrap()
            .code,
        Code::Ok,
    );
    vm.push_call_frame(Some("inner".into()), Vec::new());
    vm.add_link("alias", outer, "held")
        .unwrap_or_else(|_| panic!("link"));
    vm.add_link("dictionary_alias", outer, "dictionary")
        .unwrap_or_else(|_| panic!("link"));
    vm.add_link("member_alias", outer, "dictionary(k)")
        .unwrap_or_else(|_| panic!("link"));
    assert_eq!(
        vm.try_eval_source(
            "set local_dictionary {k local}; set d {}; set wrapper_body NOT_ENTERED"
        )
        .unwrap()
        .code,
        Code::Ok,
    );
    // A dictionary member must have its actual Jim source context before
    // the wrapper's nullable existence getter can inspect it.
    let members = vm
        .try_eval_source("list [set alias] [set member_alias] [info exists member_alias]")
        .unwrap();
    assert_eq!(members.code, Code::Ok);
    assert_eq!(members.result.to_str().as_ref(), "old held 1");
    let inner = vm.current_level();
    (vm, outer, inner)
}

#[test]
fn confined_jim_dictionary_missing_key_keeps_outer_alias_and_dictionary_member() {
    for confined in [false, true] {
        for (target, unset_target) in [("alias", "alias"), ("member_alias", "dictionary_alias(k)")]
        {
            let (mut vm, outer, inner) = jim_dictionary_scope_frames();
            vm.set_stores_confined(confined);
            // Jim's real scripted wrapper retains a missing-key target.
            // It never enters the C primitive's missing-key unset worker.
            let result = vm
                .try_eval_source(&format!(
                    "dict update d k {target} {{set wrapper_body REACHED}}"
                ))
                .unwrap();
            assert_eq!(
                vm.get_var("wrapper_body").unwrap().to_str().as_ref(),
                "REACHED"
            );
            assert_eq!(
                vm.get_var_from(outer, "held").unwrap().to_str().as_ref(),
                "old"
            );
            assert_eq!(
                vm.get_var_from(outer, "dictionary")
                    .unwrap()
                    .to_str()
                    .as_ref(),
                "k held"
            );
            if confined {
                assert_eq!(result.code, Code::Error, "{target}");
                assert!(
                    result
                        .result
                        .to_str()
                        .contains("stores are confined to the activation")
                );
                assert_eq!(vm.get_var("d").unwrap().to_str().as_ref(), "");
            } else {
                assert_eq!(result.code, Code::Ok, "{target}: {result:?}");
                let expected = if target == "alias" { "k old" } else { "k held" };
                assert_eq!(vm.get_var("d").unwrap().to_str().as_ref(), expected);
            }
            assert_eq!(vm.current_level(), inner);
            assert!(vm.refused_completion().is_none());

            // Reach the original unset independently through entered
            // Jim operands, including its dictionary-member alias spelling.
            let result = vm
                .try_eval_source(&format!("unset -nocomplain {unset_target}"))
                .unwrap();
            if confined {
                assert_eq!(result.code, Code::Error, "{target}");
                assert!(
                    result
                        .result
                        .to_str()
                        .contains("stores are confined to the activation")
                );
                assert_eq!(
                    vm.get_var_from(outer, "held").unwrap().to_str().as_ref(),
                    "old"
                );
                assert_eq!(
                    vm.get_var_from(outer, "dictionary")
                        .unwrap()
                        .to_str()
                        .as_ref(),
                    "k held"
                );
            } else {
                assert_eq!(result.code, Code::Ok, "{target}: {result:?}");
                if target == "alias" {
                    assert!(vm.get_var_from(outer, "held").is_none());
                } else {
                    assert_eq!(
                        vm.get_var_from(outer, "dictionary")
                            .unwrap()
                            .to_str()
                            .as_ref(),
                        ""
                    );
                }
            }
            // The same Jim path worker must admit a genuine local receiver,
            // independently of the wrapper's forbidden reference alias.
            let local_path = vm.try_eval_source(
                    "set path_dictionary {k OLD}; dict set path_dictionary k NEW; set path_member [dict get $path_dictionary k]; dict unset path_dictionary k; list $path_member [dict size $path_dictionary]"
                ).unwrap();
            assert_eq!(local_path.code, Code::Ok, "{local_path:?}");
            assert_eq!(local_path.result.to_str().as_ref(), "NEW 0");
            for local in ["absent", "local_dictionary(k)", "local_dictionary(missing)"] {
                let result = vm
                    .try_eval_source(&format!("unset -nocomplain {local}"))
                    .unwrap();
                assert_eq!(result.code, Code::Ok, "{local}: {result:?}");
            }
            assert_eq!(
                vm.get_var("local_dictionary").unwrap().to_str().as_ref(),
                ""
            );
            assert_eq!(vm.current_level(), inner);
            assert!(vm.refused_completion().is_none());
        }
    }
}

#[test]
fn confined_dictionary_update_missing_key_keeps_foreign_target_and_unset_traces() {
    use std::cell::Cell;
    use std::rc::Rc;

    struct ObserveUnset(Rc<Cell<usize>>);
    impl crate::command::NativeCommand for ObserveUnset {
        fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
            self.0.set(self.0.get() + 1);
            ok(Value::empty())
        }
    }
    for confined in [false, true] {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        vm.set_var("::seed", Value::string("old")).unwrap();
        let callbacks = Rc::new(Cell::new(0));
        vm.register_native_command(
            "observe_missing_key_unset",
            Rc::new(ObserveUnset(callbacks.clone())),
        );
        vm.add_var_trace_bytes(
            b"::seed",
            vec!["unset".into()],
            &Value::string("observe_missing_key_unset"),
            false,
        );
        vm.push_call_frame(Some("dictionary_update".into()), Vec::new());
        vm.set_var("d", Value::dict(Vec::new())).unwrap();
        vm.set_stores_confined(confined);
        let result = cmd_dict_update(
            &mut vm,
            &[
                Value::string("d"),
                Value::string("k"),
                Value::string("::seed"),
                Value::empty(),
            ],
        );
        if confined {
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                result.result.string_bytes().as_ref(),
                b"can't unset \"::seed\": stores are confined to the activation"
            );
            assert_eq!(vm.get_var("::seed").unwrap().to_str().as_ref(), "old");
            assert_eq!(callbacks.get(), 0);
        } else {
            assert_eq!(result.code, Code::Ok);
            assert!(vm.get_var("::seed").is_none());
            assert_eq!(callbacks.get(), 1);
        }
        for target in ["::missing_ns::x", "local"] {
            if target == "local" {
                vm.set_var("local", Value::string("old")).unwrap();
            }
            let result = cmd_dict_update(
                &mut vm,
                &[
                    Value::string("d"),
                    Value::string("k"),
                    Value::string(target),
                    Value::empty(),
                ],
            );
            assert_eq!(result.code, Code::Ok, "{target}: {result:?}");
            assert!(vm.get_var(target).is_none());
        }
    }
}

#[test]
fn host_dictionary_surface_is_independent_of_embedded_source_grammar() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_command_surface_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
    ));
    let result = cmd_dict(
        &mut vm,
        &[
            Value::string("create"),
            Value::string("key"),
            Value::string("value"),
        ],
    );
    assert_eq!(result.code, crate::Code::Ok);
    assert_eq!(&*result.result.to_str(), "key value");
    assert_eq!(
        vm.runtime_version(),
        tcl_dialect::DialectProfile::irules().vm_runtime_version
    );
}

/// A dict `Value`'s top-level `(key, value-string)` pairs.
fn top_pairs(vm: &mut Vm, v: &Value) -> Vec<(String, String)> {
    pairs(vm, v)
        .expect("dict value is a valid list")
        .into_iter()
        .map(|(k, val)| (k, val.to_str().to_string()))
        .collect()
}

/// `set_path`/`unset_path` recursing once per multi-key `dict
/// set`/`dict unset` path segment natively has no depth cap: an
/// unguarded `dict set d {*}[lrepeat 100000 k] v` overflows the native
/// stack (SIGABRT), empirically between depth 3000 and 3500 on a 2 MiB
/// thread (`cargo test`'s per-test default). The iterative
/// implementation (mirroring `get_path`'s existing iterative style) has
/// no such cap; this test checks `set_path`'s result for exact
/// correctness at depth 2000, comfortably past that crash range —
/// descending back down through the same key at every level must land
/// on the value that was set, not merely survival.
///
/// Deliberately NOT 50,000+: a dict this deep is represented as an
/// equally deep nested `Value::list` chain, and `Value` has no custom
/// `Drop` impl — the compiler-generated recursive drop glue that runs
/// when `set`/`cur` go out of scope at the end of this test is its own,
/// unrelated native-stack risk (empirically, SIGABRT between depth 3500
/// and 4000 on a 2 MiB thread for construction+drop alone), a separate
/// concern in `Value`'s representation itself, out of scope here.
#[test]
fn deeply_nested_dict_set_and_unset_survive() {
    const DEPTH: usize = 2_000;
    let vm = &mut Vm::new();
    let keys: Vec<Value> = (0..DEPTH).map(|_| Value::string("k")).collect();
    let set = set_path(vm, &Value::empty(), &keys, Value::string("v")).expect("set_path survives");
    let mut cur = set.clone();
    for _ in 0..DEPTH {
        let ps = pairs(vm, &cur).expect("valid dict at every level");
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].0, "k");
        cur = ps[0].1.clone();
    }
    assert_eq!(&*cur.to_str(), "v");

    // `unset_path` must also survive the same depth — the assertion is
    // that it returns at all, not the exact shape of what remains (each
    // level's "k" entry survives holding an emptied-out subdict, matching
    // `dict unset`'s "leaf only" removal semantics — the whole chain does
    // not collapse away).
    let _ = unset_path(vm, &set, &keys).expect("unset_path survives");
}

/// A moderately nested `dict set`/`dict unset` path (well within
/// realistic use) is byte-for-byte unaffected by the iterative
/// rewrite.
#[test]
fn moderately_nested_dict_set_and_unset_unaffected() {
    let vm = &mut Vm::new();
    let s = Value::string;

    // `dict set {} a 1` -> {a 1}.
    let d = set_path(vm, &Value::list(vec![]), &[s("a")], s("1")).unwrap();
    assert_eq!(top_pairs(vm, &d), [("a".into(), "1".into())]);

    // Updating an existing key keeps its position (order-preserving).
    let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);
    let updated = set_path(vm, &base, &[s("a")], s("9")).unwrap();
    assert_eq!(
        top_pairs(vm, &updated),
        [("a".into(), "9".into()), ("b".into(), "2".into())]
    );

    // A new key appends at the end.
    let appended = set_path(vm, &base, &[s("c")], s("3")).unwrap();
    assert_eq!(
        top_pairs(vm, &appended),
        [
            ("a".into(), "1".into()),
            ("b".into(), "2".into()),
            ("c".into(), "3".into())
        ]
    );

    // A multi-key path auto-vivifies intermediate dicts.
    let nested = set_path(vm, &Value::list(vec![]), &[s("a"), s("b")], s("1")).unwrap();
    assert_eq!(top_pairs(vm, &nested), [("a".into(), "b 1".into())]);

    // Removing a present key drops just that pair.
    let removed = unset_path(vm, &base, &[s("a")]).unwrap();
    assert_eq!(top_pairs(vm, &removed), [("b".into(), "2".into())]);

    // Removing an absent key leaves the dict unchanged.
    let untouched = unset_path(vm, &base, &[s("z")]).unwrap();
    assert_eq!(
        top_pairs(vm, &untouched),
        [("a".into(), "1".into()), ("b".into(), "2".into())]
    );

    // A nested unset rewrites only the inner dict.
    let inner = Value::list(vec![s("b"), s("1"), s("c"), s("2")]);
    let outer = Value::list(vec![s("a"), inner]);
    let nested_unset = unset_path(vm, &outer, &[s("a"), s("b")]).unwrap();
    assert_eq!(top_pairs(vm, &nested_unset), [("a".into(), "c 2".into())]);

    let error = unset_path(vm, &outer, &[s("a"), s("z"), s("q")]).unwrap_err();
    assert_eq!(
        error.result.string_bytes().as_ref(),
        b"key \"z\" not known in dictionary"
    );
}

#[test]
fn jim_dict_with_compares_original_key_units_and_skips_empty_body_writeback() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::interpreter_with_dictionary_library(profile);
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
