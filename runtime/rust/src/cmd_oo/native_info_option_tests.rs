// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original declared-variable flag validation before the actual OO target.

use super::*;

#[test]
fn variable_info_validates_original_options_before_target_lookup() {
    // Source proof: naming.tcloo.variable-info-option-order
    // docs/design/analysis/name-resolution-proofs/variable-info-option-order.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let root = interp.oo_resolve_object(b"::oo::class");
        let name = obj::Owned::fresh(obj::new_string_bytes(b"private\xff\0TAIL"));
        let declaration = DeclaredVariable {
            original: name.clone(),
            name: b"private\xff\0TAIL".to_vec(),
        };
        interp
            .oo
            .borrow_mut()
            .objects
            .get_mut(&root)
            .unwrap()
            .private_variables
            .push(declaration.clone());
        interp
            .oo
            .borrow_mut()
            .classes
            .get_mut(&root)
            .unwrap()
            .private_variables
            .push(declaration);
        for kind in [InfoOoEnsembleKind::Object, InfoOoEnsembleKind::Class] {
            let invoke = |interp: &mut Interp, target: &[u8], option: &[u8], extra: bool| {
                let mut args = vec![
                    obj::Owned::fresh(obj::new_string_bytes(b"info")),
                    obj::Owned::fresh(obj::new_string_bytes(match kind {
                        InfoOoEnsembleKind::Object => b"object",
                        InfoOoEnsembleKind::Class => b"class",
                    })),
                    obj::Owned::fresh(obj::new_string_bytes(b"variables")),
                    obj::Owned::fresh(obj::new_string_bytes(target)),
                    obj::Owned::fresh(obj::new_string_bytes(option)),
                ];
                if extra {
                    args.push(obj::Owned::fresh(obj::new_string_bytes(b"EXTRA")));
                }
                let pointers = args.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                match kind {
                    InfoOoEnsembleKind::Object => info_object(interp, &pointers),
                    InfoOoEnsembleKind::Class => info_class(interp, &pointers),
                }
            };
            let code = invoke(&mut interp, b"::oo::class", b"-private\0\xff", false);
            if engine == "tcl8.6" {
                assert_eq!(code, Code::Error);
                assert!(interp.result_bytes().starts_with(b"wrong # args:"));
            } else {
                assert_eq!(code, Code::Ok);
                let result = interp.result_obj();
                let members = interp.list_elements(&result).unwrap();
                assert_eq!(members, vec![name.as_ptr()]);
            }
            for option in [b"-priv".as_slice(), b"-private\xc0\x80", b"-bad\xff\0TAIL"] {
                assert_eq!(
                    invoke(&mut interp, b"::does_not_exist", option, false),
                    Code::Error
                );
                assert!(interp.result_bytes().starts_with(if engine == "tcl8.6" {
                    b"wrong # args:".as_slice()
                } else {
                    b"option \""
                }));
            }
            assert_eq!(
                invoke(&mut interp, b"::does_not_exist", b"-private", true),
                Code::Error
            );
            assert!(interp.result_bytes().starts_with(b"wrong # args:"));
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
                "../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_tcloo_variable_info_original/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(b"::oo::class create C; C create O"),
            Code::Ok
        );
        let class = interp.oo_resolve_object(b"C");
        let object = interp.oo_resolve_object(b"O");
        let declared = |name: &[u8]| DeclaredVariable {
            original: obj::Owned::fresh(obj::new_string_bytes(name)),
            name: name.to_vec(),
        };
        // Select introspection on the captured setup's retained rows; this
        // test does not establish declaration-installation equivalence.
        interp
            .oo
            .borrow_mut()
            .classes
            .get_mut(&class)
            .unwrap()
            .variables
            .push(declared(b"PUBLIC"));
        interp
            .oo
            .borrow_mut()
            .objects
            .get_mut(&object)
            .unwrap()
            .variables
            .push(declared(b"OPUBLIC"));
        if engine != "tcl8.6" {
            interp
                .oo
                .borrow_mut()
                .classes
                .get_mut(&class)
                .unwrap()
                .private_variables
                .push(declared(b"PRIVATE"));
            interp
                .oo
                .borrow_mut()
                .objects
                .get_mut(&object)
                .unwrap()
                .private_variables
                .push(declared(b"OPRIVATE"));
        }
        for (kind, category, target) in [
            (InfoOoEnsembleKind::Object, "object", b"O".as_slice()),
            (InfoOoEnsembleKind::Class, "class", b"C".as_slice()),
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
                let mut args = vec![
                    obj::Owned::fresh(obj::new_string_bytes(b"info")),
                    obj::Owned::fresh(obj::new_string_bytes(category.as_bytes())),
                    obj::Owned::fresh(obj::new_string_bytes(b"variables")),
                ];
                if !no_target {
                    args.push(obj::Owned::fresh(obj::new_string_bytes(if missing {
                        b"::missing"
                    } else {
                        target
                    })));
                }
                if let Some(option) = option {
                    args.push(obj::Owned::fresh(obj::new_string_bytes(option)));
                }
                if extra {
                    args.push(obj::Owned::fresh(obj::new_string_bytes(b"EXTRA")));
                }
                let pointers = args.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                let code = match kind {
                    InfoOoEnsembleKind::Object => info_object(&mut interp, &pointers),
                    InfoOoEnsembleKind::Class => info_class(&mut interp, &pointers),
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
                    code.as_int(),
                    columns[1].parse::<i64>().unwrap(),
                    "{engine}/{label}"
                );
                assert_eq!(interp.result_bytes(), bytes, "{engine}/{label}");
            }
        }
    }
}
