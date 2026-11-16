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

//! A value-position `catch` whose single-command braced body the compiler inlines.

use super::*;

// The inline emitters a proc-shaped body reaches.
//
// These paths are gated on `is_proc`, so until a proc body actually ran with
// the procedure codegen none of them executed through the `proc` command. Every
// expectation below is byte-checked against tclsh 8.6.16 and 9.0.4.

/// A value-position `catch` inlines only a body it can compile — *one* command.
///
/// The inline emitter splits the body into words and emits a single invocation
/// from them, so a two-command body became `set y 1 set z 2` (one `set` with
/// four arguments) and a comment body invoked a command named `#`. Both now
/// fall back to the dispatched `catch`.
#[test]
fn a_value_position_catch_inlines_only_a_single_command_body() {
    assert_eq!(
        run("proc p {} { set c [catch { set y 1; set z 2 } m]; return \"$c/$m\" }\nputs [p]").2,
        "0/2\n"
    );
    assert_eq!(
        run("proc p {} { set c [catch { set y 1\nset z 2 } m]; return \"$c/$m\" }\nputs [p]").2,
        "0/2\n"
    );
    assert_eq!(
        run("proc p {} { set c [catch { # nothing } m]; return \"$c/$m\" }\nputs [p]").2,
        "0/\n"
    );
    // A single command still inlines, and still reports what C reports.
    assert_eq!(
        run("proc p {} { set c [catch { error one } m]; return \"$c/$m\" }\nputs [p]").2,
        "1/one\n"
    );
    // The nested `try … on error` form takes the same rule for both its bodies.
    assert_eq!(
        run(
            "proc p {} { set c [catch { try { set a 1; error e } on error {e} { set b 2 } } m]; \
             return \"$c/$m\" }\nputs [p]"
        )
        .2,
        "0/2\n"
    );
    assert_eq!(
        run(
            "proc p {} { set c [catch { try { error e } on error {e} { set b 2; set d 3 } } m]; \
             return \"$c/$m\" }\nputs [p]"
        )
        .2,
        "0/3\n"
    );
}

/// An inlined catch body's *command name* may itself be a substitution.
///
/// `catch {$handler}` is how an event dispatcher calls a handler it looked up;
/// pushing the name as a bare literal invoked a command called `$handler`, and
/// the `catch` swallowed the resulting error, so the handler simply never ran.
#[test]
fn an_inlined_catch_body_substitutes_its_command_name() {
    assert_eq!(
        run("proc bump {} { set ::hit 1; return B }\n\
             proc p {} { set n bump; set c [catch {$n} m]; return \"$c/$m\" }\n\
             puts [p]\nputs [info exists ::hit]")
        .2,
        "0/B\n1\n"
    );
}

/// The inline `catch` needs a body it can *see*: a braced word holding exactly
/// one command, and nothing else.
///
/// Three ways a body defeats the word splitter that the segment count alone
/// does not catch. An unbraced body (`catch $script`) only *names* a script —
/// its commands are not known until run time, so compiling it as one command
/// invoked the whole script value as a command name. A trailing separator
/// (`{error boom;}`) and a leading comment (`{# note\nerror boom}`) each still
/// segment to one command, but the splitter reads `boom;` as an argument and
/// `#` as a command name. All three take the dispatched `catch`.
#[test]
fn the_inline_catch_needs_a_braced_body_that_is_only_one_command() {
    // A dynamic body, single- and multi-command alike.
    assert_eq!(
        run(
            "proc p {} { set s {set x 1; set x 2}; set c [catch $s m]; return \"$c/$m\" }\n\
             puts [p]"
        )
        .2,
        "0/2\n"
    );
    assert_eq!(
        run("proc p {} { set s {set x 9}; set c [catch $s m]; return \"$c/$m\" }\nputs [p]").2,
        "0/9\n"
    );
    // A trailing `;` is a separator, not part of the argument.
    assert_eq!(
        run("proc p {} { set c [catch {error boom;} m]; return \"$c/$m\" }\nputs [p]").2,
        "1/boom\n"
    );
    // A leading comment is not the command.
    assert_eq!(
        run("proc p {} { set c [catch {# note\nerror boom} m]; return \"$c/$m\" }\nputs [p]").2,
        "1/boom\n"
    );
}

/// A `{*}`-expanded word inside an inlined `catch` body still expands.
///
/// The single-command emitters split with `parse_cmd_parts`, which has no
/// expansion form and reads the `{*}` prefix as a braced word — so
/// `catch {cfg {*}$a}` called `cfg * {-verbose b}`. That is how `tcltest`'s
/// option parser came to report `ambiguous option *`.
#[test]
fn an_inlined_catch_body_expands_a_star_word() {
    assert_eq!(
        run("proc cfg {args} { return \"[llength $args]:$args\" }\n\
             proc p {} { set a {-verbose b}; set c [catch {cfg {*}$a} m]; return \"$c/$m\" }\n\
             puts [p]")
        .2,
        "0/2:-verbose b\n"
    );
    assert_eq!(
        run("proc cfg {args} { return \"[llength $args]:$args\" }\n\
             proc p {} { set a {x y}; set c [catch {cfg {*}$a extra} m]; return \"$c/$m\" }\n\
             puts [p]")
        .2,
        "0/3:x y extra\n"
    );
}

/// The body word reaching the inline emitter is already the `catch` argument's
/// *value*, so the braces still in it belong to the script.
///
/// `[catch {{set x 1}} m]` runs the one-word command `set x 1` and catches
/// `invalid command name`; stripping a second brace layer compiled a
/// successful `set` instead.
#[test]
fn an_inlined_catch_body_keeps_the_braces_that_belong_to_its_script() {
    assert_eq!(
        run("proc p {} { set c [catch {{set x 1}} m]; return \"$c/$m\" }\nputs [p]").2,
        "1/invalid command name \"set x 1\"\n"
    );
}

/// A braced word inside an inlined command substitution is already the finished
/// value, so it is pushed verbatim: the VM's word substitution would otherwise
/// read a value that merely looks like a braced word as one and strip a brace
/// layer that was never there.
#[test]
fn a_braced_argument_of_an_inlined_command_substitution_keeps_its_braces() {
    assert_eq!(
        run("proc p {} { return \"[string index {{}} 0][string length {{}}]\" }\nputs [p]").2,
        "{2\n"
    );
}

/// A `return`/`set` value that merely *begins and ends* with a bracket is not
/// one command substitution: `[llength $a]:[join $a ,]` is three concatenated
/// parts. Treating it as one mangled it into a single bogus invocation.
#[test]
fn a_value_that_is_not_one_whole_command_substitution_still_concatenates() {
    assert_eq!(
        run("proc p {args} { return [llength $args]:[join $args ,] }\nputs [p x y]").2,
        "2:x,y\n"
    );
    assert_eq!(
        run("proc p {args} { set r [llength $args]:[join $args ,]; return $r }\nputs [p x y]").2,
        "2:x,y\n"
    );
    // The genuine whole-word form is unaffected.
    assert_eq!(
        run("proc p {args} { return [llength $args] }\nputs [p x y]").2,
        "2\n"
    );
}
