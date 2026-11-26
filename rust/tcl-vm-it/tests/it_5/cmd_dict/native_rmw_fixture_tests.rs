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

use tcl_syntax::value::ValueOps;
use tcl_vm::{Code, Value, Vm};

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn compare_dictionary_process(
    suite: &str,
    engine: &str,
    process: DictionaryProcess<'_>,
    compared: &mut usize,
) {
    let DictionaryProcess {
        name,
        code,
        result,
        source,
        profile,
        result_window,
    } = process;
    let receipt_start = std::time::Instant::now();
    tcl_test_support::oracle_row_progress(suite, engine, name, None);
    tcl_test_support::oracle_phase_progress(suite, engine, name, "vm-new-start", receipt_start);
    // Both original C drivers call Jim_InitStaticExtensions, which loads
    // stdlib; native core bootstrap alone intentionally has no dict update.
    let mut vm = crate::fixture::distribution(profile);
    tcl_test_support::oracle_phase_progress(suite, engine, name, "vm-new-complete", receipt_start);
    tcl_test_support::oracle_phase_progress(suite, engine, name, "profile-complete", receipt_start);
    tcl_test_support::oracle_phase_progress(
        suite,
        engine,
        name,
        "compiler-complete",
        receipt_start,
    );
    let completion = vm
        .try_eval_source_bytes(source)
        .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
    tcl_test_support::oracle_phase_progress(suite, engine, name, "eval-complete", receipt_start);
    assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
    compare_dictionary_result_window(
        &mut vm,
        &completion.result,
        result,
        result_window,
        engine,
        name,
    );
    tcl_test_support::oracle_phase_progress(
        suite,
        engine,
        name,
        "assertions-complete",
        receipt_start,
    );
    drop(vm);
    tcl_test_support::oracle_phase_progress(suite, engine, name, "vm-drop-complete", receipt_start);
    *compared += 1;
    tcl_test_support::oracle_row_progress(suite, engine, name, Some(*compared));
}

#[derive(Clone, Copy)]
enum DictionaryResultWindow {
    Complete,
    CompletionAndCallerReadStatus,
}

