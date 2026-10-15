// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original ancestor publication during actual unset and object-command deletion.

use crate::interp::{Code, Interp, InterpState, ObjCommand};
use crate::namespace::{NsId, GLOBAL};
use crate::obj::TclObj;
use core::ffi::{c_int, c_void};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
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

thread_local! {
    static ORIGINAL: Cell<Option<NsId>> = const { Cell::new(None) };
}

fn observe(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    let original = ORIGINAL.with(Cell::get).unwrap();
    let namespaces = interpreter.namespaces();
    let mut selected = Some(interpreter.current_ns());
    let mut found = false;
    while let Some(namespace) = selected {
        if namespace == original {
            found = true;
            break;
        }
        selected = namespaces.parent(namespace);
    }
    drop(namespaces);
    interpreter.set_result_bytes(if found { b"1" } else { b"0" });
    Code::Ok
}

struct RetiringCommand {
    interpreter: Weak<InterpState>,
    calls: Rc<Cell<usize>>,
    outcome: Rc<Cell<Option<Code>>>,
}

unsafe extern "C" fn noop(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    0
}
unsafe extern "C" fn deleted(data: *mut c_void) {
    // SAFETY: this actual command owns and releases its client record once.
    let state = unsafe { Box::from_raw(data.cast::<RetiringCommand>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = Interp(state.interpreter.upgrade().unwrap());
    state.outcome.set(Some(interpreter.eval_str(b"during")));
}

#[test]
fn original_missing_descendant_has_the_dying_ancestor_and_is_destroyed_with_it() {
    for (version, rows) in [
        (
            tcl_dialect::TclVersion::V8_4,
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_dying_descendant_physical/8.4.20/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V8_5,
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_dying_descendant_physical/8.5.19/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_dying_descendant_physical/8.6.18/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_0,
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_dying_descendant_physical/9.0.4/stdout.tsv"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_1,
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_dying_descendant_physical/9.1.0/stdout.tsv"
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
                let mut interpreter = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(version.dialect_profile_name()),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interpreter.register_builtin(b"observe_original_ancestor", observe);
                assert_eq!(interpreter.eval_str(&setup(target)), Code::Ok);
                let original = interpreter
                    .namespaces()
                    .find_namespace(GLOBAL, b"retiring")
                    .unwrap();
                ORIGINAL.with(|stored| stored.set(Some(original)));
                let calls = Rc::new(Cell::new(0));
                let outcome = Rc::new(Cell::new(None));
                if command_window {
                    let state = Box::new(RetiringCommand {
                        interpreter: Rc::downgrade(&interpreter.0),
                        calls: Rc::clone(&calls),
                        outcome: Rc::clone(&outcome),
                    });
                    interpreter
                        .create_obj_command(
                            b"::retiring::sentinel_command",
                            ObjCommand::new(noop, Box::into_raw(state).cast(), Some(deleted)),
                        )
                        .unwrap();
                } else {
                    assert_eq!(
                        interpreter.eval_str(
                            if version == tcl_dialect::TclVersion::V8_4 {
                            b"namespace eval retiring {trace variable sentinel u ::during}"
                        } else {
                            b"namespace eval retiring {trace add variable sentinel unset ::during}"
                        }
                        ),
                        Code::Ok,
                        "{version:?}/{target:?}: trace setup result {:?}, execution {:?}, access {:?}", interpreter.result_bytes(), interpreter.native_execution_refusal(), interpreter.native_access_refusal()
                    );
                }
                assert_eq!(interpreter.eval_str(FINISH), Code::Ok);
                let label = format!(
                    "{}_{}",
                    if command_window { "COMMAND" } else { "UNSET" },
                    case
                );
                assert_eq!(
                    interpreter.result_bytes(),
                    expected(rows, &label),
                    "{version:?}/{label}"
                );
                assert_eq!(calls.get(), usize::from(command_window));
                assert_eq!(outcome.get(), command_window.then_some(Code::Ok));
                assert!(
                    interpreter.native_execution_refusal().is_none(),
                    "{version:?}/{label}"
                );
                assert_eq!(interpreter.current_ns(), GLOBAL);
                ORIGINAL.with(|stored| stored.set(None));
            }
        }
    }
}
