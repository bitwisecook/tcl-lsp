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

use super::LocalVarTable;
use tcl_runtime_api::NameBytes;

#[test]
fn native_formal_keys_keep_opaque_and_embedded_nul_identity() {
    let names = [NameBytes::from(b"\xff"), NameBytes::from(b"a\0b")];
    let mut table = LocalVarTable::from_names(&names);
    assert_eq!(table.entries(), &names);
    assert_eq!(table.intern_bytes(b"\xff"), 0);
    assert_eq!(table.intern_bytes(b"a\0b"), 1);
    assert_eq!(table.intern("a"), 2);
    let temp = table.intern_synthetic("#tmp");
    assert_eq!(temp, 3);
    assert!(!table.is_source_local_bytes(b"#tmp"));
    assert!(table.is_source_local_bytes(b"a\0b"));
}

#[test]
fn native_compiler_selection_retains_each_formal_primary_key() {
    use tcl_syntax::naming::NativeCompiledVariableProtocol;
    let names = [
        NameBytes::from(b"k\0a"),
        NameBytes::from(b"k\0b"),
        NameBytes::from(b"k\0a"),
        NameBytes::from(b"\xff"),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = NativeCompiledVariableProtocol::for_native_point(
            tcl_dialect::model::DialectPoint::for_tcl_version(version),
        )
        .unwrap();
        let mut table = LocalVarTable::from_native_names(&names);
        assert_eq!(table.entries(), &names);
        assert_eq!(table.find_native(protocol, b"k\0b"), Some(0));
        assert_eq!(table.intern_native(protocol, b"k\0c"), 0);
        assert_eq!(table.intern_native(protocol, b"k\0zz"), 4);
        assert_eq!(table.entries()[1].as_bytes(), b"k\0b");
        assert_eq!(table.entries()[4].as_bytes(), b"k\0zz");
        assert_eq!(table.find_native(protocol, b"k"), None);
    }
}

#[test]
fn native_lookup_excludes_temporaries_without_stealing_real_names() {
    let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(
        tcl_dialect::TclVersion::V8_4,
    );
    let mut table = LocalVarTable::default();
    let temp = table.intern_synthetic("#tmp");
    assert_eq!(table.find_native(protocol, b"#tmp"), None);
    let source = table.intern_native(protocol, b"#tmp");
    assert_ne!(temp, source);
    assert_eq!(table.find_native(protocol, b"#tmp"), Some(source));
    assert!(table.is_source_local_bytes(b"#tmp"));
    let second_temp = table.intern_synthetic("#tmp");
    assert_ne!(second_temp, source);
    assert_eq!(table.find_native(protocol, b"#tmp"), Some(source));
}

#[test]
fn configured_native_policy_governs_ordinary_intern_and_query_methods() {
    let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(
        tcl_dialect::TclVersion::V8_4,
    );
    let mut table = LocalVarTable::from_native_names(&[NameBytes::from(b"k\0a")]);
    table.set_native_protocol(Some(protocol));
    assert_eq!(table.intern("k\0b"), 0);
    assert_eq!(table.intern_bytes(b"k\0c"), 0);
    assert_eq!(table.find("k\0z"), Some(0));
    assert!(table.is_source_local("k\0q"));
    assert_eq!(table.entries()[0].as_bytes(), b"k\0a");
    assert_eq!(table.intern("k"), 1);
    assert_eq!(table.intern("k\0long"), 2);
    table.set_native_protocol(None);
    assert_eq!(table.find("k\0b"), None);
    assert_eq!(table.find("k\0a"), Some(0));
}
#[test]
fn native_unnamed_slots_preserve_indices_without_source_aliases() {
    use tcl_runtime_api::NameBytes;
    let slots = vec![
        Some(NameBytes::from(b"k\0a")),
        None,
        Some(NameBytes::from(b"k\0b")),
        Some(NameBytes::default()),
    ];
    let mut table = LocalVarTable::from_native_slot_names(&slots);
    assert_eq!(table.native_slot_names(), slots);
    assert_eq!(table.find_bytes(b""), Some(3));
    assert_eq!(table.find_bytes(b"k\0b"), Some(2));
    table.set_native_protocol(Some(
        tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(
            tcl_dialect::TclVersion::V9_0,
        ),
    ));
    assert_eq!(table.find_bytes(b"k\0b"), Some(0));
    assert_eq!(table.find_bytes(b""), Some(3));
}

/// A hidden name reaches one anonymous slot by every lookup, and the layout
/// keeps no name for it, as tclsh's `temp` slots have none
/// (`tcl::unsupported::disassemble` shows `slot 4, scalar, temp`, absent from
/// `info locals`, tclsh 9.0.4).
#[test]
fn a_hidden_name_reaches_an_anonymous_slot_by_every_lookup() {
    use tcl_syntax::naming::NativeCompiledVariableProtocol;
    let mut table = LocalVarTable::from_names(&[NameBytes::from(b"n")]);
    table.hide_names_with_prefixes(vec![NameBytes::from(b"__inline_1__")]);
    assert_eq!(table.find("__inline_1__x"), None);
    let slot = table.intern("__inline_1__x");
    assert_eq!(slot, 1);
    assert_eq!(table.intern_bytes(b"__inline_1__x"), slot);
    assert_eq!(table.find("__inline_1__x"), Some(slot));
    let protocol = NativeCompiledVariableProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
    assert_eq!(table.find_native(protocol, b"__inline_1__x"), Some(slot));
    assert_eq!(table.intern_native(protocol, b"__inline_1__x"), slot);
    assert_eq!(
        table.native_slot_names(),
        vec![Some(NameBytes::from(b"n")), None]
    );
    // A name outside the prefixes stays an ordinary named local.
    assert_eq!(table.intern("__inline_2__x"), 2);
    assert_eq!(
        table.native_slot_names()[2],
        Some(NameBytes::from(b"__inline_2__x"))
    );
}
