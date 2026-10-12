// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual compiled and generic `info commands` result and original operand windows.

use crate::{interp::Code, list, obj};

include!(
    "../../../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/cases.rs"
);
const WINDOWS: &str = include_str!("../../../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/windows.tsv");

fn unhex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn primary(original: *mut obj::TclObj) -> String {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() {
        return "none".into();
    }
    // SAFETY: interpreter, procedure, list or external argv ownership remains live.
    unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
        .to_str()
        .unwrap()
        .into()
}

fn resident(original: *mut obj::TclObj) -> Option<Vec<u8>> {
    if !obj::has_string_rep(original) {
        return None;
    }
    // SAFETY: original retained header owns its resident length and counted bytes;
    // this physical observation calls no string getter or updater.
    unsafe {
        Some(
            core::slice::from_raw_parts(
                (*original).bytes.cast::<u8>(),
                usize::try_from((*original).length).unwrap(),
            )
            .to_vec(),
        )
    }
}

fn assert_header(
    original: *mut obj::TclObj,
    name: &str,
    present: &str,
    bytes: &str,
    context: &str,
) {
    assert_eq!(primary(original), name, "{context}: primary before getter");
    let actual = resident(original);
    assert_eq!(
        actual.is_some(),
        present == "1",
        "{context}: string presence before getter"
    );
    assert_eq!(
        actual,
        (bytes != "-").then(|| unhex(bytes)),
        "{context}: counted resident bytes before getter"
    );
}

#[test]
fn compiled_and_generic_info_commands_retain_one_hundred_eighty_six_original_header_windows() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    // windows.tsv is a projection of exact original receipts, not a native run.
    // Every backend entry must genuinely execute before its headers are compared.
    let mut compared = 0;
    let mut pattern_caches = 0;
    for row in WINDOWS.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        let case = ORIGINAL_INFO_COMMANDS_CASES[fields[1].parse::<usize>().unwrap()];
        let context = format!("{}: {}", fields[0], case.0);
        let mut interp = super::super::tests::interpreter(fields[0]);
        let code = interp.eval_str(case.1);
        assert!(
            !interp.host_refusal_pending(),
            "{context}: prelude host {:?}",
            interp.native_access_refusal()
        );
        assert_eq!(code, Code::Ok, "{context}: genuine independent prelude");
        let definition = [b"proc".as_slice(), case.3, case.4, case.2]
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)));
        let code =
            interp.eval_original_object_vector(&definition.each_ref().map(obj::Owned::as_ptr));
        assert!(
            !interp.host_refusal_pending(),
            "{context}: definition host {:?}",
            interp.native_access_refusal()
        );
        assert_eq!(
            code,
            Code::Ok,
            "{context}: genuine original procedure binding"
        );
        let head = obj::Owned::fresh(obj::new_string_bytes(case.3));
        let argument = case
            .5
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)));
        let mut argv = vec![head.as_ptr()];
        if let Some(original) = &argument {
            argv.push(original.as_ptr());
        }
        let code = interp.eval_original_object_vector(&argv);
        assert!(
            !interp.host_refusal_pending(),
            "{context}: original invocation host {:?}",
            interp.native_access_refusal()
        );
        assert_eq!(
            code.as_int().to_string(),
            fields[2],
            "{context}: guest completion"
        );
        let original = interp.get_obj_result();
        assert_header(original, fields[4], fields[5], fields[6], &context);
        let children = if fields[8] == "-" {
            if fields[4] == "list" {
                assert_eq!(
                    list::native_list_backing(original).unwrap().len(),
                    0,
                    "{context}: original empty list"
                );
            }
            Vec::new()
        } else {
            let backing =
                list::native_list_backing(original).expect("actual native result List backing");
            let children = backing.elements().unwrap().to_vec();
            let names: Vec<_> = fields[8].split(',').collect();
            let presence: Vec<_> = fields[9].split(',').collect();
            let bytes: Vec<_> = fields[10].split(',').collect();
            assert_eq!(
                children.len(),
                names.len(),
                "{context}: actual original child count"
            );
            for (index, child) in children.iter().enumerate() {
                assert_header(
                    *child,
                    names[index],
                    presence[index],
                    bytes[index],
                    &context,
                );
            }
            children
        };
        if let Some(original) = &argument {
            assert_header(
                original.as_ptr(),
                fields[14],
                fields[15],
                fields[16],
                &context,
            );
        }
        if fields[3] == "1" {
            let procedure = interp
                .proc_def(case.3)
                .expect("genuine installed procedure");
            let body = procedure.body.checked_ptr().unwrap();
            let artifact = super::super::cache(body).expect("genuine selected body artifact");
            let pattern = artifact
                .literals
                .original(0)
                .expect("original compiled pattern");
            assert!(
                artifact.literals.original(1).is_none(),
                "{context}: native singleton object array"
            );
            assert_header(pattern, fields[11], fields[12], fields[13], &context);
            pattern_caches += 1;
        }
        // All physical windows above precede every reached string getter below.
        assert_eq!(
            interp
                .native_object_string_bytes(original)
                .unwrap()
                .as_ref(),
            unhex(fields[7]),
            "{context}: reached result bytes"
        );
        for (child, expected) in children.iter().zip(fields[10].split(',')) {
            assert_eq!(
                interp.native_object_string_bytes(*child).unwrap().as_ref(),
                unhex(expected),
                "{context}: reached child bytes"
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 186);
    assert_eq!(pattern_caches, 54);
}
