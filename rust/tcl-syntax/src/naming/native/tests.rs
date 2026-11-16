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
const VERSIONS: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

#[test]
fn qualified_inventory_reports_preserve_constructed_components_and_flat_bytes() {
    let constructed = NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments([
        b"a:".as_slice(),
        b"branch\0\xff",
    ]));
    assert_eq!(
        native_qualified_inventory_report_bytes(&constructed, b":p\0\xfe"),
        b"::a:::branch\0\xff:::p\0\xfe",
    );
    let flat =
        NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(b"a:::branch\0\xff".as_slice()));
    assert_eq!(
        native_qualified_inventory_report_bytes(&flat, b":p\0\xfe"),
        b"::a:::branch\0\xff:::p\0\xfe",
    );
    // Colliding reports do not make the selected storage interchangeable.
    assert_ne!(constructed, flat);
    for root in [
        NativeNamespaceReportParts::CConstructed(ByteNamespacePath::root()),
        NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(b"".as_slice())),
    ] {
        assert_eq!(native_qualified_inventory_report_bytes(&root, b"p"), b"::p");
    }
}

#[test]
fn qualified_inventory_report_never_reinterprets_a_flat_separator_spelling() {
    let flat = NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(b"::".as_slice()));
    assert_eq!(
        native_qualified_inventory_report_bytes(&flat, b"p"),
        b"::::::p"
    );
    let first = NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments([
        b"a:".as_slice(),
        b"b",
    ]));
    let second = NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments([
        b"a".as_slice(),
        b":b",
    ]));
    assert_ne!(first, second);
    assert_eq!(
        native_qualified_inventory_report_bytes(&first, b"p"),
        native_qualified_inventory_report_bytes(&second, b"p"),
    );
}

#[test]
fn export_clear_control_uses_exact_cstring_input_and_has_no_jim_recipe() {
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        for (original, expected) in [
            (b"-clear".as_slice(), true),
            (b"-clear\0tail", true),
            (b"-cl", false),
            (b"--", false),
            (b"-clear\xc0\x80tail", false),
            (b"prefix-clear", false),
        ] {
            assert_eq!(protocol.namespace_export_clears(original), Ok(expected));
        }
    }
    assert_eq!(
        NativeNameProtocol::Jim084.namespace_export_clears(b"-clear"),
        Err(NameProjectionUnavailable::PurposeNotModelled),
    );
}

#[test]
fn ensemble_map_prefix_construction_preserves_counted_heads_before_lookup() {
    // naming.ensemble.original-counted-map-prefix-construction
    // docs/design/analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md
    // Native312 public map heads/calls are separate from this pure API
    // projection; a namespace report alone never selects a physical holder.
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let protocol = NativeNameProtocol::C(version);
        let relative = protocol
            .ensemble_map_prefix_input(b"::N", true, b"p\0tail")
            .unwrap();
        assert_eq!(relative.as_ref(), b"::N::p\0tail");
        assert_eq!(
            protocol
                .ensemble_map_prefix_input(b"::", false, b"p\0tail")
                .unwrap()
                .as_ref(),
            b"::p\0tail"
        );
        let rooted = b"::N::p\0tail";
        assert!(
            matches!(protocol.ensemble_map_prefix_input(b"::Q", true, rooted).unwrap(), Cow::Borrowed(value) if value == rooted)
        );
        assert_eq!(
            protocol
                .command_lookup_input(NativeNameContext::root(), &relative)
                .unwrap()
                .selected(),
            b"::N::p"
        );
    }
}

#[test]
fn ensemble_map_prefix_requires_its_selected_purpose_and_actual_parent_fact() {
    // Authored pure API negatives, not native detached-holder observations.
    for protocol in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::Jim084,
    ] {
        assert_eq!(
            protocol
                .ensemble_map_prefix_input(b"::N", true, b"p")
                .unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
    }
    let protocol = NativeNameProtocol::C(TclVersion::V9_1);
    assert_eq!(
        protocol
            .ensemble_map_prefix_input(b"::N", false, b"p")
            .unwrap()
            .as_ref(),
        b"::Np"
    );
    assert_eq!(
        protocol
            .ensemble_map_prefix_input(b"::N", true, b"p")
            .unwrap()
            .as_ref(),
        b"::N::p"
    );
}

#[test]
fn trace_subject_registration_keeps_escaped_zero_distinct_from_raw_zero() {
    // Native proof: naming.variable.trace-subject-counted-zero-address
    // docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
    let raw = b"v\0tail(k)";
    let escaped = b"v\xc0\x80tail(k)";
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            protocol.trace_registration_input(raw).unwrap().selected(),
            b"v"
        );
        assert_eq!(protocol.trace_query_input(raw).unwrap().selected(), b"v");
        assert_eq!(
            protocol
                .trace_registration_input(escaped)
                .unwrap()
                .selected(),
            escaped,
        );
        assert_eq!(
            protocol.trace_query_input(escaped).unwrap().selected(),
            escaped,
        );
        assert_eq!(
            protocol.variable_root_input(raw).selected(),
            if version == TclVersion::V8_4 {
                b"v".as_slice()
            } else {
                raw
            },
        );
    }
    assert!(
        NativeNameProtocol::Jim084
            .trace_registration_input(raw)
            .is_err()
    );
}

