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

#[test]
fn invocation_context_survives_argument_projection_and_expansion() {
    let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let words = [InvocationWord::Literal("require"), InvocationWord::Expanded];
    let arguments = InvocationArguments::Structured(&words).with_dialect(dialect);
    assert_eq!(arguments.slice_from(1).dialect(), Some(dialect));
    assert_eq!(arguments.slice_from(1).exact_argv_len(), None);
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(jim.tcl_version, None);
    assert_eq!(
        jim.package_protocol,
        Some(tcl_dialect::PackageProtocol::Jim)
    );
    assert_eq!(
        jim.namespace_import_binding,
        Some(tcl_dialect::NamespaceImportBinding::SourceName)
    );
}

use super::*;

#[test]
fn structured_words_only_expose_known_literals() {
    let words = [
        InvocationWord::Literal("fixed"),
        InvocationWord::Dynamic,
        InvocationWord::Expanded,
        InvocationWord::Opaque,
    ];
    let arguments = InvocationArguments::structured(&words);

    assert_eq!(arguments.literal_at(0), Some("fixed"));
    assert_eq!(arguments.literal_at(1), None);
    assert_eq!(arguments.literal_at(2), None);
    assert_eq!(arguments.literal_at(3), None);
    assert!(!arguments.are_all_literals());
    assert!(!arguments.has_exact_argv_len());
    assert_eq!(arguments.exact_argv_len(), None);
    assert_eq!(arguments.literal_values(), None);
}

#[test]
fn literal_values_preserve_complete_known_argv() {
    let structured = [
        InvocationWord::Literal("first"),
        InvocationWord::Literal("second"),
    ];
    assert_eq!(
        InvocationArguments::structured(&structured).literal_values(),
        Some(vec!["first", "second"])
    );
}

#[test]
fn known_native_bytes_keep_argv_shape_without_a_logical_value() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let payload = [b'n', 0, 0xff, 0xed, 0xa0, 0x80];
    let words = [
        InvocationWord::KnownBytes(&payload),
        InvocationWord::Literal("tail"),
    ];
    let arguments = InvocationArguments::structured(&words);
    assert_eq!(arguments.exact_argv_len(), Some(2));
    assert_eq!(arguments.argv_at(0), InvocationArgument::Word(words[0]));
    assert_eq!(arguments.argv_at(1), InvocationArgument::Word(words[1]));
    assert_eq!(arguments.native_bytes_at(0), Some(payload.as_slice()));
    assert_eq!(arguments.native_bytes_at(1), None);
    assert_eq!(arguments.literal_at(0), None);
    assert_eq!(arguments.literal_values(), None);
    assert_eq!(words[0].kind(), InvocationWordKind::KnownBytes);
    assert!(words[0].proves_non_option());
    assert!(!InvocationWord::KnownBytes(b"-opaque\xff").proves_non_option());
    assert!(!InvocationWord::KnownBytes(b"").proves_non_option());
    let expanded = [InvocationWord::Expanded, words[0]];
    assert_eq!(
        InvocationArguments::structured(&expanded).argv_at(1),
        InvocationArgument::Indeterminate
    );
}

#[test]
fn non_option_dynamic_keeps_argv_shape_without_exposing_a_value() {
    let arguments = InvocationArguments::structured(&[InvocationWord::DynamicNonOption]);
    assert!(arguments.has_exact_argv_len());
    assert_eq!(arguments.literal_at(0), None);
    assert_eq!(arguments.literal_values(), None);
    assert_eq!(
        arguments.get(0).map(InvocationWord::kind),
        Some(InvocationWordKind::Dynamic)
    );
}

#[test]
fn argv_projection_preserves_prefix_before_expansion() {
    let arguments = [
        InvocationWord::Literal("known"),
        InvocationWord::Expanded,
        InvocationWord::Literal("shifted"),
    ];
    let arguments = InvocationArguments::structured(&arguments);

    assert_eq!(
        arguments.argv_at(0),
        InvocationArgument::Word(InvocationWord::Literal("known"))
    );
    assert_eq!(arguments.argv_at(1), InvocationArgument::Indeterminate);
    assert_eq!(arguments.argv_at(2), InvocationArgument::Indeterminate);
}
