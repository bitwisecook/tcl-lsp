// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite original native coroutine object/callback comparisons.
use super::*;
use std::cell::RefCell;
use tcl_syntax::value::ValueOps;
const ROWS: &str =
    include_str!("../../../tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv");
type Original = Value;
type Call = Completion<Value>;
#[derive(Default)]
struct Observed {
    log: Vec<String>,
    kind: Vec<u8>,
    last: Option<Value>,
    fail_b: bool,
}
struct Collector {
    tag: &'static str,
    observed: Rc<RefCell<Observed>>,
}
impl NativeCommand for Collector {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let value = args.last().cloned().unwrap_or_else(Value::empty);
        let mut state = self.observed.borrow_mut();
        state.log.push(self.tag.to_owned());
        if args.len() >= 2 {
            state.kind = vm
                .native_string_bytes(&args[args.len() - 2])
                .unwrap()
                .to_vec();
        }
        state.last = Some(value.clone());
        if self.tag == "B" && state.fail_b {
            err("B ERROR")
        } else {
            ok(value)
        }
    }
}
struct Context {
    vm: Vm,
    observed: Rc<RefCell<Observed>>,
}
fn native() -> Context {
    let mut vm =
        crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let observed = Rc::new(RefCell::new(Observed::default()));
    for (name, tag) in [
        (b"collector\xff".as_slice(), "COLLECT"),
        (b"A", "A"),
        (b"B", "B"),
    ] {
        vm.register_command_in_slot(
            tcl_runtime_api::CommandSlot {
                namespace: vm.current_ns_id(),
                simple: name.into(),
            },
            Command::Native(Rc::new(Collector {
                tag,
                observed: Rc::clone(&observed),
            })),
        );
    }
    Context { vm, observed }
}
fn object(bytes: &[u8]) -> Original {
    Value::new_native_string_bytes(bytes)
}
fn call_original(context: &mut Context, head: &Original, args: &[&Original]) -> Call {
    context.vm.invoke_host_original_object_vector(
        head,
        &args.iter().map(|arg| (*arg).clone()).collect::<Vec<_>>(),
    )
}
fn call(context: &mut Context, head: &[u8], args: &[&Original]) -> Call {
    call_original(context, &object(head), args)
}
fn assert_row(
    context: &mut Context,
    call: &Call,
    argument: Option<&Original>,
    last: bool,
    label: &str,
) {
    let fields = row(label);
    let same = if last {
        context
            .observed
            .borrow()
            .last
            .as_ref()
            .is_some_and(|last| call.result.is_same_object(last))
    } else {
        argument.is_some_and(|arg| call.result.is_same_object(arg))
    };
    assert_eq!(
        call.code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{label}/code"
    );
    assert_eq!(same, fields[2] == "1", "{label}/identity");
    assert_eq!(
        context
            .vm
            .native_string_bytes(&call.result)
            .unwrap()
            .as_ref(),
        unhex(fields[3]),
        "{label}/bytes"
    );
    assert!(context.vm.refused_completion().is_none(), "{label}/host");
}
fn observed_kind(context: &Context) -> Vec<u8> {
    context.observed.borrow().kind.clone()
}
fn observed_log(context: &Context) -> Vec<String> {
    context.observed.borrow().log.clone()
}
fn clear_log(context: &mut Context) {
    context.observed.borrow_mut().log.clear();
}
fn set_fail_b(context: &mut Context, fail: bool) {
    context.observed.borrow_mut().fail_b = fail;
}

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn row(label: &str) -> Vec<&'static str> {
    ROWS.lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}