#[test]
fn command_variable_trace_package_formal_and_hidden_extents_are_distinct() {
    let full = b"k\0z";
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            protocol
                .command_lookup_input(NativeNameContext::root(), full)
                .unwrap()
                .selected(),
            b"k"
        );
        assert_eq!(
            protocol.variable_root_input(full).selected(),
            if version == TclVersion::V8_4 {
                b"k".as_slice()
            } else {
                full
            }
        );
        assert_eq!(
            protocol.trace_registration_input(full).unwrap().selected(),
            b"k"
        );
        assert_eq!(protocol.package_key(full).selected(), b"k");
        assert_eq!(protocol.formal_name_input(full).selected(), b"k");
        assert_eq!(protocol.hidden_token_input(full).unwrap().selected(), b"k");
        assert_eq!(
            report_native_name_bytes(protocol, NativeNameReportPurpose::TraceCallbackName1, full)
                .unwrap(),
            b"k"
        );
        assert_eq!(
            protocol.variable_root_input(b"k\xc0\x80z").selected(),
            b"k\xc0\x80z"
        );
        let late = protocol.variable_root_input(b"k\0z::c");
        assert_eq!(late.qualification(), NativeNameQualification::Unqualified);
        assert_eq!(
            late.selected(),
            if version == TclVersion::V8_4 {
                b"k".as_slice()
            } else {
                b"k\0z::c"
            }
        );
        assert_eq!(
            protocol.variable_root_input(b"::ns::k\0z").selected(),
            b"::ns::k"
        );
        assert_eq!(
            protocol
                .command_lookup_input(NativeNameContext::root(), b"k\xff")
                .unwrap()
                .selected(),
            b"k\xff"
        );
    }
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(
        jim.command_lookup_input(NativeNameContext::root(), full)
            .unwrap()
            .jim_flat_key(),
        Some(full.as_slice())
    );
    assert_eq!(jim.variable_root_input(full).selected(), full);
    assert_eq!(jim.formal_name_input(full).selected(), full);
    assert_eq!(jim.package_key(full).selected(), b"k");
    assert_eq!(
        jim.trace_registration_input(full).unwrap_err(),
        NameProjectionUnavailable::PurposeNotModelled
    );
    assert_eq!(
        jim.hidden_token_input(full).unwrap_err(),
        NameProjectionUnavailable::PurposeNotModelled
    );
}

#[test]
fn formal_storage_enumeration_and_namespace_upvar_locals_have_distinct_purposes() {
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            protocol.formal_enumeration_name_input(b"k\0z").selected(),
            b"k"
        );
        assert_eq!(
            protocol.formal_storage_name_input(b"k\0z").selected(),
            if version <= TclVersion::V8_5 {
                b"k".as_slice()
            } else {
                b"k\0z"
            }
        );
        assert_eq!(
            protocol.formal_storage_name_input(b"k\xc0\x80z").selected(),
            b"k\xc0\x80z"
        );
        let list = protocol.formal_parameter_list_input(b"k\0z second");
        assert_eq!(list.purpose(), NativeNamePurpose::FormalParameterList);
        assert_eq!(list.original(), b"k\0z second");
        assert_eq!(
            list.selected(),
            if version <= TclVersion::V8_5 {
                b"k".as_slice()
            } else {
                b"k\0z second"
            }
        );
        if version == TclVersion::V8_4 {
            assert!(protocol.namespace_upvar_local_input(b"a\0z").is_err());
        } else {
            assert_eq!(
                protocol
                    .namespace_upvar_local_input(b"a\0z")
                    .unwrap()
                    .selected(),
                b"a"
            );
            assert_eq!(protocol.variable_root_input(b"a\0z").selected(), b"a\0z");
            assert_eq!(
                protocol
                    .namespace_upvar_local_input(b"a\xff")
                    .unwrap()
                    .selected(),
                b"a\xff"
            );
        }
    }
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(jim.formal_storage_name_input(b"k\0z").selected(), b"k\0z");
    assert_eq!(
        jim.formal_enumeration_name_input(b"k\0z").selected(),
        b"k\0z"
    );
    assert_eq!(
        jim.namespace_upvar_local_input(b"a\0z").unwrap().selected(),
        b"a\0z"
    );
}

#[test]
fn constant_binding_failures_have_separate_parts_codes_and_verbs() {
    use NativeVariableDiagnosticOperation::MakeConstant;
    use NativeVariableDiagnosticReason::{MissingParentNamespace, NoSuchElement, NotArray};
    use NativeVariableFailureSite::NameLookup;
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            native_constant_failure_verb(protocol, NameLookup).unwrap(),
            "const"
        );
        assert_eq!(
            native_constant_failure_verb(protocol, NativeVariableFailureSite::ValueWrite).unwrap(),
            "make constant"
        );
        for (original, reason, expected, kind, key) in [
            (
                b"::missing::k\0z".as_slice(),
                MissingParentNamespace,
                b"::missing::k".as_slice(),
                b"VARNAME".as_slice(),
                b"::missing::k".as_slice(),
            ),
            (
                b"a(k\0z)".as_slice(),
                NotArray,
                b"a(k)".as_slice(),
                b"VARNAME".as_slice(),
                b"a".as_slice(),
            ),
            (
                b"a(k\0z)".as_slice(),
                NoSuchElement,
                b"a(k)".as_slice(),
                b"ELEMENT".as_slice(),
                b"k".as_slice(),
            ),
            (
                b"a(k\xc0\x80z)".as_slice(),
                NoSuchElement,
                b"a(k\xc0\x80z)".as_slice(),
                b"ELEMENT".as_slice(),
                b"k\xc0\x80z".as_slice(),
            ),
        ] {
            let result = report_native_variable_diagnostic_at(
                protocol,
                MakeConstant,
                reason,
                NameLookup,
                NativeVariableInputForm::Combined(original),
            )
            .unwrap();
            assert_eq!(result.name, expected);
            assert_eq!(
                result.error_code,
                Some(vec![
                    b"TCL".to_vec(),
                    b"LOOKUP".to_vec(),
                    kind.to_vec(),
                    key.to_vec()
                ])
            );
        }
    }
}

