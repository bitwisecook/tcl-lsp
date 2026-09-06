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
    // Issue #1607: the list is shortened, but it is still resolved by the one
    // `Tcl_GetIndexFromObj` matcher, so tclsh's abbreviation and ambiguity
    // verdicts hold over it.
    //
    // tclsh9.0.4:
    //   interp cr j -> j   ;  interp c j -> ambiguous option "c": must be …
    //   interp {}   -> ambiguous option "": must be …
    assert_eq!(interp.eval_str(b"interp cr j"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"j".as_slice());
    assert_eq!(interp.eval_str(b"catch {interp c j} e; set e"), Code::Ok);
    assert!(interp.result_bytes().starts_with(b"ambiguous option \"c\""));
    assert_eq!(interp.eval_str(b"catch {interp {}} e; set e"), Code::Ok);
    assert!(interp.result_bytes().starts_with(b"ambiguous option \"\""));
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

/// item 5: `interp invokehidden`'s `-namespace`/`-global` options establish
/// the evaluation context (resolved from the **global** namespace regardless
/// of the caller's current one, C's `TCL_GLOBAL_ONLY`), and an unrecognized
/// option is a hard error rather than a silently-skipped no-op. There is no
/// mutual-exclusion refusal for passing both — the issue's own claim of a
/// `cannot use -global option and -namespace option together` error does not
/// hold against either tclsh 8.6.16 or 9.0.4; the last option given simply
/// wins, same as tclsh.
///
/// tclsh9.0.4:
///   interp hide {} pwd
///   catch {interp invokehidden {} -bogus pwd} e
///   => bad option "-bogus": must be -global, -namespace, or --
///   namespace eval ::ns5 { interp invokehidden {} -namespace bar pwd }
///   namespace children :: bar     ;# => ::bar      (created at the global root)
///   namespace children ::ns5      ;# => {}          (not under ::ns5)
#[test]
fn invokehidden_rejects_unknown_options_and_namespace_is_global_anchored() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"interp hide {} pwd"), Code::Ok);
    assert_eq!(
        interp.eval_str(b"catch {interp invokehidden {} -bogus pwd} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"bad option \"-bogus\": must be -global, -namespace, or --".as_slice()
    );
    assert_eq!(
        interp.eval_str(b"namespace eval ::ns5 { interp invokehidden {} -namespace bar pwd }"),
        Code::Ok
    );
    assert_eq!(interp.eval_str(b"namespace children :: bar"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"::bar".as_slice());
    assert_eq!(interp.eval_str(b"namespace children ::ns5"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"".as_slice());
}

/// item 7: `$child subcommand` and `interp subcommand` report the same
/// `bad option` shape on an unrecognized subcommand — `$child`'s previously
/// said `interp subcommand "X" is not supported in this runtime`, not a
/// tclsh error shape at all. The two lists differ (the child command object
/// never dispatches `children`/`create`/`delete`/`exists` — those are only
/// ever spelled `interp <op> path`), but both are real tclsh `bad option`
/// errors.
///
/// tclsh9.0.4:
///   interp create kid
///   catch {kid bogus} e
///   => bad option "bogus": must be alias, aliases, bgerror, debug, eval,
///      expose, hide, hidden, issafe, invokehidden, limit, marktrusted,
///      or recursionlimit
#[test]
fn child_command_bad_option_matches_the_tclsh_shape() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"interp create kid"), Code::Ok);
    assert_eq!(interp.eval_str(b"catch {kid bogus} e; set e"), Code::Ok);
    assert_eq!(
        interp.result_bytes(),
        b"bad option \"bogus\": must be alias, aliases, bgerror, debug, eval, \
          expose, hide, hidden, issafe, invokehidden, limit, marktrusted, \
          or recursionlimit"
            .as_slice()
    );
    // Issue #1607: the same table now abbreviates, exactly as tclsh's does.
    //
    // tclsh9.0.4:
    //   kid ev {set x 1} -> 1
    //   kid h            -> ambiguous option "h": must be …
    assert_eq!(interp.eval_str(b"kid ev {set x 1}"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"1".as_slice());
    assert_eq!(interp.eval_str(b"catch {kid h} e; set e"), Code::Ok);
    assert!(interp.result_bytes().starts_with(b"ambiguous option \"h\""));
}

