// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Script argv and in-use windows without observer native pins.

use super::*;
use crate::{NativeObjectLifetimeLease, command::NativeCommand, interp::Vm};
use std::{cell::RefCell, fmt::Write};
use tcl_runtime_api::{Code, Completion};

struct Observer {
    parent: NativeObjectLifetimeLease,
    backing: Rc<NativeJimScript>,
    case: usize,
    calls: usize,
    rows: String,
}
struct Inspect(Rc<RefCell<Observer>>);
fn kind(value: &Value) -> &'static str {
    match value.native_object_type_name() {
        "none" => "NULL",
        name => name,
    }
}
impl NativeCommand for Inspect {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let control = {
            let mut observer = self.0.borrow_mut();
            observer.calls += 1;
            let case = observer.case;
            let call = observer.calls;
            let backing = Rc::clone(&observer.backing);
            let invocation = vm
                .jim_errors
                .frames
                .last()
                .expect("genuine active invocation")
                .invocation
                .cached_list_representation()
                .expect("original argv backing")
                .0;
            assert_eq!(invocation.len(), args.len() + 1);
            for (original, argument) in invocation.iter().skip(1).zip(args) {
                assert!(original.is_same_object(argument));
            }
            writeln!(
                observer.rows,
                "CALL\t{case}\t{call}\t{}\t{}\t{}",
                invocation.len(),
                backing.in_use.get(),
                backing.ordinary().unwrap().linenr.get()
            )
            .unwrap();
            for (index, value) in invocation.iter().enumerate() {
                let token = backing
                    .ordinary()
                    .unwrap()
                    .objects
                    .tokens()
                    .iter()
                    .rposition(|token| token.value.is_same_object(value))
                    .map_or(-1, |index| i32::try_from(index).unwrap());
                writeln!(
                    observer.rows,
                    "ARGV\t{case}\t{call}\t{index}\t{token}\t{}\t{}\t{}",
                    kind(value),
                    usize::from(value.resident_string_bytes().is_some()),
                    value.native_object_reference_count()
                )
                .unwrap();
            }
            match case {
                5 => 1,
                6 => 2,
                7 => 3,
                _ => 0,
            }
        };
        if control != 0 {
            let observer = self.0.borrow();
            vm.native_object_list_elements_in(
                observer.parent.value(),
                tcl_syntax::native_string::NativeStringProtocol::Jim084,
            )
            .unwrap();
            drop(observer);
            let mut observer = self.0.borrow_mut();
            let (case, parent_kind, in_use) = (
                observer.case,
                kind(observer.parent.value()),
                observer.backing.in_use.get(),
            );
            writeln!(observer.rows, "SHIMMER\t{case}\t{parent_kind}\t{in_use}").unwrap();
        }
        let result = args.first().cloned().unwrap_or_else(|| {
            vm.native_jim_object_context()
                .unwrap()
                .empty_object()
                .clone()
        });
        Completion::new(
            match control {
                2 => Code::Error,
                3 => Code::Return,
                _ => Code::Ok,
            },
            result,
            Value::empty(),
        )
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
    let mut transcript = String::new();
    for (case, source) in sources.into_iter().enumerate() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let context = vm.native_jim_object_context().unwrap();
        vm.set_var_bytes(b"v", Value::new_native_string_bytes(b"V".as_slice()))
            .unwrap();
        let filename = Value::new_native_string_bytes(b"FILE".as_slice());
        let parent = Value::new_native_string_bytes(source);
        parent
            .install_native_jim_source(
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
        let backing = parent
            .prepare_native_jim_script(&context, vm.lexer_config())
            .unwrap();
        let script = backing.ordinary().unwrap();
        writeln!(
            transcript,
            "BEFORE\t{case}\t{}\t{}\t{}\t{}",
            script.objects.tokens().len(),
            script.objects.missing.map_or(32, i32::from),
            script.linenr.get(),
            backing.in_use.get()
        )
        .unwrap();
        let observer = Rc::new(RefCell::new(Observer {
            parent: parent.native_lifetime_lease(),
            backing: Rc::clone(&backing),
            case,
            calls: 0,
            rows: String::new(),
        }));
        vm.register_native_command("inspect", Rc::new(Inspect(Rc::clone(&observer))));
        let completion = vm.eval_value_at_level(vm.current_level(), &parent);
        let code = completion.code;
        // Native post-evaluation window observes the real interpreter result,
        // after Rust completion transport ownership has left the window.
        drop(completion);
        let observer = observer.borrow();
        transcript.push_str(&observer.rows);
        let same = matches!(&*parent.0.intrep.borrow(),IntRep::JimScript(header) if Rc::ptr_eq(&header.0,&backing));
        let empty = vm
            .with_native_interp_result(|result| result.is_same_object(&context.empty_object()))
            .unwrap();
        writeln!(
            transcript,
            "AFTER\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            code.as_int(),
            observer.calls,
            kind(&parent),
            usize::from(same),
            backing.in_use.get(),
            script.linenr.get(),
            usize::from(empty)
        )
        .unwrap();
    }
    let expected =
        include_str!("../../tcl-syntax/testdata/native_jim_script_execution/eval_observations.tsv");
    assert_eq!(expected.lines().count(), 51);
    assert_eq!(transcript, expected);
}
