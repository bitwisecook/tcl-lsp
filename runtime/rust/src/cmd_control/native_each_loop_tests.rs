// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use std::{cell::RefCell, rc::Rc};
use tcl_syntax::value::ValueOps;
const FIXTURES: [(&str, &str, &str); 6] = [
    (
        "tcl8.4",
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt"
        ),
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt"
        ),
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt"
        ),
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt"
        ),
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt"
        ),
        include_str!(
            "../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt"
        ),
    ),
    (
        "jim",
        include_str!("../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt"),
        include_str!("../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt"),
    ),
];
struct Inputs {
    argv: Vec<Owned>,
    roots: [*mut TclObj; 3],
    name: *mut TclObj,
    member: *mut TclObj,
    _external: Vec<Owned>,
}
fn inputs(i: &mut Interp, case: usize, shimmer: bool) -> Inputs {
    let string = |bytes: &[u8]| obj::new_string_bytes(bytes);
    let names = [string(b"v"), string(b"w")];
    let mut members = [string(b"A"), string(b"B"), string(b"C")];
    if case == 8 {
        crate::interp::drop_fresh(members[0]);
        let child = string(b"N");
        members[0] = i.new_list_object(&[child]);
    }
    let variables = i.new_list_object(&names[..if case == 2 { 2 } else { 1 }]);
    let count = match case {
        2 => 3,
        5 => 2,
        3 | 4 => 0,
        _ => 1,
    };
    let values = i.new_list_object(&members[..count]);
    for &name in &names[if case == 2 { 2 } else { 1 }..] {
        crate::interp::drop_fresh(name);
    }
    for &member in &members[count..] {
        crate::interp::drop_fresh(member);
    }
    let body = string(if matches!(case, 3 | 4) {
        b"{"
    } else {
        b"observe"
    });
    let command = string(if matches!(case, 4 | 5) {
        b"lmap"
    } else {
        b"foreach"
    });
    let mut name = names[0];
    let mut member = if matches!(case, 3 | 4) {
        core::ptr::null_mut()
    } else {
        members[0]
    };
    let mut argv = vec![command, variables, values, body];
    if matches!(case, 6 | 7) {
        crate::interp::drop_fresh(values);
        let empty = i.new_list_object(&[]);
        argv = if case == 6 {
            vec![
                command,
                variables,
                string(b"{"),
                empty,
                i.new_list_object(&[]),
                body,
            ]
        } else {
            name = core::ptr::null_mut();
            vec![
                command,
                empty,
                string(b"{"),
                variables,
                i.new_list_object(&[]),
                body,
            ]
        };
        member = core::ptr::null_mut();
    }
    if case == 9 {
        crate::interp::drop_fresh(variables);
        argv[1] = i.new_list_object(&[]);
        name = core::ptr::null_mut();
    }
    let roots = [argv[1], argv[2], body];
    let argv = argv.into_iter().map(Owned::fresh).collect::<Vec<_>>();
    let external = if shimmer {
        [name, member]
            .into_iter()
            .filter(|value| !value.is_null())
            .map(Owned::retain)
            .collect()
    } else {
        Vec::new()
    };
    if case == 1 {
        for root in &roots[..2] {
            i.native_string_bytes(root).unwrap();
        }
    }
    Inputs {
        argv,
        roots,
        name,
        member,
        _external: external,
    }
}
fn object(value: *mut TclObj, engine: &str) -> String {
    if value.is_null() {
        return "absent,-1,0,-1".into();
    }
    // SAFETY: every observed pointer is a declared original/header/cell/result
    // owner, or is protected by the shimmer producer's external pin.
    let (name, refs) = unsafe {
        let kind = obj::obj_type_ptr(value);
        let name = if kind.is_null() {
            "none"
        } else {
            std::ffi::CStr::from_ptr((*kind).name).to_str().unwrap()
        };
        (name, (*value).ref_count)
    };
    let backing = if matches!(engine, "tcl8.4" | "jim") {
        -1
    } else {
        crate::list::native_header_reference_count(value)
            .expect("actual List header ownership")
            .map_or(-1, |count| i64::try_from(count).unwrap())
    };
    format!(
        "{name},{refs},{},{backing}",
        u8::from(obj::has_string_rep(value))
    )
}
struct Observer {
    roots: [*mut TclObj; 3],
    name: *mut TclObj,
    member: *mut TclObj,
    engine: &'static str,
    case: usize,
    physical: bool,
    shimmer: bool,
    entered: RefCell<bool>,
    rows: RefCell<Vec<String>>,
}
impl Observer {
    fn snapshot(&self, i: &Interp, window: &str, cell: *mut TclObj) {
        let fields = [
            object(self.roots[0], self.engine),
            object(self.roots[1], self.engine),
            object(self.name, self.engine),
            object(self.member, self.engine),
            object(self.roots[2], self.engine),
            object(cell, self.engine),
            u8::from(!cell.is_null() && cell == self.member).to_string(),
            object(i.get_obj_result(), self.engine),
        ];
        let mut rows = self.rows.borrow_mut();
        let seq = rows.len();
        rows.push(format!(
            "S\t{}\t{seq}\t{window}\t{}",
            self.case,
            fields.join("\t")
        ));
    }
}
thread_local! {static OBSERVER:RefCell<Option<Rc<Observer>>>=const{RefCell::new(None)};}
fn observe(i: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    callback(i, false)
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
        let cell = interp.var_get(b"v").unwrap_or(core::ptr::null_mut());
        if self.physical {
            self.snapshot(interp, "write", cell);
        }
        Ok(())
    }
}
fn callback(i: &mut Interp, write: bool) -> Code {
    let observer = OBSERVER.with(|slot| slot.borrow().as_ref().unwrap().clone());
    let cell = i.var_get(b"v").unwrap_or(core::ptr::null_mut());
    if observer.physical {
        observer.snapshot(i, if write { "write" } else { "body" }, cell);
    }
    if !write && observer.shimmer && !observer.entered.replace(true) {
        for root in [observer.roots[1], observer.roots[0]] {
            if let Err(error) = i.native_char_len(&root) {
                return i.report_cmd_error(error.into());
            }
        }
        if observer.physical {
            observer.snapshot(i, "shimmer", cell);
        }
    }
    i.set_result_bytes(if write { b"" } else { b"BODY" });
    Code::Ok
}
fn unhex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn run(physical: bool) -> (usize, usize) {
    let mut completions = 0;
    let mut windows = 0;
    for (engine, original, shimmer_fixture) in FIXTURES {
        for (shimmer, fixture) in [(false, original), (true, shimmer_fixture)] {
            for case in 0..10 {
                let case_label = format!("{case}/shimmer={shimmer}/physical={physical}");
                tcl_test_support::oracle_row_progress(
                    "generic-each-loop",
                    engine,
                    &case_label,
                    None,
                );
                let phase = std::time::Instant::now();
                let profile = crate::environment::profile_for_dialect(engine);
                let mut interp = {
                    let interp = Interp::with_native_core(
                        crate::interp::default_host(),
                        profile,
                        tcl_registry::special_vars::NativeBootstrapInputs {
                            package_path: Vec::new(),
                            default_library: None,
                        },
                    )
                    .expect("actual profile selected before native core bootstrap");
                    if engine != "jim" {
                        assert!(
                            interp
                                .native_compiler_cache_epochs(crate::namespace::GLOBAL)
                                .is_some(),
                            "{engine}: authentic compiler/resolver stamps"
                        );
                    }
                    interp
                };
                interp.register_builtin(b"observe", observe);
                let inputs = inputs(&mut interp, case, shimmer);
                let observer = Rc::new(Observer {
                    roots: inputs.roots,
                    name: inputs.name,
                    member: inputs.member,
                    engine,
                    case,
                    physical,
                    shimmer,
                    entered: RefCell::new(false),
                    rows: RefCell::new(Vec::new()),
                });
                OBSERVER.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&observer)));
                let _native_observer = if engine != "jim" {
                    let name = Owned::fresh(obj::new_string_bytes(b"v"));
                    Some(interp.add_native_variable_observer(name.as_ptr(),
                        &[tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write],
                        observer.clone()).expect("actual direct native variable observer"))
                } else {
                    None
                };

                if physical {
                    observer.snapshot(&interp, "before", core::ptr::null_mut());
                }
                let argv = inputs.argv.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{case} genuine native each-loop execution"
                );
                if physical {
                    observer.snapshot(&interp, "after", core::ptr::null_mut());
                }
                let prefix = format!("R\t{case}\t");
                let row = fixture
                    .lines()
                    .find(|row| row.starts_with(&prefix))
                    .unwrap();
                let fields = row.split('\t').collect::<Vec<_>>();
                assert_eq!(
                    code.as_int(),
                    fields[2].parse::<i64>().unwrap(),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(fields[3]),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                completions += 1;
                if physical {
                    let prefix = format!("S\t{case}\t");
                    let expected = fixture
                        .lines()
                        .filter(|row| row.starts_with(&prefix))
                        .collect::<Vec<_>>();
                    let actual = observer.rows.borrow();
                    assert_eq!(
                        actual.len(),
                        expected.len(),
                        "{engine}/{case}/shimmer={shimmer}"
                    );
                    for (actual, expected) in actual.iter().zip(expected) {
                        assert_eq!(actual, expected, "{engine}/{case}/shimmer={shimmer}");
                        windows += 1;
                    }
                }
                OBSERVER.with(|slot| slot.borrow_mut().take());
                tcl_test_support::oracle_phase_progress(
                    "generic-each-loop",
                    engine,
                    &case_label,
                    "compare",
                    phase,
                );
                tcl_test_support::oracle_row_progress(
                    "generic-each-loop",
                    engine,
                    &case_label,
                    Some(completions),
                );
            }
        }
    }
    (completions, windows)
}
#[test]
fn generic_each_loops_match_all_120_original_native_completions() {
    assert_eq!(run(false), (120, 0));
}
#[test]
fn generic_each_loops_match_all_406_original_native_physical_windows() {
    assert_eq!(run(true), (120, 406));
}