#[test]
fn constant_declaration_reports_original_operand_and_its_own_tuple() {
    use NativeVariableDiagnosticOperation::MakeConstant;
    use NativeVariableDiagnosticReason::{AlreadyExists, ArrayElement, IsArray};
    use NativeVariableFailureSite::ValueWrite;
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        for (original, reason, expected) in [
            (b"a(k\0z)".as_slice(), ArrayElement, b"a(k".as_slice()),
            (b"a\0z".as_slice(), AlreadyExists, b"a".as_slice()),
            (b"a\xc0\x80z".as_slice(), IsArray, b"a\xc0\x80z".as_slice()),
            (b"a\xff".as_slice(), AlreadyExists, b"a\xff".as_slice()),
        ] {
            let result = report_native_variable_diagnostic_at(
                NativeNameProtocol::C(version),
                MakeConstant,
                reason,
                ValueWrite,
                NativeVariableInputForm::Combined(original),
            )
            .unwrap();
            assert_eq!(result.name, expected);
            assert_eq!(
                result.error_code,
                Some(vec![b"TCL".to_vec(), b"LOOKUP".to_vec(), b"CONST".to_vec()])
            );
            assert_eq!(result.missing_lookup_root, None);
        }
    }
    for protocol in [
        NativeNameProtocol::C(TclVersion::V8_6),
        NativeNameProtocol::Jim084,
    ] {
        assert!(
            report_native_variable_diagnostic_at(
                protocol,
                MakeConstant,
                AlreadyExists,
                ValueWrite,
                NativeVariableInputForm::Combined(b"a")
            )
            .is_err()
        );
    }
    assert!(
        report_native_variable_diagnostic_at(
            NativeNameProtocol::C(TclVersion::V9_0),
            MakeConstant,
            AlreadyExists,
            NativeVariableFailureSite::NameLookup,
            NativeVariableInputForm::Combined(b"a")
        )
        .is_err()
    );
}

#[test]
fn increment_constant_failure_has_its_own_write_disposition() {
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let projection = report_native_variable_diagnostic_at(
            NativeNameProtocol::C(version),
            NativeVariableDiagnosticOperation::Increment,
            NativeVariableDiagnosticReason::Constant,
            NativeVariableFailureSite::ValueWrite,
            NativeVariableInputForm::Combined(b"k\0suffix"),
        )
        .unwrap();
        assert_eq!(projection.name, b"k");
        assert_eq!(
            projection.error_code,
            Some(vec![b"TCL".to_vec(), b"WRITE".to_vec(), b"CONST".to_vec()])
        );
    }
    assert!(
        report_native_variable_diagnostic_at(
            NativeNameProtocol::C(TclVersion::V8_6),
            NativeVariableDiagnosticOperation::Increment,
            NativeVariableDiagnosticReason::Constant,
            NativeVariableFailureSite::ValueWrite,
            NativeVariableInputForm::Combined(b"k"),
        )
        .is_err()
    );
}

fn assert_constant_variable_diagnostics(protocol: NativeNameProtocol) {
    use NativeVariableDiagnosticOperation::{Unset, Write};
    use NativeVariableDiagnosticReason::Constant;
    use NativeVariableFailureSite::{ValueUnset, ValueWrite};
    let write = report_native_variable_diagnostic_at(
        protocol,
        Write,
        Constant,
        ValueWrite,
        NativeVariableInputForm::Combined(b"k\0z"),
    )
    .unwrap();
    let unset = report_native_variable_diagnostic_at(
        protocol,
        Unset,
        Constant,
        ValueUnset,
        NativeVariableInputForm::Combined(b"k\0z"),
    )
    .unwrap();
    assert_eq!(write.name, b"k");
    assert_eq!(
        write.error_code,
        Some(vec![
            b"TCL".to_vec(),
            b"WRITE".to_vec(),
            b"VARNAME".to_vec()
        ])
    );
    assert_eq!(
        unset.error_code,
        Some(vec![b"TCL".to_vec(), b"UNSET".to_vec(), b"CONST".to_vec()])
    );
}

