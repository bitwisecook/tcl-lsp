// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original descendant namespace allocation and real teardown callbacks.

use super::{Vm, native_name_world::NativeNameWorld, ok};
use crate::{
    Value,
    command::{Command, NativeCommand},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use tcl_core_types::{ByteNamespacePath, NsId, ROOT_NS};
use tcl_runtime_api::{Code, Completion};

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn expected(rows: &str, label: &str) -> Vec<u8> {
    let field = |kind: &str| {
        let selected: Vec<_> = rows
            .lines()
            .filter(|line| line.starts_with(&format!("{kind}|{label}|")))
            .collect();
        assert_eq!(selected.len(), 1, "{kind}/{label}");
        let fields: Vec<_> = selected[0].split('|').collect();
        assert_eq!(fields[2], "0", "{kind}/{label}");
        String::from_utf8(unhex(fields[3])).unwrap()
    };
    let public = field("PUBLIC_LOOKUPS");
    let public: Vec<_> = public.split_ascii_whitespace().collect();
    let contents = field("CONTENTS");
    let during = [
        field("CREATE"),
        field("ORIGINAL_ANCESTOR"),
        public[1].to_owned(),
        public[2].to_owned(),
        contents,
    ]
    .join(" ");
    format!("{{{during}}} {} {}", field("FINAL"), field("CALLBACK_ONCE")).into_bytes()
}

fn setup(target: &[u8]) -> Vec<u8> {
    [br#"
set ::target {"#.as_slice(), target, br#"}
set ::observations UNREACHED
set ::callback_count 0
namespace eval retiring {
    namespace eval Existing {variable before BEFORE}
    variable sentinel HELD
}
proc during args {
    incr ::callback_count
    namespace eval $::target {
        variable value HELD
        proc p {} {return CHILD}
        set ::observations [list [namespace current] [observe_original_ancestor] [namespace exists [namespace current]] [info exists value] [set value] [p]]
    }
}
"#].concat()
}

const FINISH: &[u8] = br#"
namespace delete ::retiring
list $::observations [namespace exists ::retiring] [namespace exists $::target] [info exists ${::target}::value] [info commands ${::target}::p] $::callback_count
"#;

struct ObserveAncestor {
    original: NsId,
    world: Weak<RefCell<NativeNameWorld>>,
}
impl NativeCommand for ObserveAncestor {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        assert!(
            self.world
                .upgrade()
                .is_some_and(|world| Rc::ptr_eq(&world, &vm.name_world))
        );
        assert!(vm.native_namespace_token_has_owner(self.original));
        let mut selected = Some(vm.current_ns_id());
        let world = vm.name_world.borrow();
        let mut found = false;
        while let Some(namespace) = selected {
            if namespace == self.original {
                found = true;
                break;
            }
            selected = world
                .ns_parents
                .get(namespace.0 as usize)
                .copied()
                .flatten();
        }
        ok(Value::int(i64::from(found)))
    }
}

struct RetiringCommand(Rc<Cell<usize>>);
impl NativeCommand for RetiringCommand {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }
    fn retire(&self, vm: &mut Vm) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        vm.eval_source("during").unwrap()
    }
}

#[test]
fn original_missing_descendant_has_the_dying_ancestor_and_is_destroyed_with_it() {
    for (version, rows) in [
        (
            tcl_dialect::TclVersion::V8_4,
            include_str!(
                "../../../tcl-registry/tests/data/native_dying_descendant_physical/8.4.20/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V8_5,
            include_str!(
                "../../../tcl-registry/tests/data/native_dying_descendant_physical/8.5.19/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            include_str!(
                "../../../tcl-registry/tests/data/native_dying_descendant_physical/8.6.18/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_0,
            include_str!(
                "../../../tcl-registry/tests/data/native_dying_descendant_physical/9.0.4/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_1,
            include_str!(
                "../../../tcl-registry/tests/data/native_dying_descendant_physical/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for (case, target) in [
            ("MISSING_CHILD", b"::retiring::New".as_slice()),
            ("MISSING_PARENT", b"::retiring::New::"),
            ("DEEP_CHILD", b"::retiring::New::Leaf"),
            ("EXISTING_CHILD", b"::retiring::Existing"),
        ] {
            for command_window in [false, true] {
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(version.dialect_profile_name()),
                );
                assert_eq!(
                    vm.eval_source(std::str::from_utf8(&setup(target)).unwrap())
                        .unwrap()
                        .code,
                    Code::Ok
                );
                let original = vm
                    .native_namespace_context_for_path(&ByteNamespacePath::from_segments([
                        b"retiring".as_slice(),
                    ]))
                    .unwrap();
                vm.register_command(
                    "observe_original_ancestor",
                    Command::Native(Rc::new(ObserveAncestor {
                        original,
                        world: Rc::downgrade(&vm.name_world),
                    })),
                );
                let calls = Rc::new(Cell::new(0));
                if command_window {
                    vm.register_command(
                        "::retiring::sentinel_command",
                        Command::Native(Rc::new(RetiringCommand(Rc::clone(&calls)))),
                    );
                } else {
                    assert_eq!(
                        vm.eval_source(if version == tcl_dialect::TclVersion::V8_4 {
                            "namespace eval retiring {trace variable sentinel u ::during}"
                        } else {
                            "namespace eval retiring {trace add variable sentinel unset ::during}"
                        })
                        .unwrap()
                        .code,
                        Code::Ok
                    );
                }
                let completion = vm
                    .eval_source(std::str::from_utf8(FINISH).unwrap())
                    .unwrap();
                let label = format!(
                    "{}_{}",
                    if command_window { "COMMAND" } else { "UNSET" },
                    case
                );
                assert_eq!(
                    completion.code,
                    Code::Ok,
                    "{version:?}/{label}/{completion:?}"
                );
                assert_eq!(
                    vm.native_name_operand_bytes(&completion.result)
                        .unwrap()
                        .as_ref(),
                    expected(rows, &label),
                    "{version:?}/{label}"
                );
                assert_eq!(calls.get(), usize::from(command_window));
                assert!(vm.execution_refusal.is_none(), "{version:?}/{label}");
                assert_eq!(vm.current_ns_id(), ROOT_NS);
            }
        }
    }
}
