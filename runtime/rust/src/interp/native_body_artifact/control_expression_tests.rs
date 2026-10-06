// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native control and expression compiler windows.
use super::tests::interpreter;
use super::*;

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn profile(version: &str) -> &'static str {
    match version {
        "8.4.20" => "tcl8.4",
        "8.5.19" => "tcl8.5",
        "8.6.18" => "tcl8.6",
        "9.0.4" => "tcl9.0",
        "9.1.0" => "tcl9.1",
        _ => panic!("native version"),
    }
}
fn define(interp: &mut Interp, bytes: &[u8]) -> Vec<obj::Owned> {
    let original = [b"proc".as_slice(), b"p", b"x", bytes]
        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
        .to_vec();
    let argv = original.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    assert_eq!(interp.eval_original_object_vector(&argv), Code::Ok);
    original
}
fn call(interp: &mut Interp, argument: &[u8]) -> (Code, Vec<obj::Owned>) {
    let original = [b"p".as_slice(), argument]
        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
        .to_vec();
    let argv = original.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    (interp.eval_original_object_vector(&argv), original)
}
fn run_outputs(table: &str, expected: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = row.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(profile(f[0]));
        let _definition = define(&mut interp, &decode(f[2]));
        if f[0] != "8.4.20" {
            let procedure = interp.proc_def(b"p").expect("original declaration");
            let _prepared =
                interp.prepare_original_c_body(procedure.body.as_ptr(), GLOBAL, Some(&procedure));
        }
        let (code, _original) = call(&mut interp, f[5].as_bytes());
        assert_eq!(
            code,
            Code::from_int(f[3].parse().unwrap()),
            "{}/{}: {:?}",
            f[0],
            f[1],
            interp.native_access_refusal()
        );
        assert_eq!(
            interp
                .native_object_string_bytes(interp.result_obj())
                .unwrap()
                .as_ref(),
            decode(f[4]),
            "{}/{}",
            f[0],
            f[1]
        );
        compared += 1;
    }
    assert_eq!(compared, expected);
}
#[test]
fn registered_control_artifacts_match_75_native_controls() {
    run_outputs(
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_control_expression/control75.tsv"
        ),
        75,
    );
}
#[test]
fn compound_expression_programs_match_40_native_controls() {
    run_outputs(
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_control_expression/compound40.tsv"
        ),
        40,
    );
}
fn storage(original: *mut TclObj) -> String {
    obj::check_native_liveness(original).unwrap();
    let descriptor = obj::obj_type_ptr(original);
    // Only the current original descriptor and resident bytes are observed;
    // no object updater, duplication or additional native reference is reached.
    let class = if descriptor.is_null() {
        "none"
    } else {
        unsafe { core::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
    };
    let mut result = format!(
        "{class},{},{}",
        usize::from(obj::has_string_rep(original)),
        unsafe { (*original).ref_count }
    );
    if obj::has_string_rep(original) {
        result.push(',');
        for byte in obj::bytes_of(original) {
            use std::fmt::Write;
            write!(result, "{byte:02x}").unwrap();
        }
    }
    result
}
fn run_expression_storage(table: &str, expected_count: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = row.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(profile(f[0]));
        let _definition = define(&mut interp, &decode(f[2]));
        assert_eq!(f[3], "0", "native definition");
        let (code, _original) = call(&mut interp, b"3");
        assert_eq!(
            code,
            Code::from_int(f[4].parse().unwrap()),
            "{}/{}: {:?}",
            f[0],
            f[1],
            interp.native_access_refusal()
        );
        assert_eq!(
            storage(interp.result_obj()),
            f[5],
            "{}/{} original result before GetString",
            f[0],
            f[1]
        );
        let procedure = interp.proc_def(b"p").expect("original declaration");
        let artifact = cache(procedure.body.as_ptr()).expect("original Bytecode primary");
        let expected = f[6].split('|').collect::<Vec<_>>();
        assert!(
            artifact.literals.original(expected.len()).is_none(),
            "{}/{} exact literal count",
            f[0],
            f[1]
        );
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(
                storage(artifact.literals.original(index).expect("original literal")),
                expected,
                "{}/{} original literal {index} before GetString",
                f[0],
                f[1]
            );
        }
        compared += 1;
    }
    assert_eq!(compared, expected_count);
}

#[test]
fn compiled_expression_storage_matches_40_original_native_windows() {
    run_expression_storage(
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_control_expression/storage40.tsv"
        ),
        40,
    );
}

