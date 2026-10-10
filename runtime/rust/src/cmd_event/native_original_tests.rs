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
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/9.1.0.tsv"),
    ),
    (
        "jimtcl",
        include_str!("../../../../rust/tcl-registry/tests/data/native_event_original/jim.tsv"),
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
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let code = interp.eval_str(source.as_bytes());
            assert_eq!(
                code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{profile}/{case}"
            );
            assert_eq!(interp.result_bytes(), unhex(fields[2]), "{profile}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 126);
}

#[test]
fn original_list_event_scripts_retain_counted_name_objects() {
    // Native proof: naming.event.original-list-script-byte-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-list-script-byte-boundaries.md
    use crate::obj;
    use tcl_syntax::value::ValueOps;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for name in [b"k\xff".as_slice(), b"k\0tail", b"k\xc0\x80tail"] {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let key = obj::Owned::fresh(crate::interp::new_string(name));
            let set = obj::Owned::fresh(crate::interp::new_string(b"set"));
            let value = obj::Owned::fresh(crate::interp::new_string(b"VALUE"));
            let script = obj::Owned::fresh(interp.new_list(vec![
                set.as_ptr(),
                key.as_ptr(),
                value.as_ptr(),
            ]));
            let after = obj::Owned::fresh(crate::interp::new_string(b"after"));
            let zero = obj::Owned::fresh(crate::interp::new_string(b"0"));
            assert_eq!(
                interp.dispatch(&[after.as_ptr(), zero.as_ptr(), script.as_ptr()]),
                crate::interp::Code::Ok,
                "{engine}/{name:?}"
            );
            let update = obj::Owned::fresh(crate::interp::new_string(b"update"));
            assert_eq!(
                interp.dispatch(&[update.as_ptr()]),
                crate::interp::Code::Ok,
                "{engine}/{name:?}"
            );
            assert_eq!(
                interp.dispatch(&[set.as_ptr(), key.as_ptr()]),
                crate::interp::Code::Ok,
                "{engine}/{name:?}"
            );
            assert_eq!(interp.result_bytes(), b"VALUE");
        }
    }
}

#[test]
fn original_vwait_extended_forms_decline_before_global_observers_or_callbacks() {
    // Implementation contract: naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
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
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let id = interp.events_mut().push(
                0,
                std::time::Duration::ZERO,
                tcl_cmd_core::event::EventKind::Timer,
                tcl_cmd_core::event::EventKind::Timer,
                crate::obj::Owned::fresh(crate::interp::new_string(b"set originalCallback BAD")),
            );
            let mut words = vec![crate::obj::Owned::fresh(crate::interp::new_string(
                b"vwait",
            ))];
            words.extend(
                form.iter()
                    .map(|s| crate::obj::Owned::fresh(crate::interp::new_string(s.as_bytes()))),
            );
            let argv = words
                .iter()
                .map(crate::obj::Owned::as_ptr)
                .collect::<Vec<_>>();
            assert_eq!(
                interp.dispatch(&argv),
                crate::interp::Code::Error,
                "{engine}/{form:?}"
            );
            let refusal = interp
                .native_access_refusal()
                .expect("typed unavailable extended event capability");
            assert!(
                format!("{refusal:?}").contains("extended native vwait grammar"),
                "{engine}/{form:?}: {refusal:?}"
            );
            assert_eq!(
                interp.events_mut().ids(false),
                vec![id],
                "{engine}/{form:?}: no callback processed"
            );
        }
    }
}