#[test]
fn variable_diagnostics_retain_original_input_and_actual_failure_site() {
    use NativeVariableDiagnosticOperation::{Read, Unset};
    use NativeVariableDiagnosticReason::{NoSuchElement, NotArray};
    use NativeVariableFailureSite::{NameLookup, ValueRead};
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        let input = NativeVariableInputForm::Combined(b"a(k\0z)");
        let read =
            report_native_variable_diagnostic_at(protocol, Read, NoSuchElement, ValueRead, input)
                .unwrap();
        let unset =
            report_native_variable_diagnostic_at(protocol, Unset, NoSuchElement, NameLookup, input)
                .unwrap();
        let scalar =
            report_native_variable_diagnostic_at(protocol, Read, NotArray, NameLookup, input)
                .unwrap();
        assert_eq!(read.name, b"a(k");
        assert_eq!(unset.name, b"a(k)");
        assert_eq!(scalar.name, b"a(k)");
        if version >= TclVersion::V8_6 {
            assert_eq!(
                read.error_code,
                Some(vec![b"TCL".to_vec(), b"READ".to_vec(), b"VARNAME".to_vec()])
            );
            assert_eq!(
                unset.error_code,
                Some(vec![
                    b"TCL".to_vec(),
                    b"LOOKUP".to_vec(),
                    b"ELEMENT".to_vec(),
                    b"k".to_vec()
                ])
            );
            assert_eq!(scalar.missing_lookup_root, Some(b"a".to_vec()));
        } else {
            assert_eq!(read.error_code, None);
        }
        let control = report_native_variable_diagnostic_at(
            protocol,
            Read,
            NoSuchElement,
            ValueRead,
            NativeVariableInputForm::Combined(b"a(k\xc0\x80z)"),
        )
        .unwrap();
        assert_eq!(control.name, b"a(k\xc0\x80z)");
        let separate = report_native_variable_diagnostic_at(
            protocol,
            Read,
            NoSuchElement,
            ValueRead,
            NativeVariableInputForm::Separate {
                root: b"a",
                element: Some(b"k\0z"),
            },
        )
        .unwrap();
        assert_eq!(separate.name, b"a(k)");
        if version >= TclVersion::V9_0 {
            assert_constant_variable_diagnostics(protocol);
        }
    }
    let jim = NativeNameProtocol::Jim084;
    let input = NativeVariableInputForm::Combined(b"a(k\0z)");
    let read =
        report_native_variable_diagnostic_at(jim, Read, NoSuchElement, ValueRead, input).unwrap();
    let unset =
        report_native_variable_diagnostic_at(jim, Unset, NotArray, NameLookup, input).unwrap();
    assert_eq!(read.name, b"a(k)");
    assert_eq!(unset.name, b"a(k");
    assert_eq!(unset.reason, NoSuchElement);
    assert_eq!(unset.error_code, None);
}

fn retired_namespace_original_inputs() -> [(TclVersion, &'static str, &'static str); 5] {
    [
        (
            TclVersion::V8_4,
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.4.20/stdout.txt"
            ),
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.4.20/stdout.txt"
            ),
        ),
        (
            TclVersion::V8_5,
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.5.19/stdout.txt"
            ),
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.5.19/stdout.txt"
            ),
        ),
        (
            TclVersion::V8_6,
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.6.18/stdout.txt"
            ),
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
            ),
        ),
        (
            TclVersion::V9_0,
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/9.0.4/stdout.txt"
            ),
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/9.0.4/stdout.txt"
            ),
        ),
        (
            TclVersion::V9_1,
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/9.1.0/stdout.txt"
            ),
            include_str!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/9.1.0/stdout.txt"
            ),
        ),
    ]
}

#[test]
fn retired_namespace_write_sites_match_retained_original_c_rows() {
    use NativeVariableDiagnosticOperation::Write;
    use NativeVariableDiagnosticReason::RetiredNamespace;
    use NativeVariableFailureSite::{IndexedNameLookup, NameLookup, ValueWrite};
    let inputs = retired_namespace_original_inputs();
    for (version, compiled, object_command) in inputs {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        for (capture, label, site, original) in [
            (compiled, "literal-scalar", ValueWrite, b"foo".as_slice()),
            (
                compiled,
                "literal-element",
                IndexedNameLookup,
                b"foo(3)".as_slice(),
            ),
            (
                object_command,
                "variable-head-element",
                NameLookup,
                b"foo(3)".as_slice(),
            ),
        ] {
            let prefix = format!("ROW {label} ");
            let line = capture
                .lines()
                .find(|line| line.starts_with(&prefix))
                .unwrap();
            let row = crate::list::split_native_list_bytes(line.as_bytes(), protocol).unwrap();
            let expected = crate::list::split_native_list_bytes(&row[3], protocol).unwrap();
            let projection = report_native_variable_diagnostic_at(
                NativeNameProtocol::C(version),
                Write,
                RetiredNamespace,
                site,
                NativeVariableInputForm::Combined(original),
            )
            .unwrap();
            let mut message = b"can't set \"".to_vec();
            message.extend_from_slice(&projection.name);
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(projection.reason.message().as_bytes());
            assert_eq!(message.as_slice(), expected[3].as_ref());
            let code = if expected[4].as_ref() == b"NONE" {
                None
            } else {
                Some(
                    crate::list::split_native_list_bytes(&expected[4], protocol)
                        .unwrap()
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect::<Vec<_>>(),
                )
            };
            assert_eq!(projection.error_code, code);
            assert_eq!(
                projection.missing_lookup_root.as_deref(),
                (version >= TclVersion::V8_6 && site == NameLookup).then_some(b"foo".as_slice())
            );
        }
    }
}

