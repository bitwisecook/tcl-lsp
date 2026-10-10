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
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").unwrap();
        output
    })
}

fn unhex(text: &str) -> Vec<u8> {
    if text == "-" {
        return Vec::new();
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
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

fn source_list_originals(
    mode: i32,
    context: &std::rc::Rc<NativeJimObjectContext>,
) -> (Value, Value, Option<Value>) {
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
            context,
        )
        .unwrap();
    let parent = Value::native_list_constructor(vec![child.clone()], NativeStringProtocol::Jim084);
    parent.bind_native_jim_context(context).unwrap();
    let external = if mode == 1 {
        Some(child)
    } else {
        drop(child);
        None
    };
    (parent, filename, external)
}

#[test]
fn last_list_header_retires_source_filename_before_lifetime_view() {
    let context = NativeJimObjectContext::new(jim()).unwrap();
    context.select_numeric_host(std::rc::Rc::new(tcl_host_native::NativeHost::new()));
    let mut observed = String::new();
    for mode in 0..4 {
        let (parent, filename, external) = source_list_originals(mode, &context);
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

#[test]
fn original_jim_source_length_preserves_resident_bytes_and_child_source() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    use tcl_syntax::value::ValueOps;
    let mut vm = crate::Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        std::rc::Rc::new(crate::host_native::NativeHost::new()),
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = vm.native_jim_object_context().unwrap();
    let parent = Value::new_native_string_bytes(b"A  B".as_slice());
    parent
        .install_native_jim_source(
            NativeJimSourceInfo {
                filename: Value::new_native_string_bytes(b"source-check.tcl".as_slice()),
                line: 17,
            },
            &context,
        )
        .unwrap();
    assert_eq!(
        parent.stock_list_input_class(),
        tcl_registry::native_stock_list::NativeStockListInputClass::JimSource
    );
    let mut observed = String::new();
    writeln!(
        observed,
        "SOURCE_BEFORE|{}|{}",
        kind(&parent),
        usize::from(parent.resident_string_bytes().is_some())
    )
    .unwrap();
    let length = vm.list_len(&parent).unwrap();
    writeln!(
        observed,
        "SOURCE_AFTER|{}|{length}|{}|{}",
        kind(&parent),
        usize::from(parent.resident_string_bytes().is_some()),
        hex(&parent.resident_string_bytes().unwrap())
    )
    .unwrap();
    let member = vm.list_index(&parent, 0).unwrap().unwrap();
    let info = member.pin_native_jim_source_info(&context).unwrap();
    writeln!(
        observed,
        "SOURCE_CHILD|{}|{}|{}",
        kind(&member),
        info.line,
        hex(&info.filename.resident_string_bytes().unwrap())
    )
    .unwrap();
    let native =
        include_str!("../../tcl-registry/tests/data/native_source_list_length241/jim/stdout");
    assert_eq!(
        observed.lines().collect::<Vec<_>>(),
        native
            .lines()
            .filter(|row| row.starts_with("SOURCE_"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn original_source_length_and_concat_channels_match_all_native_providers() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    macro_rules! fixture {
        ($file:literal) => {
            include_bytes!(concat!(
                "../../tcl-registry/tests/data/native_source_list_length241/",
                $file
            ))
        };
    }
    macro_rules! rows {
        ($provider:literal) => {
            include_str!(concat!(
                "../../tcl-registry/tests/data/native_source_list_length241/",
                $provider,
                "/stdout"
            ))
        };
    }
    let sources: [&[u8]; 3] = [
        fixture!("original-0.tcl"),
        fixture!("original-1.tcl"),
        fixture!("original-2.tcl"),
    ];
    let mut compared = 0;
    for (engine, native) in [
        ("tcl8.4", rows!("8.4.20")),
        ("tcl8.5", rows!("8.5.19")),
        ("tcl8.6", rows!("8.6.18")),
        ("tcl9.0", rows!("9.0.4")),
        ("tcl9.1", rows!("9.1.0")),
        ("jim", rows!("jim")),
    ] {
        let mut vm = crate::Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        for (index, source) in sources.into_iter().enumerate() {
            let completion = vm.try_eval_source_bytes(source).unwrap();
            let label = format!("ORIGINAL_{index}");
            let row = native
                .lines()
                .find(|row| row.split('|').next() == Some(label.as_str()))
                .unwrap();
            let fields = row.split('|').collect::<Vec<_>>();
            assert_eq!(
                completion.code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{engine}/{index}"
            );
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                unhex(fields[2]),
                "{engine}/{index}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 18);
}

#[test]
fn native_jim_public_source_uses_original_script_without_c_compiler_or_library_grants() {
    // This is an authentic backend entry/availability control. Original110,
    // Native236/245 and other public source comparators retain their own evidence.
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::core(profile);
    let completion = vm
        .try_eval_source("namespace eval N {}; proc N::p {} {return VALUE}; N::p")
        .unwrap();
    assert_eq!(completion.code, crate::Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"VALUE");

    let counted = vm
        .try_eval_source_bytes(b"set {raw\xff} COUNTED; set {raw\xff}")
        .unwrap();
    assert_eq!(counted.code, crate::Code::Ok);
    assert_eq!(counted.result.string_bytes().as_ref(), b"COUNTED");
    assert_eq!(
        vm.get_var_bytes(b"raw\xff")
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"COUNTED"
    );

    let completion = vm
        .try_eval_source_at(
            "info script",
            tcl_runtime_api::script_source_location::ScriptSourceLocation {
                file: "actual-source.tcl".into(),
                line: 17,
            },
        )
        .unwrap();
    assert_eq!(completion.code, crate::Code::Ok);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"actual-source.tcl"
    );

    let unavailable = vm
        .try_eval_source("catch {{dict update} d k v {}} result; set result")
        .unwrap();
    assert_eq!(unavailable.code, crate::Code::Ok);
    assert_eq!(
        unavailable.result.string_bytes().as_ref(),
        b"invalid command name \"dict update\""
    );
    assert!(vm.lookup_command("dict update").is_none());

    // Jim parses a whole original Script before executing its first command.
    let malformed = vm
        .try_eval_source("set must_not_run YES; set missing {")
        .unwrap();
    assert_eq!(malformed.code, crate::Code::Error);
    assert!(vm.get_var_bytes(b"must_not_run").is_none());
}

#[test]
fn original_c_source_still_requires_its_independent_compile_service() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        assert!(vm.try_eval_source("set must_not_run YES").is_err());
        assert!(vm.get_var_bytes(b"must_not_run").is_none());
    }
}
