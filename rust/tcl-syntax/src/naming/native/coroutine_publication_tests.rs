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

type CoroutinePublicationCase<'a> = (&'a [u8], &'a [&'a [u8]], &'a [u8]);

#[test]
fn coroutine_slots_keep_current_holders_and_native_byte_extents() {
    // Native proof: naming.coroutine.original-publication-boundaries
    // docs/design/analysis/name-resolution-proofs/coroutine-original-publication-boundaries.md
    let current = ByteNamespacePath::from_segments([b"N".as_slice()]);
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let protocol = NativeNameProtocol::C(version);
        let cases: &[CoroutinePublicationCase<'_>] = &[
            (b"c\0::Missing::tail", &[b"N"], b"c"),
            (b"c\xc0\x80z", &[b"N"], b"c\xc0\x80z"),
            (b"c\xffz", &[b"N"], b"c\xffz"),
            (b"c\xed\xa0\x80z", &[b"N"], b"c\xed\xa0\x80z"),
            (b"Q::c", &[b"N", b"Q"], b"c"),
            (b"::Q::c", &[b"Q"], b"c"),
            (b"", &[b"N"], b""),
            (b"::", &[], b""),
            (b"Q::", &[b"N", b"Q"], b""),
        ];
        for &(original, holders, simple) in cases {
            let slot = protocol
                .coroutine_publication_slot(NativeNameContext::new(&current), original)
                .unwrap();
            assert_eq!(
                slot.namespace,
                ByteNamespacePath::from_segments(holders.iter().copied())
            );
            assert_eq!(slot.simple.as_bytes(), simple);
        }
        let error = protocol
            .coroutine_unknown_namespace_error(b"Missing::c\0z")
            .unwrap();
        assert_eq!(
            error.message,
            b"can't create procedure \"Missing::c\": unknown namespace"
        );
        assert_eq!(error.error_code, b"TCL LOOKUP NAMESPACE");
    }
    for protocol in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::C(TclVersion::V8_5),
        NativeNameProtocol::Jim084,
    ] {
        assert_eq!(
            protocol
                .coroutine_publication_slot(NativeNameContext::new(&current), b"c")
                .unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
    }
}
#[test]
fn c_value_report_name_keeps_original_compound_parts() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    // The native source proves the ::a(k) name; extent controls are Rust ownership coverage.
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let protocol = NativeNameProtocol::C(version);
        assert_eq!(
            report_native_c_variable_value_name(
                protocol,
                NativeVariableInputForm::Separate {
                    root: b"::a",
                    element: Some(b"k")
                }
            )
            .unwrap(),
            b"::a(k)"
        );
        let input = NativeVariableInputForm::Separate {
            root: b"a\0tail",
            element: Some(b"k\0tail"),
        };
        let name = report_native_c_variable_value_name(protocol, input).unwrap();
        assert_eq!(name, b"a(k)");
        assert_eq!(
            report_native_variable_diagnostic_at(
                protocol,
                NativeVariableDiagnosticOperation::Read,
                NativeVariableDiagnosticReason::NoSuchElement,
                NativeVariableFailureSite::ValueRead,
                input
            )
            .unwrap()
            .name,
            name
        );
        assert_eq!(
            report_native_c_variable_value_name(
                protocol,
                NativeVariableInputForm::Combined(b"a\0(k)")
            )
            .unwrap(),
            b"a"
        );
    }
    assert_eq!(
        report_native_c_variable_value_name(
            NativeNameProtocol::Jim084,
            NativeVariableInputForm::Separate {
                root: b"a",
                element: Some(b"k")
            }
        ),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}