#[test]
fn retired_namespace_read_and_unset_report_no_such_variable() {
    use NativeVariableDiagnosticOperation::{Read, Unset};
    use NativeVariableDiagnosticReason::{DetachedElement, NoSuchVariable, RetiredNamespace};
    use NativeVariableFailureSite::{NameLookup, ValueRead, ValueUnset};
    for (version, retired) in VERSIONS
        .into_iter()
        .flat_map(|version| [(version, RetiredNamespace), (version, DetachedElement)])
    {
        let protocol = NativeNameProtocol::C(version);
        for (operation, site, verb) in [
            (Read, ValueRead, b"READ".as_slice()),
            (Read, NameLookup, b"READ"),
            (Unset, ValueUnset, b"UNSET"),
            (Unset, NameLookup, b"UNSET"),
        ] {
            let projection = report_native_variable_diagnostic_at(
                protocol,
                operation,
                retired,
                site,
                NativeVariableInputForm::Combined(b"y"),
            )
            .unwrap();
            assert_eq!(projection.name, b"y");
            assert_eq!(projection.reason, NoSuchVariable);
            assert_eq!(
                projection.error_code,
                (version >= TclVersion::V8_6).then(|| vec![
                    b"TCL".to_vec(),
                    verb.to_vec(),
                    b"VARNAME".to_vec()
                ])
            );
        }
    }
}

#[test]
fn c85_tags_only_the_missing_element_unset() {
    use NativeVariableDiagnosticOperation::{Read, Unset};
    use NativeVariableDiagnosticReason::NoSuchElement;
    use NativeVariableFailureSite::NameLookup;
    let code = |version, operation| {
        report_native_variable_diagnostic_at(
            NativeNameProtocol::C(version),
            operation,
            NoSuchElement,
            NameLookup,
            NativeVariableInputForm::Combined(b"arr(k)"),
        )
        .unwrap()
        .error_code
    };
    assert_eq!(
        code(TclVersion::V8_5, Unset),
        Some(vec![
            b"TCL".to_vec(),
            b"LOOKUP".to_vec(),
            b"ELEMENT".to_vec(),
            b"k".to_vec()
        ])
    );
    assert_eq!(code(TclVersion::V8_5, Read), None);
    assert_eq!(code(TclVersion::V8_4, Unset), None);
}

#[test]
fn indexed_retired_lookup_rejects_missing_or_incompatible_purpose() {
    use NativeVariableDiagnosticOperation::{Read, Write};
    use NativeVariableDiagnosticReason::{IsArray, RetiredNamespace};
    use NativeVariableFailureSite::IndexedNameLookup;
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        assert!(
            report_native_variable_diagnostic_at(
                protocol,
                Write,
                RetiredNamespace,
                IndexedNameLookup,
                NativeVariableInputForm::Combined(b"foo")
            )
            .is_err()
        );
        for (operation, reason) in [(Read, RetiredNamespace), (Write, IsArray)] {
            assert!(
                report_native_variable_diagnostic_at(
                    protocol,
                    operation,
                    reason,
                    IndexedNameLookup,
                    NativeVariableInputForm::Combined(b"foo(3)")
                )
                .is_err()
            );
        }
    }
    assert!(
        report_native_variable_diagnostic_at(
            NativeNameProtocol::Jim084,
            Write,
            RetiredNamespace,
            IndexedNameLookup,
            NativeVariableInputForm::Combined(b"foo(3)")
        )
        .is_err()
    );
}

#[test]
fn c_array_root_write_failure_uses_the_measured_value_store_tuple() {
    // Native proof: naming.variable.original-array-root-write-diagnostic
    // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
    // The source observations identify output; this checks the selected Rust site.
    use NativeVariableDiagnosticOperation::{Read, Write};
    use NativeVariableDiagnosticReason::IsArray;
    use NativeVariableFailureSite::{NameLookup, ValueWrite};
    let input = NativeVariableInputForm::Combined(b"a");
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        let projection =
            report_native_variable_diagnostic_at(protocol, Write, IsArray, ValueWrite, input)
                .unwrap();
        assert_eq!(projection.name, b"a");
        assert_eq!(projection.reason, IsArray);
        let code = (version >= TclVersion::V8_6)
            .then(|| vec![b"TCL".to_vec(), b"WRITE".to_vec(), b"VARNAME".to_vec()]);
        assert_eq!(projection.error_code, code);
        assert!(projection.missing_lookup_root.is_none());
        assert!(
            report_native_variable_diagnostic_at(protocol, Read, IsArray, ValueWrite, input)
                .is_err()
        );
        if version >= TclVersion::V8_6 {
            assert!(
                report_native_variable_diagnostic_at(protocol, Write, IsArray, NameLookup, input)
                    .is_err()
            );
        }
    }
}

#[test]
fn jim_evaluated_element_read_failures_keep_the_selected_root_and_key() {
    use NativeVariableDiagnosticOperation::{Read, Write};
    use NativeVariableDiagnosticReason::{NoSuchElement, NoSuchVariable, NotArray};
    use NativeVariableFailureSite::NameLookup;
    let input = NativeVariableInputForm::Separate {
        root: b"arr",
        element: Some(b"missing"),
    };
    for (reason, expected) in [
        (NoSuchElement, b"arr(missing)".as_slice()),
        (NotArray, b"arr(missing)".as_slice()),
        (NoSuchVariable, b"arr".as_slice()),
    ] {
        let projection = report_native_variable_diagnostic_at(
            NativeNameProtocol::Jim084,
            Read,
            reason,
            NameLookup,
            input,
        )
        .unwrap();
        assert_eq!(projection.name, expected);
        assert_eq!(projection.reason, reason);
        assert_eq!(projection.error_code, None);
    }
    assert!(
        report_native_variable_diagnostic_at(
            NativeNameProtocol::Jim084,
            Write,
            NotArray,
            NameLookup,
            input,
        )
        .is_err()
    );
}

