// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::interp::{Code, Interp};
use std::{cell::RefCell, fmt::Write};
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueOps;

fn descriptor(value: *mut TclObj) -> &'static str {
    let kind = obj::obj_type_ptr(value);
    if kind.is_null() {
        "none"
    } else {
        // SAFETY: only exact immortal native type descriptors are observed.
        unsafe { std::ffi::CStr::from_ptr((*kind).name).to_str().unwrap() }
    }
}
fn refs(value: *mut TclObj) -> isize {
    // SAFETY: caller retains the original native header while observing it.
    unsafe { (*value).ref_count }
}
fn snap(rows: &mut String, case: usize, window: &str, value: *mut TclObj) {
    let (length, headers, cached) = physical_state(value)
        .map_or((-1, -1, false), |(length, headers, cached)| {
            (length as i64, headers as i64, cached)
        });
    writeln!(
        rows,
        "S\t{case}\t{window}\t{}\t{}\t{}\t{length}\t{headers}\t{}",
        descriptor(value),
        u8::from(obj::has_string_rep(value)),
        refs(value),
        u8::from(cached)
    )
    .unwrap();
}
const FIXTURES: [(TclVersion, &str, &str); 2] = [
    (
        TclVersion::V9_0,
        include_str!("../../testdata/native_arithseries/tcl9.0.tsv"),
        include_str!("../../testdata/native_arithseries/tcl9.0-loop.tsv"),
    ),
    (
        TclVersion::V9_1,
        include_str!("../../testdata/native_arithseries/tcl9.1.tsv"),
        include_str!("../../testdata/native_arithseries/tcl9.1-loop.tsv"),
    ),
];
const SCRIPTS: [&[u8]; 9] = [
    b"lseq 5",
    b"lseq 0",
    b"lseq -1",
    b"lseq 1 to 9 by 2",
    b"lseq 0 to 0.5 by 0.1",
    b"lseq 3 count 4 by 0",
    b"lseq 100000001",
    b"lseq 9223372036854775807 count 2",
    b"lseq 1 to 5 by -1",
];

#[test]
fn original_arithmetic_series_matches_all_196_native_storage_windows() {
    let mut windows = 0;
    for (version, expected, _) in FIXTURES {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let protocol = NativeStringProtocol::C(version);
        let mut observed = String::new();
        for (case, script) in SCRIPTS.into_iter().enumerate() {
            let code = interp.eval_str(script);
            writeln!(observed, "R\t{case}\t{}", code.as_int()).unwrap();
            if code != Code::Ok {
                writeln!(
                    observed,
                    "E\t{case}\t{}",
                    std::str::from_utf8(&interp.result_bytes()).unwrap()
                )
                .unwrap();
                continue;
            }
            let original = obj::Owned::retain(interp.result_obj());
            interp.set_result_bytes(b"");
            let value = original.as_ptr();
            snap(&mut observed, case, "original", value);
            let length = interp.list_len(&value).unwrap();
            writeln!(observed, "L\t{case}\t0\t{length}").unwrap();
            snap(&mut observed, case, "afterlength", value);
            let duplicate = obj::Owned::fresh(obj::duplicate(value));
            snap(&mut observed, case, "dup-original", value);
            snap(&mut observed, case, "duplicate", duplicate.as_ptr());
            if length > 0 {
                let a = interp.list_index(&duplicate.as_ptr(), 0).unwrap().unwrap();
                let b = interp.list_index(&duplicate.as_ptr(), 0).unwrap().unwrap();
                writeln!(
                    observed,
                    "I\t{case}\t{}\t{}\t{}\t{}",
                    descriptor(a),
                    u8::from(obj::has_string_rep(a)),
                    refs(a),
                    u8::from(a == b)
                )
                .unwrap();
                let spelling = crate::dict::native_object_bytes(a, protocol).unwrap();
                writeln!(
                    observed,
                    "V\t{case}\t{}",
                    std::str::from_utf8(&spelling).unwrap()
                )
                .unwrap();
                crate::interp::drop_fresh(a);
                crate::interp::drop_fresh(b);
                snap(&mut observed, case, "afterindex", duplicate.as_ptr());
            }
            drop(duplicate);
            snap(&mut observed, case, "afterdrop", value);
            if length < 10 {
                let elements = interp.list_elements(&value).unwrap();
                snap(&mut observed, case, "afterelements", value);
                if let Some(&first) = elements.first() {
                    let fresh = interp.list_index(&value, 0).unwrap().unwrap();
                    writeln!(
                        observed,
                        "C\t{case}\t{}\t{}\t{}",
                        refs(first),
                        refs(fresh),
                        u8::from(first == fresh)
                    )
                    .unwrap();
                    crate::interp::drop_fresh(fresh);
                }
                let spelling = crate::dict::native_object_bytes(value, protocol).unwrap();
                writeln!(
                    observed,
                    "T\t{case}\t{}",
                    std::str::from_utf8(&spelling).unwrap()
                )
                .unwrap();
                snap(&mut observed, case, "afterstring", value);
            }
        }
        assert_eq!(
            observed, expected,
            "original native {version:?} cache/residency/header/ref windows"
        );
        assert_eq!(observed.lines().count(), 98);
        windows += observed.lines().count();
    }
    assert_eq!(windows, 196);
}