fn compare_dictionary_result_window(
    vm: &mut Vm,
    actual: &Value,
    expected: &[u8],
    window: DictionaryResultWindow,
    engine: &str,
    name: &str,
) {
    match window {
        DictionaryResultWindow::Complete => {
            let actual = vm.native_string_bytes(actual).unwrap();
            assert_eq!(actual.as_ref(), expected, "{engine}/{name}");
        }
        DictionaryResultWindow::CompletionAndCallerReadStatus => {
            // Independent update226/C85, update398/C86 and with405/C86
            // lifetime observations reach final free of the selected header.
            // These captured diagnostics/status fields remain fixed public
            // observations; status and outer completion follow the free and
            // certify no storage, runtime completion or effect permission.
            // The later freed-value bytes supply no equivalence premise.
            let expected = vm.new_bytes(expected);
            let expected_fields = vm.list_elements(&expected).unwrap();
            let actual_fields = vm.list_elements(actual).unwrap();
            assert_eq!(expected_fields.len(), 4, "{engine}/{name}");
            assert_eq!(actual_fields.len(), 4, "{engine}/{name}");
            for index in 0..3 {
                let expected = vm.native_string_bytes(&expected_fields[index]).unwrap();
                let actual = vm.native_string_bytes(&actual_fields[index]).unwrap();
                assert_eq!(
                    actual.as_ref(),
                    expected.as_ref(),
                    "{engine}/{name}/{index}"
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct DictionaryProcess<'a> {
    name: &'a str,
    code: i64,
    result: &'a [u8],
    source: &'a [u8],
    profile: &'static tcl_dialect::DialectProfile,
    result_window: DictionaryResultWindow,
}

#[test]
fn dictionary_body_writeback_compares_original_native_observation_windows() {
    // Native proofs: naming.dictionary.update-with-body-and-observer-frontiers;
    // docs/design/analysis/name-resolution-proofs/dictionary-update-with-body-and-observer-frontiers.md
    // naming.dictionary.c85-original-update-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-c85-original-update-write-error-object-lifetime.md
    // naming.dictionary.original-update-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-original-update-write-error-object-lifetime.md
    // naming.dictionary.original-with-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-original-with-write-error-object-lifetime.md
    // All 329 captured outer completions remain compared; 326 retain whole
    // public observations. Independent update and with lifetime observations
    // bound three diagnostic/status comparisons. Those later captured fields are
    // not defined storage, runtime completion or effect guarantees.
    let sources = include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/cases.tsv")
        .lines()
        .map(|row| tcl_test_support::fixture::tsv_split_once(row).unwrap())
        .collect::<std::collections::BTreeMap<_, _>>();
    let engines = [
        (
            "tcl8.4",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            55,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            53,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            53,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/9.1.0.tsv"),
        ),
        (
            "jim",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_body/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    let mut whole_results = 0;
    let mut bounded_results = 0;
    for (engine, count, expected) in engines {
        assert_eq!(expected.lines().count(), count);
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        for row in expected.lines() {
            let (name, remainder) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
            let (code, result) = tcl_test_support::fixture::tsv_split_once(remainder).unwrap();
            let code: i64 = code.parse().unwrap();
            let result = unhex(result);
            let source = unhex(sources[name]);
            let result_window = if matches!(engine, "tcl8.5" | "tcl8.6")
                && name == "compiled-update-dict-write-error"
            {
                assert_eq!(
                    source.as_slice(),
                    include_bytes!(
                        "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl"
                    )
                );
                bounded_results += 1;
                DictionaryResultWindow::CompletionAndCallerReadStatus
            } else if engine == "tcl8.6" && name == "compiled-with-dict-write-error" {
                assert_eq!(
                    source.as_slice(),
                    include_bytes!(
                        "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl"
                    )
                );
                bounded_results += 1;
                DictionaryResultWindow::CompletionAndCallerReadStatus
            } else {
                whole_results += 1;
                DictionaryResultWindow::Complete
            };
            compare_dictionary_process(
                "dictionary-body329",
                engine,
                DictionaryProcess {
                    name,
                    code,
                    result: &result,
                    source: &source,
                    profile,
                    result_window,
                },
                &mut compared,
            );
        }
    }
    assert_eq!(compared, 329);
    assert_eq!(whole_results, 326);
    assert_eq!(bounded_results, 3);
}

#[test]
fn c85_dictionary_write_error_retains_defined_host_storage() {
    // Native proof: naming.dictionary.c85-original-update-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-c85-original-update-write-error-object-lifetime.md
    // Current-build private counters and public bytes are separate evidence.
    // These live-header/value assertions test the VM's safe ownership policy;
    // they do not reproduce or certify native freed-object content.
    let baseline = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stdout"
    );
    let observed = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stdout"
    );
    assert_eq!(baseline, observed);
    assert_eq!(
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stderr"
        ),
        "DICT_RELEASE_OBSERVER|errors=1|refcount_before_release=1|same_cell=1|defined_scalar=1|final_free=1\n"
    );
    for negative in [
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stderr"
        ),
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stderr"
        ),
    ] {
        assert_eq!(
            negative,
            "DICT_RELEASE_OBSERVER|errors=0|refcount_before_release=-1|same_cell=-1|defined_scalar=-1|final_free=0\n"
        );
    }
    let source = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl"
    );
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.5").unit_profile();
    let mut vm = crate::fixture::interpreter(profile);
    let completion = vm.try_eval_source_bytes(source).unwrap();
    assert_eq!(completion.code, Code::Ok);
    let fields = vm.list_elements(&completion.result).unwrap();
    assert_eq!(fields.len(), 4);
    for (field, expected) in fields[..3].iter().zip([
        b"1".as_slice(),
        b"can't set \"d\": WRITE".as_slice(),
        b"0".as_slice(),
    ]) {
        assert_eq!(vm.native_string_bytes(field).unwrap().as_ref(), expected);
    }
    assert!(fields[3].native_object_is_live());
    assert_eq!(
        vm.native_string_bytes(&fields[3]).unwrap().as_ref(),
        b"k BASE"
    );
    let key = vm.new_bytes(b"k");
    let member = tcl_cmd_core::dict::get(&mut vm, &fields[3], &[key]).unwrap();
    assert!(member.native_object_is_live());
    assert_eq!(vm.native_string_bytes(&member).unwrap().as_ref(), b"BASE");
}

#[test]
fn c86_dictionary_write_error_retains_its_own_defined_host_storage() {
    // naming.dictionary.original-update-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-original-update-write-error-object-lifetime.md
    // The native counter window is independent of the VM's safe policy.
    // Neither native freed contents nor original compiled frame ownership
    // are reconstructed from this software header/member control.
    let baseline = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/tcl8.6/baseline-case9.stdout"
    );
    let observed = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/tcl8.6/observed-case9.stdout"
    );
    assert_eq!(baseline, observed);
    assert_eq!(
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/tcl8.6/observed-case9.stderr"
        ),
        "DICT_RELEASE_OBSERVER|errors=1|refcount_before_release=1|same_cell=1|defined_scalar=1|final_free=1\n",
    );
    for control in [
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/tcl8.6/observed-case0.stderr"
        ),
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/tcl8.6/observed-case19.stderr"
        ),
    ] {
        assert_eq!(
            control,
            "DICT_RELEASE_OBSERVER|errors=0|refcount_before_release=-1|same_cell=-1|defined_scalar=-1|final_free=0\n"
        );
    }
    let source = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-source.tcl"
    );
    assert_eq!(source.as_slice(), include_bytes!("../../../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl").as_slice());
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let mut vm = crate::fixture::interpreter(profile);
    let completion = vm.try_eval_source_bytes(source).unwrap();
    assert_eq!(completion.code, Code::Ok);
    let fields = vm.list_elements(&completion.result).unwrap();
    assert_eq!(fields.len(), 4);
    for (field, expected) in fields[..3].iter().zip([
        b"1".as_slice(),
        b"can't set \"d\": WRITE".as_slice(),
        b"0".as_slice(),
    ]) {
        assert_eq!(vm.native_string_bytes(field).unwrap().as_ref(), expected);
    }
    assert!(fields[3].native_object_is_live());
    assert_eq!(
        vm.native_string_bytes(&fields[3]).unwrap().as_ref(),
        b"k BASE"
    );
    let key = vm.new_bytes(b"k");
    let member = tcl_cmd_core::dict::get(&mut vm, &fields[3], &[key]).unwrap();
    assert!(member.native_object_is_live());
    assert_eq!(vm.native_string_bytes(&member).unwrap().as_ref(), b"BASE");
}