#[test]
fn compiled_syntax_context_retains_same_native_message_and_options() {
    for (version, table) in [
        (
            "8.6.18",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv"
            ),
        ),
        (
            "9.0.4",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv"
            ),
        ),
        (
            "9.1.0",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv"
            ),
        ),
    ] {
        let mut interp = interpreter(profile(version));
        let _definition = define(&mut interp, b"expr {$x ? (1/0) : (1+2)}");
        let (code, _call) = call(&mut interp, b"3");
        assert_eq!(
            code,
            Code::Error,
            "{version}: {:?}",
            interp.native_access_refusal()
        );
        let procedure = interp.proc_def(b"p").expect("original declaration");
        let artifact = cache(procedure.body.as_ptr()).expect("original Bytecode primary");
        let context = interp
            .error_stack
            .borrow()
            .original_inner_context()
            .expect("original innerContext");
        let strings = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let members = crate::list::list_elements_native_checked(context, strings).unwrap();
        assert_eq!(members.len(), 3);
        if version == "9.1.0" {
            let same_message = crate::dict::native_dict_pairs(members[2], strings)
                .unwrap()
                .into_iter()
                .any(|(key, value)| obj::bytes_of(key) == b"-errorinfo" && value == members[1]);
            assert!(
                same_message,
                "C91 options retain the original Syntax message"
            );
        }
        for line in table.lines().filter(|line| line.starts_with("CTX\t5\t")) {
            let fields = line.split('\t').collect::<Vec<_>>();
            let index = fields[2].parse::<usize>().unwrap();
            let original = members[index];
            if index == 0 {
                let name = obj::native_instruction_name::cache(original)
                    .expect("actual instname descriptor");
                let opcodes = table
                    .lines()
                    .find(|line| line.starts_with("OPCODES\t"))
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(name.opcode(), opcodes[1].parse::<u8>().unwrap());
                assert_eq!(storage(original), "instname,0,1");
                let descriptor = obj::obj_type_ptr(original);
                // SAFETY: the original retained context owns this live descriptor/header.
                unsafe {
                    assert!((*descriptor).free_int_rep_proc.is_none());
                    assert!((*descriptor).dup_int_rep_proc.is_none());
                    assert!((*descriptor).set_from_any_proc.is_none());
                    assert!((*descriptor).update_string_proc.is_some());
                }
                let duplicate = obj::Owned::fresh(obj::duplicate(original));
                assert_eq!(
                    obj::native_instruction_name::cache(duplicate.as_ptr()),
                    Some(name)
                );
                assert_eq!(storage(duplicate.as_ptr()), "instname,0,1");
                assert_eq!(
                    interp
                        .native_object_string_bytes(duplicate.as_ptr())
                        .unwrap()
                        .as_ref(),
                    b"syntax"
                );
                assert!(!obj::has_string_rep(original));
                assert!(
                    crate::dict::native_object_bytes(
                        original,
                        tcl_syntax::native_string::NativeStringProtocol::Jim084
                    )
                    .is_err()
                );
            } else {
                assert_eq!(storage(original), fields[5], "{version}: {line}");
                let slot = fields[3].parse::<usize>().unwrap();
                assert_eq!(artifact.literals.original(slot), Some(original));
                assert_eq!(original == interp.result_obj(), fields[4] == "1");
            }
        }
    }
}

#[test]
fn folded_logical_literal_ownership_matches_eight_native_collisions() {
    run_expression_storage(
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv"
        ),
        8,
    );
}

