// SPDX-License-Identifier: AGPL-3.0-or-later
//! Paired unchanged native inputs and physical windows, before observation getters.
use crate::{
    interp::Interp,
    list,
    obj::{self, TclObj},
};
use tcl_syntax::value::ValueOps;
include!("../../../../rust/tcl-cmd-core/tests/data/native_concat/cases.rs");
fn original(interp: &mut Interp, input: Input) -> obj::Owned {
    let protocol = interp
        .native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    obj::Owned::fresh(match input {
        Input::Bytes(bytes) => obj::new_string_bytes(bytes),
        Input::List(members) | Input::Rendered(members) => {
            let value = list::new_list_obj_native(
                &members
                    .iter()
                    .map(|bytes| obj::new_string_bytes(bytes))
                    .collect::<Vec<_>>(),
                protocol,
            );
            if matches!(input, Input::Rendered(_)) {
                interp.native_object_string_bytes(value).unwrap();
            }
            value
        }
        Input::Parsed(bytes) => {
            let value = obj::new_string_bytes(bytes);
            if protocol.is_jim084() {
                crate::native_source::bind_context(
                    value,
                    &interp.native_jim_object_context().unwrap(),
                )
                .unwrap();
            }
            list::list_elements_native_checked(value, protocol).unwrap();
            value
        }
    })
}
fn snapshot(value: *mut TclObj, version: Option<tcl_dialect::TclVersion>) -> String {
    // SAFETY: every observed original is owned by the fixture or an actual List backing.
    state(
        obj::native_object_snapshot(value).unwrap(),
        unsafe { (*value).ref_count } as usize,
        version,
    )
}
#[test]
fn original_concat_matches_all_twenty_native_windows_per_engine() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        for case in 0..20 {
            let input = inputs(case);
            let values: Vec<_> = input
                .iter()
                .map(|&item| original(&mut interp, item))
                .collect();
            let argv: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
            let children: Vec<_> = input.iter().zip(&argv).map(|(input, &value)| {
                if matches!(input, Input::List(members) | Input::Rendered(members) if !members.is_empty()) {
                    Some(list::list_elements_native_checked(value, protocol).unwrap()[0])
                } else { None }
            }).collect();
            let before = argv
                .iter()
                .map(|&value| snapshot(value, protocol.tcl_version()))
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(
                before,
                row(engine, case, "before")[3],
                "{engine}/{case} before"
            );
            let result =
                obj::Owned::fresh(tcl_cmd_core::list::concat_selected(&mut interp, &argv).unwrap());
            let expected = row(engine, case, "result");
            assert_eq!(
                snapshot(result.as_ptr(), protocol.tcl_version()),
                expected[3],
                "{engine}/{case} result"
            );
            assert!(argv.first().is_none_or(|&value| value != result.as_ptr()));
            let shared = argv.first().is_some_and(|&first| {
                match (
                    list::native_list_backing(first),
                    list::native_list_backing(result.as_ptr()),
                ) {
                    (Some(left), Some(right)) => {
                        left.elements().unwrap().as_ptr() == right.elements().unwrap().as_ptr()
                    }
                    _ => false,
                }
            });
            assert_eq!(
                format!("backing_first={}", usize::from(shared)),
                expected[5],
                "{engine}/{case} backing"
            );
            let cref = children
                .iter()
                .map(|child| {
                    child.map_or_else(
                        || "-1".to_owned(),
                        |value| {
                            // SAFETY: the unchanged source List retains this original child.
                            unsafe { (*value).ref_count.to_string() }
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(
                format!("children={cref}"),
                expected[6],
                "{engine}/{case} children"
            );
            let after = argv
                .iter()
                .map(|&value| snapshot(value, protocol.tcl_version()))
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(
                after,
                row(engine, case, "after")[3],
                "{engine}/{case} after"
            );
            assert_eq!(
                hex(&interp.native_object_string_bytes(result.as_ptr()).unwrap()),
                row(engine, case, "value")[3],
                "{engine}/{case} value"
            );
        }
    }
}
#[test]
fn compiled_concat_preserves_all_eight_original_c_result_headers() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (case, &body) in COMPILED_BODIES.iter().enumerate() {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let words = [
                obj::Owned::fresh(obj::new_string_bytes(b"proc")),
                obj::Owned::fresh(obj::new_string_bytes(b"p")),
                obj::Owned::fresh(obj::new_string_bytes(b"")),
                obj::Owned::fresh(obj::new_string_bytes(body)),
            ];
            let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(
                interp.eval_original_object_vector(&argv),
                crate::interp::Code::Ok,
                "{engine}/{case} definition {:?}",
                interp.result_bytes()
            );
            let head = obj::Owned::fresh(obj::new_string_bytes(b"p"));
            let code = interp.eval_original_object_vector(&[head.as_ptr()]);
            let expected = compiled_row(engine, case);
            assert_eq!(
                code,
                crate::interp::Code::Ok,
                "{engine}/{case} {:?}",
                interp.result_bytes()
            );
            let version = interp.native_invocation_dialect().tcl_version;
            assert_eq!(
                snapshot(interp.result_obj(), version),
                expected[4],
                "{engine}/{case}"
            );
        }
    }
}
fn abstract_state(value: *mut TclObj) -> String {
    if crate::native_arithseries::is_series(value) {
        format!(
            "arithseries,{},{},{},{}",
            usize::from(obj::has_string_rep(value)),
            usize::from(crate::native_arithseries::has_element_cache(value)),
            // SAFETY: the original series or concat result owns the live header.
            unsafe { (*value).ref_count },
            crate::native_arithseries::native_header_count(value)
        )
    } else {
        assert!(core::ptr::eq(
            obj::obj_type_ptr(value),
            &list::TCL_LIST_TYPE
        ));
        // SAFETY: the actual concat result owns this live List header.
        format!(
            "list,{},-1,{},-1",
            usize::from(obj::has_string_rep(value)),
            unsafe { (*value).ref_count }
        )
    }
}
#[test]
fn concat_uses_original_abstract_index_before_element_materialisation() {
    for (engine, rows) in [
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.0.4.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.1.0.tsv"
            ),
        ),
    ] {
        for case in 0..3 {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let protocol = interp
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let plan = tcl_cmd_core::lseq::decode(&[b"1", b"3", b"1"])
                .unwrap_or_else(|_| panic!("original native arithmetic-series input"));
            let series = tcl_cmd_core::lseq::prepare_series(&plan).unwrap().unwrap();
            let original =
                obj::Owned::fresh(crate::native_arithseries::new_series(series, protocol).unwrap());
            let expected = |window: &str| {
                let prefix = format!("A\t{case}\t{window}\t");
                rows.lines()
                    .find(|row| row.starts_with(&prefix))
                    .unwrap()
                    .split('\t')
                    .nth(3)
                    .unwrap()
            };
            assert_eq!(abstract_state(original.as_ptr()), expected("before"));
            if case == 0 {
                let before = crate::native_arithseries::has_element_cache(original.as_ptr());
                let first = interp
                    .native_concat_first_bytes(&original.as_ptr())
                    .unwrap()
                    .unwrap();
                assert_eq!(&*first.bytes, b"1");
                // SAFETY: the abstract hook returned this live fresh child, still rc0.
                assert_eq!(unsafe { (*first.temporary.unwrap()).ref_count }, 0);
                interp.release_native_concat_first(first);
                assert_eq!(
                    crate::native_arithseries::has_element_cache(original.as_ptr()),
                    before
                );
            } else {
                let prefix = obj::Owned::fresh(list::new_list_obj_native(
                    &[obj::new_string_bytes(b"a")],
                    protocol,
                ));
                let argv = if case == 1 {
                    vec![original.as_ptr()]
                } else {
                    vec![prefix.as_ptr(), original.as_ptr()]
                };
                let result = obj::Owned::fresh(
                    tcl_cmd_core::list::concat_selected(&mut interp, &argv).unwrap(),
                );
                assert_eq!(abstract_state(result.as_ptr()), expected("result"));
                assert_eq!(abstract_state(original.as_ptr()), expected("after"));
                continue;
            }
            assert_eq!(abstract_state(original.as_ptr()), expected("after"));
        }
    }
}
#[test]
fn string_concat_does_not_issue_a_canonical_list_guarantee() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let words = [
            obj::Owned::fresh(obj::new_string_bytes(b"{")),
            obj::Owned::fresh(obj::new_string_bytes(b"foo")),
        ];
        let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
        let result =
            obj::Owned::fresh(tcl_cmd_core::list::concat_selected(&mut interp, &argv).unwrap());
        assert_eq!(
            &*interp.native_object_string_bytes(result.as_ptr()).unwrap(),
            b"{ foo"
        );
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        if protocol.is_jim084() {
            crate::native_source::bind_context(
                result.as_ptr(),
                &interp.native_jim_object_context().unwrap(),
            )
            .unwrap();
        }
        let parsed = list::list_elements_native_checked(result.as_ptr(), protocol);
        if protocol.is_jim084() {
            assert_eq!(parsed.unwrap().len(), 1);
        } else {
            assert!(matches!(
                parsed,
                Err(tcl_syntax::value::ValueError::NativeListParse { .. })
            ));
        }
    }
}

