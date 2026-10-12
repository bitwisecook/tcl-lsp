// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original callback windows; the observer retains no native argv references.

use super::*;
use crate::interp::{Code, Interp};
use std::{cell::RefCell, fmt::Write};

struct Observer {
    parent: *mut TclObj,
    backing: Rc<NativeJimScript>,
    case: usize,
    calls: usize,
    rows: String,
}
thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}
fn kind(value: *mut TclObj) -> String {
    let descriptor = obj::obj_type_ptr(value);
    if descriptor.is_null() {
        return "NULL".into();
    }
    // SAFETY: the original live primary descriptor owns a static native name.
    unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
        .to_str()
        .unwrap()
        .into()
}
fn inspect(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let (parent, control) = OBSERVER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let observer = slot.as_mut().expect("active original observer");
        observer.calls += 1;
        let case = observer.case;
        let call = observer.calls;
        let backing = &observer.backing;
        writeln!(
            observer.rows,
            "CALL\t{case}\t{call}\t{}\t{}\t{}",
            argv.len(),
            backing.in_use.get(),
            backing.ordinary().unwrap().linenr.get()
        )
        .unwrap();
        for (index, &value) in argv.iter().enumerate() {
            let token = backing
                .ordinary()
                .unwrap()
                .objects
                .tokens()
                .iter()
                .rposition(|token| token.value.as_ptr() == value)
                .map_or(-1, |index| index as i32);
            // SAFETY: callback borrows the original active vector.
            let refs = unsafe { (*value).ref_count };
            writeln!(
                observer.rows,
                "ARGV\t{case}\t{call}\t{index}\t{token}\t{}\t{}\t{refs}",
                kind(value),
                usize::from(obj::has_string_rep(value))
            )
            .unwrap();
        }
        (
            observer.parent,
            match case {
                5 => 1,
                6 => 2,
                7 => 3,
                _ => 0,
            },
        )
    });
    if control != 0 {
        crate::list::list_elements_native_checked(
            parent,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        OBSERVER.with(|slot| {
            let mut slot = slot.borrow_mut();
            let observer = slot.as_mut().unwrap();
            writeln!(
                observer.rows,
                "SHIMMER\t{}\t{}\t{}",
                observer.case,
                kind(parent),
                observer.backing.in_use.get()
            )
            .unwrap();
        });
    }
    interp.set_result(argv.get(1).copied().unwrap_or_else(|| {
        interp
            .native_jim_object_context()
            .unwrap()
            .empty_object()
            .as_ptr()
    }));
    match control {
        2 => Code::Error,
        3 => Code::Return,
        _ => Code::Ok,
    }
}

#[test]
fn original_script_execution_matches_51_native_callback_windows() {
    let sources: [&[u8]; 10] = [
        b"inspect X",
        b"inspect X; inspect Y",
        b"inspect X; set x {bad",
        b"inspect X; inspect [error FAIL]",
        b"",
        b"inspect X",
        b"inspect X",
        b"inspect X",
        b"inspect {*} {X Y}",
        b"inspect pre${v}post",
    ];
    let mut observed = String::new();
    for (case, source) in sources.into_iter().enumerate() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        interp.register_builtin(b"inspect", inspect);
        assert_eq!(interp.eval_str(b"set v V"), Code::Ok);
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(obj::new_string_bytes(b"FILE"));
        let parent = Owned::fresh(obj::new_string_bytes(source));
        native_source::install_source(
            parent.as_ptr(),
            NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        let backing = prepare(parent.as_ptr(), &context, interp.lexer_config()).unwrap();
        let script = backing.ordinary().unwrap();
        writeln!(
            observed,
            "BEFORE\t{case}\t{}\t{}\t{}\t{}",
            script.objects.tokens().len(),
            script.objects.missing.map_or(32, i32::from),
            script.linenr.get(),
            backing.in_use.get()
        )
        .unwrap();
        OBSERVER.with(|slot| {
            *slot.borrow_mut() = Some(Observer {
                parent: parent.as_ptr(),
                backing: Rc::clone(&backing),
                case,
                calls: 0,
                rows: String::new(),
            })
        });
        let code = interp.eval_body_obj(parent.as_ptr());
        let observer = OBSERVER.with(|slot| slot.borrow_mut().take().unwrap());
        observed.push_str(&observer.rows);
        let same = if obj::obj_type_ptr(parent.as_ptr()) == &JIM_SCRIPT_TYPE {
            // SAFETY: the original Script primary owns this actual header.
            let header =
                unsafe { &*(obj::internal_rep(parent.as_ptr()) as usize as *const ScriptHeader) };
            Rc::ptr_eq(&header.0, &backing)
        } else {
            false
        };
        writeln!(
            observed,
            "AFTER\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            code.as_int(),
            observer.calls,
            kind(parent.as_ptr()),
            usize::from(same),
            backing.in_use.get(),
            script.linenr.get(),
            usize::from(interp.result_obj() == context.empty_object().as_ptr())
        )
        .unwrap();
    }
    let expected = include_str!(
        "../../../rust/tcl-syntax/testdata/native_jim_script_execution/eval_observations.tsv"
    );
    assert_eq!(expected.lines().count(), 51);
    assert_eq!(observed, expected);
}