#[test]
fn combined_and_separate_elements_keep_native_input_form_and_release() {
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        let combined = protocol.combined_variable_input(b"a(k\0z)");
        let separated = protocol.separate_variable_input(b"a", Some(b"k\0z"));
        assert_eq!(combined.root().selected(), b"a");
        assert_eq!(
            combined.element().unwrap().selected(),
            if version < TclVersion::V9_0 {
                b"k".as_slice()
            } else {
                b"k\0z"
            }
        );
        assert_eq!(
            separated.element().unwrap().selected(),
            if version == TclVersion::V8_4 {
                b"k".as_slice()
            } else {
                b"k\0z"
            }
        );
        assert_eq!(
            combined.original(),
            NativeVariableInputForm::Combined(b"a(k\0z)")
        );
        assert_eq!(
            separated.original(),
            NativeVariableInputForm::Separate {
                root: b"a",
                element: Some(b"k\0z")
            }
        );
    }
}

#[test]
fn c_api_publication_and_script_declaration_retain_different_contexts() {
    let path = ByteNamespacePath::from_segments([b"n".as_slice()]);
    for version in VERSIONS {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            protocol
                .command_publication_slot(NativeNameContext::new(&path), b"x")
                .unwrap(),
            ByteCommandSlot::new(path.clone(), NameBytes::from(b"x"))
        );
        assert_eq!(
            protocol
                .command_c_api_publication_slot(NativeNameContext::new(&path), b"x\0z")
                .unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"x"))
        );
        assert_eq!(
            protocol
                .command_c_api_publication_slot(NativeNameContext::new(&path), b"child::x")
                .unwrap(),
            ByteCommandSlot::new(path.with_child(b"child"), NameBytes::from(b"x"))
        );
    }
}

#[test]
fn jim_context_composition_does_not_borrow_c_namespace_parsing() {
    let path = ByteNamespacePath::from_segments([b"src".as_slice()]);
    let context = NativeNameContext::with_jim_namespace(&path, b"src");
    let jim = NativeNameProtocol::Jim084;
    let relative = jim.command_publication_input(context, b"k\0z").unwrap();
    assert_eq!(relative.original(), b"k\0z");
    assert_eq!(relative.selected(), b"src::k");
    let absolute = jim
        .command_publication_input(context, b"::src::k\0z")
        .unwrap();
    assert_eq!(absolute.selected(), b"::src::k\0z");
    assert_eq!(absolute.jim_flat_key(), Some(b"src::k\0z".as_slice()));
    assert_eq!(
        jim.jim_command_lookup_keys(context, b"k\0z").unwrap(),
        vec![NameBytes::from(b"src::k"), NameBytes::from(b"k\0z")]
    );
    assert_eq!(
        jim.namespace_address_input(context, b"k\0z")
            .unwrap()
            .selected(),
        b"src::k\0z"
    );
    assert_eq!(
        jim.namespace_address_input(context, b"::k\0z")
            .unwrap()
            .selected(),
        b"k"
    );
    assert_eq!(
        jim.command_publication_input(NativeNameContext::new(&path), b"x")
            .unwrap_err(),
        NameProjectionUnavailable::MissingJimNamespaceObject
    );
}

#[test]
fn analytical_slot_projection_never_reconstructs_opaque_identity() {
    let colon = ByteCommandSlot::new(
        ByteNamespacePath::from_segments([b"a:".as_slice()]),
        NameBytes::from(b"p"),
    );
    assert_eq!(
        checked_command_slot_utf8(&colon).unwrap().namespace,
        vec!["a:"]
    );
    assert_eq!(native_command_full_name_bytes(&colon), b"::a:::p");
    assert_eq!(
        native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &colon),
        None
    );
    let raw = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"k\xff"));
    assert_eq!(native_command_full_name_bytes(&raw), b"::k\xff");
    assert!(checked_command_slot_utf8(&raw).is_err());
    let nul = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"k\0z"));
    assert_eq!(native_command_full_name_bytes(&nul), b"::k\0z");
    assert!(checked_command_slot_utf8(&nul).is_ok());
    assert_eq!(
        native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &nul),
        None
    );
    assert_eq!(
        native_command_source_spelling(NativeNameProtocol::Jim084, &nul).as_deref(),
        Some("::k\0z")
    );
    let ordinary = ByteCommandSlot::new(
        ByteNamespacePath::from_segments([b"n".as_slice()]),
        NameBytes::from(b"p"),
    );
    assert_eq!(
        native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &ordinary)
            .as_deref(),
        Some("::n::p")
    );
}