include!("../../../../rust/tcl-registry/tests/data/native_concat_expansion/cases.rs");
fn expansion_cache_name(
    cache: &tcl_syntax::native_object::NativeObjectCacheSnapshot,
) -> &'static str {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    match cache {
        Cache::None => "NULL",
        Cache::String { .. } => "string",
        other => panic!("unexpected actual expanded concat result {other:?}"),
    }
}
#[test]
fn compiled_concat_expansion_preserves_all_48_original_native_windows() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (case, body) in EXPANSION_BODIES.iter().enumerate() {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let words = [
                obj::Owned::fresh(obj::new_string_bytes(b"proc")),
                obj::Owned::fresh(obj::new_string_bytes(b"p")),
                obj::Owned::fresh(obj::new_string_bytes(b"a c")),
                obj::Owned::fresh(obj::new_string_bytes(body)),
            ];
            let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(
                interp.eval_original_object_vector(&argv),
                crate::interp::Code::Ok,
                "{engine}/{case}: {:?}",
                interp.result_bytes()
            );
            let call = [
                obj::Owned::fresh(obj::new_string_bytes(b"p")),
                obj::Owned::fresh(obj::new_string_bytes(b"D E")),
                obj::Owned::fresh(obj::new_string_bytes(b"C")),
            ];
            let argv: Vec<_> = call.iter().map(obj::Owned::as_ptr).collect();
            let code = interp.eval_original_object_vector(&argv);
            let expected = expansion_result(engine, case);
            assert_eq!(
                code.as_int().to_string(),
                expected[2],
                "{engine}/{case}: {:?}",
                interp.result_bytes()
            );
            let result = interp.result_obj();
            let snapshot = obj::native_object_snapshot(result).unwrap();
            // SAFETY: the actual interpreter result owns this live original header.
            let references = unsafe { (*result).ref_count };
            assert_eq!(
                format!(
                    "{}\t{}\t{}",
                    expansion_cache_name(&snapshot.cache),
                    usize::from(snapshot.resident.is_some()),
                    references
                ),
                expected[3..6].join("\t"),
                "{engine}/{case} original result header"
            );
            assert_eq!(
                hex(&interp.native_object_string_bytes(result).unwrap()),
                expected[6],
                "{engine}/{case} original result"
            );
        }
    }
}
