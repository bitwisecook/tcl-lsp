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

use super::*;

const OBSERVED: &str =
    include_str!("../../tests/data/native_jim_namespace_canonical/current-holder-20261015.tsv");

fn row(kind: &str, phase: &str) -> Vec<&'static str> {
    OBSERVED
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split('|').collect();
            (fields[0] == kind && fields[1] == phase).then(|| fields[2..].to_vec())
        })
        .unwrap_or_else(|| panic!("missing original observation {kind}/{phase}"))
}

#[test]
fn canonical_uses_the_original_holder_and_reached_namespace_getter() {
    // Original SDK: experiments/name-handling/results/jim-namespace-canonical-current-holder.
    assert_eq!(OBSERVED.lines().count(), 80);
    for (label, name, explicit) in [
        ("CURRENT", None, false),
        ("ABSOLUTE", Some("::target"), false),
        ("RELATIVE", Some("child"), false),
        ("EXPLICIT", Some("child"), true),
    ] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
        let original = Value::int(42);
        let prefix = Value::int(84);
        let context =
            crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
        vm.push_call_frame(Some("canonical_current_holder_control".into()), Vec::new());
        vm.install_jim_procedure_namespace_owner(None, original.clone());
        vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
            context,
        ));
        assert!(original.resident_string_bytes().is_none());
        let mut arguments = vec![Value::string("canonical")];
        if explicit {
            arguments.push(prefix.clone());
        }
        if let Some(name) = name {
            arguments.push(Value::string(name));
        }
        let completion = cmd_namespace(&mut vm, &arguments);
        let phase = format!("{label}_REUSE");
        assert_eq!(completion.code, Code::Ok, "{phase}");
        assert_eq!(
            original.resident_string_bytes().is_some(),
            row("CALLER_AFTER", &phase)[0] == "1",
            "{phase}"
        );
        assert_eq!(
            prefix.resident_string_bytes().is_some(),
            row("PREFIX_AFTER", &phase)[0] == "1",
            "{phase}"
        );
        assert_eq!(
            completion.result.is_same_object(&original),
            row("RESULT_IS_CALLER", &phase)[0] == "1",
            "{phase}"
        );
        assert_eq!(
            vm.current_namespace_execution_context()
                .resolved_token()
                .is_some(),
            label == "RELATIVE",
            "{phase}"
        );
        let expected = row("RESULT", &phase);
        assert_eq!(expected[0], "0");
        let expected: Vec<_> = expected[1]
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(
            vm.native_name_operand_bytes_checked(&completion.result)
                .unwrap()
                .as_ref(),
            expected,
            "{phase}"
        );
        assert!(vm.execution_refusal.is_none(), "{phase}");
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[test]
fn equal_explicit_prefix_does_not_issue_the_current_namespace_coordinate() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
    let original = Value::int(42);
    let prefix = Value::int(42);
    assert!(!original.is_same_object(&prefix));
    let context =
        crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
    vm.push_call_frame(Some("canonical_explicit_prefix_control".into()), Vec::new());
    vm.install_jim_procedure_namespace_owner(None, original.clone());
    vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(context));
    let completion = cmd_namespace(
        &mut vm,
        &[
            Value::string("canonical"),
            prefix.clone(),
            Value::string("child"),
        ],
    );
    assert_eq!(completion.code, Code::Ok);
    assert!(prefix.resident_string_bytes().is_some());
    assert!(original.resident_string_bytes().is_none());
    assert!(
        vm.current_namespace_execution_context()
            .resolved_token()
            .is_none()
    );
    assert!(vm.execution_refusal.is_none());
    vm.pop_ns();
    vm.pop_call_frame();
}
