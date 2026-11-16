// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(have_tommath)]
#[test]
fn measured_bare_matches_runtime_requires_independent_authored_policy() {
    use tcl_registry::invocation_words::{
        LogicalExpressionParseProvider, LogicalSourceWordProvider,
    };
    let fixture =
        include_str!("../../../../rust/tcl-syntax/tests/data/f5-matches-21.1.0.1-0.0.26.tsv");
    leak_free(|interp| {
        interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(super::parse_runtime_expr(interp, b"\"abcd\" matches \"a*\"").is_err());
        assert!(interp.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            crate::environment::profile_for_dialect("tcl9.0"),
        ));
        // Quoted operands retain their independently installed source-word
        // interpretation; the expression parser cannot supply that policy.
        assert!(interp.source_string_protocol().is_none());
        assert!(interp.set_logical_source_word_provider(
            LogicalSourceWordProvider::Tcl84CoreSimulation,
            crate::environment::profile_for_dialect("tcl9.0"),
        ));
        for row in fixture.lines().take(6) {
            let fields = row.split('\t').collect::<Vec<_>>();
            assert_eq!(
                super::eval_expr_prepared_source(interp, None, fields[0].as_bytes()),
                Code::Ok,
                "{row}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_bytes(), fields[2].as_bytes(), "{row}");
        }
        for (expression, expected) in [
            (r#""foobar" contains "oob""#, "1"),
            (r#""foobar" starts_with "foo""#, "1"),
            (r#""foobar" ends_with "foo""#, "0"),
            (r#""foobar" matches_glob "f?o*""#, "1"),
        ] {
            assert_eq!(
                super::eval_expr_prepared_source(interp, None, expression.as_bytes()),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), expected.as_bytes());
        }
        assert_eq!(
            super::eval_expr_prepared_source(interp, None, br#""foobar" matches_regex "o+b""#),
            Code::Error
        );
    });
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(name));
            for row in fixture.lines() {
                let expression = row.split('\t').next().unwrap();
                assert_eq!(
                    super::eval_expr_prepared_source(interp, None, expression.as_bytes()),
                    Code::Error,
                    "{name}: {row}"
                );
            }
        });
    }
}

use crate::counters;
use crate::interp::{Code, Interp};

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?} → {:?}; host refusal: {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes()),
        i.native_access_refusal()
    );
    i.result_bytes()
}

#[cfg(have_tommath)]
#[test]
fn logical_parser_requires_an_independent_host_and_separate_cache_policy() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
    leak_free(|interp| {
        let source = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        interp.set_dialect_profile(source);
        assert!(super::parse_runtime_expr(interp, b"{\xff\0tail}").is_err());
        assert!(!interp.set_logical_expression_parse_provider(provider, source));
        assert!(interp.set_logical_expression_parse_provider(provider, host));
        let object = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{\xff\0tail}"));
        super::parse_runtime_expr_cached(interp, Some(object.as_ptr()), b"{\xff\0tail}")
            .expect("explicit logical parsing");
        let dialect = interp.native_invocation_dialect();
        assert!(crate::expr::cached_expr(object.as_ptr(), dialect, None).is_none());
        assert!(crate::expr::cached_expr(
            object.as_ptr(),
            dialect,
            interp.logical_expression_parse_policy()
        )
        .is_some());
        assert_eq!(crate::interp::obj_bytes(object.as_ptr()), b"{\xff\0tail}");
        assert!(dialect.execution_point().is_none());
    });
}

