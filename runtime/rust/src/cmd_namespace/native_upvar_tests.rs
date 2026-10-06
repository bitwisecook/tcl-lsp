// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace-upvar operands and selected native argument grammars.

use super::*;

fn actual(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("actual native core issuer")
}

fn decode(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn namespace_upvar_original_vectors_match_eighteen_native_arity_windows() {
    let rows = include_str!(
        "../../../../rust/tcl-registry/tests/data/native_namespace_upvar_arguments/rows.tsv"
    );
    let mut compared = 0;
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let mut interp = actual(fields[0]);
        assert_eq!(interp.eval_str(b"namespace eval n {}"), Code::Ok);
        let mut words = vec![
            obj::Owned::fresh(obj::new_string_bytes(b"namespace")),
            obj::Owned::fresh(obj::new_string_bytes(b"upvar")),
            obj::Owned::fresh(obj::new_string_bytes(b"n")),
        ];
        match fields[1] {
            "0" => {}
            "1" => words.push(obj::Owned::fresh(obj::new_string_bytes(b"a"))),
            "2" => words.extend([
                obj::Owned::fresh(obj::new_string_bytes(b"a")),
                obj::Owned::fresh(obj::new_string_bytes(b"b")),
            ]),
            other => panic!("unknown original case {other}"),
        }
        let original: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
        let code = interp.eval_original_object_vector(&original);
        assert_eq!(code.as_int(), fields[2].parse::<i64>().unwrap(), "{row}");
        assert_eq!(interp.result_bytes(), decode(fields[3]), "{row}");
        compared += 1;
    }
    assert_eq!(compared, 18);
}

#[test]
fn namespace_upvar_grammar_and_target_cells_match_36_native_results() {
    const CASES: &str =
        include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/cases.tsv");
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../rust/tcl-vm/tests/data/native_namespace_upvar/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, expected) in engines {
        assert_eq!(CASES.lines().count(), expected.lines().count());
        for (input, expected) in CASES.lines().zip(expected.lines()) {
            let (case, source) = input.split_once('\t').unwrap();
            let fields: Vec<_> = expected.split('\t').collect();
            assert_eq!(fields[0], case);
            let mut interp = actual(engine);
            let code = interp.eval_str(&decode(source));
            assert_eq!(
                code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{engine}/{case}"
            );
            assert_eq!(interp.result_bytes(), decode(fields[2]), "{engine}/{case}");
            compared += 1;
        }
    }
    assert_eq!(compared, 36);
}

#[test]
fn jim_namespace_upvar_forwards_root_worker_without_changing_variable_frame() {
    let mut interp = actual("jim");
    let source = include_bytes!("../../../../rust/tcl-registry/tests/data/native_namespace_upvar_arguments/jim-forwarding.tcl");
    assert_eq!(interp.eval_str(source), Code::Ok);
    assert_eq!(interp.result_bytes(), b"VALUE {FORWARD 0 ::missing::a {}}");
}