#[test]
fn compiled_return_context_retains_original_operands_without_annotation() {
    let sources = [
        b"return -level 0 -code error BODY".as_slice(),
        b"return -level 0 -code error [set y BODY]",
        b"error BODY",
    ];
    for (version, table) in [
        (
            "8.6.18",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv"
            ),
        ),
        (
            "9.0.4",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv"
            ),
        ),
        (
            "9.1.0",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv"
            ),
        ),
    ] {
        for (case, source) in sources.iter().enumerate() {
            let mut interp = interpreter(profile(version));
            let _definition = define(&mut interp, source);
            let (code, _call) = call(&mut interp, b"3");
            assert_eq!(code, Code::Error, "{version}/{case}");
            let procedure = interp.proc_def(b"p").expect("original declaration");
            let artifact = cache(procedure.body.as_ptr()).expect("original Bytecode");
            let native = table
                .lines()
                .find(|line| line.starts_with(&format!("CASE\t{case}\t")))
                .unwrap()
                .split('\t')
                .collect::<Vec<_>>();
            assert_eq!(storage(interp.result_obj()), native[4], "{version}/{case}");
            let context = interp
                .error_stack
                .borrow()
                .original_inner_context()
                .expect("original innerContext");
            let strings = interp
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let members = crate::list::list_elements_native_checked(context, strings).unwrap();
            assert_eq!(members.len(), 3);
            for line in table
                .lines()
                .filter(|line| line.starts_with(&format!("CTX\t{case}\t")))
            {
                let fields = line.split('\t').collect::<Vec<_>>();
                let index = fields[2].parse::<usize>().unwrap();
                let original = members[index];
                if index == 0 {
                    assert_eq!(storage(original), "instname,0,1");
                    let name = obj::native_instruction_name::cache(original).unwrap();
                    let native = table
                        .lines()
                        .find(|line| line.starts_with(&format!("INST\t{case}\t")))
                        .unwrap()
                        .split('\t')
                        .collect::<Vec<_>>();
                    assert_eq!(name.opcode(), native[2].parse::<u8>().unwrap());
                    assert_eq!(name.string_bytes(), decode(native[3]));
                } else {
                    assert_eq!(storage(original), fields[5], "{version}/{case}: {line}");
                    assert_eq!(
                        artifact.literals.original(fields[3].parse().unwrap()),
                        Some(original)
                    );
                    assert_eq!(original == interp.result_obj(), fields[4] == "1");
                }
            }
        }
    }
}

#[test]
fn c84_logical_storage_and_short_circuit_match_eight_native_controls() {
    run_expression_storage(
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_control_expression/logical84.tsv"
        ),
        8,
    );
}

#[test]
fn compiled_catch_store_order_matches_four_native_alias_controls() {
    let mut compared = 0;
    for row in include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_control_expression/catch4.tsv"
    )
    .lines()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(profile(fields[0]));
        let body = decode(fields[1]);
        let definition = [b"proc".as_slice(), b"p", b"", body.as_slice()]
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)));
        let argv = definition
            .iter()
            .map(obj::Owned::as_ptr)
            .collect::<Vec<_>>();
        assert_eq!(interp.eval_original_object_vector(&argv), Code::Ok);
        let head = obj::Owned::fresh(obj::new_string_bytes(b"p"));
        let code = interp.eval_original_object_vector(&[head.as_ptr()]);
        assert_eq!(
            code,
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {:?}",
            fields[0],
            interp.native_access_refusal()
        );
        assert_eq!(
            interp
                .native_object_string_bytes(interp.result_obj())
                .unwrap()
                .as_ref(),
            decode(fields[3]),
            "{}",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 4);
}

#[test]
fn catch_publication_and_store_order_match_six_native_engines() {
    let mut compared = 0;
    for row in include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_control_expression/catch-publication6.tsv"
    )
    .lines()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        let mut interp = if fields[0] == "jim" {
            Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .expect("actual Jim core entry")
        } else {
            interpreter(profile(fields[0]))
        };
        assert_eq!(
            interp.eval_str(&decode(fields[1])),
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {:?}",
            fields[0],
            interp.native_access_refusal()
        );
        assert_eq!(
            interp
                .native_object_string_bytes(interp.result_obj())
                .unwrap()
                .as_ref(),
            decode(fields[3]),
            "{} original publication callbacks and variable stores",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 6);
}

#[test]
fn c91_catch_reset_releases_the_original_global_syntax_message() {
    let mut interp = interpreter("tcl9.1");
    let _definition = define(&mut interp, b"catch {expr {1/0}} result; return $result");
    let procedure = interp.proc_def(b"p").expect("original procedure");
    let _prepared =
        interp.prepare_original_c_body(procedure.body.as_ptr(), GLOBAL, Some(&procedure));
    let artifact = cache(procedure.body.as_ptr()).expect("original Bytecode");
    let original = artifact.literals.original(0).expect("Syntax message");
    let published = crate::vars::get(
        &interp.frames.borrow(),
        &interp.namespaces.borrow(),
        GLOBAL,
        b"::errorInfo",
    );
    assert_eq!(published, Some(original), "reached compiler publication");
    assert_eq!(storage(original), "none,1,3,646976696465206279207a65726f");

    let (code, _call) = call(&mut interp, b"3");
    assert_eq!(code, Code::Ok);
    assert_eq!(interp.result_obj(), original, "same original result header");
    assert_eq!(storage(original), "none,1,4,646976696465206279207a65726f");
    let published = crate::vars::get(
        &interp.frames.borrow(),
        &interp.namespaces.borrow(),
        GLOBAL,
        b"::errorInfo",
    )
    .expect("published appended errorInfo");
    assert_ne!(
        published, original,
        "END_CATCH publishes the private COW header"
    );
}
