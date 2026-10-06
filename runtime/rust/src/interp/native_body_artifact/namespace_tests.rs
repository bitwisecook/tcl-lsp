// Copyright (c) 2026 tcl-lsp contributors. SPDX-License-Identifier: AGPL-3.0-or-later

use super::tests::interpreter;
use super::*;

mod upvar_inputs {
    include!(
        "../../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs"
    );
    include!(
        "../../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/execution_cases.rs"
    );
}

#[test]
fn original_c85_namespace_upvar_artifact_preserves_native_pools_and_locals() {
    for row in include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv"
    )
    .lines()
    .skip(1)
    {
        let fields: Vec<_> = row.split('\t').collect();
        let (label, body) = upvar_inputs::CASES[fields[0].parse::<usize>().unwrap()];
        let mut interp = interpreter("tcl8.5");
        let original = obj::Owned::fresh(new_string(body));
        interp.define_proc(
            b"p",
            vec![
                Param {
                    name: b"pairs".to_vec(),
                    default: None,
                },
                Param {
                    name: b"local".to_vec(),
                    default: None,
                },
            ],
            original.as_ptr(),
        );
        let procedure = interp.proc_def(b"p").unwrap();
        let artifact = interp
            .prepare_original_c_body(original.as_ptr(), GLOBAL, Some(&procedure))
            .expect("actual compilation succeeds")
            .expect("retained original artifact");
        assert!(!interp.host_refusal_pending(), "{label}");
        let links: usize = artifact.scripts.values().flat_map(|script| &script.commands)
            .filter_map(|command| match &command.operation {
                Operation::NamespaceBindings(scope) if scope.kind == tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Upvar
                    && scope.generic.is_none() => Some(scope.bindings.len()),
                _ => None,
            }).sum();
        assert_eq!(links, fields[1].parse::<usize>().unwrap(), "{label}");
        assert_layout(&artifact, fields[2], label);
        let expected = fields[3].split(',').map(decode).collect::<Vec<_>>();
        for (index, expected) in expected.iter().enumerate() {
            let original = artifact
                .literals
                .original(index)
                .expect("native original pool slot");
            let actual = ValueOps::native_string_bytes(&mut interp, &original).unwrap();
            assert_eq!(actual.as_ref(), expected.as_slice(), "{label}/{index}");
        }
        assert!(
            artifact.literals.original(expected.len()).is_none(),
            "{label}: exact pool extent"
        );
    }
}

#[test]
fn original_c85_namespace_upvar_artifact_matches_original_namespace_lifetimes() {
    for &(label, body, expected_code, expected, visited) in upvar_inputs::EXECUTIONS {
        let mut interp = interpreter("tcl8.5");
        assert_eq!(
            interp.eval_str(b"namespace eval ::N {variable x X; variable y Y}"),
            Code::Ok
        );
        let original = obj::Owned::fresh(new_string(body));
        interp.define_proc(b"p", Vec::new(), original.as_ptr());
        let name = obj::Owned::fresh(new_string(b"p"));
        let code = interp.dispatch(&[name.as_ptr()]);
        assert_eq!(
            code,
            if expected_code == 0 {
                Code::Ok
            } else {
                Code::Error
            },
            "{label}"
        );
        assert_eq!(interp.result_bytes(), expected, "{label}");
        assert!(!interp.host_refusal_pending(), "{label}");
        assert_eq!(interp.eval_str(b"info exists ::visited"), Code::Ok);
        assert_eq!(
            interp.result_bytes(),
            if visited {
                b"1".as_slice()
            } else {
                b"0".as_slice()
            },
            "{label}: target evaluates before namespace lookup"
        );
        let procedure = interp.proc_def(b"p").unwrap();
        let artifact = cache(procedure.body.as_ptr()).expect("genuine original procedure bytecode");
        assert!(artifact.scripts.values().flat_map(|script| &script.commands).any(|command|
            matches!(&command.operation, Operation::NamespaceBindings(scope) if scope.kind == tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Upvar)), "{label}");
    }
}

