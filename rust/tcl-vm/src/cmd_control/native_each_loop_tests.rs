// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::command::NativeCommand;
use crate::{NativeObjectLifetimeLease, Value, Vm};
use std::{cell::RefCell, rc::Rc};
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::value::ValueOps;

const FIXTURES: [(&str, &str, &str); 6] = [
    (
        "tcl8.4",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt"),
    ),
    (
        "jim",
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/original/jim.txt"),
        include_str!("../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt"),
    ),
];
struct Inputs {
    argv: Vec<Value>,
    roots: [NativeObjectLifetimeLease; 3],
    name: Option<NativeObjectLifetimeLease>,
    member: Option<NativeObjectLifetimeLease>,
    _external_pins: Vec<Value>,
}
fn inputs(vm: &mut Vm, case: usize, shimmer: bool) -> Inputs {
    let strings = vm
        .native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let string = |bytes: &[u8]| Value::new_native_string_bytes(bytes);
    let list = |values| Value::native_list_constructor(values, strings);
    let mut names = vec![string(b"v"), string(b"w")];
    let name = names[0].native_lifetime_lease();
    names.truncate(if case == 2 { 2 } else { 1 });
    let mut members = vec![string(b"A"), string(b"B"), string(b"C")];
    if case == 8 {
        members[0] = list(vec![string(b"N")]);
    }
    let member = members[0].native_lifetime_lease();
    members.truncate(match case {
        2 => 3,
        5 => 2,
        3 | 4 => 0,
        _ => 1,
    });
    let variables = list(names);
    let values = list(members);
    let body = string(if matches!(case, 3 | 4) {
        b"{"
    } else {
        b"observe"
    });
    let mut argv = vec![variables, values, body];
    let mut name = Some(name);
    let mut member = Some(member);
    if matches!(case, 3 | 4) {
        member = None;
    }
    if matches!(case, 6 | 7) {
        let variables = argv.remove(0);
        let body = argv.pop().unwrap();
        argv.clear();
        let empty = list(Vec::new());
        argv = if case == 6 {
            vec![variables, string(b"{"), empty, list(Vec::new()), body]
        } else {
            name = None;
            vec![empty, string(b"{"), variables, list(Vec::new()), body]
        };
        member = None;
    }
    if case == 9 {
        argv[0] = list(Vec::new());
        name = None;
    }
    let roots = [
        argv[0].native_lifetime_lease(),
        argv[1].native_lifetime_lease(),
        argv.last().unwrap().native_lifetime_lease(),
    ];
    let mut external = Vec::new();
    if shimmer {
        for value in [&name, &member].into_iter().flatten() {
            external.push(value.value().clone());
        }
    }
    if case == 1 {
        for root in &roots[..2] {
            vm.native_string_bytes(root.value()).unwrap();
        }
    }
    Inputs {
        argv,
        roots,
        name,
        member,
        _external_pins: external,
    }
}
fn object(value: Option<&Value>, engine: &str) -> String {
    let Some(value) = value else {
        return "absent,-1,0,-1".into();
    };
    let backing = if matches!(engine, "tcl8.4" | "jim") {
        -1
    } else {
        value.cached_list_representation().map_or(-1, |(items, _)| {
            i64::try_from(items.native_header_reference_count()).unwrap()
        })
    };
    format!(
        "{},{},{},{}",
        value.native_object_type_name(),
        value.native_object_reference_count(),
        u8::from(value.resident_string_bytes().is_some()),
        backing
    )
}
struct Observer {
    roots: [NativeObjectLifetimeLease; 3],
    name: Option<NativeObjectLifetimeLease>,
    member: Option<NativeObjectLifetimeLease>,
    engine: &'static str,
    case: usize,
    shimmer: bool,
    physical: bool,
    entered: RefCell<bool>,
    rows: RefCell<Vec<String>>,
}
impl Observer {
    fn snapshot(
        &self,
        vm: &Vm,
        window: &str,
        cell: Option<&Value>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let result = vm.with_native_interp_result(|result| object(Some(result), self.engine))?;
        let fields = [
            object(Some(self.roots[0].value()), self.engine),
            object(Some(self.roots[1].value()), self.engine),
            object(
                self.name.as_ref().map(NativeObjectLifetimeLease::value),
                self.engine,
            ),
            object(
                self.member.as_ref().map(NativeObjectLifetimeLease::value),
                self.engine,
            ),
            object(Some(self.roots[2].value()), self.engine),
            object(cell, self.engine),
            u8::from(
                cell.zip(self.member.as_ref())
                    .is_some_and(|(cell, member)| cell.is_same_object(member.value())),
            )
            .to_string(),
            result,
        ];
        let mut rows = self.rows.borrow_mut();
        let seq = rows.len();
        rows.push(format!(
            "S\t{}\t{seq}\t{window}\t{}",
            self.case,
            fields.join("\t")
        ));
        Ok(())
    }
}
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Vm> for Observer {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        vm: &mut Vm,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        let observed = vm
            .get_var_bytes(b"v")
            .map(|value| value.native_lifetime_lease());
        if self.physical {
            self.snapshot(
                vm,
                "write",
                observed.as_ref().map(NativeObjectLifetimeLease::value),
            )?;
        }
        Ok(())
    }
}
struct Observe(Rc<Observer>);
impl NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        let observed = vm
            .get_var_bytes(b"v")
            .map(|value| value.native_lifetime_lease());
        if self.0.physical
            && let Err(error) = self.0.snapshot(
                vm,
                "body",
                observed.as_ref().map(NativeObjectLifetimeLease::value),
            )
        {
            return crate::command::completion_from_cmd_error(vm, error.into());
        }
        if self.0.shimmer && !self.0.entered.replace(true) {
            for root in [&self.0.roots[1], &self.0.roots[0]] {
                if let Err(error) = vm.native_char_len(root.value()) {
                    return crate::command::completion_from_cmd_error(vm, error.into());
                }
            }
            if self.0.physical
                && let Err(error) = self.0.snapshot(
                    vm,
                    "shimmer",
                    observed.as_ref().map(NativeObjectLifetimeLease::value),
                )
            {
                return crate::command::completion_from_cmd_error(vm, error.into());
            }
        }
        crate::interp::ok(Value::new_native_string_bytes(b"BODY".as_slice()))
    }
}
fn unhex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn compare_each_physical_windows(
    observer: &Observer,
    fixture: &str,
    engine: &str,
    case: usize,
    shimmer: bool,
) -> usize {
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
    let windows = expected.len();
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual, expected, "{engine}/{case}/shimmer={shimmer}");
    }
    windows
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
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile);
                let inputs = inputs(&mut vm, case, shimmer);
                let observer = Rc::new(Observer {
                    roots: inputs.roots.clone(),
                    name: inputs.name.clone(),
                    member: inputs.member.clone(),
                    engine,
                    case,
                    shimmer,
                    physical,
                    entered: RefCell::new(false),
                    rows: RefCell::new(Vec::new()),
                });
                vm.register_native_command("observe", Rc::new(Observe(Rc::clone(&observer))));
                let _native_observer = if engine == "jim" {
                    None
                } else {
                    Some(vm.add_native_variable_observer(
                        &Value::new_native_string_bytes(b"v".as_slice()),
                        &[tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write],
                        observer.clone(),
                    ).expect("actual direct native variable observer"))
                };
                if physical {
                    observer
                        .snapshot(&vm, "before", None)
                        .expect("actual original interpreter result observer");
                }
                let completion = vm
                    .try_invoke_command(
                        if matches!(case, 4 | 5) {
                            "lmap"
                        } else {
                            "foreach"
                        },
                        &inputs.argv,
                    )
                    .expect("actual native each-loop command execution");
                if physical {
                    observer
                        .snapshot(&vm, "after", None)
                        .expect("actual original interpreter result observer");
                }
                let prefix = format!("R\t{case}\t");
                let row = fixture
                    .lines()
                    .find(|row| row.starts_with(&prefix))
                    .unwrap();
                let fields = row.split('\t').collect::<Vec<_>>();
                assert_eq!(
                    completion.code.as_int(),
                    fields[2].parse::<i64>().unwrap(),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    unhex(fields[3]).as_slice(),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                completions += 1;
                if physical {
                    windows +=
                        compare_each_physical_windows(&observer, fixture, engine, case, shimmer);
                }
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

#[test]
fn original_procedure_bytecode_reconciles_script_and_procedure_contexts() {
    use crate::value::NativeBytecodeContext;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    let body = Value::new_native_string_bytes(b"set x".as_slice());
    let declaration = [Value::string("p"), Value::string("x"), body.clone()];
    assert_eq!(
        vm.try_invoke_command("proc", &declaration).unwrap().code,
        Code::Ok
    );
    assert!(body.native_bytecode_cache().is_none());
    let declaration_owner = vm.proc_def("p").unwrap();
    let source = declaration_owner.body_src.native_lifetime_lease();
    let first = vm.try_invoke_command("p", &[Value::string("ONE")]).unwrap();
    assert_eq!(first.code, Code::Ok);
    assert_eq!(
        vm.native_string_bytes(&first.result).unwrap().as_ref(),
        b"ONE"
    );
    let first_cache = source.value().native_bytecode_cache().unwrap();
    assert!(first_cache.context == NativeBytecodeContext::Procedure);
    let script = vm
        .try_invoke_command("eval", &[source.value().clone()])
        .unwrap();
    assert_eq!(script.code, Code::Error);
    let script_cache = source.value().native_bytecode_cache().unwrap();
    assert!(script_cache.context == NativeBytecodeContext::Script);
    assert!(!Rc::ptr_eq(&first_cache.unit.asm, &script_cache.unit.asm));
    let second = vm.try_invoke_command("p", &[Value::string("TWO")]).unwrap();
    assert_eq!(second.code, Code::Ok);
    assert_eq!(
        vm.native_string_bytes(&second.result).unwrap().as_ref(),
        b"TWO"
    );
    let second_cache = source.value().native_bytecode_cache().unwrap();
    assert!(second_cache.context == NativeBytecodeContext::Procedure);
    assert!(!Rc::ptr_eq(&first_cache.unit.asm, &second_cache.unit.asm));
}
