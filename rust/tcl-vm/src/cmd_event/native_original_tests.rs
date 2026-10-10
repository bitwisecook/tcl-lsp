// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite comparisons against retained original native event controls.

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
const SOURCE_CASES: &[(&str, &str)] = &[
    (
        "GLOBAL_FROM_NAMESPACE",
        "set ::done 0; namespace eval N {variable x LOCAL; after 0 {set x GLOBAL; set ::done 1}; vwait ::done; list $x $::x}",
    ),
    (
        "GLOBAL_FROM_PROC",
        "set ::x OLD; proc p {} {set x LOCAL; after 0 {set x NEW}; vwait x; list $x $::x}; p",
    ),
    (
        "SAME_WRITE",
        "set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 0 {set x NEXT; lappend log NEXT}; vwait x; set log",
    ),
    (
        "ARRAY_SAME_WRITE",
        "array set a {k OLD}; after 0 {set a(k) OLD}; after 0 {set a(k) NEW}; vwait a(k); set a(k)",
    ),
    (
        "UNSET_WRITE",
        "set x OLD; after 0 {unset x}; vwait x; info exists x",
    ),
    (
        "CONCAT_TRIM",
        "set done 0; after 0 { set} { done } {1 }; vwait done; set done",
    ),
    (
        "PURE_LIST_SCRIPT",
        "set done 0; set body [list set done {A B}]; after 0 $body; vwait done; set done",
    ),
    (
        "CANCEL_COUNTED",
        "set done 0; after 0 {set done 1}; after cancel {set done 1}; update; set done",
    ),
    (
        "CANCEL_CONCAT",
        "set done 0; after 0 {set done 1}; after cancel { set} { done 1 }; update; set done",
    ),
    (
        "INFO_SCRIPT",
        "set id [after idle {set done 1}]; lindex [after info $id] 0",
    ),
    (
        "INFO_KIND",
        "set id [after 0 {set done 1}]; lindex [after info $id] 1",
    ),
    ("INFO_MISSING", "after info after#99999999"),
    (
        "UPDATE_IDLE",
        "set x OLD; after 0 {set x TIMER}; after idle {set x IDLE}; update idletasks; set x",
    ),
    (
        "DELAY_NO_EVENTS",
        "set x OLD; after 0 {set x TIMER}; after 1; set x",
    ),
    ("AFTER_ABBREVIATION", "after in"),
    ("AFTER_AMBIGUOUS", "after i"),
    ("UPDATE_ABBREVIATION", "update i"),
    (
        "BATCH_CANCEL",
        "set log {}; after 0 {lappend ::log FIRST; after cancel $::later}; set ::later [after 0 {lappend ::log SECOND}]; update; set log",
    ),
    (
        "BATCH_NEW",
        "set log {}; after 0 {lappend ::log FIRST; after 0 {lappend ::log NEW}}; after 0 {lappend ::log SECOND}; update; set log",
    ),
    (
        "SAME_NESTED",
        "set x SAME; set log {}; after 0 {set x SAME; lappend log SAME; after 0 {set x NEXT; lappend log NEXT}}; vwait x; set log",
    ),
    (
        "ARRAY_NESTED",
        "array set a {k OLD}; after 0 {set a(k) OLD; after 0 {set a(k) NEW}}; vwait a(k); set a(k)",
    ),
];
const PROVIDERS: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!("../../../tcl-registry/tests/data/native_event_original/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tcl-registry/tests/data/native_event_original/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tcl-registry/tests/data/native_event_original/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tcl-registry/tests/data/native_event_original/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tcl-registry/tests/data/native_event_original/9.1.0.tsv"),
    ),
    (
        "jimtcl",
        include_str!("../../../tcl-registry/tests/data/native_event_original/jim.tsv"),
    ),
];