#[test]
fn original_coroutine_probe_and_injection_match_counted_object_controls() {
    // Native proof: naming.coroutine.original-probe-and-injection-object-identity
    // docs/design/analysis/name-resolution-proofs/coroutine-original-probe-and-injection-object-identity.md
    let mut comparisons = 0;
    for arbitrary in [false, true] {
        for case in ["RAW_ZERO", "ENCODED_ZERO", "OPAQUE_FF", "SURROGATE_D800"] {
            let prefix = format!(
                "TRANSPORT_{}_{}",
                if arbitrary { "YIELDTO" } else { "YIELD" },
                case
            );
            let mut context = native();
            let argument = object(&unhex(row(&format!("PUB_GLOBAL_{case}_INPUT"))[3]));
            let start = if arbitrary {
                call(
                    &mut context,
                    b"coroutine",
                    &[
                        &object(b"c"),
                        &object(b"yieldto"),
                        &object(b"collector\xff"),
                        &object(b"READY"),
                    ],
                )
            } else {
                call(
                    &mut context,
                    b"coroutine",
                    &[&object(b"c"), &object(b"yield"), &object(b"READY")],
                )
            };
            assert_row(
                &mut context,
                &start,
                None,
                false,
                &format!("{prefix}_START"),
            );
            let name = object(b"r\xff");
            let renamed = call(&mut context, b"rename", &[&object(b"c"), &name]);
            assert_row(
                &mut context,
                &renamed,
                None,
                false,
                &format!("{prefix}_RENAME"),
            );
            let probe = call(
                &mut context,
                b"coroprobe",
                &[&name, &object(b"collector\xff"), &argument],
            );
            assert_row(
                &mut context,
                &probe,
                Some(&argument),
                false,
                &format!("{prefix}_PROBE"),
            );
            let injected = call(
                &mut context,
                b"coroinject",
                &[&name, &object(b"collector\xff"), &argument],
            );
            assert_row(
                &mut context,
                &injected,
                Some(&argument),
                false,
                &format!("{prefix}_INJECT"),
            );
            let kind = call(&mut context, b"::tcl::unsupported::corotype", &[&name]);
            assert_row(&mut context, &kind, None, false, &format!("{prefix}_TYPE"));
            let resumed = call_original(&mut context, &name, &[&argument]);
            assert_row(
                &mut context,
                &resumed,
                Some(&argument),
                false,
                &format!("{prefix}_RESUME"),
            );
            assert_row(
                &mut context,
                &resumed,
                None,
                true,
                &format!("{prefix}_RESUME_CALLBACK_OBJECT"),
            );
            assert_eq!(
                observed_kind(&context),
                unhex(row(&format!("{prefix}_KIND_COLLECT"))[3])
            );
            comparisons += 7;
        }
    }
    assert_eq!(comparisons, 56);
}
#[test]
fn original_injection_order_and_error_continuation_match_native_callbacks() {
    // Native proof: naming.coroutine.original-injection-order-and-completion
    // docs/design/analysis/name-resolution-proofs/coroutine-original-injection-order-and-completion.md
    for arbitrary in [false, true] {
        for fail_b in [false, true] {
            let prefix = format!(
                "ORDER_{}_{}",
                if arbitrary { "YIELDTO" } else { "YIELD" },
                if fail_b { "B_ERROR" } else { "SUCCESS" }
            );
            let mut context = native();
            set_fail_b(&mut context, fail_b);
            let start = if arbitrary {
                call(
                    &mut context,
                    b"coroutine",
                    &[
                        &object(b"c"),
                        &object(b"yieldto"),
                        &object(b"collector\xff"),
                        &object(b"READY"),
                    ],
                )
            } else {
                call(
                    &mut context,
                    b"coroutine",
                    &[&object(b"c"), &object(b"yield"), &object(b"READY")],
                )
            };
            assert_row(
                &mut context,
                &start,
                None,
                false,
                &format!("{prefix}_START"),
            );
            clear_log(&mut context);
            for (head, value) in [(b"A".as_slice(), b"PA".as_slice()), (b"B", b"PB")] {
                let code = call(
                    &mut context,
                    b"coroinject",
                    &[&object(b"c"), &object(head), &object(value)],
                );
                assert_row(
                    &mut context,
                    &code,
                    None,
                    false,
                    &format!("{prefix}_QUEUE_{}", std::str::from_utf8(head).unwrap()),
                );
            }
            let resumed = call(&mut context, b"c", &[&object(b"RESUMED")]);
            assert_row(
                &mut context,
                &resumed,
                None,
                true,
                &format!("{prefix}_RESUME"),
            );
            let expected: Vec<_> = ROWS
                .lines()
                .filter_map(|line| {
                    let label = line.split('|').next()?;
                    label
                        .strip_prefix(&format!("{prefix}_CALLBACK_"))
                        .filter(|tag| *tag == "A" || *tag == "B")
                        .map(str::to_owned)
                })
                .collect();
            assert_eq!(observed_log(&context), expected, "{prefix}/order");
            assert_eq!(
                observed_kind(&context),
                unhex(row(&format!("{prefix}_KIND_A"))[3])
            );
        }
    }
}