#[cfg(have_tommath)]
#[test]
fn logical_parser_host_changes_retire_cached_policy_without_rebinding_objects() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
    leak_free(|interp| {
        let source = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        let replacement = crate::environment::profile_for_dialect("tcl9.1");
        interp.set_dialect_profile(source);
        assert!(interp.set_logical_expression_parse_provider(provider, host));
        let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{\xff\0tail}"));
        let pointer = original.as_ptr();
        let dialect = interp.native_invocation_dialect();
        let policy = interp.logical_expression_parse_policy();
        let parsed = super::parse_runtime_expr_cached(interp, Some(pointer), b"{\xff\0tail}")
            .expect("authored logical parser");

        assert!(interp.set_logical_expression_parse_provider(provider, host));
        assert!(!interp.set_logical_expression_parse_provider(provider, source));
        assert_eq!(interp.logical_expression_parse_policy(), policy);
        let retained = crate::expr::cached_expr(pointer, dialect, policy).unwrap();
        assert!(std::rc::Rc::ptr_eq(&parsed, &retained));

        assert!(interp.set_logical_expression_parse_provider(provider, replacement));
        let current_policy = interp.logical_expression_parse_policy();
        assert_ne!(current_policy, policy);
        assert!(crate::expr::cached_expr(pointer, dialect, current_policy).is_none());
        assert!(std::rc::Rc::ptr_eq(
            &parsed,
            &crate::expr::cached_expr(pointer, dialect, policy).unwrap(),
        ));
        assert_eq!(original.as_ptr(), pointer);
        assert_eq!(crate::interp::obj_bytes(pointer), b"{\xff\0tail}");

        let reparsed = super::parse_runtime_expr_cached(interp, Some(pointer), b"{\xff\0tail}")
            .expect("replacement host policy");
        assert!(!std::rc::Rc::ptr_eq(&parsed, &reparsed));
        assert!(crate::expr::cached_expr(pointer, dialect, policy).is_none());
        assert!(std::rc::Rc::ptr_eq(
            &reparsed,
            &crate::expr::cached_expr(pointer, dialect, current_policy).unwrap(),
        ));
        assert!(interp
            .native_compiler_cache_epochs(crate::namespace::GLOBAL)
            .is_none());
    });
}

#[cfg(have_tommath)]
#[test]
fn logical_expression_materialises_original_storage_under_its_actual_host() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    leak_free(|interp| {
        let source = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        interp.set_dialect_profile(source);
        assert!(interp.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            host,
        ));
        let recipe = tcl_registry::InvocationDialect::of_profile(host)
            .byte_array_string_recipe(None)
            .unwrap();
        let object = crate::obj::Owned::fresh(crate::bytearray::new_byte_array(b"1+2", recipe));
        assert!(!crate::obj::has_string_rep(object.as_ptr()));
        super::parse_runtime_expr_cached(interp, Some(object.as_ptr()), b"IGNORED")
            .expect("actual host byte-array materialisation and logical syntax");
        assert_eq!(crate::obj::bytes_of(object.as_ptr()), b"1+2");
        assert!(crate::expr::cached_expr(
            object.as_ptr(),
            interp.native_invocation_dialect(),
            interp.logical_expression_parse_policy()
        )
        .is_some());
    });
}

#[cfg(have_tommath)]
#[test]
fn jim_safe_integer_expression_matches_54_original_native_objects() {
    fn unhex(encoded: &str) -> Vec<u8> {
        if encoded == "-" {
            return Vec::new();
        }
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let context = interp.native_jim_object_context().unwrap();
        interp.set_result_bytes(b"k 10");
        let mut compared = 0;
        for row in include_str!(
            "../../../../rust/tcl-syntax/testdata/native_jim_safe_expression/observations.tsv"
        )
        .lines()
        {
            let fields: Vec<_> = row.split('\t').collect();
            let bytes = unhex(fields[9]);
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(&bytes));
            crate::native_source::bind_context(original.as_ptr(), &context).unwrap();
            let filename = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"file"));
            match fields[0] {
                "0" => {}
                "1" => crate::native_source::install_source(
                    original.as_ptr(),
                    crate::native_source::NativeJimSourceInfo { filename, line: 7 },
                    &context,
                )
                .unwrap(),
                "2" => {
                    crate::obj::jim_character_count(original.as_ptr());
                }
                _ => panic!("fixed native original mode"),
            }
            let result = super::native_jim_wide_expression(interp, original.as_ptr());
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
            let kind = crate::obj::obj_type_ptr(original.as_ptr());
            let primary = if kind.is_null() {
                "none"
            } else if kind == &crate::expr::JIM_EXPR_TYPE {
                "expression"
            } else if kind == &crate::native_source::JIM_SOURCE_TYPE {
                "source"
            } else if kind == &crate::obj::TCL_INT_TYPE {
                "int"
            } else if kind == &crate::obj::JIM_STRING_TYPE {
                "string"
            } else {
                panic!("unexpected original Jim primary")
            };
            assert_eq!(primary, fields[6], "mode{} case{}", fields[0], fields[1]);
            assert_eq!(
                crate::obj::has_string_rep(original.as_ptr()),
                fields[7] == "1"
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[8]),
                "mode{} case{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 54);
    });
}

