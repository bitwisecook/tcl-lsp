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

use tcl_bytecode_tables::{LiteralTable, LocalVarTable};

#[test]
fn private_original_literal_has_ordered_slot_without_string_receipt() {
    let mut table = tcl_bytecode_tables::LiteralTable::new();
    let string = table.intern_bytes(b"VALUE");
    let original = table.register_private_original();
    assert_eq!((string, original), (0, 1));
    assert_eq!(
        table.entries()[string].byte_payload(),
        Some(b"VALUE".as_slice())
    );
    assert_eq!(table.entries()[original].byte_payload(), None);
    assert_ne!(table.entries()[original], "");
    assert_eq!(
        table.entries()[original].allocation(),
        &tcl_bytecode_tables::NativeLiteralAllocation::PrivateOriginal
    );
    assert_eq!(
        table.native_actions(),
        &[
            tcl_bytecode_tables::NativeLiteralAction::Register(0),
            tcl_bytecode_tables::NativeLiteralAction::Register(1)
        ]
    );
}

#[test]
fn literal_table_intern() {
    let mut lit = LiteralTable::new();
    let a = lit.intern("hello");
    let b = lit.intern("world");
    let c = lit.intern("hello"); // dedup
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(c, 0); // same as first
    assert_eq!(lit.len(), 2);
}

#[test]
fn literal_table_register() {
    let mut lit = LiteralTable::new();
    let a = lit.register("x");
    let b = lit.register("x"); // no dedup
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(lit.len(), 2);
}

#[test]
fn literal_pool_preserves_native_bytes_without_unicode_aliasing() {
    let mut table = LiteralTable::new();
    let raw = table.intern_bytes(&[0xff]);
    let unicode = table.intern("ÿ");
    assert_ne!(raw, unicode);
    assert_eq!(table.intern_bytes(&[0xff]), raw);
    assert_eq!(table.entries()[raw].bytes(), &[0xff]);
    assert!(table.entries()[raw].unicode().is_err());
    assert_eq!(table.entries()[unicode].unicode(), Ok("ÿ"));
}

#[test]
fn local_var_table_intern() {
    let mut lvt = LocalVarTable::new(&["a", "b"]);
    assert_eq!(lvt.len(), 2);
    assert_eq!(lvt.intern("a"), 0); // pre-populated
    assert_eq!(lvt.intern("c"), 2); // new slot
    assert_eq!(lvt.entries(), &["a", "b", "c"]);
}
