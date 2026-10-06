// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object native increment command and physical publication controls.

use super::{Local, Vm};
use crate::Value;
use std::rc::Rc;
use tcl_registry::InvocationDialect;
use tcl_syntax::{
    native_object::NativeObjectCacheSnapshot, number::Number, scalar_getter::NativeScalarCache,
    value::ValueOps,
};

fn shape(index: usize, dialect: InvocationDialect) -> Option<Value> {
    let cache = |cache| Value::from_native_scalar_cache(cache, None, dialect).unwrap();
    Some(match index {
        0 => Value::new_native_string_bytes(b"4".as_slice()),
        1 | 10 => {
            let integer = if index == 1 { 4 } else { 1 };
            if dialect.native_string_protocol().unwrap().tcl_version()
                == Some(tcl_dialect::TclVersion::V8_4)
            {
                cache(NativeScalarCache::Tcl84Long(integer))
            } else {
                cache(NativeScalarCache::Number(Number::Int(integer)))
            }
        }
        2 => cache(NativeScalarCache::Number(Number::Int(4))),
        3 => Value::double(4.0),
        4 => Value::new_native_string_bytes(b"1.5".as_slice()),
        5 => cache(NativeScalarCache::Number(Number::Int(i64::MAX))),
        6 => Value::new_native_string_bytes(b"BAD".as_slice()),
        7 => Value::new_native_string_bytes(b"1 + 2".as_slice()),
        8 => Value::double(1.0),
        9 => Value::double(1.5),
        11 => Value::new_native_string_bytes(b"1\0BAD".as_slice()),
        12 => return None,
        _ => unreachable!("fixed native increment shape"),
    })
}

fn physical_state(original: Option<&Value>, dialect: InvocationDialect) -> String {
    let Some(original) = original else {
        return "missing,0,0".into();
    };
    let snapshot = original.native_object_snapshot();
    let (name, integer) = match snapshot.cache {
        NativeObjectCacheSnapshot::None => ("none", None),
        NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Tcl84Long(integer)) => {
            ("int", Some(integer))
        }
        NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Number(Number::Int(integer))) => {
            let name = if dialect.native_string_protocol().unwrap().tcl_version()
                == Some(tcl_dialect::TclVersion::V8_4)
            {
                "wideInt"
            } else {
                "int"
            };
            (name, Some(integer))
        }
        NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Number(Number::Double(_))) => {
            ("double", None)
        }
        NativeObjectCacheSnapshot::Other if original.native_expression_cache_present() => {
            ("expression", None)
        }
        NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Number(Number::Big { .. })) => {
            ("bignum", None)
        }
        other => panic!("unselected increment fixture cache: {other:?}"),
    };
    let mut result = format!(
        "{name},{},{}",
        usize::from(snapshot.resident.is_some()),
        original.native_object_reference_count()
    );
    if let Some(integer) = integer {
        use std::fmt::Write;
        write!(result, ",{integer}").expect("writing into a String");
    }
    result
}

fn current(vm: &Vm) -> Option<&Value> {
    let selected = vm.resolve_var_from_bytes(b"x", vm.current_level())?;
    let cell = vm.var_arena.get(selected.id?)?;
    match cell.state() {
        Local::Scalar(value) => Some(value),
        _ => None,
    }
}

fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

const INCREMENT_FIXTURES: [(&str, &str); 6] = [
    (
        "tcl8.4",
        include_str!("../../tests/data/native_legacy_increment/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../tests/data/native_legacy_increment/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../tests/data/native_legacy_increment/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../tests/data/native_legacy_increment/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../tests/data/native_legacy_increment/9.1.0.tsv"),
    ),
    (
        "jim",
        include_str!("../../tests/data/native_legacy_increment/Jim.tsv"),
    ),
];

const INCREMENT_SHAPES: [(usize, usize); 15] = [
    (0, 10),
    (1, 10),
    (2, 10),
    (3, 10),
    (4, 10),
    (5, 10),
    (0, 8),
    (0, 9),
    (0, 7),
    (0, 6),
    (6, 6),
    (12, 6),
    (12, 10),
    (0, 11),
    (6, 10),
];

fn increment_fixture_host(engine: &str) -> Option<Rc<tcl_host_native::NativeHost>> {
    if engine == "tcl8.4" {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let formatter =
            tcl_test_support::native_integer_formatter::load_pinned_c84_integer_formatter(&root)
                .expect("explicit pinned C84 native updater capability");
        Some(Rc::new(
            tcl_host_native::NativeHost::new().with_integer_formatter(formatter),
        ))
    } else {
        None
    }
}

fn selected_increment_host(
    host: &Option<Rc<tcl_host_native::NativeHost>>,
) -> Rc<dyn tcl_platform::Host> {
    match host {
        Some(host) => host.clone(),
        None => Rc::new(tcl_host_native::NativeHost::new()),
    }
}

fn increment_window_field<'a>(fields: &[&'a str], key: &str) -> &'a str {
    fields
        .iter()
        .find_map(|field| field.strip_prefix(key))
        .unwrap()
}

