// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original counted formal/body/argument comparisons with retained result identity.

use crate::{
    interp::{Code, Interp},
    obj::{self, Owned},
};

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn call(interp: &mut Interp, head: &[u8], arguments: &[*mut obj::TclObj]) -> Code {
    let head = Owned::fresh(obj::new_string_bytes(head));
    let mut words = vec![head.as_ptr()];
    words.extend_from_slice(arguments);
    interp.dispatch(&words)
}

fn assert_row(
    interp: &Interp,
    code: Code,
    argument: Option<*mut obj::TclObj>,
    rows: &str,
    label: &str,
) {
    assert!(
        !interp.host_refusal_pending(),
        "{label}: native access {:?}, compilation admission {:?}",
        interp.native_access_refusal(),
        interp.native_compilation_admission_error(),
    );
    let fields: Vec<_> = rows
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect();
    // Sample the retained result object before its only string getter.
    let same = argument.is_some_and(|argument| interp.get_obj_result() == argument);
    assert_eq!(code.as_int(), fields[1].parse::<i64>().unwrap(), "{label}");
    assert_eq!(same, fields[2] == "1", "{label}");
    assert_eq!(interp.result_bytes(), unhex(fields[3]), "{label}");
    assert!(!interp.host_refusal_pending(), "{label}");
}

#[test]
fn counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries() {
    // Native proof: naming.formal.whole-object-return-renaming
    // docs/design/analysis/name-resolution-proofs/formal-whole-object-return-renaming.md
    // Native proof: naming.formal.raw-zero-list-and-binding-renaming
    // docs/design/analysis/name-resolution-proofs/formal-raw-zero-list-and-binding-renaming.md
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|row| row.starts_with("INPUT|")) {
            let fields: Vec<_> = input.split('|').collect();
            let name = fields[1];
            let parameter = unhex(fields[2]);
            let body = unhex(fields[3]);
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            for (head, parameter, body, label) in [
                (
                    b"original".as_slice(),
                    parameter.as_slice(),
                    body.as_slice(),
                    format!("{name}_DECLARE_ORIGINAL"),
                ),
                (
                    b"renamed".as_slice(),
                    b"a".as_slice(),
                    b"return $a".as_slice(),
                    format!("{name}_DECLARE_RENAMED"),
                ),
            ] {
                let original =
                    [head, parameter, body].map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
                let code = call(
                    &mut interp,
                    b"proc",
                    &original.each_ref().map(Owned::as_ptr),
                );
                assert_row(&interp, code, None, rows, &label);
            }
            let prefix = format!("VALUE|{name}_");
            for value in rows.lines().filter(|row| row.starts_with(&prefix)) {
                let fields: Vec<_> = value.split('|').collect();
                let bytes = unhex(fields[2]);
                let argument = Owned::fresh(obj::new_string_bytes(&bytes));
                for (head, suffix) in [
                    (b"original".as_slice(), "ORIGINAL"),
                    (b"renamed".as_slice(), "RENAMED"),
                ] {
                    let code = call(&mut interp, head, &[argument.as_ptr()]);
                    assert_row(
                        &interp,
                        code,
                        Some(argument.as_ptr()),
                        rows,
                        &format!("{}_{suffix}", fields[1]),
                    );
                }
            }
        }
    }
}