#[cfg(have_tommath)]
#[test]
fn jim_expression_preparation_uses_native_cache_action() {
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let context = interp.native_jim_object_context().unwrap();
        let filename = crate::obj::Owned::fresh(crate::obj::new_wide_int_obj(17));
        for (bytes, preserves_source) in [
            (&b""[..], true),
            (&b"{"[..], true),
            (&b"\""[..], true),
            (&b"["[..], true),
            (&b"$"[..], false),
            (&b"1+"[..], false),
            (&b"1 2"[..], false),
            (&b"."[..], false),
        ] {
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
            crate::native_source::install_source(
                original.as_ptr(),
                crate::native_source::NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 10,
                },
                &context,
            )
            .unwrap();
            assert!(
                super::parse_runtime_expr_cached(interp, Some(original.as_ptr()), bytes).is_err()
            );
            let expected = if preserves_source {
                &crate::native_source::JIM_SOURCE_TYPE
            } else {
                &crate::expr::JIM_EXPR_TYPE
            };
            assert!(std::ptr::eq(
                crate::obj::obj_type_ptr(original.as_ptr()),
                expected
            ));
            assert!(!crate::obj::has_string_rep(filename.as_ptr()));
            assert_eq!(crate::obj::bytes_of(original.as_ptr()), bytes);
        }
    });
}

#[cfg(have_tommath)]
#[test]
fn expression_source_preparation_replaces_actual_typed_storage() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let one = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"1"));
            let plus = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"+"));
            let two = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"2"));
            for valid in [true, false] {
                let elements = [one.as_ptr(), plus.as_ptr(), two.as_ptr()];
                let source = crate::obj::Owned::fresh(
                    interp.new_list_object(&elements[..if valid { 3 } else { 2 }]),
                );
                let previous = crate::obj::obj_type_ptr(source.as_ptr());
                let bytes = crate::interp::obj_bytes(source.as_ptr());
                let result =
                    super::parse_runtime_expr_cached(interp, Some(source.as_ptr()), &bytes);
                assert_eq!(result.is_ok(), valid, "{environment}");
                if valid || environment != "tcl8.4" {
                    assert!(
                        core::ptr::eq(
                            crate::obj::obj_type_ptr(source.as_ptr()),
                            if environment == "jim" {
                                &crate::expr::JIM_EXPR_TYPE
                            } else {
                                &crate::expr::TCL_EXPR_TYPE
                            }
                        ),
                        "{environment}"
                    );
                } else {
                    assert_eq!(crate::obj::obj_type_ptr(source.as_ptr()), previous);
                }
                assert_eq!(
                    crate::interp::obj_bytes(source.as_ptr()),
                    if valid { &b"1 + 2"[..] } else { &b"1 +"[..] }
                );
            }
        });
    }
}

#[cfg(have_tommath)]
#[test]
fn source_word_simulation_is_independent_of_expression_parser_installation() {
    leak_free(|interp| {
        let source = tcl_dialect::DialectProfile::irules();
        let host = crate::environment::profile_for_dialect("tcl9.0");
        let provider =
            tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation;
        interp.set_dialect_profile(source);
        assert!(interp.source_string_protocol().is_none());
        assert!(!interp.set_logical_source_word_provider(provider, source));
        assert!(interp.set_logical_source_word_provider(provider, host));
        assert!(interp.logical_expression_parse_policy().is_none());
        assert_eq!(
            interp.source_string_protocol(),
            Some(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V8_4
            ))
        );
        assert_eq!(
            interp.do_expression_subst(br"\u0000", false),
            Ok(vec![0xc0, 0x80])
        );
        assert!(interp
            .native_invocation_dialect()
            .execution_point()
            .is_none());
    });
}

#[cfg(have_tommath)]
#[test]
fn original_expression_objects_preserve_opaque_source_and_values() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let stored = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"7"));
            interp
                .var_set(b"\xff", stored.as_ptr())
                .expect("original byte variable name");
            let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"expr"));
            for (source, expected) in [
                (&b"\"\xff\0tail\""[..], &b"\xff\0tail"[..]),
                (&b"{\xff\0tail}"[..], &b"\xff\0tail"[..]),
                (&b"${\xff}"[..], &b"7"[..]),
                (&b"\"${\xff}\""[..], &b"7"[..]),
                (&b"0 && \"${\xffmissing}\""[..], &b"0"[..]),
            ] {
                let object = crate::obj::Owned::fresh(crate::obj::new_string_bytes(source));
                for _ in 0..2 {
                    assert_eq!(
                        super::expr_cmd(interp, &[head.as_ptr(), object.as_ptr()]),
                        Code::Ok,
                        "{environment}: {:?}",
                        interp.result_bytes()
                    );
                    assert_eq!(interp.result_bytes(), expected, "{environment}");
                    assert_eq!(crate::interp::obj_bytes(object.as_ptr()), source);
                }
            }
        });
    }
}

