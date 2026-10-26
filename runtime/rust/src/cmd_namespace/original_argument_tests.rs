// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact original getter and public state windows; software refusals are separate.

use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) const ORIGINALS: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.4.20/execute.stdout"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.5.19/execute.stdout"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.6.18/execute.stdout"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/9.0.4/execute.stdout"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/9.1.0/execute.stdout"
        ),
    ),
];

pub(crate) const JIM_ORIGINAL: &str = include_str!(
    "../../../../rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499d/providers/jim/execute.stdout"
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Event {
    label: char,
    exports: Vec<u8>,
    commands: [bool; 3],
}
thread_local! {
    static PAYLOADS: RefCell<Vec<(char, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    static REFUSAL: Cell<u8> = const { Cell::new(0) };
}
extern "C" fn update(original: *mut TclObj) {
    let (label, payload) = PAYLOADS.with(|values| {
        values.borrow()[usize::try_from(obj::internal_rep(original)).unwrap()].clone()
    });
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    let patterns = interpreter
        .namespaces()
        .exports(interpreter.current_ns())
        .to_vec();
    let exports = patterns
        .iter()
        .enumerate()
        .fold(Vec::new(), |mut bytes, (i, pattern)| {
            if i != 0 {
                bytes.push(b' ');
            }
            bytes.extend_from_slice(pattern);
            bytes
        });
    let commands =
        [b"::d::a".as_slice(), b"::d::b", b"::d::p"].map(|name| interpreter.command_exists(name));
    EVENTS.with(|events| {
        events.borrow_mut().push(Event {
            label,
            exports,
            commands,
        })
    });
    match REFUSAL.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("original naming test updater");
        }
        2 => {
            let old = interpreter.runtime_context();
            let mut changed = old.clone();
            changed.packages = vec![("naming-getter-control".to_owned(), "1.0".to_owned())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&old).unwrap();
        }
        _ => {}
    }
    unsafe { obj::set_native_updater_string_rep(original, &payload, false) };
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalNamingStage".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
pub(crate) fn instance(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
pub(crate) fn lazy(label: char, payload: &[u8]) -> obj::Owned {
    let index = PAYLOADS.with(|values| {
        let mut values = values.borrow_mut();
        let index = values.len();
        values.push((label, payload.to_vec()));
        index
    });
    obj::Owned::fresh(obj::alloc_typed(&LAZY, u64::try_from(index).unwrap()))
}
pub(crate) fn reset() {
    PAYLOADS.with(|values| values.borrow_mut().clear());
    EVENTS.with(|values| values.borrow_mut().clear());
    REFUSAL.with(|value| value.set(0));
}
pub(crate) fn failure(mode: u8) {
    REFUSAL.with(|value| value.set(mode));
}
pub(crate) fn text(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
pub(crate) fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub(crate) fn row<'a>(capture: &'a str, case: usize, phase: &str) -> Vec<&'a str> {
    capture
        .lines()
        .find(|line| {
            let words: Vec<_> = line.split('|').collect();
            words.first() == Some(&"ROW")
                && words[1].parse::<usize>() == Ok(case)
                && words[2] == phase
        })
        .unwrap()
        .split('|')
        .collect()
}
pub(crate) fn compare_events_at(capture: &str, case: usize, phase: &str) {
    let count = row(capture, case, phase)[5]
        .strip_prefix("events=")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let originals: Vec<_> = capture
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('|').collect();
            if fields.first() != Some(&"EVENT") || fields[1].parse::<usize>() != Ok(case) {
                return None;
            }
            let exports = fields[5].strip_prefix("exports=0:").map(unhex);
            Some((
                fields[3].chars().next().unwrap(),
                exports,
                [fields[6] == "a=1", fields[7] == "b=1", fields[8] == "p=1"],
            ))
        })
        .take(count)
        .collect();
    EVENTS.with(|events| {
        let actual = events.borrow();
        assert_eq!(actual.len(), originals.len(), "case {case}");
        for (actual, (label, exports, commands)) in actual.iter().zip(originals) {
            assert_eq!(actual.label, label, "case {case}");
            if let Some(exports) = exports {
                assert_eq!(actual.exports, exports, "case {case}");
            }
            assert_eq!(actual.commands, commands, "case {case}");
        }
    });
}
pub(crate) fn compare_worker(interpreter: &mut Interp, code: Code, capture: &str, case: usize) {
    let original = row(capture, case, "worker");
    assert_eq!(
        code.as_int(),
        original[3].parse::<i64>().unwrap(),
        "case {case}"
    );
    assert!(!interpreter.host_refusal_pending(), "case {case}");
    assert_eq!(
        interpreter.result_bytes(),
        unhex(original[4]),
        "case {case}"
    );
    compare_events_at(capture, case, "worker");
}
/// Clear fixture TLS while the actual interpreter/object owners still exist,
/// including when an assertion unwinds. This does not reset a native first cause.
pub(crate) struct FixtureCleanup(pub fn());
impl Drop for FixtureCleanup {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// The same Rc holder owned by the reached Jim namespace frame. Cloning it
/// keeps its actual NsId live without another native object reference.
pub(crate) struct JimFixtureNamespace {
    pub(crate) token: NsId,
    pub(crate) owner: Rc<obj::Owned>,
}
thread_local! {
    static CAPTURED_NAMESPACE: RefCell<Option<JimFixtureNamespace>> = const { RefCell::new(None) };
}
fn capture_namespace(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    let token = interpreter.current_ns();
    let owner = interpreter
        .namespaces()
        .jim_namespace_original_owner(token)
        .expect("actual reached Jim namespace frame holder");
    CAPTURED_NAMESPACE.with(|captured| {
        assert!(captured.borrow().is_none());
        *captured.borrow_mut() = Some(JimFixtureNamespace { token, owner });
    });
    Code::Ok
}
pub(crate) fn jim_fixture_namespace(interpreter: &mut Interp, name: &[u8]) -> JimFixtureNamespace {
    assert!(interpreter
        .native_invocation_dialect()
        .native_name_protocol()
        .unwrap()
        .is_jim084());
    CAPTURED_NAMESPACE.with(|captured| captured.borrow_mut().take());
    let _cleanup = FixtureCleanup(|| {
        CAPTURED_NAMESPACE.with(|captured| captured.borrow_mut().take());
    });
    let prior = obj::Owned::retain(interpreter.get_obj_result());
    interpreter.register_builtin(b"consumerFixtureCurrentNamespace", capture_namespace);
    let namespace = text(name);
    let body = text(b"consumerFixtureCurrentNamespace");
    let code = namespace_call(interpreter, b"eval", &[&namespace, &body]);
    assert_eq!(
        code,
        Code::Ok,
        "original Jim namespace fixture: refusal={:?}, callback_captured={}, result={:?}",
        interpreter.native_execution_refusal(),
        CAPTURED_NAMESPACE.with(|captured| captured.borrow().is_some()),
        obj::native_object_snapshot(interpreter.get_obj_result()),
    );
    let captured = CAPTURED_NAMESPACE
        .with(|captured| captured.borrow_mut().take())
        .expect("namespace eval reached the actual fixture callback");
    interpreter.set_result(prior.as_ptr());
    captured
}

thread_local! {
    static FIXTURE_CALL: RefCell<Option<(NsId, Vec<*mut TclObj>)>> = const { RefCell::new(None) };
}
fn fixture_namespace_call(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    let (namespace, original) = FIXTURE_CALL.with(|call| call.borrow_mut().take().unwrap());
    assert_eq!(interpreter.current_ns(), namespace);
    assert_ne!(interpreter.frames.borrow().current_frame_index(), 0);
    namespace_cmd(interpreter, &original)
}
fn namespace_call_in_fixture(
    interpreter: &mut Interp,
    namespace: Option<&JimFixtureNamespace>,
    sub: &[u8],
    inputs: &[&obj::Owned],
) -> Code {
    let Some(namespace) = namespace else {
        return namespace_call(interpreter, sub, inputs);
    };
    let head = text(b"namespace");
    let selector = text(sub);
    let mut original = vec![head.as_ptr(), selector.as_ptr()];
    original.extend(inputs.iter().map(|input| input.as_ptr()));
    FIXTURE_CALL.with(|call| {
        assert!(call.borrow().is_none());
        *call.borrow_mut() = Some((namespace.token, original));
    });
    let _cleanup = FixtureCleanup(|| {
        FIXTURE_CALL.with(|call| call.borrow_mut().take());
    });
    interpreter.register_builtin(b"consumerFixtureNamespaceCall", fixture_namespace_call);
    let body = text(b"consumerFixtureNamespaceCall");
    let actual = interpreter
        .namespaces()
        .jim_namespace_original_owner(namespace.token)
        .expect("live captured Jim namespace holder");
    assert!(Rc::ptr_eq(&actual, &namespace.owner));
    drop(actual);
    // Enter the captured holder; public namespace eval constructs another role token.
    interpreter.ns_eval_in_token(
        namespace.token,
        &[],
        None,
        true,
        Some((
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
            body.as_ptr(),
        )),
        None,
    )
}

fn namespace_call(interpreter: &mut Interp, sub: &[u8], inputs: &[&obj::Owned]) -> Code {
    let head = text(b"namespace");
    let sub = text(sub);
    let mut argv = vec![head.as_ptr(), sub.as_ptr()];
    argv.extend(inputs.iter().map(|input| input.as_ptr()));
    namespace_cmd(interpreter, &argv)
}

#[test]
fn original_namespace_patterns_match_all_c_getter_and_public_effect_windows() {

    // Exact C499c selected windows only. C84's callback export axis stays unasserted;
    // no binding-generation, header-count, Jim or whole-dispatch equivalence is claimed.
    let compared: usize = ORIGINALS
        .iter()
        .map(|&(engine, capture)| compare_namespace_patterns(engine, capture))
        .sum();
    assert_eq!(compared, 60);
}

#[test]
fn original_namespace_patterns_match_reached_jim_getter_and_public_effect_windows() {

    // The independent Jim499d worker uses global original operands. Jim499c's
    // missing-variable setup does not establish getter order. Export's ignored
    // inputs and unsupported forget remain distinct from the measured imports.
    assert_eq!(compare_namespace_patterns("jim", JIM_ORIGINAL), 12);
}

fn compare_namespace_patterns(engine: &str, capture: &str) -> usize {
    let mut compared = 0;
    for case in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14] {
        reset();
        let mut interpreter = instance(engine);
        assert_eq!(interpreter.eval_str(b"namespace eval ::s {proc a {} {}; proc b {} {}; namespace export *}; namespace eval ::d {}; set READY READY"), Code::Ok);
        let _cleanup = FixtureCleanup(|| {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        });
        let jim_namespace =
            (engine == "jim").then(|| jim_fixture_namespace(&mut interpreter, b"::d"));
        let current = jim_namespace.as_ref().map_or_else(
            || interpreter.find_namespace_id(b"::d").unwrap(),
            |original| original.token,
        );
        if jim_namespace.is_none() {
            interpreter.set_current_ns(current);
        }
        if matches!(case, 0 | 1 | 2 | 3 | 4 | 12 | 14) {
            assert_eq!(
                namespace_call_in_fixture(
                    &mut interpreter,
                    jim_namespace.as_ref(),
                    b"export",
                    &[&text(b"old")]
                ),
                Code::Ok
            );
        }
        if case == 8 {
            assert_eq!(
                namespace_call_in_fixture(
                    &mut interpreter,
                    jim_namespace.as_ref(),
                    b"import",
                    &[&text(b"::s::*")]
                ),
                Code::Ok
            );
        }
        let (sub, clear, inputs): (&[u8], bool, &[&[u8]]) = match case {
            0 => (b"export", false, &[]),
            1 => (b"export", true, &[b"::bad", b"tail*"]),
            2 => (b"export", false, &[b"one*", b"::bad", b"last*"]),
            3 => (b"export", false, &[b"::bad", b"*"]),
            4 => (b"export", false, &[b"one", b"-clear", b"last"]),
            5 => (b"import", false, &[b"::s::a", b"::missing::*", b"::s::b"]),
            6 => (b"import", true, &[b"::s::a", b"::missing::*", b"::s::b"]),
            7 => (b"import", false, &[b"unqualified", b"::s::a"]),
            8 => (b"forget", false, &[b"::s::a", b"::missing::*", b"::s::b"]),
            12 => (b"export", false, &[b"ok\0::bad", b"last"]),
            13 => (b"import", false, &[b"::s::a\0::bad", b"::missing::*"]),
            14 => (b"export", false, &["é*".as_bytes(), b"::bad", b"last"]),
            _ => unreachable!(),
        };
        let values: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(i, payload)| lazy(char::from(b'A' + u8::try_from(i).unwrap()), payload))
            .collect();
        let flag = clear.then(|| {
            text(if sub == b"export" {
                b"-clear"
            } else {
                b"-force"
            })
        });
        let mut operands: Vec<_> = flag.iter().collect();
        operands.extend(values.iter());
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let code =
            namespace_call_in_fixture(&mut interpreter, jim_namespace.as_ref(), sub, &operands);
        compare_worker(&mut interpreter, code, capture, case);
        let exports = interpreter.namespaces().exports(current).to_vec();
        set_list_bytes(&mut interpreter, &exports);
        assert_eq!(
            interpreter.result_bytes(),
            unhex(row(capture, case, "exports")[4])
        );
        interpreter.set_current_ns(GLOBAL);
        assert_eq!(
            interpreter.eval_str(
                b"list [info commands ::d::a] [info commands ::d::b] [info commands ::d::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            interpreter.result_bytes(),
            unhex(row(capture, case, "commands")[4])
        );
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        drop(values);
        reset();
        compared += 1;
    }
    compared
}

#[test]
fn original_namespace_getter_refusal_keeps_prefix_effects_and_later_argument_unread() {

    // Software Host/ABA transport control, independently of original C chronology.
    for failure in [1, 2] {
        reset();
        let mut interpreter = instance("tcl8.6");
        let later = lazy('C', b"last");
        let failed = lazy('B', b"second");
        let first = text(b"one");
        let before = unsafe { (*failed.as_ptr()).ref_count };
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        REFUSAL.with(|value| value.set(failure));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        assert_eq!(
            namespace_call(&mut interpreter, b"export", &[&first, &failed, &later]),
            Code::Error
        );
        assert_eq!(interpreter.namespaces().exports(GLOBAL), &[b"one".to_vec()]);
        assert_eq!(EVENTS.with(|events| events.borrow().len()), 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(obj::bytes_of(failed.as_ptr()), b"second");
        assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, before);
        assert!(interpreter.native_execution_refusal().is_some());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        drop(failed);
        drop(later);
        reset();
    }
}
