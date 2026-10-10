// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declared-variable option validation precedes the actual OO target lookup.

use super::*;
use tcl_syntax::value::ValueOps;

#[test]
fn variable_info_validates_original_options_before_target_lookup() {
    // Source proof: naming.tcloo.variable-info-option-order
    // docs/design/analysis/name-resolution-proofs/variable-info-option-order.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let root = object_key(&mut vm, &Value::string("::oo::class")).unwrap();
        let name = Value::new_native_string_bytes(b"private\xff\0TAIL".as_slice());
        let declaration = DeclaredVariable {
            original: name.clone(),
            name: NameBytes::from(name.string_bytes().as_ref()),
        };
        vm.oo
            .objects
            .get_mut(&root)
            .unwrap()
            .private_variables
            .push(declaration.clone());
        vm.oo
            .classes
            .get_mut(&root)
            .unwrap()
            .private_variables
            .push(declaration);
        for kind in [InfoOoEnsembleKind::Object, InfoOoEnsembleKind::Class] {
            let invoke = |vm: &mut Vm, args: &[Value]| match kind {
                InfoOoEnsembleKind::Object => info_object(vm, args),
                InfoOoEnsembleKind::Class => info_class(vm, args),
            };
            let arguments = [
                Value::string("variables"),
                Value::string("::oo::class"),
                Value::new_native_string_bytes(b"-private\0\xff".as_slice()),
            ];
            let completion = invoke(&mut vm, &arguments);
            if engine == "tcl8.6" {
                assert_eq!(completion.code, Code::Error);
                assert!(
                    completion
                        .result
                        .string_bytes()
                        .starts_with(b"wrong # args:")
                );
            } else {
                assert_eq!(completion.code, Code::Ok);
                let members = vm.list_elements(&completion.result).unwrap();
                assert_eq!(members.len(), 1);
                assert!(members[0].is_same_object(&name));
            }
            for option in [b"-priv".as_slice(), b"-private\xc0\x80", b"-bad\xff\0TAIL"] {
                let completion = invoke(
                    &mut vm,
                    &[
                        Value::string("variables"),
                        Value::string("::does_not_exist"),
                        Value::new_native_string_bytes(option),
                    ],
                );
                assert_eq!(completion.code, Code::Error);
                assert!(
                    completion
                        .result
                        .string_bytes()
                        .starts_with(if engine == "tcl8.6" {
                            b"wrong # args:".as_slice()
                        } else {
                            b"option \""
                        })
                );
            }
            let completion = invoke(
                &mut vm,
                &[
                    Value::string("variables"),
                    Value::string("::does_not_exist"),
                    Value::string("-private"),
                    Value::string("EXTRA"),
                ],
            );
            assert_eq!(completion.code, Code::Error);
            assert!(
                completion
                    .result
                    .string_bytes()
                    .starts_with(b"wrong # args:")
            );
        }
    }
}

#[test]
fn variable_info_matches_original_native_option_and_target_failures() {
    // Native proof: naming.tcloo.variable-info-original-option-boundaries
    // docs/design/analysis/name-resolution-proofs/variable-info-original-option-boundaries.md
    for (engine, captured) in [
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_tcloo_variable_info_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_tcloo_variable_info_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_tcloo_variable_info_original/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        assert_eq!(
            vm.invoke_command(
                "::oo::class",
                &[Value::string("create"), Value::string("C")]
            )
            .code,
            Code::Ok
        );
        assert_eq!(
            vm.invoke_command("C", &[Value::string("create"), Value::string("O")])
                .code,
            Code::Ok
        );
        let class = object_key(&mut vm, &Value::string("C")).unwrap();
        let object = object_key(&mut vm, &Value::string("O")).unwrap();
        let declared = |name: &str| DeclaredVariable {
            original: Value::string(name),
            name: NameBytes::from(name.as_bytes()),
        };
        // This test selects introspection on the captured setup's retained
        // declaration rows. It does not test the declaration installer.
        vm.oo
            .classes
            .get_mut(&class)
            .unwrap()
            .variables
            .push(declared("PUBLIC"));
        vm.oo
            .objects
            .get_mut(&object)
            .unwrap()
            .variables
            .push(declared("OPUBLIC"));
        if engine != "tcl8.6" {
            vm.oo
                .classes
                .get_mut(&class)
                .unwrap()
                .private_variables
                .push(declared("PRIVATE"));
            vm.oo
                .objects
                .get_mut(&object)
                .unwrap()
                .private_variables
                .push(declared("OPRIVATE"));
        }
        for (kind, category, target) in [
            (InfoOoEnsembleKind::Object, "object", "O"),
            (InfoOoEnsembleKind::Class, "class", "C"),
        ] {
            for (case, option, missing, extra, no_target) in [
                ("BASE", None, false, false, false),
                (
                    "PRIVATE_EXACT",
                    Some(b"-private".as_slice()),
                    false,
                    false,
                    false,
                ),
                (
                    "PRIVATE_RAW_ZERO",
                    Some(b"-private\0\xff".as_slice()),
                    false,
                    false,
                    false,
                ),
                (
                    "PRIVATE_ENCODED_ZERO",
                    Some(b"-private\xc0\x80".as_slice()),
                    false,
                    false,
                    false,
                ),
                (
                    "PRIVATE_PREFIX",
                    Some(b"-priv".as_slice()),
                    false,
                    false,
                    false,
                ),
                (
                    "BAD_RAW_ZERO",
                    Some(b"-bad\xff\0TAIL".as_slice()),
                    false,
                    false,
                    false,
                ),
                (
                    "MISSING_BAD",
                    Some(b"-bad\xff\0TAIL".as_slice()),
                    true,
                    false,
                    false,
                ),
                (
                    "MISSING_PRIVATE",
                    Some(b"-private".as_slice()),
                    true,
                    false,
                    false,
                ),
                (
                    "MISSING_EXTRA",
                    Some(b"-private".as_slice()),
                    true,
                    true,
                    false,
                ),
                ("MISSING_BASE", None, true, false, false),
                ("NO_TARGET", None, false, false, true),
            ] {
                let mut args = vec![Value::string("variables")];
                if !no_target {
                    args.push(Value::string(if missing { "::missing" } else { target }));
                }
                if let Some(option) = option {
                    args.push(Value::new_native_string_bytes(option));
                }
                if extra {
                    args.push(Value::string("EXTRA"));
                }
                let completion = match kind {
                    InfoOoEnsembleKind::Object => info_object(&mut vm, &args),
                    InfoOoEnsembleKind::Class => info_class(&mut vm, &args),
                };
                let label = format!("DIRECT_{category}_{case}");
                let line = captured
                    .lines()
                    .find(|line| line.split('|').next() == Some(label.as_str()))
                    .unwrap();
                let columns = line.split('|').collect::<Vec<_>>();
                let bytes = columns[2]
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(
                    completion.code.as_int(),
                    columns[1].parse::<i64>().unwrap(),
                    "{engine}/{label}"
                );
                assert_eq!(
                    completion.result.string_bytes().as_ref(),
                    bytes.as_slice(),
                    "{engine}/{label}"
                );
            }
        }
    }
}
