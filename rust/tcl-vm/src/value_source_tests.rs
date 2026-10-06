// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{NativeJimObjectContext, NativeJimSourceInfo, Value};
use std::fmt::Write;
use tcl_syntax::native_string::NativeStringProtocol;

fn jim() -> tcl_registry::InvocationDialect {
    tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    )
}

fn kind(value: &Value) -> &'static str {
    use tcl_syntax::{
        native_object::NativeObjectCacheSnapshot as Cache, number::Number,
        scalar_getter::NativeScalarCache,
    };
    if value.native_jim_source_cache_present() {
        return "source";
    }
    match value.native_object_snapshot().cache {
        Cache::None => "none",
        Cache::Numeric(NativeScalarCache::Number(Number::Int(17))) => "int",
        Cache::List { .. } => "list",
        cache => panic!("unexpected original cache {cache:?}"),
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
    let context = NativeJimObjectContext::new(jim()).unwrap();
    let mut observed = String::new();
    for mode in 1..4 {
        for row in
            include_str!("../../tcl-syntax/testdata/native_jim_source_list/inputs.tsv").lines()
        {
            let (case, encoded) = row.split_once('\t').unwrap();
            let filename = if mode == 2 {
                Value::int(17)
            } else {
                Value::new_native_string_bytes(&b"f\0\xff"[..])
            };
            let file_type = kind(&filename);
            let file_string = usize::from(filename.resident_string_bytes().is_some());
            let mut parent = Value::new_native_string_bytes(unhex(encoded));
            parent
                .install_native_jim_source(
                    NativeJimSourceInfo {
                        filename: filename.clone(),
                        line: 10,
                    },
                    &context,
                )
                .unwrap();
            if mode == 3 {
                parent = parent.duplicate_native_object_in(NativeStringProtocol::Jim084);
            }
            writeln!(
                observed,
                "P\t{mode}\t{case}\t{}\t{}\t{file_type}\t{file_string}\t{}",
                kind(&parent),
                usize::from(parent.resident_string_bytes().is_some()),
                filename.native_object_reference_count()
            )
            .unwrap();
            let members = parent
                .native_object_list_elements(NativeStringProtocol::Jim084)
                .unwrap();
            writeln!(
                observed,
                "L\t{mode}\t{case}\t{}\t{}\t{}\t{}\t{}\t{}",
                kind(&parent),
                usize::from(parent.resident_string_bytes().is_some()),
                members.len(),
                kind(&filename),
                usize::from(filename.resident_string_bytes().is_some()),
                filename.native_object_reference_count()
            )
            .unwrap();
            for (index, member) in members.iter().enumerate() {
                assert!(member.native_jim_source_cache_present());
                let references = member.native_object_reference_count();
                let info = member.pin_native_jim_source_info(&context).unwrap();
                writeln!(
                    observed,
                    "E\t{mode}\t{case}\t{index}\tsource\t1\t{references}\t{}\t{}\t{}",
                    usize::from(info.filename.is_same_object(&filename)),
                    info.line,
                    hex(&member.resident_string_bytes().unwrap())
                )
                .unwrap();
            }
            drop(members);
            drop(parent);
            writeln!(
                observed,
                "D\t{mode}\t{case}\t{}",
                filename.native_object_reference_count()
            )
            .unwrap();
        }
    }
    let expected =
        include_str!("../../tcl-syntax/testdata/native_jim_source_list/observations.tsv")
            .lines()
            .filter(|row| row.split('\t').nth(1) != Some("0"))
            .collect::<Vec<_>>();
    assert_eq!(expected.len(), 222);
    assert_eq!(observed.lines().collect::<Vec<_>>(), expected);
}

#[test]
fn weak_context_association_preserves_ownership_and_refuses_other_interpreters() {
    let original = Value::new_native_string_bytes(&b"a b"[..]);
    let first = NativeJimObjectContext::new(jim()).unwrap();
    let second = NativeJimObjectContext::new(jim()).unwrap();
    original.bind_native_jim_context(&first).unwrap();
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(first.empty_object().native_object_reference_count(), 1);
    assert!(original.bind_native_jim_context(&second).is_err());
    drop(first);
    assert!(original.native_jim_context().is_err());
    assert!(original.bind_native_jim_context(&second).is_err());
    assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"a b");
}

#[test]
fn last_list_header_retires_source_filename_before_lifetime_view() {
    let context = NativeJimObjectContext::new(jim()).unwrap();
    context.select_numeric_host(std::rc::Rc::new(tcl_host_native::NativeHost::new()));
    let mut observed = String::new();
    for mode in 0..4 {
        let filename = Value::new_native_string_bytes(b"FILE".as_slice());
        let child = Value::new_native_string_bytes(if mode == 3 {
            b"4".as_slice()
        } else {
            b"CHILD".as_slice()
        });
        child
            .install_native_jim_source(
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
        let parent =
            Value::native_list_constructor(vec![child.clone()], NativeStringProtocol::Jim084);
        parent.bind_native_jim_context(&context).unwrap();
        let external = if mode == 1 {
            Some(child)
        } else {
            drop(child);
            None
        };
        let view = parent.cached_list_representation().unwrap().0;
        let weak = view[0].downgrade_native_object();
        let copied =
            (mode == 2).then(|| parent.duplicate_native_object_in(NativeStringProtocol::Jim084));
        writeln!(
            observed,
            "BEFORE\t{mode}\t{}\t{}\t1\t1",
            filename.native_object_reference_count(),
            view[0].native_object_reference_count()
        )
        .unwrap();
        if mode == 3 {
            parent
                .native_scalar_probe(
                    jim(),
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
                )
                .unwrap()
                .unwrap();
            writeln!(
                observed,
                "SHIMMER\t3\t0\t4\t{}",
                filename.native_object_reference_count()
            )
            .unwrap();
            assert!(view.elements().is_err());
        } else {
            drop(parent);
            writeln!(
                observed,
                "AFTER_FIRST\t{mode}\t{}",
                filename.native_object_reference_count()
            )
            .unwrap();
            if let Some(external) = external {
                writeln!(
                    observed,
                    "EXTERNAL_LIVE\t1\t{}\tsource",
                    external.native_object_reference_count()
                )
                .unwrap();
                assert!(view.elements().unwrap()[0].is_same_object(&external));
                drop(external);
                writeln!(
                    observed,
                    "AFTER_EXTERNAL\t1\t{}",
                    filename.native_object_reference_count()
                )
                .unwrap();
            }
            if let Some(copied) = copied {
                assert!(view.elements().is_ok());
                drop(copied);
                writeln!(
                    observed,
                    "AFTER_LAST\t2\t{}",
                    filename.native_object_reference_count()
                )
                .unwrap();
            }
            assert!(view.elements().is_err());
        }
        assert!(weak.upgrade().is_none());
        assert_eq!(
            <crate::Vm as tcl_syntax::value::ValueOps>::same_object(
                &crate::Vm::new(),
                &view[0],
                &view[0]
            ),
            None,
            "retired memory identity cannot authorize a native shortcut",
        );
        assert!(
            view[0]
                .native_string_bytes(NativeStringProtocol::Jim084)
                .is_err()
        );
        assert!(
            Value::from_retained_native_list_backing(&view, NativeStringProtocol::Jim084).is_err()
        );
    }
    let expected =
        include_str!("../../tcl-syntax/testdata/native_jim_list_last_header/observations.tsv");
    assert_eq!(expected.lines().count(), 11);
    assert_eq!(observed, expected);
}
