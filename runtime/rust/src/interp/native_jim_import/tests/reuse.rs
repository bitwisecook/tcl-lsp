// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

const OBSERVATIONS: &str =
    include_str!("../../../../tests/data/native_jim_import_source_reuse/observations-20261015.tsv");
fn script(label: &str) -> &'static str {
    include_str!("../../../../tests/data/native_jim_import_source_reuse/scripts.tsv")
        .lines()
        .find_map(|line| line.split_once('|').filter(|(key, _)| *key == label))
        .unwrap()
        .1
}
fn observed(actual: &mut Interp, label: &str, code: Code) {
    let row = format!("{label}|{}|{}", code.as_int(), hex(&actual.result_bytes()));
    assert!(
        OBSERVATIONS.lines().any(|line| line == row),
        "{row}; refusal={:?}",
        actual.native_execution_refusal()
    );
}
fn run_observed(actual: &mut Interp, label: &str) {
    let code = actual.eval_str(script(label).as_bytes());
    observed(actual, label, code);
}
fn import_observed(actual: &mut Interp, label: &str) {
    let arguments: Vec<_> = script(label)
        .strip_prefix("namespace import ")
        .unwrap()
        .split(' ')
        .map(|pattern| Owned::fresh(obj::new_string_bytes(pattern.as_bytes())))
        .collect();
    entered_import(actual, &arguments, |actual, traversal, body, frame| {
        let code = actual.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            body,
            frame,
        );
        observed(actual, label, code);
        traversal.normal_completion(actual).unwrap();
        assert!(actual.native_execution_refusal().is_none());
    });
}

#[test]
fn genuine_import_source_reuses_the_body_after_alias_and_namespace_replacement() {
    let mut actual = jim();
    run_observed(&mut actual, "SETUP");
    for (import, value, replacement) in [
        ("REUSE_1", "VALUE_1", Some("REPLACE")),
        ("REUSE_2", "VALUE_2", Some("RECREATE")),
        ("REUSE_3", "VALUE_3", None),
        ("REPEATED_PATTERN", "VALUE_4", None),
    ] {
        import_observed(&mut actual, import);
        run_observed(&mut actual, value);
        if let Some(replacement) = replacement {
            run_observed(&mut actual, replacement);
        }
    }
}

thread_local! {
    static ORIGINAL_ALIAS: std::cell::Cell<Option<crate::interp::BuiltinFn>> = const {
        std::cell::Cell::new(None)
    };
}
fn original_alias_then_break(actual: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let code = ORIGINAL_ALIAS.with(|original| original.get().unwrap()(actual, argv));
    if code == Code::Ok {
        Code::Break
    } else {
        code
    }
}

#[test]
fn genuine_import_source_cannot_certify_full_traversal_after_a_publishing_break() {
    // Original C completed successfully after one publication, leaving q unvisited.
    assert!(OBSERVATIONS.lines().any(|line| line == "BREAK_IMPORT|0|"));
    assert!(OBSERVATIONS
        .lines()
        .any(|line| line == "BREAK_PUBLISHED|0|3a3a70"));
    assert!(OBSERVATIONS
        .lines()
        .any(|line| line == "BREAK_COMMANDS|0|3a3a70207b7d"));
    let mut actual = jim();
    run_observed(&mut actual, "BREAK_SETUP");
    let generation = actual
        .namespaces()
        .resolve_generation(crate::namespace::GLOBAL, b"alias")
        .unwrap();
    let Some(Command::Builtin(original)) = actual.raw_command_by_generation(generation) else {
        panic!("actual original alias implementation")
    };
    ORIGINAL_ALIAS.with(|saved| saved.set(Some(original)));
    actual.register_builtin(b"alias", original_alias_then_break);
    let argument = Owned::fresh(obj::new_string_bytes(
        script("BREAK_IMPORT")
            .strip_prefix("namespace import ")
            .unwrap()
            .as_bytes(),
    ));
    entered_import(
        &mut actual,
        &[argument],
        |actual, traversal, body, frame| {
            assert_eq!(
                actual.eval_original_body_framed(
                    tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                    body,
                    frame,
                ),
                Code::Error
            );
            assert_eq!(
                actual.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original Import interrupted inventory traversal"
                    )
                )
            );
            assert!(traversal.normal_completion(actual).is_err());
        },
    );
    ORIGINAL_ALIAS.with(|saved| saved.set(None));
}
