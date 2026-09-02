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

/// item 3: `interp`'s bad-option list must advertise only subcommands it
/// dispatches. `target` is cheap (this runtime's two alias shapes make the
/// interp-path trivial to compute) and now has an arm; `cancel`/`share`/
/// `transfer` need infrastructure this runtime has none of (script
/// cancellation, cross-interp channel sharing) and are dropped from the
/// list rather than left advertised-but-undispatchable.
///
/// tclsh9.0.4 (for contrast — this runtime's list intentionally differs,
/// naming only what it implements):
///   catch {interp bogus} e
///   => bad option "bogus": must be alias, aliases, bgerror, cancel,
///      children, create, debug, delete, eval, exists, expose, hide,
///      hidden, issafe, invokehidden, limit, marktrusted, recursionlimit,
///      share, target, or transfer
#[test]
fn interp_bad_option_list_advertises_only_dispatched_subcommands() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"catch {interp bogus} e; set e"), Code::Ok);
    assert_eq!(
        interp.result_bytes(),
        b"bad option \"bogus\": must be alias, aliases, bgerror, children, \
          create, debug, delete, eval, exists, expose, hide, hidden, issafe, \
          invokehidden, limit, marktrusted, recursionlimit, or target"
            .as_slice()
    );
    // Every name still in the list dispatches — `cancel`/`share`/`transfer`
    // must not appear (removed from the option list, not silently rejected
    // via the fallthrough).
    assert_eq!(interp.eval_str(b"catch {interp cancel} e; set e"), Code::Ok);
    assert!(interp.result_bytes().starts_with(b"bad option \"cancel\""));
}

/// item 3: `interp target path alias` — the interp-path from this interp to
/// `alias`'s target interpreter. A same-interp alias's target is the
/// interpreter it is installed in, so `interp target {} name` for a
/// same-interp alias returns the empty list (tclsh9.0.4-pinned:
/// `Tcl_GetInterpPath` returns `{}` when asker and target coincide).
///
/// tclsh9.0.4:
///   proc foo {} {return hi}
///   interp alias {} bar {} foo
///   interp target {} bar        ;# => {}  (empty list)
///   catch {interp target {} nosuch} e
///   => alias "nosuch" in path "" not found
#[test]
fn interp_target_of_a_same_interp_alias_is_the_empty_path() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"proc foo {} {return hi}"), Code::Ok);
    assert_eq!(interp.eval_str(b"interp alias {} bar {} foo"), Code::Ok);
    assert_eq!(interp.eval_str(b"interp target {} bar"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"".as_slice());
    assert_eq!(
        interp.eval_str(b"catch {interp target {} nosuch} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"alias \"nosuch\" in path \"\" not found".as_slice()
    );
}