#[test]
fn original_event_scripts_global_waits_and_timer_turns_match_native_controls() {
    // Native proofs: naming.event.original-script-global-context, naming.event.global-wait-trigger, naming.event.timer-turn-order, naming.event.original-option-query-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-script-global-context.md
    // docs/design/analysis/name-resolution-proofs/event-global-wait-trigger.md
    // docs/design/analysis/name-resolution-proofs/event-timer-turn-order.md
    // docs/design/analysis/name-resolution-proofs/event-original-option-query-boundaries.md
    let mut comparisons = 0;
    for &(profile, rows) in PROVIDERS {
        for &(case, source) in SOURCE_CASES {
            let fields = rows
                .lines()
                .find(|row| row.starts_with(&format!("{case}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(profile),
            );
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(
                completion.code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{profile}/{case}"
            );
            let bytes = vm.native_name_operand_bytes(&completion.result).unwrap();
            assert_eq!(bytes.as_ref(), unhex(fields[2]), "{profile}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 126);
}

#[test]
fn original_list_event_scripts_retain_counted_name_objects() {
    // Native proof: naming.event.original-list-script-byte-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-list-script-byte-boundaries.md
    use crate::Value;
    use tcl_syntax::value::ValueOps;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for name in [b"k\xff".as_slice(), b"k\0tail", b"k\xc0\x80tail"] {
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            let key = Value::new_native_string_bytes(name);
            let script = vm.new_list(vec![
                Value::string("set"),
                key.clone(),
                Value::string("VALUE"),
            ]);
            let after = Value::new_native_string_bytes(b"after".as_slice());
            let completion =
                vm.invoke_host_original_object_vector(&after, &[Value::string("0"), script]);
            assert!(completion.code.is_ok(), "{engine}/{name:?}");
            let update = Value::new_native_string_bytes(b"update".as_slice());
            let completion = vm.invoke_host_original_object_vector(&update, &[]);
            assert!(completion.code.is_ok(), "{engine}/{name:?}");
            let set = Value::new_native_string_bytes(b"set".as_slice());
            let completion = vm.invoke_host_original_object_vector(&set, &[key]);
            assert!(completion.code.is_ok(), "{engine}/{name:?}");
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                b"VALUE"
            );
        }
    }
}

#[test]
fn original_vwait_extended_forms_decline_before_global_observers_or_callbacks() {
    // Implementation contract: naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
    use crate::Value;
    for (engine, forms) in [
        (
            "tcl9.0",
            vec![
                vec!["--"],
                vec!["-variable", "x"],
                vec!["-timeout", "1"],
                vec!["x", "y"],
            ],
        ),
        (
            "tcl9.1",
            vec![
                vec!["--"],
                vec!["-variable", "x"],
                vec!["-timeout", "1"],
                vec!["x", "y"],
            ],
        ),
        (
            "jimtcl",
            vec![
                vec!["-signal", "x"],
                vec!["x", "break"],
                vec!["-signal", "x", "break"],
            ],
        ),
    ] {
        for form in forms {
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            let id = vm.events.push(
                0,
                std::time::Duration::ZERO,
                tcl_cmd_core::event::EventKind::Timer,
                tcl_cmd_core::event::EventKind::Timer,
                Value::new_native_string_bytes(b"set originalCallback BAD".as_slice()),
            );
            let head = Value::new_native_string_bytes(b"vwait".as_slice());
            let args = form
                .iter()
                .map(|s| Value::new_native_string_bytes(s.as_bytes()))
                .collect::<Vec<_>>();
            let completion = vm.invoke_host_original_object_vector(&head, &args);
            assert_eq!(completion.code, crate::Code::Error, "{engine}/{form:?}");
            let refusal = vm
                .execution_refusal
                .as_ref()
                .expect("typed unavailable extended event capability");
            assert!(
                format!("{refusal:?}").contains("extended native vwait grammar"),
                "{engine}/{form:?}: {refusal:?}"
            );
            assert_eq!(
                vm.events.ids(false),
                vec![id],
                "{engine}/{form:?}: no callback processed"
            );
        }
    }
}
