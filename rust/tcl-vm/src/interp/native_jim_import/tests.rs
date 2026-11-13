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

//! Public Import entry against the recorded jimsh 0.84 observations.

use crate::{Code, Vm};
use std::fmt::Write;
use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;

const CASES: &str = include_str!(
    "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/cases.tsv"
);
const OBSERVATIONS: &str = include_str!(
    "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/observations-20261015.tsv"
);

fn jim() -> Vm {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
    vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
    vm
}

fn observed(vm: &mut Vm, phase: &str, label: &str, input: &str) {
    let completion = vm.eval_source(input).unwrap();
    let hex = completion
        .result
        .to_str()
        .bytes()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
    let row = format!("{phase}|{label}|{}|{hex}", completion.code.as_int());
    assert!(
        OBSERVATIONS.lines().any(|line| line == row),
        "{row}; refusal={:?}",
        vm.execution_refusal
    );
    assert!(vm.execution_refusal.is_none(), "{row}");
}

#[test]
fn public_import_matches_recorded_alias_chain_completions() {
    for label in ["SIMPLE", "ALIAS", "DEEP_ALIAS", "EMPTY", "EMPTY_REST"] {
        let input = CASES
            .lines()
            .find_map(|line| line.strip_prefix(label)?.strip_prefix('|'))
            .unwrap();
        let mut vm = jim();
        let setup = vm
            .eval_source(include_str!(
                "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/setup.tcl"
            ))
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        observed(&mut vm, "IMPORT", label, input);
        observed(
            &mut vm,
            "COMMANDS",
            label,
            include_str!(
                "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/commands-report.tcl"
            ),
        );
        observed(
            &mut vm,
            "ALIAS",
            label,
            include_str!(
                "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/alias-report.tcl"
            ),
        );
    }
}

#[test]
fn repeated_alias_walk_refuses_where_jimsh_never_returns() {
    let observation = include_str!(
        "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/cycle-observation-20261015.tsv"
    );
    assert!(observation.starts_with("CYCLE_SETUP|0|"));
    assert!(observation.ends_with("CYCLE_EVAL_ENTER\n"));
    let mut vm = jim();
    let setup = vm
        .eval_source(include_str!(
            "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/cycle-setup.tcl"
        ))
        .unwrap();
    assert_eq!(setup.code, Code::Ok);
    let failure = vm
        .eval_source(include_str!(
            "../../../../../runtime/rust/tests/data/native_jim_import_source_completion/cycle-input.tcl"
        ))
        .unwrap_err();
    assert!(matches!(
        failure,
        crate::error::TclError::Host(crate::error::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "Jim namespace import cyclic alias chain"
                )
            )
        ))
    ));
    assert!(vm.lookup_command("a").is_none());
    assert_eq!(vm.current_level(), 0);
}

#[test]
fn each_candidate_walk_starts_fresh() {
    let mut vm = jim();
    let completion = vm
        .eval_source(
            "namespace eval S {proc p {} {return P}}; alias S::a S::p; alias S::b S::a; \
             namespace eval D {namespace import ::S::*}; list [D::a] [D::b] [D::p]",
        )
        .unwrap();
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(&*completion.result.to_str(), "P P P");
    assert!(vm.execution_refusal.is_none());
}
