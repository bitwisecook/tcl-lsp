// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::{InvocationDialect, InvocationWord, InvocationWords};

fn selected(
    head: &'static str,
    operands: &[InvocationWord<'_>],
    environment: &str,
) -> (InvocationFacts, InvocationDialect) {
    let context = crate::model::ingress::static_context_for(environment);
    let dialect = InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment(environment).unit_profile(),
    );
    let words =
        InvocationWords::structured(InvocationWord::Literal(head), operands).with_dialect(dialect);
    (
        context
            .commands()
            .resolve_structured_invocation(words, dialect.authoring_query())
            .resolved()
            .expect("native descriptor")
            .facts(),
        dialect,
    )
}

fn field<'a>(row: &'a str, name: &str) -> &'a str {
    row.split_whitespace()
        .find_map(|part| {
            part.split_once('=')
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
        })
        .expect("native field")
}

fn native_class(before: &str) -> NativeStockListInputClass {
    use NativeStockListInputClass as Class;
    match before {
        "string" => Class::String,
        "list" => Class::List,
        "dict" => Class::Dictionary,
        "bytearray" => Class::ByteArray,
        "boolean" | "booleanString" => Class::Boolean,
        "int" | "wideInt" | "double" => Class::Numeric,
        other => panic!("unexpected native class {other}"),
    }
}

fn resident_bytes(row: &str) -> Option<&'static [u8]> {
    if field(row, "beforestring") != "1" {
        return None;
    }
    Some(match field(row, "mode").parse::<u8>().unwrap() {
        0 | 7 | 9 | 11 | 13 => b"",
        2 => b"{",
        1 | 3 | 5 => b"0",
        4 | 6 | 8 | 10 | 14 | 15 => b"2",
        12 => b"2 3",
        16 => b"true",
        _ => unreachable!(),
    })
}

#[test]
fn stock_length_native_same_object_matrix() {
    // Native proof: naming.list.stock-length-same-original-storage
    // docs/design/analysis/name-resolution-proofs/list.stock-length-same-original-storage.md

    let fixtures = [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-syntax/tests/data/native_list_methods/stock_length/8.4.20.txt"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-syntax/tests/data/native_list_methods/stock_length/8.5.19.txt"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-syntax/tests/data/native_list_methods/stock_length/8.6.18.txt"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-syntax/tests/data/native_list_methods/stock_length/9.0.4.txt"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-syntax/tests/data/native_list_methods/stock_length/9.1.0.txt"
            ),
        ),
        (
            "jim",
            include_str!("../../../tcl-syntax/tests/data/native_list_methods/stock_length/jim.txt"),
        ),
    ];
    let operands = [InvocationWord::Dynamic];
    let mut rows = 0;
    for (environment, fixture) in fixtures {
        let (facts, dialect) = selected("llength", &operands, environment);
        let protocol = facts
            .stock_list_length_protocol(
                InvocationArguments::structured(&operands).with_dialect(dialect),
            )
            .unwrap();
        assert_eq!(protocol.argument(), 0);
        for row in fixture.lines() {
            rows += 1;
            assert!(matches!(field(row, "code"), "0" | "1"));
            if field(row, "code") != "0" {
                continue;
            }
            match protocol
                .normal_cache_disposition(native_class(field(row, "before")), resident_bytes(row))
                .unwrap()
            {
                NativeStockListCacheDisposition::List => {
                    assert_eq!(field(row, "after"), "list", "{environment}: {row}");
                }
                NativeStockListCacheDisposition::Preserved => assert_eq!(
                    field(row, "before"),
                    field(row, "after"),
                    "{environment}: {row}"
                ),
                NativeStockListCacheDisposition::Unknown => {}
            }
        }
    }
    assert_eq!(rows, 204);
}

#[test]
fn unknown_stock_cache_and_empty_preservation_do_not_mint_list() {
    use NativeStockListCacheDisposition as Cache;
    use NativeStockListInputClass as Class;
    let operands = [InvocationWord::Dynamic];
    for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (mut facts, dialect) = selected("llength", &operands, environment);
        let args = InvocationArguments::structured(&operands).with_dialect(dialect);
        let protocol = facts.stock_list_length_protocol(args).unwrap();
        assert_eq!(
            protocol.normal_cache_disposition(Class::String, Some(b"")),
            Some(Cache::Unknown)
        );
        if dialect.tcl_version.unwrap() >= TclVersion::V9_0 {
            assert_eq!(
                protocol.normal_cache_disposition(Class::Unknown, Some(b"2")),
                Some(Cache::Unknown)
            );
            assert_eq!(
                protocol.normal_cache_disposition(Class::Numeric, None),
                Some(Cache::Preserved)
            );
        } else {
            assert_eq!(
                protocol.normal_cache_disposition(Class::Numeric, None),
                Some(Cache::List)
            );
            assert_eq!(
                protocol.normal_cache_disposition(Class::Unknown, None),
                Some(Cache::Unknown)
            );
        }
        facts.operation = SemanticOperationId::Invoke;
        assert_eq!(facts.stock_list_length_protocol(args), None);
    }
}

