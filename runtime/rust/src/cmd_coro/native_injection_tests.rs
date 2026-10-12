// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite original native coroutine object/callback comparisons.
use super::*;
use crate::interp::obj_bytes;
const ROWS: &str = include_str!(
    "../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv"
);
type Original = obj::Owned;
type Call = Code;
type Context = Interp;
fn collector(interp: &mut Interp, argv: &[*mut TclObj], tag: &[u8]) -> Code {
    let mut log = interp
        .var_get(b"::injectionLog")
        .map(obj_bytes)
        .unwrap_or_default();
    if !log.is_empty() {
        log.push(b' ');
    }
    log.extend_from_slice(tag);
    let log = obj::Owned::fresh(obj::new_string_bytes(&log));
    interp.var_set(b"::injectionLog", log.as_ptr()).unwrap();
    if argv.len() >= 3 {
        interp
            .var_set(b"::injectionKind", argv[argv.len() - 2])
            .unwrap();
    }
    let last = *argv.last().unwrap();
    interp.var_set(b"::injectionLast", last).unwrap();
    interp.set_result(last);
    if tag == b"B"
        && interp
            .var_get(b"::injectionFailB")
            .map(obj_bytes)
            .as_deref()
            == Some(b"1".as_slice())
    {
        interp.set_result_bytes(b"B ERROR");
        Code::Error
    } else {
        Code::Ok
    }
}
fn collect(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    collector(interp, argv, b"COLLECT")
}
fn collect_a(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    collector(interp, argv, b"A")
}
fn collect_b(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    collector(interp, argv, b"B")
}
fn native() -> Context {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"collector\xff", collect);
    interp.register_builtin(b"A", collect_a);
    interp.register_builtin(b"B", collect_b);
    interp
}
fn object(bytes: &[u8]) -> Original {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn call_original(context: &mut Context, head: &Original, args: &[&Original]) -> Call {
    let mut argv = vec![head.as_ptr()];
    argv.extend(args.iter().map(|arg| arg.as_ptr()));
    context.dispatch(&argv)
}
fn call(context: &mut Context, head: &[u8], args: &[&Original]) -> Call {
    call_original(context, &object(head), args)
}
fn assert_row(
    context: &Context,
    call: &Call,
    argument: Option<&Original>,
    last: bool,
    label: &str,
) {
    let fields = row(label);
    let same = if last {
        context.var_get(b"::injectionLast") == Some(context.result_obj())
    } else {
        argument.is_some_and(|arg| context.result_obj() == arg.as_ptr())
    };
    assert_eq!(
        call.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{label}/code"
    );
    assert_eq!(same, fields[2] == "1", "{label}/identity");
    assert_eq!(context.result_bytes(), unhex(fields[3]), "{label}/bytes");
    assert!(!context.host_refusal_pending(), "{label}/host");
}
fn observed_kind(context: &Context) -> Vec<u8> {
    context.var_get(b"::injectionKind").map(obj_bytes).unwrap()
}
fn observed_log(context: &Context) -> Vec<String> {
    String::from_utf8(context.var_get(b"::injectionLog").map(obj_bytes).unwrap())
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
fn clear_log(context: &mut Context) {
    let empty = object(b"");
    context.var_set(b"::injectionLog", empty.as_ptr()).unwrap();
}
fn set_fail_b(context: &mut Context, fail: bool) {
    let value = object(if fail { b"1" } else { b"0" });
    context
        .var_set(b"::injectionFailB", value.as_ptr())
        .unwrap();
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
            assert_row(&context, &start, None, false, &format!("{prefix}_START"));
            let name = object(b"r\xff");
            let renamed = call(&mut context, b"rename", &[&object(b"c"), &name]);
            assert_row(&context, &renamed, None, false, &format!("{prefix}_RENAME"));
            let probe = call(
                &mut context,
                b"coroprobe",
                &[&name, &object(b"collector\xff"), &argument],
            );
            assert_row(
                &context,
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
                &context,
                &injected,
                Some(&argument),
                false,
                &format!("{prefix}_INJECT"),
            );
            let kind = call(&mut context, b"::tcl::unsupported::corotype", &[&name]);
            assert_row(&context, &kind, None, false, &format!("{prefix}_TYPE"));
            let resumed = call_original(&mut context, &name, &[&argument]);
            assert_row(
                &context,
                &resumed,
                Some(&argument),
                false,
                &format!("{prefix}_RESUME"),
            );
            assert_row(
                &context,
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
            assert_row(&context, &start, None, false, &format!("{prefix}_START"));
            clear_log(&mut context);
            for (head, value) in [(b"A".as_slice(), b"PA".as_slice()), (b"B", b"PB")] {
                let code = call(
                    &mut context,
                    b"coroinject",
                    &[&object(b"c"), &object(head), &object(value)],
                );
                assert_row(
                    &context,
                    &code,
                    None,
                    false,
                    &format!("{prefix}_QUEUE_{}", std::str::from_utf8(head).unwrap()),
                );
            }
            let resumed = call(&mut context, b"c", &[&object(b"RESUMED")]);
            assert_row(&context, &resumed, None, true, &format!("{prefix}_RESUME"));
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