#[test]
fn namespace_reports_and_source_aliases_do_not_replace_constructed_identity() {
    for version in VERSIONS {
        let c = NativeNameProtocol::C(version);
        assert_eq!(c.namespace_tail_bytes(b"::k:z"), b"k:z");
        assert_eq!(c.namespace_qualifier_bytes(b"::k:z"), b"");
        assert_eq!(c.namespace_tail_bytes(b"k\0z::c"), b"k");
        assert_eq!(c.namespace_qualifier_bytes(b"a:::b"), b"a");
        assert_eq!(
            native_namespace_source_spelling(
                c,
                &ByteNamespacePath::from_segments([b"a:".as_slice()])
            ),
            Some(String::from("::a:"))
        );
        assert_eq!(
            native_namespace_source_spelling(
                c,
                &ByteNamespacePath::from_segments([b"a".as_slice()])
            ),
            Some(String::from("::a"))
        );
    }
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(jim.namespace_tail_bytes(b"::k:z"), b"::k:z");
    assert_eq!(jim.namespace_tail_bytes(b"k\0z::c"), b"k\0z::c");
    assert_eq!(jim.namespace_qualifier_bytes(b"a:::b"), b"a:");
    assert_eq!(native_jim_namespace_source_spelling(b"a\0z"), None);
    assert_eq!(
        native_jim_namespace_source_spelling(b"a:::b"),
        Some(String::from("::a:::b"))
    );
    assert_eq!(
        native_namespace_source_spelling(jim, &ByteNamespacePath::root()),
        Some(String::from("::"))
    );
}

#[test]
fn array_receiver_selection_and_trace_reporting_keep_original_and_cell_parts() {
    for version in VERSIONS {
        let c = NativeNameProtocol::C(version);
        let input = c.combined_variable_input(b"a\0z(k)");
        if version < TclVersion::V9_0 {
            assert_eq!(
                input.root().selected(),
                if version == TclVersion::V8_4 {
                    b"a".as_slice()
                } else {
                    b"a\0z"
                }
            );
            assert_eq!(input.element().unwrap().selected(), b"k");
        } else {
            assert_eq!(input.root().selected(), b"a\0z(k)");
            assert!(input.element().is_none());
        }
        let trace = report_native_variable_access_trace_names(
            c,
            NativeVariableTraceReportingInput {
                original_part1: b"a(k\0z)",
                original_part2: None,
                actual_element_key: Some(b"k\0z"),
            },
        )
        .unwrap();
        assert_eq!(trace.name1, b"a(k");
        assert_eq!(
            trace.name2,
            if version < TclVersion::V9_0 {
                b"".as_slice()
            } else {
                b"k"
            }
        );
        let separated = report_native_variable_access_trace_names(
            c,
            NativeVariableTraceReportingInput {
                original_part1: b"a",
                original_part2: Some(b"k\0z"),
                actual_element_key: Some(b"k\0z"),
            },
        )
        .unwrap();
        assert_eq!(
            separated,
            NativeVariableTraceNames {
                name1: b"a",
                name2: b"k"
            }
        );
        let modified = report_native_variable_access_trace_names(
            c,
            NativeVariableTraceReportingInput {
                original_part1: b"a(k\xc0\x80z)",
                original_part2: None,
                actual_element_key: Some(b"k\xc0\x80z"),
            },
        )
        .unwrap();
        assert_eq!(
            modified,
            NativeVariableTraceNames {
                name1: b"a",
                name2: b"k\xc0\x80z"
            }
        );
    }
    assert!(
        NativeNameProtocol::Jim084
            .combined_variable_input(b"a\0z(k)")
            .element()
            .is_none()
    );
}

#[test]
fn original_lambda_namespace_preserves_counted_object_and_lookup_purposes() {
    // naming.procedure.original-root-and-colon-holder-publication
    // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
    // naming.lambda.original-namespace-constructor-and-getter
    // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
    // SDK raw String00 and ByteArray materialised C080 remain independent.
    // These projections retain selected getter bytes without claiming an
    // original object header or namespace existence from the byte recipe.
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = NativeNameProtocol::C(version);
        for (original, selected, components) in [
            (
                b":ns".as_slice(),
                b":::ns".as_slice(),
                vec![b"ns".as_slice()],
            ),
            (b":::ns", b":::ns", vec![b"ns".as_slice()]),
            (b"", b"::", vec![]),
            (b":ns\0suffix", b":::ns\0suffix", vec![b"ns".as_slice()]),
            (b"::ns\0suffix", b"::ns\0suffix", vec![b"ns".as_slice()]),
            (
                b"\xc0\x80ns",
                b"::\xc0\x80ns",
                vec![b"\xc0\x80ns".as_slice()],
            ),
        ] {
            let input = recipe.lambda_namespace_input(original).unwrap();
            assert_eq!(input.original(), original);
            assert_eq!(input.selected(), selected);
            assert_eq!(input.purpose(), NativeNamePurpose::LambdaNamespace);
            assert_eq!(
                recipe.lambda_namespace_path(original).unwrap(),
                ByteNamespacePath::from_segments(components)
            );
        }
    }
    for recipe in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::Jim084,
    ] {
        assert!(recipe.lambda_namespace_input(b":ns").is_err());
        assert!(recipe.lambda_namespace_path(b":ns").is_err());
    }
}

#[test]
fn namespace_creation_parent_route_uses_its_original_cstring_extent() {
    for version in TclVersion::ALL {
        let protocol = NativeNameProtocol::C(version);
        for (input, parent) in [
            (b"retiring".as_slice(), false),
            (b"retiring::", true),
            (b"retiring::::", true),
            (b"retiring::Missing", false),
            (b"retiring::Missing::", true),
            (b"retiring\0::::", false),
            (b"retiring::\0Tail", true),
            (b"retiring:", false),
        ] {
            assert_eq!(
                protocol.namespace_creation_reuses_parent(input, false),
                Ok(parent)
            );
            assert!(
                protocol
                    .namespace_creation_reuses_parent(input, true)
                    .is_err()
            );
            assert!(
                NativeNameProtocol::Jim084
                    .namespace_creation_reuses_parent(input, false)
                    .is_err()
            );
        }
    }
}