#[test]
fn empty_list_root_provider_requires_generic_normal_result() {
    // Native proof: naming.list.empty-result-compiled-pool-reuse
    // docs/design/analysis/name-resolution-proofs/list.empty-result-compiled-pool-reuse.md

    use crate::list_object_methods::NativeListMethod;
    let operands = [];
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let (facts, dialect) = selected("list", &operands, environment);
        let args = InvocationArguments::structured(&operands).with_dialect(dialect);
        let provider = facts
            .normal_empty_list_root_provider(args, NativeCompilationSelection::Generic)
            .unwrap();
        assert!(provider.world_is_closed_for(NativeListMethod::Length));
        assert!(!provider.world_is_closed_for(NativeListMethod::StringAccess));
        assert_eq!(
            facts.normal_empty_list_root_provider(args, NativeCompilationSelection::Unknown),
            None
        );
    }
    for fixture in [
        include_str!(
            "../../../tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.4.20.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.5.19.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.6.18.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.0.4.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.1.0.txt"
        ),
    ] {
        let generic = fixture.lines().next().unwrap();
        assert_eq!(field(generic, "same"), "0");
        assert_eq!(field(generic, "second"), "string");
    }
}

#[test]
fn object_length_dispatch_requires_actual_storage_and_distinct_logical_origin() {
    use NativeObjectLengthAction as Action;
    use NativeStockListInputClass as Class;
    for version in TclVersion::ALL {
        let protocol = InvocationDialect::for_version(version)
            .native_object_length_protocol()
            .unwrap();
        assert_eq!(protocol.logical_provider(), None);
        assert_eq!(
            protocol.action(Class::List, false),
            Some(Action::CachedList)
        );
        assert_eq!(
            protocol.action(Class::Numeric, false),
            Some(if version >= TclVersion::V9_0 {
                Action::Constant(1)
            } else {
                Action::ConvertToList
            })
        );
        assert_eq!(
            protocol.action(Class::ByteArray, false),
            Some(Action::ConvertToList)
        );
        assert_eq!(
            protocol.action(Class::ByteArray, true),
            Some(if version >= TclVersion::V8_5 {
                Action::Constant(0)
            } else {
                Action::ConvertToList
            })
        );
        assert_eq!(protocol.action(Class::Unknown, false), None);
    }
    let logical = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    assert_eq!(logical.native_object_length_protocol(), None);
    assert_eq!(logical.object_length_protocol(None), None);
    let provider = LogicalListLengthProvider::Tcl84CoreSimulation;
    let protocol = logical.object_length_protocol(Some(provider)).unwrap();
    assert_eq!(protocol.logical_provider(), Some(provider));
    assert_eq!(
        protocol.action(Class::Numeric, false),
        Some(Action::ConvertToList)
    );
}

#[test]
fn original_jim_source_length_requires_its_own_dialect_and_physical_class() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    // This pure policy selects conversion; it does not supply a Source object or context.
    use NativeObjectLengthAction as Action;
    use NativeStockListCacheDisposition as Cache;
    use NativeStockListInputClass as Class;
    let operands = [InvocationWord::Dynamic];
    let (facts, jim) = selected("llength", &operands, "jim");
    let protocol = jim.native_object_length_protocol().unwrap();
    assert_eq!(
        protocol.action(Class::JimSource, false),
        Some(Action::ConvertToList)
    );
    assert_eq!(protocol.action(Class::Unknown, false), None);
    assert_eq!(
        facts
            .stock_list_length_protocol(
                InvocationArguments::structured(&operands).with_dialect(jim)
            )
            .unwrap()
            .normal_cache_disposition(Class::JimSource, Some(b"A  B")),
        Some(Cache::List)
    );
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let protocol = dialect.native_object_length_protocol().unwrap();
        assert_eq!(protocol.action(Class::JimSource, false), None);
        assert_eq!(protocol.action(Class::JimSource, true), None);
        let (facts, _) = selected(
            "llength",
            &operands,
            match version {
                TclVersion::V8_4 => "tcl8.4",
                TclVersion::V8_5 => "tcl8.5",
                TclVersion::V8_6 => "tcl8.6",
                TclVersion::V9_0 => "tcl9.0",
                TclVersion::V9_1 => "tcl9.1",
            },
        );
        assert_eq!(
            facts
                .stock_list_length_protocol(
                    InvocationArguments::structured(&operands).with_dialect(dialect)
                )
                .unwrap()
                .normal_cache_disposition(Class::JimSource, Some(b"A  B")),
            None
        );
    }
}