#[test]
fn c86_dictionary_with_write_error_retains_independent_defined_host_storage() {
    // naming.dictionary.original-with-write-error-object-lifetime
    // docs/design/analysis/name-resolution-proofs/dictionary-original-with-write-error-object-lifetime.md
    // The native witness samples WithFinish before final free. Subsequent
    // public fields are process observations; the VM's retained live storage
    // is an independent software policy, never native freed-content identity.
    let baseline = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/tcl8.6/baseline-case19.stdout"
    );
    let observed = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/tcl8.6/observed-case19.stdout"
    );
    assert_eq!(baseline, observed);
    assert_eq!(
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/tcl8.6/observed-case19.stderr"
        ),
        concat!(
            "WITH_BEFORE_SET|refcount=0|allocated=1\n",
            "WITH_SET_ERROR|same_cell=1|defined_scalar=1|final_free=0\n",
            "WITH_FINAL_FREE|count=1\n",
            "WITH_BODY_RETURN|entered=1|before_refcount=0|allocated=1|failed=1|same_cell=1|defined_scalar=1|final_free=1\n",
        ),
    );
    assert_eq!(
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/tcl8.6/observed-case10.stderr"
        ),
        concat!(
            "WITH_BEFORE_SET|refcount=0|allocated=1\n",
            "WITH_BODY_RETURN|entered=1|before_refcount=0|allocated=1|failed=0|same_cell=-1|defined_scalar=-1|final_free=0\n",
            "WITH_FINAL_FREE|count=1\n",
        ),
    );
    assert_eq!(
        include_str!(
            "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/tcl8.6/observed-case9.stderr"
        ),
        "WITH_BODY_RETURN|entered=0|before_refcount=-1|allocated=-1|failed=0|same_cell=-1|defined_scalar=-1|final_free=0\n",
    );
    let source = include_bytes!(
        "../../../../tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl"
    );
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let mut vm = crate::fixture::interpreter(profile);
    let completion = vm.try_eval_source_bytes(source).unwrap();
    assert_eq!(completion.code, Code::Ok);
    let fields = vm.list_elements(&completion.result).unwrap();
    assert_eq!(fields.len(), 4);
    for (field, expected) in fields[..3].iter().zip([
        b"1".as_slice(),
        b"can't set \"d\": WRITE".as_slice(),
        b"0".as_slice(),
    ]) {
        assert_eq!(vm.native_string_bytes(field).unwrap().as_ref(), expected);
    }
    assert!(fields[3].native_object_is_live());
    assert_eq!(
        vm.native_string_bytes(&fields[3]).unwrap().as_ref(),
        b"k BASE"
    );
    let key = vm.new_bytes(b"k");
    let member = tcl_cmd_core::dict::get(&mut vm, &fields[3], &[key]).unwrap();
    assert!(member.native_object_is_live());
    assert_eq!(vm.native_string_bytes(&member).unwrap().as_ref(), b"BASE");
}

// Native proof: naming.variable.dictionary-rmw-command-versus-selected-local-cell
// docs/design/analysis/name-resolution-proofs/variable.dictionary-rmw-command-versus-selected-local-cell.md
#[test]
fn dictionary_command_and_local_cell_publication_match_345_complete_native_processes() {
    const CASES: &str =
        include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/cases.tsv");
    let sources = CASES
        .lines()
        .map(|row| tcl_test_support::fixture::tsv_split_once(row).unwrap())
        .collect::<std::collections::BTreeMap<_, _>>();
    let engines = [
        (
            "tcl8.4",
            60,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            57,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            56,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/9.1.0.tsv"),
        ),
        (
            "jim",
            60,
            include_str!("../../../../tcl-vm/tests/data/native_dictionary_rmw/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, count, expected) in engines {
        assert_eq!(expected.lines().count(), count);
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        for row in expected.lines() {
            let (name, remainder) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
            let (code, result) = tcl_test_support::fixture::tsv_split_once(remainder).unwrap();
            let code: i64 = code.parse().unwrap();
            let result = unhex(result);
            let source = unhex(sources[name]);
            compare_dictionary_process(
                "dictionary-rmw345",
                engine,
                DictionaryProcess {
                    name,
                    code,
                    result: &result,
                    source: &source,
                    profile,
                    result_window: DictionaryResultWindow::Complete,
                },
                &mut compared,
            );
        }
    }
    assert_eq!(compared, 345);
}