#[cfg(have_tommath)]
#[test]
fn expression_arguments_use_native_concatenation_and_selected_cardinality() {
    let case = tcl_syntax::execution_conformance::vectors(
        tcl_syntax::execution_conformance::ExecutionDomain::CommandBinding,
    )
    .into_iter()
    .find(|case| case.id == "expression_argument_concatenation_preserves_native_values")
    .expect("shared native expression argument vector");
    let script = format!("list [catch {{{}}} value] $value", case.source);
    for reference in tcl_test_support::available_tclshs() {
        let expected = tcl_test_support::run_script(&reference.path, case.script().as_bytes())
            .expect("native expression arguments")
            .strict_text()
            .expect("native result");
        leak_free(|interp| {
            interp.set_runtime_version(reference.version);
            assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
            assert_eq!(interp.result_bytes(), expected.as_bytes());
        });
    }
    if let Some(reference) = tcl_test_support::locate_jimsh().expect("Jim discovery") {
        let expected = tcl_test_support::run_script(&reference.path, case.script().as_bytes())
            .expect("native Jim expression arguments")
            .strict_text()
            .expect("native result");
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            );
            assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
            assert_eq!(interp.result_bytes(), expected.as_bytes());
        });
    }
}

#[cfg(have_tommath)]
#[test]
fn expression_variable_references_preserve_literal_indices_and_fire_traces() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|i| {
            let profile = if dialect == "jim" {
                Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                    "jim",
                    &[],
                    "Jim",
                    tcl_dialect::model::DialectPoint::canonical(
                        tcl_dialect::model::Release::JIM_0_84,
                    ),
                )))
            } else {
                tcl_dialect::DialectProfile::find(dialect).expect("profile")
            };
            i.set_dialect_profile(profile);
            assert_eq!(ok(i, br#"set {a([step])} LITERAL; set a(k) LIVE; set hits 0; proc step {} {incr ::hits; return k}; set literal [expr {${a([step])}}]; set live [expr {$a([step])}]; list $literal $live $hits"#), b"LITERAL LIVE 1", "{dialect}");
            if dialect != "jim" {
                assert_eq!(ok(i, br#"set x 5; set reads 0; proc tr {args} {incr ::reads}; trace add variable x read tr; list [expr {$x + 1}] $reads"#), b"6 1", "{dialect}");
            }
        });
    }
}

/// `upvar 1` from a procedure a `namespace eval` body calls resolves the
/// name as that body would: its namespace variable.
#[test]
fn upvar_from_a_namespace_eval_call_reads_the_namespace_variable() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            assert_eq!(
                    ok(
                        interp,
                        b"namespace eval ns {set x world; proc up {vn} {upvar 1 $vn v; return $v}; up x}"
                    ),
                    b"world",
                    "{dialect}"
                );
        });
    }
}

/// `source -encoding` decodes the file under the named encoding.
#[test]
fn source_decodes_under_the_named_encoding() {
    let path = std::env::temp_dir().join(format!("tclrt_source_latin1_{}.tcl", std::process::id()));
    std::fs::write(&path, b"set s \"caf\xe9\"\n").unwrap();
    for dialect in ["tcl8.6", "tcl9.0"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            let script = format!(
                    "source -encoding iso8859-1 {{{}}}; list [string length $s] [scan [string index $s end] %c]",
                    path.display()
                );
            assert_eq!(ok(interp, script.as_bytes()), b"4 233", "{dialect}");
        });
    }
    std::fs::remove_file(&path).unwrap();
}

/// A double read as a string under 8.4 is read back from its 12-digit
/// text; from 8.5 the shortest round-trip spelling keeps the value.
#[test]
fn a_double_read_as_a_string_is_reread_from_its_text_under_tcl84() {
    for (dialect, after) in [
        ("tcl8.4", b"0".as_slice()),
        ("tcl8.5", b"1"),
        ("tcl9.0", b"1"),
    ] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            assert_eq!(
                ok(interp, b"set v [expr {1.0/3}]; expr {$v*3 == 1}"),
                b"1",
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"string length $v; expr {$v*3 == 1}"),
                after,
                "{dialect}"
            );
        });
    }
}