#[test]
fn jim_lambda_namespace_keeps_the_counted_original_context_object() {
    for original in [
        b"AbsentLambda".as_slice(),
        b"::AbsentLambda",
        b"::::AbsentLambda",
        b":AbsentLambda",
        b"Absent:Lambda",
        b"Absent\0Tail",
        b"",
        b"Absent::Child",
    ] {
        let input = NativeNameProtocol::Jim084
            .jim_lambda_namespace_input(original)
            .unwrap();
        assert_eq!(input.original(), original);
        assert_eq!(input.selected(), original);
        assert_eq!(input.purpose(), NativeNamePurpose::JimLambdaNamespace);
        assert_eq!(input.qualification(), NativeNameQualification::Unqualified);
        for version in TclVersion::ALL {
            assert!(
                NativeNameProtocol::C(version)
                    .jim_lambda_namespace_input(original)
                    .is_err()
            );
        }
    }
}

#[test]
fn original_root_procedure_publication_preserves_a_literal_colon() {
    // naming.procedure.original-root-and-colon-holder-publication
    // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
    // C84/85 TclProcObjCmd only prepends a non-global holder's fullName.
    // CLI source observations are independent of these pure slot assertions.
    for version in VERSIONS {
        let recipe = NativeNameProtocol::C(version);
        let context = NativeNameContext::root();
        assert_eq!(
            recipe
                .command_publication_slot(context, b":source")
                .unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(":source"))
        );
        assert_eq!(
            recipe
                .command_publication_slot(context, b":::source")
                .unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("source"))
        );
    }
}

#[test]
fn native_terminal_colon_namespace_keeps_lookup_context_and_release_publication() {
    let namespace = ByteNamespacePath::from_segments([b"a:".as_slice()]);
    let context = NativeNameContext::new(&namespace);
    for version in VERSIONS {
        let c = NativeNameProtocol::C(version);
        assert_eq!(
            c.command_lookup_slot(context, b"p").unwrap(),
            ByteCommandSlot::new(namespace.clone(), NameBytes::from("p"))
        );
        let published = c.command_publication_slot(context, b"p").unwrap();
        assert_eq!(
            published.namespace,
            if version <= TclVersion::V8_5 {
                ByteNamespacePath::from_segments(["a"])
            } else {
                namespace.clone()
            }
        );
        assert_eq!(published.simple, "p");
        assert_eq!(
            c.command_c_api_publication_slot(context, b"p").unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("p"))
        );
    }
}

#[test]
fn variable_diagnostics_report_parts_and_keep_lookup_keys_separate() {
    for version in VERSIONS {
        let c = NativeNameProtocol::C(version);
        for operation in [
            NativeVariableDiagnosticOperation::Read,
            NativeVariableDiagnosticOperation::Unset,
        ] {
            for (input, expected, key) in [
                (b"k\0z".as_slice(), b"k".as_slice(), b"k".as_slice()),
                (b"a(k\0z)", b"a(k)", b"a"),
                (b"N\0z::k", b"N", b"N"),
                (b"N\xff::k", b"N\xff::k", b"N\xff::k"),
            ] {
                let reported = report_native_variable_diagnostic(
                    c,
                    operation,
                    NativeVariableDiagnosticReason::NoSuchVariable,
                    NativeVariableInputForm::Combined(input),
                )
                .unwrap();
                assert_eq!(reported.name, expected);
                assert_eq!(
                    reported.missing_lookup_root,
                    (version >= TclVersion::V8_6).then(|| key.to_vec())
                );
            }
        }
    }
    let jim_read = report_native_variable_diagnostic(
        NativeNameProtocol::Jim084,
        NativeVariableDiagnosticOperation::Read,
        NativeVariableDiagnosticReason::NoSuchVariable,
        NativeVariableInputForm::Combined(b"a(k\0z)"),
    )
    .unwrap();
    assert_eq!(jim_read.name, b"a");
    let jim_unset = report_native_variable_diagnostic(
        NativeNameProtocol::Jim084,
        NativeVariableDiagnosticOperation::Unset,
        NativeVariableDiagnosticReason::NotArray,
        NativeVariableInputForm::Combined(b"a(k\0z)"),
    )
    .unwrap();
    assert_eq!(jim_unset.name, b"a(k");
}

#[test]
fn authored_provider_never_authenticates_vendor_native_name_recipes() {
    let vendor = DialectPoint::of_name_and_release(Some("f5-irules"), None).expect("known vendor");
    assert_eq!(NativeNameProtocol::for_point(vendor), None);
    assert_eq!(NativeStringProtocol::for_point(vendor), None);
    assert_eq!(NamePolicyProtocol::for_native_point(vendor), None);
    let logical = NamePolicyProtocol::authored_tcl(TclVersion::V8_4);
    assert_eq!(logical.authority(), NamePolicyAuthority::AuthoredSimulation);
    assert_eq!(logical.recipe(), NativeNameProtocol::C(TclVersion::V8_4));
    assert_eq!(
        NamePolicyProtocol::for_native_point(DialectPoint::for_tcl_version(TclVersion::V9_1))
            .unwrap()
            .authority(),
        NamePolicyAuthority::Native
    );
}