/// item 4 (fixed before this lane started, pinned here as a regression
/// guard): the delete form of `rename` — `rename name ""` — words its
/// missing-source error `can't delete`, not `can't rename`.
///
/// tclsh9.0.4:
///   catch {rename nosuch ""} e
///   => can't delete "nosuch": command doesn't exist
#[test]
fn rename_delete_form_says_cant_delete() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(b"catch {rename nosuch {}} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"can't delete \"nosuch\": command doesn't exist".as_slice()
    );
}

/// item 6 (fixed before this lane started, pinned here as a regression
/// guard): `interp hide`/`expose` misses raise, and `expose` refuses an
/// occupied destination instead of overwriting it.
///
/// tclsh9.0.4:
///   catch {interp hide {} nosuchcmd} e     ;# => unknown command "nosuchcmd"
///   catch {interp expose {} nosuchhidden} e ;# => unknown hidden command "nosuchhidden"
///   proc visible {} {return v}
///   interp hide {} pwd myhidden
///   catch {interp expose {} myhidden visible} e
///   => exposed command "visible" already exists
///   visible   ;# => v   (untouched)
#[test]
fn hide_and_expose_misses_and_collisions_raise() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(b"catch {interp hide {} nosuchcmd} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"unknown command \"nosuchcmd\"".as_slice()
    );
    assert_eq!(
        interp.eval_str(b"catch {interp expose {} nosuchhidden} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"unknown hidden command \"nosuchhidden\"".as_slice()
    );
    assert_eq!(interp.eval_str(b"proc visible {} {return v}"), Code::Ok);
    assert_eq!(interp.eval_str(b"interp hide {} pwd myhidden"), Code::Ok);
    assert_eq!(
        interp.eval_str(b"catch {interp expose {} myhidden visible} e; set e"),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"exposed command \"visible\" already exists".as_slice()
    );
    assert_eq!(interp.eval_str(b"visible"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"v".as_slice());
}

/// A command-delete trace that re-creates the command as an **identical**
/// alias leaves that new alias alive: command identity is per command token,
/// not per target/prefix shape, so the in-flight `rename foo {}` must not
/// delete the binding the callback just installed.
///
/// tclsh9.0.4 (and 8.6.16):
///   proc real {args} {return R}
///   interp alias {} foo {} real
///   trace add command foo delete {apply {{old new op} {interp alias {} foo {} real}}}
///   rename foo {}
///   info commands foo   ;# => foo   (the re-created alias survives)
///   foo                 ;# => R
#[test]
fn a_delete_trace_recreating_an_identical_alias_keeps_the_new_binding() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"proc real {args} {return R}"), Code::Ok);
    assert_eq!(interp.eval_str(b"interp alias {} foo {} real"), Code::Ok);
    assert_eq!(
        interp.eval_str(
            b"trace add command foo delete {apply {{old new op} {interp alias {} foo {} real}}}"
        ),
        Code::Ok
    );
    assert_eq!(interp.eval_str(b"rename foo {}"), Code::Ok);
    assert_eq!(interp.eval_str(b"info commands foo"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"foo".as_slice());
    assert_eq!(interp.eval_str(b"foo"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"R".as_slice());
}

/// R1: `$child hide|expose` takes the same one- *or* two-word form the
/// ensemble does. The arm used to guard on `argv.len() == 3`, so the two-word
/// spelling fell through to the `bad option` fallthrough and reported
/// `bad option "hide"` — an option the same message lists as valid.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   kid eval {proc lst {} {}}
///   kid hide lst mylst
///   kid hidden                     ;# => mylst
///   kid expose mylst lst2
///   kid eval {info commands lst2}  ;# => lst2
#[test]
fn child_hide_and_expose_accept_the_two_word_form() {
    let mut interp = Interp::new();
    assert_eq!(interp.eval_str(b"interp create kid"), Code::Ok);
    assert_eq!(interp.eval_str(b"kid eval {proc lst {} {}}"), Code::Ok);
    assert_eq!(interp.eval_str(b"kid hide lst mylst"), Code::Ok);
    assert_eq!(interp.eval_str(b"kid hidden"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"mylst".as_slice());
    assert_eq!(interp.eval_str(b"kid expose mylst lst2"), Code::Ok);
    assert_eq!(interp.eval_str(b"kid eval {info commands lst2}"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"lst2".as_slice());
}

/// R1, the arity errors: C's `NRChildCmd` names the *child command* in its
/// `wrong # args` text, never the `interp` ensemble.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   set out {}
///   foreach script {{kid hide} {kid hide set tok extra}
///                   {kid expose} {kid expose a b c}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   set out
///   => {wrong # args: should be "kid hide cmdName ?hiddenCmdName?"} {TCL WRONGARGS} …
#[test]
fn child_hide_and_expose_arity_errors_name_the_child() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              set out {}
              foreach script {
                  {kid hide}
                  {kid hide set tok extra}
                  {kid expose}
                  {kid expose a b c}
              } {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{wrong # args: should be \"kid hide cmdName ?hiddenCmdName?\"} \
          {TCL WRONGARGS} \
          {wrong # args: should be \"kid hide cmdName ?hiddenCmdName?\"} \
          {TCL WRONGARGS} \
          {wrong # args: should be \"kid expose hiddenCmdName ?cmdName?\"} \
          {TCL WRONGARGS} \
          {wrong # args: should be \"kid expose hiddenCmdName ?cmdName?\"} \
          {TCL WRONGARGS}"
            .as_slice()
    );
}

/// R2: the shorthand goes through the same structural checks as the ensemble
/// form. It used to call `hide_command(&cmd, &cmd)` directly, so
/// `kid hide ::foo::bar` silently filed a namespaced command under the token
/// `::foo::bar` — a token C has never allowed. The last element re-reads the
/// command to prove the refusal left it in place.
///
/// The two directions are asymmetric because C's are: the one-word form spells
/// source and destination alike, so the same word is a *token* when hiding and
/// a *destination* when exposing, and the two report different errors.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   kid eval {namespace eval foo {proc bar {} {}}; namespace eval ns {}}
///   set out {}
///   foreach script {{kid hide ::foo::bar} {kid hide ::set} {kid hide set ::tok}
///                   {kid expose set ns::y} {kid expose ::set}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   lappend out [kid eval {info commands ::foo::bar}]
#[test]
fn child_hide_and_expose_enforce_the_structural_checks() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              kid eval {namespace eval foo {proc bar {} {}}; namespace eval ns {}}
              set out {}
              foreach script {
                  {kid hide ::foo::bar}
                  {kid hide ::set}
                  {kid hide set ::tok}
                  {kid expose set ns::y}
                  {kid expose ::set}
              } {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              lappend out [kid eval {info commands ::foo::bar}]
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{cannot use namespace qualifiers in hidden command token (rename)} \
          {TCL VALUE HIDDENTOKEN} \
          {cannot use namespace qualifiers in hidden command token (rename)} \
          {TCL VALUE HIDDENTOKEN} \
          {cannot use namespace qualifiers in hidden command token (rename)} \
          {TCL VALUE HIDDENTOKEN} \
          {cannot expose to a namespace (use expose to toplevel, then rename)} \
          {TCL EXPOSE NON_GLOBAL} \
          {cannot expose to a namespace (use expose to toplevel, then rename)} \
          {TCL EXPOSE NON_GLOBAL} ::foo::bar"
            .as_slice()
    );
}

/// R2b / R2c: `Tcl_ExposeCommand`'s check order is observable when more than
/// one check applies. It tests the *destination* for `::` first, then looks the
/// token up, then refuses an occupied destination — and it has **no**
/// token-qualifier check at all, so a qualified token is simply a token that is
/// not in the hidden table. Its destination test is a raw `strstr`, so a
/// leading `::` fails it too.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid; interp hide kid list tok
///   set out {}
///   foreach script {{interp expose kid ::tok plain} {interp expose kid tok ::plain}
///                   {interp expose kid nosuchtok set}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   => {unknown hidden command "::tok"} {TCL LOOKUP HIDDENTOKEN ::tok} …
#[test]
fn expose_check_order_matches_c() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              interp hide kid list tok
              set out {}
              foreach script {
                  {interp expose kid ::tok plain}
                  {interp expose kid tok ::plain}
                  {interp expose kid nosuchtok set}
              } {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{unknown hidden command \"::tok\"} {TCL LOOKUP HIDDENTOKEN ::tok} \
          {cannot expose to a namespace (use expose to toplevel, then rename)} \
          {TCL EXPOSE NON_GLOBAL} \
          {unknown hidden command \"nosuchtok\"} \
          {TCL LOOKUP HIDDENTOKEN nosuchtok}"
            .as_slice()
    );
}

/// R2d / R2e: `Tcl_HideCommand`'s order is the mirror question — token
/// qualifiers, *then* resolve the source, *then* reject a non-global source,
/// *then* refuse an occupied token. The runtime used to test non-global before
/// existence and occupancy before both, so a missing source could be reported
/// as either of the other two.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   kid eval {namespace eval ns {proc x {} {}}}
///   interp hide kid list existingtok
///   set out {}
///   foreach script {{interp hide kid ns::nosuch tok} {interp hide kid nosuch existingtok}
///                   {interp hide kid ns::x tok} {interp hide kid set existingtok}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   => {unknown command "ns::nosuch"} {TCL LOOKUP COMMAND ns::nosuch} …
#[test]
fn hide_check_order_matches_c() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              kid eval {namespace eval ns {proc x {} {}}}
              interp hide kid list existingtok
              set out {}
              foreach script {
                  {interp hide kid ns::nosuch tok}
                  {interp hide kid nosuch existingtok}
                  {interp hide kid ns::x tok}
                  {interp hide kid set existingtok}
              } {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{unknown command \"ns::nosuch\"} {TCL LOOKUP COMMAND ns::nosuch} \
          {unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
          {can only hide global namespace commands (use rename then hide)} \
          {TCL HIDE NON_GLOBAL} \
          {hidden command named \"existingtok\" already exists} \
          {TCL HIDE ALREADY_HIDDEN}"
            .as_slice()
    );
}

/// R3: `$child invokehidden` parses its options. The arm used to take
/// `argv[2]` as the command word unconditionally, so `-global` was looked up as
/// a hidden command name. `-global` and `-namespace ns` both switch the child's
/// evaluation context for the one call; passing both is legal and the **last**
/// one wins (there is no mutual-exclusion error in C — see
/// `invokehidden_rejects_unknown_options_and_namespace_is_global_anchored`).
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid; interp hide kid set
///   set out {}
///   lappend out [kid invokehidden -global set g 9] [kid eval {info exists ::g}]
///   lappend out [kid invokehidden -namespace foo set q 1] [kid eval {info exists ::foo::q}]
///   lappend out [kid invokehidden -global -namespace foo set r 2] \
///               [kid eval {list [info exists ::r] [info exists ::foo::r]}]
///   lappend out [kid invokehidden -- set w 4]
///   foreach script {{kid invokehidden -bogus set x 1} {kid invokehidden}
///                   {kid invokehidden -namespace}} { catch $script m; lappend out $m }
///   => 9 1 1 1 2 {0 1} 4 {bad option "-bogus": must be -global, -namespace, or --} …
#[test]
fn child_invokehidden_honours_global_and_namespace_and_rejects_bad_options() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              interp hide kid set
              set out {}
              lappend out [kid invokehidden -global set g 9] [kid eval {info exists ::g}]
              lappend out [kid invokehidden -namespace foo set q 1] \
                          [kid eval {info exists ::foo::q}]
              lappend out [kid invokehidden -global -namespace foo set r 2] \
                          [kid eval {list [info exists ::r] [info exists ::foo::r]}]
              lappend out [kid invokehidden -- set w 4]
              foreach script {
                  {kid invokehidden -bogus set x 1}
                  {kid invokehidden}
                  {kid invokehidden -namespace}
              } {
                  catch $script m
                  lappend out $m
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"9 1 1 1 2 {0 1} 4 \
          {bad option \"-bogus\": must be -global, -namespace, or --} \
          {wrong # args: should be \"kid invokehidden ?-namespace ns? ?-global? \
          ?--? cmd ?arg ..?\"} \
          {wrong # args: should be \"kid invokehidden ?-namespace ns? ?-global? \
          ?--? cmd ?arg ..?\"}"
            .as_slice()
    );
}

/// R4: every arity message names the right noun, and the lenient arms count
/// their words. `kid` alone said `"interp cmd ?arg ...?"`; `kid eval` with no
/// script evaluated the empty string; `kid hidden extra` and its peers accepted
/// the surplus word silently.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   set out {}
///   foreach script {{kid} {kid eval} {kid hidden extra} {kid aliases extra}
///                   {kid issafe extra} {interp hidden kid extra}
///                   {interp issafe kid extra}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   => {wrong # args: should be "kid cmd ?arg ...?"} {TCL WRONGARGS} …
#[test]
fn child_and_interp_arity_errors_use_the_right_noun() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              set out {}
              foreach script {
                  {kid}
                  {kid eval}
                  {kid hidden extra}
                  {kid aliases extra}
                  {kid issafe extra}
                  {interp hidden kid extra}
                  {interp issafe kid extra}
              } {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{wrong # args: should be \"kid cmd ?arg ...?\"} {TCL WRONGARGS} \
          {wrong # args: should be \"kid eval arg ?arg ...?\"} {TCL WRONGARGS} \
          {wrong # args: should be \"kid hidden\"} {TCL WRONGARGS} \
          {wrong # args: should be \"kid aliases\"} {TCL WRONGARGS} \
          {wrong # args: should be \"kid issafe\"} {TCL WRONGARGS} \
          {wrong # args: should be \"interp hidden ?path?\"} {TCL WRONGARGS} \
          {wrong # args: should be \"interp issafe ?path?\"} {TCL WRONGARGS}"
            .as_slice()
    );
}

/// R6: a `rename` miss carries C's structured error code. The message was
/// already right (item 4); only the `-errorcode` was `NONE`, because the miss
/// went through `Interp::error` rather than `error_with_code`.
///
/// tclsh9.0.4 (and 8.6.16):
///   set out {}
///   foreach script {{rename nosuch {}} {rename nosuch x}} {
///       catch $script m o; lappend out $m [dict get $o -errorcode]
///   }
///   => {can't delete "nosuch": command doesn't exist} {TCL LOOKUP COMMAND nosuch}
///      {can't rename "nosuch": command doesn't exist} {TCL LOOKUP COMMAND nosuch}
#[test]
fn rename_miss_carries_the_lookup_errorcode() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"set out {}
              foreach script {{rename nosuch {}} {rename nosuch x}} {
                  catch $script m o
                  lappend out $m [dict get $o -errorcode]
              }
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{can't delete \"nosuch\": command doesn't exist} \
          {TCL LOOKUP COMMAND nosuch} \
          {can't rename \"nosuch\": command doesn't exist} \
          {TCL LOOKUP COMMAND nosuch}"
            .as_slice()
    );
}

/// The `$child` arity errors name the word the *call* used, not the child's
/// table key. C builds every one of them with `Tcl_WrongNumArgs(interp, 1,
/// objv, …)`, so the noun is `objv[0]`; the runtime was passing the name
/// stored in `Command::ChildInterp`, which stops matching as soon as the
/// command is reached under any other spelling.
///
/// A test that only ever calls the child `kid` cannot tell the two apart —
/// which is exactly how this survived the first pass — so this one renames the
/// child first, and checks the qualified spelling too. Every arm that takes an
/// arity check is covered, including `recursionlimit` / `bgerror` / `limit`,
/// which carried the same defect before the shorthand work.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   rename kid foo
///   set out {}
///   foreach script {{foo} {foo eval} {foo issafe extra} {foo hidden extra}
///                   {foo aliases extra} {foo hide} {foo expose a b c}
///                   {foo invokehidden} {foo recursionlimit a b} {foo bgerror a b}
///                   {foo debug a b c} {foo limit} {::foo hidden extra}} {
///       catch $script m; lappend out $m
///   }
///   lappend out [foo eval {set x renamed-child-still-works}]
///   => {wrong # args: should be "foo cmd ?arg ...?"} … {wrong # args: should be
///      "::foo hidden"} renamed-child-still-works
#[test]
fn child_arity_errors_name_the_word_the_call_used() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              rename kid foo
              set out {}
              foreach script {
                  {foo}
                  {foo eval}
                  {foo issafe extra}
                  {foo hidden extra}
                  {foo aliases extra}
                  {foo hide}
                  {foo expose a b c}
                  {foo invokehidden}
                  {foo recursionlimit a b}
                  {foo bgerror a b}
                  {foo debug a b c}
                  {foo limit}
                  {::foo hidden extra}
              } {
                  catch $script m
                  lappend out $m
              }
              lappend out [foo eval {set x renamed-child-still-works}]
              set out"
        ),
        Code::Ok
    );
    assert_eq!(
        interp.result_bytes(),
        b"{wrong # args: should be \"foo cmd ?arg ...?\"} \
          {wrong # args: should be \"foo eval arg ?arg ...?\"} \
          {wrong # args: should be \"foo issafe\"} \
          {wrong # args: should be \"foo hidden\"} \
          {wrong # args: should be \"foo aliases\"} \
          {wrong # args: should be \"foo hide cmdName ?hiddenCmdName?\"} \
          {wrong # args: should be \"foo expose hiddenCmdName ?cmdName?\"} \
          {wrong # args: should be \"foo invokehidden ?-namespace ns? ?-global? \
          ?--? cmd ?arg ..?\"} \
          {wrong # args: should be \"foo recursionlimit ?newlimit?\"} \
          {wrong # args: should be \"foo bgerror ?cmdPrefix?\"} \
          {wrong # args: should be \"foo debug ?-frame ?bool??\"} \
          {wrong # args: should be \"foo limit limitType ?-option value ...?\"} \
          {wrong # args: should be \"::foo hidden\"} renamed-child-still-works"
            .as_slice()
    );
}

/// The seam where the two mechanisms meet: the child's option word is
/// resolved through the shared `OptionTable` owner (so it abbreviates), while
/// the arity noun is the word the *call* used. Each half answers a different
/// question — "which subcommand is this?" and "what was this command called?"
/// — and a call that abbreviates a subcommand on a renamed child needs both
/// answers at once, which is why neither test above can see this case.
///
/// The subcommand half of the noun is the *canonical* word, not the
/// abbreviation: `Tcl_WrongNumArgs` expands an index-typed argument back to
/// its table entry (`tclIndexObj.c`), and `Tcl_GetIndexFromObj` has already
/// retyped `objv[1]` by the time C reaches the arity check. So the command
/// word is echoed as written and the subcommand word as tabled.
///
/// tclsh9.0.4 (and 8.6.16):
///   interp create kid
///   rename kid foo
///   set out {}
///   foreach script {{foo hidd extra} {foo alias} {foo ex} {foo hid} {::foo ev}
///                   {foo invo} {foo debu -frame 1 x} {foo recur 1 2} {foo {}}
///                   {foo nosuch}} {
///       catch $script m; lappend out $m
///   }
///   lappend out [foo ev {set x still-works}]
///   => {wrong # args: should be "foo hidden"} … {ambiguous option ""…} …
///      {bad option "nosuch"…} still-works
#[test]
fn child_abbreviations_resolve_while_arity_errors_name_the_written_word() {
    let mut interp = Interp::new();
    assert_eq!(
        interp.eval_str(
            b"interp create kid
              rename kid foo
              set out {}
              foreach script {
                  {foo hidd extra}
                  {foo alias}
                  {foo ex}
                  {foo hid}
                  {::foo ev}
                  {foo invo}
                  {foo debu -frame 1 x}
                  {foo recur 1 2}
                  {foo {}}
                  {foo nosuch}
              } {
                  catch $script m
                  lappend out $m
              }
              lappend out [foo ev {set x still-works}]
              set out"
        ),
        Code::Ok
    );
    let options = "alias, aliases, bgerror, debug, eval, expose, hide, hidden, \
issafe, invokehidden, limit, marktrusted, or recursionlimit";
    assert_eq!(
        String::from_utf8_lossy(&interp.result_bytes()),
        format!(
            "{{wrong # args: should be \"foo hidden\"}} \
{{wrong # args: should be \"foo alias aliasName ?targetName? ?arg ...?\"}} \
{{wrong # args: should be \"foo expose hiddenCmdName ?cmdName?\"}} \
{{ambiguous option \"hid\": must be {options}}} \
{{wrong # args: should be \"::foo eval arg ?arg ...?\"}} \
{{wrong # args: should be \"foo invokehidden ?-namespace ns? ?-global? ?--? cmd ?arg ..?\"}} \
{{wrong # args: should be \"foo debug ?-frame ?bool??\"}} \
{{wrong # args: should be \"foo recursionlimit ?newlimit?\"}} \
{{ambiguous option \"\": must be {options}}} \
{{bad option \"nosuch\": must be {options}}} still-works"
        )
    );
}