/// `incr` creates a missing variable from 8.5; 8.4 reads it first and
/// raises "no such variable".
#[test]
fn incr_of_a_missing_variable_follows_the_selected_release() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            let code = interp.eval_str(b"incr n");
            if dialect == "tcl8.4" {
                assert_eq!(code, Code::Error, "{dialect}");
                assert_eq!(
                    interp.result_bytes(),
                    b"can't read \"n\": no such variable",
                    "{dialect}"
                );
            } else {
                assert_eq!(code, Code::Ok, "{dialect}");
                assert_eq!(interp.result_bytes(), b"1", "{dialect}");
            }
        });
    }
}

#[cfg(have_tommath)]
#[test]
fn native_expression_and_increment_follow_selected_numeric_towers() {
    let ok = |interp: &mut Interp, source: &[u8]| {
        let host = interp.host();
        let environment = host.numeric_environment();
        let before = environment.map(tcl_platform::NumericEnvironment::state);
        let code = interp.eval_str(source);
        let after =
            (code != Code::Ok).then(|| environment.map(tcl_platform::NumericEnvironment::state));
        assert_eq!(
                code,
                Code::Ok,
                "eval {:?} → {:?}; host refusal: {:?}; dialect: {:?}; numeric before: {:?}; failure after: {:?}",
                String::from_utf8_lossy(source),
                String::from_utf8_lossy(&interp.result_bytes()),
                interp.native_access_refusal(),
                interp.dialect_profile().name,
                before,
                after
            );
        interp.result_bytes()
    };
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            // Each dialect's success vectors start with clear host errno.
            interp
                .host()
                .numeric_environment()
                .expect("native numeric fixture host")
                .reset()
                .expect("zero-errno success precondition");
            let fixed = matches!(dialect, "tcl8.4" | "jim");
            assert_eq!(
                ok(interp, b"expr {(9223372036854775807 + 1) < 0}"),
                if fixed {
                    b"1".as_slice()
                } else {
                    b"0".as_slice()
                },
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"set n 9223372036854775807; incr n; expr {$n < 0}"),
                if fixed {
                    b"1".as_slice()
                } else {
                    b"0".as_slice()
                },
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"set n 18446744073709551615; incr n"),
                if fixed {
                    b"0".as_slice()
                } else {
                    b"18446744073709551616".as_slice()
                },
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"set n 010; incr n"),
                if matches!(dialect, "jim" | "tcl9.0" | "tcl9.1") {
                    b"11".as_slice()
                } else {
                    b"9".as_slice()
                },
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"expr {-(-9223372036854775808) < 0}"),
                if fixed {
                    b"1".as_slice()
                } else {
                    b"0".as_slice()
                },
                "{dialect}"
            );
            assert_eq!(
                ok(interp, b"expr {\"010\"}"),
                if dialect == "jim" {
                    b"010".as_slice()
                } else if fixed {
                    b"8".as_slice()
                } else if matches!(dialect, "tcl9.0" | "tcl9.1") {
                    b"10".as_slice()
                } else {
                    b"8".as_slice()
                }
            );
            assert_eq!(
                ok(interp, b"set n 18446744073709551615; expr {$n + 0.0 < 0.0}"),
                if fixed {
                    b"1".as_slice()
                } else {
                    b"0".as_slice()
                }
            );
            assert_eq!(
                ok(interp, b"set n 18446744073709551615; expr {$n > 0.0}"),
                if fixed {
                    b"0".as_slice()
                } else {
                    b"1".as_slice()
                }
            );
            if dialect == "tcl8.4" {
                assert_eq!(
                    interp.eval_str(b"set n 18446744073709551616; incr n"),
                    Code::Error
                );
                assert_eq!(
                    ok(interp, b"set ::errorCode"),
                    b"ARITH IOVERFLOW {integer value too large to represent}"
                );
            } else if dialect == "jim" {
                assert_eq!(ok(interp, b"set n 18446744073709551616; incr n"), b"0");
                assert_eq!(ok(interp, b"expr {1 << 64}"), b"1");
                assert_eq!(interp.eval_str(b"expr {bool(1)}"), Code::Error);
            }
        });
    }
}