#[test]
fn native_increment_commands_match_180_original_object_controls() {
    let engines = INCREMENT_FIXTURES;
    let shapes = INCREMENT_SHAPES;
    let mut compared = 0;
    for (engine, observations) in engines {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let host = increment_fixture_host(engine);
        for row in observations.lines() {
            // The native probe constructs a fresh interpreter for each row.
            // In particular, a previous Jim error must not suppress this
            // invocation's first error-stack capture.
            let selected_host = selected_increment_host(&host);
            let mut vm = crate::native_fixture::core_with_host(profile, selected_host);
            let dialect = vm.native_invocation_dialect();
            let fields = row.split('\t').collect::<Vec<_>>();
            let index = fields[0].parse::<usize>().unwrap();
            let shared = fields[1] == "1";
            let case = format!("{index}/shared={shared}");
            tcl_test_support::oracle_row_progress("increment180", engine, &case, None);
            let start = std::time::Instant::now();
            let original = shape(shapes[index].0, dialect);
            let identity = original.as_ref().map(Value::native_object_identity);
            let retained = if shared { original.clone() } else { None };
            if let Some(original) = original {
                vm.set_var_bytes(b"x", original).unwrap();
            }
            let arguments = [
                Value::new_native_string_bytes(b"x".as_slice()),
                shape(shapes[index].1, dialect).unwrap(),
            ];
            let value = |key: &str| increment_window_field(&fields, key);
            assert_eq!(
                physical_state(current(&vm), dialect),
                value("before-current="),
                "{engine}/{case}"
            );
            assert_eq!(
                physical_state(Some(&arguments[1]), dialect),
                value("before-amount="),
                "{engine}/{case}"
            );
            let completion = vm
                .try_invoke_command("incr", &arguments)
                .unwrap_or_else(|error| {
                    panic!("{engine}/{case}: unexpected command refusal: {error:?}")
                });
            tcl_test_support::oracle_phase_progress(
                "increment180",
                engine,
                &case,
                "command-complete",
                start,
            );
            let code = completion.code;
            let (completion, borrowed_result) = if engine == "jim" {
                // The native window borrows Jim_GetResult. The actual context
                // owns that result; the API's returned Completion is a separate
                // Rust owner and must be released before the physical snapshot.
                let result = completion.result.native_lifetime_lease();
                drop(completion);
                (None, Some(result))
            } else {
                (Some(completion), None)
            };
            let result = match &borrowed_result {
                Some(result) => result.value(),
                None => &completion.as_ref().expect("C result owner").result,
            };
            assert_eq!(
                physical_state(current(&vm), dialect),
                value("after-current="),
                "{engine}/{case}"
            );
            assert_eq!(
                physical_state(Some(&arguments[1]), dialect),
                value("after-amount="),
                "{engine}/{case}"
            );
            if shared {
                assert_eq!(
                    physical_state(retained.as_ref(), dialect),
                    value("original-current="),
                    "{engine}/{case}"
                );
            }
            let same = current(&vm)
                .is_some_and(|current| Some(current.native_object_identity()) == identity);
            assert_eq!(
                usize::from(same).to_string(),
                value("identity="),
                "{engine}/{case}"
            );
            assert_eq!(code.as_int().to_string(), value("code="), "{engine}/{case}");
            assert_eq!(
                vm.native_string_bytes(result).unwrap().as_ref(),
                unhex(value("result=")).as_slice(),
                "{engine}/{case}"
            );
            compared += 1;
            tcl_test_support::oracle_row_progress("increment180", engine, &case, Some(compared));
        }
    }
    assert_eq!(compared, 180);
}

#[test]
fn jim_safe_integer_expression_matches_54_original_native_objects() {
    fn unhex(encoded: &str) -> Vec<u8> {
        if encoded == "-" {
            return Vec::new();
        }
        encoded
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut vm = crate::native_fixture::core(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    let context = vm.native_jim_object_context().unwrap();
    context.publish_result(&Value::new_native_string_bytes(b"k 10".as_slice()));
    let mut compared = 0;
    for row in
        include_str!("../../../tcl-syntax/testdata/native_jim_safe_expression/observations.tsv")
            .lines()
    {
        let fields: Vec<_> = row.split('\t').collect();
        let bytes = unhex(fields[9]);
        let original = Value::new_native_string_bytes(bytes.as_slice());
        original.bind_native_jim_context(&context).unwrap();
        let filename = Value::new_native_string_bytes(b"file".as_slice());
        match fields[0] {
            "0" => {}
            "1" => original
                .install_native_jim_source(
                    crate::value::NativeJimSourceInfo { filename, line: 7 },
                    &context,
                )
                .unwrap(),
            "2" => {
                original
                    .native_character_count(tcl_dialect::StringCharacterModel::Jim084Utf8, None)
                    .unwrap();
            }
            _ => panic!("fixed native original mode"),
        }
        let result = vm.native_jim_wide_expression(&original);
        if let Err(error) = &result {
            assert!(
                error.native_access_refusal().is_none(),
                "native getter guest failure must retain its channel"
            );
        }
        assert_eq!(
            result.is_err(),
            fields[4] == "1",
            "mode{} case{}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            result.unwrap_or(123),
            fields[5].parse::<i64>().unwrap(),
            "mode{} case{}",
            fields[0],
            fields[1]
        );
        let primary = if original.native_expression_cache_present() {
            "expression"
        } else if original.native_jim_source_cache_present() {
            "source"
        } else {
            match original.native_object_snapshot().cache {
                NativeObjectCacheSnapshot::Numeric(NativeScalarCache::Number(Number::Int(_))) => {
                    "int"
                }
                NativeObjectCacheSnapshot::JimString { .. } => "string",
                NativeObjectCacheSnapshot::None => "none",
                _ => panic!("unexpected original Jim primary"),
            }
        };
        assert_eq!(primary, fields[6], "mode{} case{}", fields[0], fields[1]);
        assert_eq!(original.resident_string_bytes().is_some(), fields[7] == "1");
        assert_eq!(
            context
                .result_object()
                .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
                .unwrap()
                .as_ref(),
            unhex(fields[8]),
            "mode{} case{}",
            fields[0],
            fields[1]
        );
        compared += 1;
    }
    assert_eq!(compared, 54);
}