struct Observer {
    case: usize,
    root: *mut TclObj,
    rows: RefCell<String>,
}
impl Observer {
    fn snap(&self, interp: &Interp, window: &str) {
        let member = interp.var_get(b"v").unwrap_or(core::ptr::null_mut());
        let (_, headers, cached) = physical_state(self.root).unwrap();
        writeln!(
            self.rows.borrow_mut(),
            "S\t{}\t{window}\t{}\t{}\t{}\t{headers}\t{}\t{}\t{}\t{}",
            self.case,
            descriptor(self.root),
            u8::from(obj::has_string_rep(self.root)),
            refs(self.root),
            u8::from(cached),
            if member.is_null() {
                "none"
            } else {
                descriptor(member)
            },
            u8::from(!member.is_null() && obj::has_string_rep(member)),
            if member.is_null() { -1 } else { refs(member) }
        )
        .unwrap();
    }
}
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp> for Observer {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interp: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        self.snap(interp, "write");
        Ok(())
    }
}
thread_local! { static OBSERVER: RefCell<Option<Rc<Observer>>> = const { RefCell::new(None) }; }
fn stop(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    OBSERVER.with(|slot| slot.borrow().as_ref().unwrap().snap(interp, "body"));
    interp.set_result_bytes(b"");
    Code::Break
}
#[test]
fn original_lazy_foreach_matches_all_36_native_windows_including_huge_break() {
    let mut windows = 0;
    for (version, _, expected) in FIXTURES {
        let mut observed = String::new();
        for (case, script) in [SCRIPTS[0], SCRIPTS[1], SCRIPTS[6], SCRIPTS[4]]
            .into_iter()
            .enumerate()
        {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.register_builtin(b"stop", stop);
            assert_eq!(interp.eval_str(script), Code::Ok);
            let root = obj::Owned::retain(interp.result_obj());
            interp.set_result_bytes(b"");
            let observer = Rc::new(Observer {
                case,
                root: root.as_ptr(),
                rows: RefCell::new(String::new()),
            });
            OBSERVER.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&observer)));
            let name = obj::Owned::fresh(obj::new_string_bytes(b"v"));
            let _watch = interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write],
                    observer.clone(),
                )
                .unwrap();
            let head = obj::Owned::fresh(obj::new_string_bytes(b"foreach"));
            let body = obj::Owned::fresh(obj::new_string_bytes(b"stop"));
            observer.snap(&interp, "before");
            let code =
                interp.dispatch(&[head.as_ptr(), name.as_ptr(), root.as_ptr(), body.as_ptr()]);
            observer.snap(&interp, "after");
            assert_eq!(code, Code::Ok);
            assert!(interp.native_access_refusal().is_none());
            let mut rows = observer.rows.borrow_mut();
            writeln!(
                rows,
                "R\t{case}\t0\t{}",
                std::str::from_utf8(&interp.result_bytes()).unwrap()
            )
            .unwrap();
            observed.push_str(&rows);
            OBSERVER.with(|slot| slot.borrow_mut().take());
        }
        assert_eq!(
            observed, expected,
            "native {version:?} lazy Length/Index/WRITE ownership"
        );
        assert_eq!(observed.lines().count(), 18);
        windows += observed.lines().count();
    }
    assert_eq!(windows, 36);
}

#[test]
fn arithmetic_elements_and_string_refusals_preserve_original_lazy_header() {
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    assert_eq!(interp.eval_str(b"lseq 100000001"), Code::Ok);
    let original = obj::Owned::retain(interp.result_obj());
    interp.set_result_bytes(b"");
    assert_eq!(interp.list_len(&original.as_ptr()).unwrap(), 100_000_001);
    for operation in [0, 1] {
        let error = if operation == 0 {
            interp.list_elements(&original.as_ptr()).unwrap_err()
        } else {
            crate::dict::native_object_bytes(
                original.as_ptr(),
                NativeStringProtocol::C(TclVersion::V9_0),
            )
            .unwrap_err()
        };
        assert!(matches!(error, ValueError::NativeMaterialization(_)));
        assert_eq!(
            physical_state(original.as_ptr()),
            Some((100_000_001, 1, false))
        );
        assert!(!obj::has_string_rep(original.as_ptr()));
    }
    assert!(
        capture_native_each_loop_abstract(original.as_ptr(), NativeStringProtocol::Jim084).is_err()
    );
    assert_eq!(
        physical_state(original.as_ptr()),
        Some((100_000_001, 1, false))
    );
}

#[test]
fn public_arithmetic_length_and_index_do_not_materialize_huge_series() {
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        for (script, expected) in [
            (
                b"set sequence [lseq 100000001]; llength $sequence".as_slice(),
                b"100000001".as_slice(),
            ),
            (b"lindex $sequence 0", b"0"),
            (b"lindex $sequence end", b"100000000"),
            (b"lindex $sequence {0 0}", b"0"),
            (b"lindex $sequence -1", b""),
        ] {
            assert_eq!(interp.eval_str(script), Code::Ok, "{script:?}");
            assert_eq!(interp.result_bytes(), expected);
            let sequence = interp.var_get(b"sequence").unwrap();
            assert!(!obj::has_string_rep(sequence));
            assert!(!physical_state(sequence).unwrap().2);
            assert!(interp.native_access_refusal().is_none());
        }
    }
}
