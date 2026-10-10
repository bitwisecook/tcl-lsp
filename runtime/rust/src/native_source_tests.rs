// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{
    bind_context, context, install_source, pin_source_info, NativeJimObjectContext,
    NativeJimSourceInfo, JIM_SOURCE_TYPE,
};
use crate::obj::{self, Owned};
use std::fmt::Write;
use tcl_syntax::native_string::NativeStringProtocol;

fn jim() -> tcl_registry::InvocationDialect {
    tcl_registry::InvocationDialect::of_profile(crate::environment::profile_for_dialect("jim"))
}
fn references(value: &Owned) -> obj::TclSize {
    // SAFETY: Owned retains the live object throughout inspection.
    unsafe { (*value.as_ptr()).ref_count }
}
fn kind(value: *mut obj::TclObj) -> &'static str {
    let primary = obj::obj_type_ptr(value);
    if primary.is_null() {
        "none"
    } else if std::ptr::eq(primary, &JIM_SOURCE_TYPE) {
        "source"
    } else if std::ptr::eq(primary, &crate::list::TCL_LIST_TYPE) {
        "list"
    } else if std::ptr::eq(primary, &obj::TCL_INT_TYPE) {
        assert_eq!(obj::wide_of(value), 17);
        "int"
    } else {
        panic!("unexpected original filename cache")
    }
}

fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".into();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn unhex(text: &str) -> Vec<u8> {
    if text == "-" {
        return Vec::new();
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn source_owned_list_conversion_matches_222_native_windows() {
    crate::counters::reset();
    {
        let context = NativeJimObjectContext::new(jim()).unwrap();
        let mut observed = String::new();
        for mode in 1..4 {
            for row in
                include_str!("../../../rust/tcl-syntax/testdata/native_jim_source_list/inputs.tsv")
                    .lines()
            {
                let (case, encoded) = row.split_once('\t').unwrap();
                let filename = Owned::fresh(if mode == 2 {
                    obj::new_wide_int_obj(17)
                } else {
                    obj::new_string_bytes(b"f\0\xff")
                });
                let file_type = kind(filename.as_ptr());
                let file_string = usize::from(obj::has_string_rep(filename.as_ptr()));
                let mut parent = Owned::fresh(obj::new_string_bytes(&unhex(encoded)));
                install_source(
                    parent.as_ptr(),
                    NativeJimSourceInfo {
                        filename: filename.clone(),
                        line: 10,
                    },
                    &context,
                )
                .unwrap();
                if mode == 3 {
                    parent = Owned::fresh(obj::duplicate(parent.as_ptr()));
                }
                writeln!(
                    observed,
                    "P\t{mode}\t{case}\t{}\t{}\t{file_type}\t{file_string}\t{}",
                    kind(parent.as_ptr()),
                    usize::from(obj::has_string_rep(parent.as_ptr())),
                    references(&filename)
                )
                .unwrap();
                let members = crate::list::list_elements_native_checked(
                    parent.as_ptr(),
                    NativeStringProtocol::Jim084,
                )
                .unwrap();
                writeln!(
                    observed,
                    "L\t{mode}\t{case}\t{}\t{}\t{}\t{}\t{}\t{}",
                    kind(parent.as_ptr()),
                    usize::from(obj::has_string_rep(parent.as_ptr())),
                    members.len(),
                    kind(filename.as_ptr()),
                    usize::from(obj::has_string_rep(filename.as_ptr())),
                    references(&filename)
                )
                .unwrap();
                for (index, &member) in members.iter().enumerate() {
                    assert!(std::ptr::eq(obj::obj_type_ptr(member), &JIM_SOURCE_TYPE));
                    // SAFETY: the original List backing owns this member.
                    let refs = unsafe { (*member).ref_count };
                    let info = pin_source_info(member, &context).unwrap();
                    writeln!(
                        observed,
                        "E\t{mode}\t{case}\t{index}\tsource\t1\t{refs}\t{}\t{}\t{}",
                        usize::from(info.filename.as_ptr() == filename.as_ptr()),
                        info.line,
                        hex(&obj::bytes_of(member))
                    )
                    .unwrap();
                }
                drop(parent);
                writeln!(observed, "D\t{mode}\t{case}\t{}", references(&filename)).unwrap();
            }
        }
        let expected = include_str!(
            "../../../rust/tcl-syntax/testdata/native_jim_source_list/observations.tsv"
        )
        .lines()
        .filter(|row| row.split('\t').nth(1) != Some("0"))
        .collect::<Vec<_>>();
        assert_eq!(expected.len(), 222);
        assert_eq!(observed.lines().collect::<Vec<_>>(), expected);
    }
    assert_eq!(crate::counters::finalize(), 0);
}

#[test]
fn weak_context_does_not_own_objects_and_cannot_be_reissued() {
    crate::counters::reset();
    {
        let original = Owned::fresh(obj::new_string_bytes(b"a b"));
        let first = NativeJimObjectContext::new(jim()).unwrap();
        let second = NativeJimObjectContext::new(jim()).unwrap();
        bind_context(original.as_ptr(), &first).unwrap();
        assert_eq!(references(&original), 1);
        assert_eq!(references(&first.current_filename_object_ref()), 1);
        assert!(bind_context(original.as_ptr(), &second).is_err());
        drop(first);
        assert!(context(original.as_ptr()).is_err());
        assert!(bind_context(original.as_ptr(), &second).is_err());
        assert_eq!(obj::bytes_of(original.as_ptr()), b"a b");
    }
    assert_eq!(crate::counters::finalize(), 0);
}

#[test]
fn last_list_header_retires_source_filename_before_lifetime_view() {
    crate::counters::reset();
    {
        let context = NativeJimObjectContext::new(jim()).unwrap();
        let mut observed = String::new();
        for mode in 0..4 {
            let filename = Owned::fresh(obj::new_string_bytes(b"FILE"));
            let child = Owned::fresh(obj::new_string_bytes(if mode == 3 {
                b"4"
            } else {
                b"CHILD"
            }));
            install_source(
                child.as_ptr(),
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
            let parent = Owned::fresh(crate::list::new_list_obj_native(
                &[child.as_ptr()],
                NativeStringProtocol::Jim084,
            ));
            let external = if mode == 1 {
                Some(child)
            } else {
                drop(child);
                None
            };
            let view = crate::list::native_list_backing(parent.as_ptr()).unwrap();
            let copied = (mode == 2).then(|| Owned::fresh(obj::duplicate(parent.as_ptr())));
            let member = view.elements().unwrap()[0];
            writeln!(
                observed,
                "BEFORE\t{mode}\t{}\t{}\t1\t1",
                references(&filename),
                unsafe { (*member).ref_count }
            )
            .unwrap();
            if mode == 3 {
                obj::change_type(parent.as_ptr(), &obj::TCL_INT_TYPE, 4);
                writeln!(observed, "SHIMMER\t3\t0\t4\t{}", references(&filename)).unwrap();
                assert!(view.elements().is_err());
            } else {
                drop(parent);
                writeln!(observed, "AFTER_FIRST\t{mode}\t{}", references(&filename)).unwrap();
                if let Some(external) = external {
                    writeln!(
                        observed,
                        "EXTERNAL_LIVE\t1\t{}\tsource",
                        references(&external)
                    )
                    .unwrap();
                    assert_eq!(view.elements().unwrap()[0], external.as_ptr());
                    drop(external);
                    writeln!(observed, "AFTER_EXTERNAL\t1\t{}", references(&filename)).unwrap();
                }
                if let Some(copied) = copied {
                    assert!(view.elements().is_ok());
                    drop(copied);
                    writeln!(observed, "AFTER_LAST\t2\t{}", references(&filename)).unwrap();
                }
                assert!(view.elements().is_err());
            }
            drop(view);
        }
        let expected = include_str!(
            "../../../rust/tcl-syntax/testdata/native_jim_list_last_header/observations.tsv"
        );
        assert_eq!(expected.lines().count(), 11);
        assert_eq!(observed, expected);
    }
    assert_eq!(crate::counters::finalize(), 0);
}

#[test]
fn original_jim_source_length_preserves_resident_bytes_and_child_source() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    use tcl_syntax::value::ValueOps;
    crate::counters::reset();
    {
        let mut interp = crate::interp::Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(obj::new_string_bytes(b"source-check.tcl"));
        let parent = Owned::fresh(obj::new_string_bytes(b"A  B"));
        install_source(
            parent.as_ptr(),
            NativeJimSourceInfo { filename, line: 17 },
            &context,
        )
        .unwrap();
        assert_eq!(
            obj::stock_list_input_class(parent.as_ptr()),
            tcl_registry::native_stock_list::NativeStockListInputClass::JimSource
        );
        let mut observed = String::new();
        writeln!(
            observed,
            "SOURCE_BEFORE|{}|{}",
            kind(parent.as_ptr()),
            usize::from(obj::has_string_rep(parent.as_ptr()))
        )
        .unwrap();
        let length = interp.list_len(&parent.as_ptr()).unwrap();
        writeln!(
            observed,
            "SOURCE_AFTER|{}|{length}|{}|{}",
            kind(parent.as_ptr()),
            usize::from(obj::has_string_rep(parent.as_ptr())),
            hex(&obj::bytes_of(parent.as_ptr()))
        )
        .unwrap();
        let member = interp.list_index(&parent.as_ptr(), 0).unwrap().unwrap();
        let info = pin_source_info(member, &context).unwrap();
        writeln!(
            observed,
            "SOURCE_CHILD|{}|{}|{}",
            kind(member),
            info.line,
            hex(&obj::bytes_of(info.filename.as_ptr()))
        )
        .unwrap();
        let native = include_str!(
            "../../../rust/tcl-registry/tests/data/native_source_list_length241/jim/stdout"
        );
        assert_eq!(
            observed.lines().collect::<Vec<_>>(),
            native
                .lines()
                .filter(|row| row.starts_with("SOURCE_"))
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(crate::counters::finalize(), 0);
}