/// A loop's condition is parsed **once**, not once per iteration: the
/// parsed AST caches on the condition object as `TCL_EXPR_TYPE`. A
/// `parse_runtime_expr` call that re-lexed, re-parsed and re-validated
/// the condition text on every evaluation would cost this repeatedly on
/// every pass of a hot loop.
#[cfg(have_tommath)]
#[test]
fn a_loop_condition_parses_once_for_the_whole_loop() {
    leak_free(|i| {
        crate::expr::reset_expr_parse_count();
        assert_eq!(ok(i, b"set n 0; while {$n < 20} { incr n }; set n"), b"20");
        assert_eq!(
            crate::expr::expr_parse_count(),
            1,
            "twenty-one condition evaluations, one parse"
        );
    });
}

/// The cached rep sits on the condition object, keeps its spelling, and is
/// reused on the next evaluation of that same object.
#[cfg(have_tommath)]
#[test]
fn a_condition_object_caches_its_ast_and_keeps_its_spelling() {
    leak_free(|i| {
        let cond = crate::obj::new_string_bytes(b"1 < 2");
        // SAFETY: the test owns this reference until it releases it below.
        unsafe { crate::obj::incr_ref_count(cond) };

        crate::expr::reset_expr_parse_count();
        assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
        assert_eq!(crate::expr::expr_parse_count(), 1);
        assert!(
            core::ptr::eq(crate::obj::obj_type_ptr(cond), &crate::expr::TCL_EXPR_TYPE),
            "the parse caches on the condition object"
        );
        assert_eq!(
            crate::interp::obj_bytes(cond),
            b"1 < 2",
            "the spelling survives the shimmer"
        );

        assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
        assert_eq!(
            crate::expr::expr_parse_count(),
            1,
            "the second evaluation reuses the cached AST"
        );

        // A shimmer to any other rep drops the cache, exactly like any
        // internal rep, and the next evaluation re-parses.
        assert_eq!(crate::list::list_length(cond), Ok(3));
        assert!(!core::ptr::eq(
            crate::obj::obj_type_ptr(cond),
            &crate::expr::TCL_EXPR_TYPE
        ));
        assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
        assert_eq!(crate::expr::expr_parse_count(), 2);

        // SAFETY: balances the reference taken above.
        unsafe { crate::obj::decr_ref_count(cond) };
    });
}

/// The cache records the release its registry validation ran under, so an
/// embedder pinning another emulated Tcl re-validates rather than reusing an
/// admission that release never granted.
#[cfg(have_tommath)]
#[test]
fn a_cached_expression_is_revalidated_when_the_release_changes() {
    leak_free(|i| {
        let cond = crate::obj::new_string_bytes(b"1 < 2");
        // SAFETY: the test owns this reference until it releases it below.
        unsafe { crate::obj::incr_ref_count(cond) };

        crate::expr::reset_expr_parse_count();
        assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
        assert_eq!(crate::expr::expr_parse_count(), 1);

        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
        assert_eq!(
            crate::expr::expr_parse_count(),
            2,
            "a release change stales the cached admission"
        );

        // SAFETY: balances the reference taken above.
        unsafe { crate::obj::decr_ref_count(cond) };
    });
}

#[test]
fn raw_cr_escapes_match_tcl9() {
    leak_free(|i| {
        // TclParseBackslash only collapses backslash-LF. Raw CR and CRLF
        // handed to the parser remain data; brace words preserve the slash.
        assert_eq!(ok(i, b"set x \"a\\\rb\"; set x"), b"a\rb");
        assert_eq!(ok(i, b"set x \"a\\\r\nb\"; set x"), b"a\r\nb");
        assert_eq!(ok(i, b"set x {a\\\rb}; set x"), b"a\\\rb");
        assert_eq!(ok(i, b"set x \"a\\\nb\"; set x"), b"a b");
    });
}

#[test]
fn subst_variables_commands_backslashes() {
    leak_free(|i| {
        ok(i, b"set x hello");
        assert_eq!(ok(i, b"subst {$x world}"), b"hello world");
        assert_eq!(ok(i, b"subst {[set x] world}"), b"hello world");
        // -nocommands leaves [ ] literal but still substitutes $x.
        assert_eq!(ok(i, b"subst -nocommands {$x [foo]}"), b"hello [foo]");
        // -novariables leaves $x literal.
        assert_eq!(ok(i, b"subst -novariables {$x}"), b"$x");
        // an unset variable is an error.
        assert_eq!(i.eval_str(b"subst {$nope}"), Code::Error);
        i.eval_str(b"unset -nocomplain x");
    });
}

