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

//! Oracle-pinned regression coverage for the r6a-rename-interp lane (#1412).
//!
//! Every expected message/result below was taken from a real `rename`/`interp`
//! call against `tclsh9.0` (9.0.4), and cross-checked against `tclsh8.6`
//! (8.6.16) wherever a comment says so. Each test runs the exact sheet quoted
//! in its comment, so a reader can paste it into a real `tclsh` and re-derive
//! the expectation without this harness.

use tcl_runtime::interp::{Code, Interp};

/// item 1: `rename` onto an occupied destination refuses and leaves both
/// commands intact (`can't rename to "X": command already exists`), rather
/// than silently destroying the destination.
///
/// tclsh9.0.4:
///   proc a {} {return A}; proc b {} {return B}
///   catch {rename a b} e   ;# => can't rename to "b": command already exists
///   info commands a        ;# => a   (untouched)
///   b                       ;# => B   (untouched)
#[test]
fn rename_onto_occupied_destination_refuses_and_leaves_both_intact() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"proc a {} {return A}"), Code::Ok);
    assert_eq!(interp.eval_str(b"proc b {} {return B}"), Code::Ok);
    assert_eq!(interp.eval_str(b"catch {rename a b} e; set e"), Code::Ok);
    assert_eq!(
        interp.result_bytes(),
        b"can't rename to \"b\": command already exists".as_slice()
    );
    assert_eq!(interp.eval_str(b"info commands a"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"a".as_slice());
    assert_eq!(interp.eval_str(b"b"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"B".as_slice());
}

/// item 1, self-rename corner: C's `TclRenameCommand` checks the
/// destination's hash table *before* removing the source, so a same-slot
/// self-rename finds the source itself occupying the slot and refuses too.
///
/// tclsh9.0.4:
///   proc foo {} {return F}
///   catch {rename foo foo} e   ;# => can't rename to "foo": command already exists
///   foo                        ;# => F   (untouched)
#[test]
fn rename_onto_its_own_name_also_refuses() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"proc foo {} {return F}"), Code::Ok);
    assert_eq!(
        interp.eval_str(b"catch {rename foo foo} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"can't rename to \"foo\": command already exists".as_slice()
    );
    assert_eq!(interp.eval_str(b"foo"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"F".as_slice());
}

/// item 2: `rename` across namespaces re-homes a proc — C's
/// `TclRenameCommand` reassigns `cmdPtr->nsPtr`, so `namespace current`
/// inside the body reports the *new* namespace, not the definition-time one.
///
/// tclsh9.0.4:
///   namespace eval ::src { proc p {} { return [namespace current] } }
///   namespace eval ::dst {}
///   rename ::src::p ::dst::p
///   ::dst::p   ;# => ::dst
#[test]
fn rename_across_namespaces_rehomes_a_proc() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(b"namespace eval ::src { proc p {} { return [namespace current] } }"),
        Code::Ok
    );
    assert_eq!(interp.eval_str(b"namespace eval ::dst {}"), Code::Ok);
    assert_eq!(interp.eval_str(b"rename ::src::p ::dst::p"), Code::Ok);
    assert_eq!(interp.eval_str(b"::dst::p"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"::dst".as_slice());
}
