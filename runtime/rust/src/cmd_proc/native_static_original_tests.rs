// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native original-list static declarations and retained value identity.

use crate::{
    interp::{Code, Interp},
    obj::{self, Owned},
};
use tcl_syntax::value::ValueOps;

fn instance(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn string(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn list(interp: &Interp, members: &[&Owned]) -> Owned {
    Owned::fresh(
        interp.new_list_object(
            &members
                .iter()
                .map(|object| object.as_ptr())
                .collect::<Vec<_>>(),
        ),
    )
}
fn call(interp: &mut Interp, head: &[u8], arguments: &[&Owned]) -> Code {
    let head = string(head);
    let mut argv = vec![head.as_ptr()];
    argv.extend(arguments.iter().map(|object| object.as_ptr()));
    interp.dispatch(&argv)
}
fn assert_row(
    interp: &mut Interp,
    code: Code,
    expected: Option<&Owned>,
    captured: &str,
    label: &str,
) {
    let native = row(captured, label);
    let same = expected.is_some_and(|value| interp.get_obj_result() == value.as_ptr());
    assert_eq!(code.as_int(), native[1].parse::<i64>().unwrap(), "{label}");
    assert_eq!(same, native[2] == "1", "{label}");
    let result = interp.get_obj_result();
    assert_eq!(
        ValueOps::native_string_bytes(interp, &result)
            .unwrap()
            .as_ref(),
        unhex(native[3]),
        "{label}"
    );
    assert!(!interp.host_refusal_pending(), "{label}");
}
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn row<'a>(captured: &'a str, label: &str) -> Vec<&'a str> {
    captured
        .lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn original_static_member_objects_match_native_counted_names_and_value_capture() {
    // naming.procedure-static.original-member-object-boundaries
    // docs/design/analysis/name-resolution-proofs/procedure-static-original-member-object-boundaries.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in captured.lines().filter(|line| line.starts_with("INPUT|")) {
            let fields: Vec<_> = input.split('|').collect();
            let label = fields[1];
            let bytes = unhex(fields[2]);
            let mut interp = instance(engine);
            let name = string(&bytes);
            let member = string(b"VALUE");
            let literal = list(&interp, &[&member]);
            let specifier = list(&interp, &[&name, &literal]);
            let statics = list(&interp, &[&specifier]);
            let command = string(b"p");
            let parameters = string(b"");
            let mut body = b"return ${".to_vec();
            body.extend_from_slice(&bytes);
            body.push(b'}');
            let body = string(&body);
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let entered = result == Code::Ok;
            assert_row(
                &mut interp,
                result,
                None,
                captured,
                &format!("{label}_LITERAL_DEFINE"),
            );
            if entered {
                let result = call(&mut interp, b"p", &[]);
                assert_row(
                    &mut interp,
                    result,
                    Some(&literal),
                    captured,
                    &format!("{label}_LITERAL_CALL"),
                );
            }
        }
        for reference in [false, true] {
            let mut interp = instance(engine);
            let name = string(b"x\xff");
            let first = string(b"FIRST");
            let result = call(&mut interp, b"set", &[&name, &first]);
            assert_row(&mut interp, result, None, captured, "SOURCE_SET");
            let specifier = string(if reference { b"&x\xff" } else { b"x\xff" });
            let statics = list(&interp, &[&specifier]);
            let command = string(b"p");
            let parameters = string(b"");
            let body = string(b"return ${x\xff}");
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let entered = result == Code::Ok;
            let label = if reference { "REFERENCE" } else { "COPY" };
            assert_row(
                &mut interp,
                result,
                None,
                captured,
                &format!("{label}_DEFINE"),
            );
            if entered {
                let second = string(b"SECOND");
                let result = call(&mut interp, b"set", &[&name, &second]);
                assert_row(&mut interp, result, None, captured, "SOURCE_REPLACE");
                let result = call(&mut interp, b"p", &[]);
                assert_row(
                    &mut interp,
                    result,
                    Some(if reference { &second } else { &first }),
                    captured,
                    &format!("{label}_CALL"),
                );
            }
        }
        for duplicate in [false, true] {
            let mut interp = instance(engine);
            let name = string(b"x\xff");
            let value = string(b"V");
            let specifier = if duplicate {
                list(&interp, &[&name, &value])
            } else {
                name.clone()
            };
            let statics = if duplicate {
                list(&interp, &[&specifier, &specifier])
            } else {
                list(&interp, &[&specifier])
            };
            let command = string(b"p");
            let parameters = string(b"");
            let body = string(b"return ${x\xff}");
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let label = if duplicate { "DUPLICATE" } else { "MISSING" };
            assert_row(
                &mut interp,
                result,
                None,
                captured,
                &format!("{label}_DEFINE"),
            );
            let result = call(&mut interp, b"info", &[&string(b"commands"), &command]);
            assert_row(
                &mut interp,
                result,
                None,
                captured,
                &format!("{label}_COMMAND"),
            );
        }
    }
}

#[test]
fn original_jim_static_links_match_native_frame_storage_reuse_boundaries() {
    // naming.procedure-static.jim-original-link-frame-storage
    // docs/design/analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md
    // naming.procedure-static.jim-native-link-frame-reuse-boundaries
    // docs/design/analysis/name-resolution-proofs/procedure-static-jim-native-link-frame-reuse-boundaries.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_jim_link_frame_storage/jim/stdout.tsv"
            ),
        ),
    ] {
        let rows = captured.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 18, "{engine}");
        for rows in rows.chunks_exact(3) {
            assert!(rows[0].starts_with("VERSION|0|"), "{engine}");
            let input = rows[1].split('|').collect::<Vec<_>>();
            let expected = rows[2].split('|').collect::<Vec<_>>();
            assert_eq!(input[0], "INPUT");
            assert_eq!(input[1], expected[0]);
            let source = String::from_utf8(unhex(input[2])).unwrap();
            let mut interp = instance(engine);
            let code = interp.eval_str(source.as_bytes());
            assert_eq!(
                code.as_int(),
                expected[1].parse::<i64>().unwrap(),
                "{engine}: {}",
                expected[0]
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(expected[2]),
                "{engine}: {}",
                expected[0]
            );
            assert!(!interp.host_refusal_pending(), "{engine}: {}", expected[0]);
        }
    }
}