/// `subst`'s option words go through the one shared
/// `tcl-cmd-core::prefix` matcher, so every miss is worded exactly as
/// `Tcl_GetIndexFromObj` at flags `0` words it (`TclSubstOptions`,
/// `tclCmdMZ.c:3341`): the empty word prefixes all three entries and is
/// therefore `ambiguous`, not `bad`.
#[test]
fn subst_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -nobackslashes, -nocommands, or -novariables";
    leak_free(|i| {
        assert_eq!(i.eval_str(b"subst {} abc"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous option \"\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"subst -no abc"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous option \"-no\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"subst -q abc"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("bad option \"-q\": {MUST}").as_bytes()
        );
        // Unique prefixes still resolve (subst-7.7).
        assert_eq!(ok(i, b"subst -nov {$x}"), b"$x");
        assert_eq!(ok(i, br"subst -nob {a\tb}"), br"a\tb");
        assert_eq!(ok(i, b"subst -noc {[cmd]}"), b"[cmd]");
    });
}

/// Tcl 8.6/9.0 (`Tcl_ParseVarName`, `generic/tclParse.c`) consumes the
/// complete colon separator run before looking up an unbraced variable.
#[test]
fn subst_variable_colon_runs_match_tcl() {
    leak_free(|i| {
        ok(i, b"namespace eval a {}");
        ok(i, b"set ::a::b VALUE");
        ok(i, b"set ::a::arr(k) ARRAY");
        assert_eq!(
            ok(i, b"subst {$a:::b $::a:::b $a:::arr(k) $::a:::arr(k)}"),
            b"VALUE VALUE ARRAY ARRAY"
        );
        ok(i, b"namespace eval foo {}");
        ok(i, b"set ::foo::: EMPTY");
        assert_eq!(ok(i, b"subst {$foo:::}"), b"EMPTY");
        assert_eq!(i.eval_str(b"subst {$missing:::b}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't read \"missing:::b\": no such variable"
        );
    });
}

/// An unterminated `${…}` is `Tcl_ParseVarName`'s
/// `missing close-brace for variable name` — **not** a name that runs to
/// end-of-input, and not a literal `$`.
///
/// A `.unwrap_or(len)` scan of the name would silently swallow the rest
/// of the template (and, for `${a\`, drop the backslash and succeed
/// where C errors). The scan reports
/// [`tcl_lexer::BracedVarEnd::Unterminated`] instead, and the eval loop
/// raises.
///
/// The error is raised in evaluation order, so a `[...]` earlier in the
/// same template has already run — verified against tclsh 8.6.16 and
/// 9.0.4, which both leave the counter at 1.
/// Templates are built with `binary format H*` so the *script* parser's own
/// brace matching cannot reshape the bytes `subst` is handed — writing
/// `subst {${abc}` instead makes the script parser hand over `${abc}`,
/// which is a perfectly terminated reference and tests nothing.
#[test]
fn unterminated_braced_var_raises_missing_close_brace() {
    const MSG: &[u8] = b"missing close-brace for variable name";
    leak_free(|i| {
        for hex in [
            &b"247B616263"[..],   // `${abc`  — no closer at all
            &b"7A247B617B62"[..], // `z${a{b` — unbalanced `{`
            &b"247B615C"[..],     // `${a\`   — trailing lone backslash
        ] {
            let script = [b"subst [binary format H* ".as_ref(), hex, b"]"].concat();
            assert_eq!(i.eval_str(&script), Code::Error, "{hex:?}");
            assert_eq!(i.result_bytes(), MSG, "{hex:?}");
        }
        // A *closed* form still resolves normally: `${a{b}c}` under the
        // default 9.x nesting rule names the variable `a{b}c`.
        ok(i, b"set {a{b}c} WORLD");
        assert_eq!(
            ok(i, b"subst [binary format H* 247B617B627D637D]"),
            b"WORLD"
        );

        // Side effects before the bad `${` survive, as in C: `[incr c]${a{b`
        // leaves the counter at 1 on both tclsh 8.6.16 and 9.0.4.
        ok(i, b"set c 0");
        assert_eq!(
            i.eval_str(b"subst [binary format H* 5B696E637220635D247B617B62]"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), MSG);
        assert_eq!(ok(i, b"set c"), b"1");
    });
}

/// A hand-rolled `starts_with` filter for `interp limit`'s option matcher
/// could only ever say `bad option` (never `ambiguous`), and would
/// hand-build its own `", or"` enumeration beside the one
/// `prefix::choice_list_bytes` owns. Both come from
/// `OptionTable::abbreviating` instead.
///
/// Byte-checked against tclsh 8.6.16 and 9.0.4.
#[test]
fn interp_limit_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -command, -granularity, -milliseconds, or -seconds";
    leak_free(|i| {
        ok(i, b"interp create i");
        // The empty word prefixes every entry ⇒ ambiguous, not bad.
        assert_eq!(i.eval_str(b"interp limit i time {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous option \"\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"interp limit i time -"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous option \"-\": {MUST}").as_bytes()
        );
        // A word prefixing nothing is bad.
        assert_eq!(i.eval_str(b"interp limit i time -zz"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("bad option \"-zz\": {MUST}").as_bytes()
        );
        // A unique prefix still resolves.
        assert_eq!(i.eval_str(b"interp limit i time -sec 5"), Code::Ok);
    });
}

/// `interp debug`'s option word is a `Tcl_GetIndexFromObj`
/// table whose noun is `debug option` (`debugTypes[]`, `tclInterp.c`), so
/// `-f`/`-fr` abbreviate and the one-entry table never says `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp debug i {}     -> bad debug option "": must be -frame
///   interp debug i -x     -> bad debug option "-x": must be -frame
///   interp debug i -f     -> 0
///   interp debug i -fr 1  -> 1
///   i debug -f            -> 1   (after the latch)
///   i debug -x 1 2        -> wrong # args: should be "i debug ?-frame ?bool??"
#[test]
fn interp_debug_option_uses_c_noun_and_abbreviates() {
    leak_free(|i| {
        ok(i, b"interp create i");
        assert_eq!(i.eval_str(b"interp debug i {}"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad debug option \"\": must be -frame");
        assert_eq!(i.eval_str(b"interp debug i -x"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad debug option \"-x\": must be -frame");
        assert_eq!(ok(i, b"interp debug i -f"), b"0");
        assert_eq!(ok(i, b"interp debug i -fr 1"), b"1");
        assert_eq!(ok(i, b"i debug -f"), b"1");
        assert_eq!(i.eval_str(b"i debug -x 1 2"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"i debug ?-frame ?bool??\""
        );
    });
}

/// `interp limit`'s type word is `Tcl_GetIndexFromObj(…,
/// "limit type", 0)` (`limitTypes[]`, `tclInterp.c`), so `c`/`t`
/// abbreviate and the empty word — a prefix of both entries — is
/// `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp limit i {} -> ambiguous limit type "": must be commands or time
///   interp limit i x  -> bad limit type "x": must be commands or time
///   interp limit i c  -> -command {} -granularity 1 -value {}
///   i limit {}        -> ambiguous limit type "": must be commands or time
#[test]
fn interp_limit_type_word_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be commands or time";
    leak_free(|i| {
        ok(i, b"interp create i");
        assert_eq!(i.eval_str(b"interp limit i {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous limit type \"\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"interp limit i x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("bad limit type \"x\": {MUST}").as_bytes()
        );
        assert_eq!(
            ok(i, b"interp limit i c"),
            b"-command {} -granularity 1 -value {}"
        );
        assert_eq!(i.eval_str(b"i limit {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous limit type \"\": {MUST}").as_bytes()
        );
        assert_eq!(ok(i, b"i limit c"), b"-command {} -granularity 1 -value {}");
    });
}

#[test]
fn unset_nocomplain_and_dashdash() {
    leak_free(|i| {
        // -nocomplain suppresses the no-such-variable error.
        assert_eq!(i.eval_str(b"unset -nocomplain nope"), Code::Ok);
        // without it, unsetting a missing var errors.
        assert_eq!(i.eval_str(b"unset alsonope"), Code::Error);
        // `--` ends options, so a var literally named -nocomplain is unset.
        ok(i, b"set -nocomplain 1");
        assert_eq!(i.eval_str(b"unset -- -nocomplain"), Code::Ok);
        assert_eq!(i.eval_str(b"info exists -nocomplain"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
    });
}
