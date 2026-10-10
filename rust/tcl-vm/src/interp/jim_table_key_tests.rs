// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original public Jim key spelling controls, separate from key header identity.

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields: Vec<_> = rows
        .lines()
        .find(|line| line.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect();
    (
        fields[1].parse().unwrap(),
        fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

type OriginalControl = (&'static str, &'static [u8], [&'static str; 6]);
const CONTROLS: &[OriginalControl] = &[
    ("root-replacement", include_bytes!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/root-replacement.tcl").as_slice(), [
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.4.20/root-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.5.19/root-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.6.18/root-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/9.0.4/root-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/9.1.0/root-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/jim/root-replacement/stdout"),
    ]),
    ("rename-and-fresh-publication", include_bytes!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/rename-and-fresh-publication.tcl").as_slice(), [
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.4.20/rename-and-fresh-publication/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.5.19/rename-and-fresh-publication/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/8.6.18/rename-and-fresh-publication/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/9.0.4/rename-and-fresh-publication/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/9.1.0/rename-and-fresh-publication/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys340/jim/rename-and-fresh-publication/stdout"),
    ]),
    ("namespace-replacement", include_bytes!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/namespace-replacement.tcl").as_slice(), [
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/8.4.20/namespace-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/8.5.19/namespace-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/8.6.18/namespace-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/9.0.4/namespace-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/9.1.0/namespace-replacement/stdout"),
        include_str!("../../../tcl-registry/tests/data/native_jim_command_table_keys341/jim/namespace-replacement/stdout"),
    ]),
 ];

#[test]
fn original_jim_table_key_public_results_match_replacement_rename_and_namespace_sources() {
    // Native proof: naming.jim.original-command-table-key-publication
    // docs/design/analysis/name-resolution-proofs/jim-original-command-table-key-publication.md
    // Complete original public output does not establish private original key/header identity.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            vm.install_scripted_library(tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble);
            let completion = vm
                .eval_source(std::str::from_utf8(source).unwrap())
                .unwrap();
            assert_eq!(
                completion.code.as_int(),
                expected_code,
                "{engine}/{case}: {completion:?}; host={:?}",
                vm.refused_completion()
            );
            assert_eq!(
                completion.result.string_bytes(),
                expected_result,
                "{engine}/{case}"
            );
            assert!(vm.refused_completion().is_none(), "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 18);
}
