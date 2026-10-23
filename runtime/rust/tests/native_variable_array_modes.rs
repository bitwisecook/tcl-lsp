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

#![cfg(have_tommath)]

use std::rc::Rc;
use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};

#[test]
fn variable_arity_and_array_name_modes_match_original_oracles() {
    let source = include_bytes!(
        "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/conformance.tcl"
    );
    for (environment, expected) in [
        ("tcl8.4", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.4.20/conformance-stdout.txt").as_slice()),
        ("tcl8.5", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.5.19/conformance-stdout.txt").as_slice()),
        ("tcl8.6", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.6.18/conformance-stdout.txt").as_slice()),
        ("tcl9.0", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-9.0.4/conformance-stdout.txt").as_slice()),
        ("tcl9.1", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-9.1.0/conformance-stdout.txt").as_slice()),
        ("jim", include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/jim-5bac7c9/conformance-stdout.txt").as_slice()),
    ] {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        ).expect("matching native core");
        assert_eq!(interp.eval_str(source), Code::Ok, "{environment}: {:?}; refusal={:?}", interp.result_bytes(), interp.native_access_refusal());
        let mut observed = interp.result_bytes();
        observed.push(b'\n');
        assert_eq!(observed, expected, "{environment}");
    }
}

#[test]
fn variable_declaration_upper_arity_matches_original_oracles() {
    for (environment, oracle) in [
        (
            "tcl8.4",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.4.20/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.5.19/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-8.6.18/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-9.0.4/extra-arity-stdout.txt"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/c-9.1.0/extra-arity-stdout.txt"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/native-variable-array-modes/jim-5bac7c9/extra-arity-stdout.txt"
            ),
        ),
    ] {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("matching native core");
        for (row, source) in oracle.lines().zip([
            "variable ::nh_extra_a A ::nh_extra_b",
            "variable ::nh_extra_c C ::nh_extra_d D",
        ]) {
            let wanted = tcl_syntax::list::split_list(row).expect("retained original row");
            let code = interp.eval_str(source.as_bytes());
            let expected_code = match wanted[2].as_ref() {
                "0" => Code::Ok,
                "1" => Code::Error,
                other => panic!("unexpected original completion: {other}"),
            };
            assert_eq!(
                code,
                expected_code,
                "{environment}: {source}; refusal={:?}",
                interp.native_access_refusal()
            );
            assert_eq!(
                interp.result_bytes(),
                wanted[3].as_bytes(),
                "{environment}: {source}"
            );
        }
    }
}

#[test]
fn array_name_mode_validation_boundaries_match_original_oracles() {
    let cases = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../experiments/name-handling/array-mode-validation");
    for (environment, original) in [
        (
            "tcl8.4",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/c-8.4.20.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/c-8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/c-8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/c-9.0.4.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/c-9.1.0.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../rust/tcl-registry/tests/data/source_contract_vectors/array-mode-validation/regression-vectors/jim-5bac7c9.tsv"
            ),
        ),
    ] {
        for row in original.lines() {
            let (input, expected) = row.split_once('\t').expect("retained oracle vector");
            // These adapter admission questions remain separately recorded gaps.
            if matches!(environment, "tcl8.4" | "tcl8.5")
                && matches!(
                    input,
                    "cases/arity-no-name.tcl"
                        | "traces/populated-too-many.tcl"
                        | "traces/populated-fail-too-many.tcl"
                )
            {
                continue;
            }
            // The original Jim shell never reaches these trace questions.
            if environment == "jim" && input.starts_with("traces/") {
                continue;
            }
            let source =
                std::fs::read_to_string(cases.join(input)).expect("retained original input");
            let adapted = format!(
                "set ::__observations {{}}\n{}\njoin $::__observations \"\\n\"",
                source.replace("puts [list", "lappend ::__observations [list")
            );
            let mut interp = Interp::with_native_core(
                Rc::new(NativeHost::new()),
                tcl_registry::model::resolve_environment(environment).unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("matching native core");
            let code = interp.eval_str(adapted.as_bytes());
            if environment != "jim"
                && matches!(
                    input,
                    "cases/populated-invalid-regexp.tcl" | "cases/populated-regexp-valid.tcl"
                )
            {
                assert_eq!(interp.native_access_refusal(), Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("array names original regular-expression worker")), "{environment}: {input}");
                continue;
            }
            assert_eq!(
                code,
                Code::Ok,
                "{environment}: {input}; result={:?}; refusal={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            let mut observed = interp.result_bytes();
            observed.push(b'\n');
            assert_eq!(
                observed,
                expected.replace("\\n", "\n").as_bytes(),
                "{environment}: {input}"
            );
        }
    }
}
