// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native command-log frame entries and actual return-dictionary key order.

const OBSERVATIONS: &str =
    include_str!("../../../tcl-syntax/tests/data/native_error_log/observations.txt");
const CASES: &[(&str, &str)] = &[
    (
        "caught",
        "proc p {} {catch {error BODY} m o; list [dict get $o -errorstack] [dict keys $o]}; p",
    ),
    (
        "unwound",
        "proc q {} {error BODY}; catch {q} m o; list [dict get $o -errorstack] [dict keys $o]",
    ),
    (
        "return",
        "proc r {} {return -code error BODY}; catch {r} m o; list [dict get $o -errorstack] [dict keys $o]",
    ),
    (
        "shifted",
        "proc u {} {catch {uplevel 1 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]}; u",
    ),
    (
        "explicit",
        "proc e {} {catch {return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY} m o; list [dict get $o -errorstack] [dict keys $o]}; e",
    ),
];

fn check(native: &str, phase: &str, actual: &str) {
    let expected = OBSERVATIONS
        .lines()
        .find_map(|row| row.strip_prefix(&format!("{native}|{phase}|")))
        .unwrap();
    check_record(native, phase, expected, actual);
}

fn check_record(native: &str, phase: &str, expected: &str, actual: &str) {
    let expected = tcl_syntax::list::split_list(expected).unwrap();
    let actual = tcl_syntax::list::split_list(actual).unwrap();
    assert_eq!(actual.len(), 2, "{native} {phase}");
    assert_eq!(
        actual[1], expected[1],
        "{native} {phase}: actual carried/private dictionary order"
    );
    let expected_stack = tcl_syntax::list::split_list(&expected[0]).unwrap();
    let actual_stack = tcl_syntax::list::split_list(&actual[0]).unwrap();
    // INNER has an independently selected compiled/interpreted producer. This
    // corpus proves frame logging and dictionary ordering, not that opcode axis.
    assert_eq!(
        &actual_stack[2..],
        &expected_stack[2..],
        "{native} {phase}: reached CALL/UP frame entries"
    );
}

#[test]
fn reached_logs_match_15_native_frame_and_dictionary_windows() {
    let mut matched = 0;
    for (release, native) in [("tcl8.6", "8.6"), ("tcl9.0", "9.0"), ("tcl9.1", "9.1")] {
        let profile = tcl_registry::model::ingress::resolve_environment(release).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        for (phase, script) in CASES {
            let completion = vm.eval_source(script).unwrap();
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{native} {phase}: {:?}",
                completion.result
            );
            check(
                native,
                phase,
                core::str::from_utf8(&completion.result.string_bytes())
                    .expect("native diagnostic record is UTF-8"),
            );
            matched += 1;
        }
    }
    assert_eq!(matched, 15);
}

#[test]
fn special_and_nested_frames_match_12_native_command_logs() {
    let observations =
        include_str!("../../../tcl-syntax/tests/data/native_error_log/special_frames.txt");
    let mut matched = 0;
    for (release, native) in [
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(release).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        for (phase, script, special) in [
            (
                "root",
                "catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",
                false,
            ),
            (
                "special-shift",
                "catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",
                true,
            ),
            (
                "special-same",
                "catch {uplevel 0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",
                true,
            ),
            (
                "nested-shift",
                "proc p {} {catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]}; namespace eval N {p}",
                false,
            ),
        ] {
            if special {
                // Actual host-pushed special frame, matching Tcl_PushCallFrame's
                // empty objc rather than a namespace command's invocation.
                vm.push_ns_eval_frame("", Vec::new());
            }
            let completion = vm.eval_source(script).unwrap();
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{native} {phase}"
            );
            let expected = observations
                .lines()
                .find_map(|row| row.strip_prefix(&format!("{native}|{phase}|0|")))
                .unwrap();
            check_record(
                native,
                phase,
                expected,
                core::str::from_utf8(&completion.result.string_bytes()).unwrap(),
            );
            if special {
                vm.pop_call_frame();
            }
            matched += 1;
        }
    }
    assert_eq!(matched, 12);
}