const MATRIX_BODIES: [&[u8]; 20] = [
    b"variable v",
    b"variable {}",
    b"variable :::",
    b"variable {a)}",
    b"variable v $val w [set x 3]",
    b"variable v 1 {a)} 2",
    b"variable ${ns}::v",
    b"variable ${ns}v",
    b"variable ${ns}::[set x 1]",
    b"variable ${ns}::v 1 ${ns}v 2",
    b"variable {*}{v 1}",
    b"variable",
    b"global v",
    b"global {}",
    b"global :::",
    b"global {a)}",
    b"global ${ns}::v",
    b"global ${ns}v",
    b"global v {a)}",
    b"global {*}{v w}",
];
const ATTEMPT_BODIES: [&[u8]; 5] = [
    b"variable v [list VALUE] ${ns}v [list LATE]",
    b"variable v [list VALUE] {a)} [list LATE]",
    b"global [list ::N]::v ${ns}v",
    b"global [list ::N]::v {a)}",
    b"variable [list ::N]::v [list VALUE] ${ns}v [list LATE]",
];
const MATRIX: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.4.20.tsv"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.5.19.tsv"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_namespace_bindings/8.6.18.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.0.4.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_namespace_bindings/9.1.0.tsv"
        ),
    ),
];
const ATTEMPTS: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!("../../../tests/data/native_namespace_binding_artifacts/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tests/data/native_namespace_binding_artifacts/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tests/data/native_namespace_binding_artifacts/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tests/data/native_namespace_binding_artifacts/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tests/data/native_namespace_binding_artifacts/9.1.0.tsv"),
    ),
];

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn prepare(interp: &mut Interp, body: &[u8]) -> Result<Option<Rc<NativeBodyArtifact>>, Code> {
    assert_eq!(interp.eval_str(b"namespace eval N {}"), Code::Ok);
    let original = obj::Owned::fresh(new_string(body));
    interp.define_proc(
        b"p",
        vec![
            Param {
                name: b"ns".to_vec(),
                default: None,
            },
            Param {
                name: b"val".to_vec(),
                default: None,
            },
        ],
        original.as_ptr(),
    );
    let procedure = interp.proc_def(b"p").expect("original native declaration");
    interp.prepare_original_c_body(original.as_ptr(), GLOBAL, Some(&procedure))
}

fn counts(artifact: &NativeBodyArtifact) -> (usize, usize) {
    let mut variables = 0;
    let mut globals = 0;
    for script in artifact.scripts.values() {
        for command in &script.commands {
            if let Operation::NamespaceBindings(scope) = &command.operation {
                if scope.generic.is_none() {
                    match scope.kind {
                        tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Variable => variables += scope.bindings.len(),
                        tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Global => globals += scope.bindings.len(),
                        tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Upvar => {}
                    }
                }
            }
        }
    }
    (variables, globals)
}

fn assert_layout(artifact: &NativeBodyArtifact, hex: &str, label: &str) {
    let expected: Vec<_> = hex.split(',').map(decode).collect();
    let layout = artifact
        .compiled_local_layout()
        .expect("actual native compiled locals");
    let actual: Vec<_> = layout
        .names
        .iter()
        .map(|name| {
            name.as_ref()
                .expect("native named slot")
                .as_bytes()
                .to_vec()
        })
        .collect();
    assert_eq!(actual, expected, "{label}");
}

#[test]
fn original_namespace_artifacts_match_native_instruction_and_local_matrix() {
    for (profile, table) in MATRIX {
        for row in table.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            let case: usize = fields[0].parse().unwrap();
            let mut interp = interpreter(profile);
            let prepared = prepare(&mut interp, MATRIX_BODIES[case]);
            assert!(
                !interp.host_refusal_pending(),
                "{profile}/{case}: {:?}",
                interp.result_bytes()
            );
            if fields[2] == "-1" {
                assert!(
                    prepared.is_err(),
                    "{profile}/{case}: original C8.4 parser rejection"
                );
                continue;
            }
            let artifact = prepared
                .expect("native compilation succeeds")
                .expect("registered namespace artifact");
            assert_eq!(
                counts(&artifact),
                (fields[2].parse().unwrap(), fields[3].parse().unwrap()),
                "{profile}/{case}"
            );
            assert_layout(&artifact, fields[4], &format!("{profile}/{case}"));
        }
    }
}

#[test]
fn original_namespace_declined_attempt_retains_native_pool_order_and_private_headers() {
    for (profile, table) in ATTEMPTS {
        for row in table.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            let case: usize = fields[0].parse().unwrap();
            let mut interp = interpreter(profile);
            let artifact = prepare(&mut interp, ATTEMPT_BODIES[case])
                .expect("native attempt preparation")
                .expect("registered namespace artifact");
            assert!(!interp.host_refusal_pending(), "{profile}/{case}");
            assert_eq!(
                counts(&artifact),
                (0, 0),
                "{profile}/{case}: declined attempt emits no bindings"
            );
            assert_layout(&artifact, fields[4], &format!("{profile}/{case}"));
            let expected_count: usize = fields[5].parse().unwrap();
            assert!(
                artifact.literals.original(expected_count).is_none(),
                "{profile}/{case}: exact pool extent"
            );
            let mut private_headers = Vec::new();
            for (index, expected) in fields[6].split(',').enumerate() {
                let (kind, hex) = expected.split_once(':').unwrap();
                let original = artifact
                    .literals
                    .original(index)
                    .expect("retained original literal header");
                if kind == "list" {
                    assert!(
                        crate::list::is_pure_list(original),
                        "{profile}/{case}/{index}: original list primary"
                    );
                    assert!(
                        !private_headers.contains(&original),
                        "{profile}/{case}/{index}: private header is distinct"
                    );
                    private_headers.push(original);
                }
                let bytes = ValueOps::native_string_bytes(&mut interp, &original)
                    .expect("native literal getter");
                assert_eq!(bytes.as_ref(), decode(hex), "{profile}/{case}/{index}");
            }
            assert_eq!(fields[6].split(',').count(), expected_count);
        }
    }
}
