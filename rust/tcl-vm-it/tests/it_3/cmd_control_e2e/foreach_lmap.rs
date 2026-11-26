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

//! The `foreach` and `lmap` runtime fallbacks (`cmd_control.rs`): list words, arity, control flow and error frames.

use super::*;

// foreach / lmap  (cmd_control.rs)

/// The runtime `foreach` fallback: single var, multiple vars per group, and
/// multiple groups (short groups pad with empty).
#[test]
fn foreach_runtime_variants() {
    // single var
    // tclsh stdout: "1\n2\n3\n"
    assert_eq!(run("set c foreach; $c x {1 2 3} {puts $x}").2, "1\n2\n3\n");
    // two vars per group consume the list two at a time
    // tclsh stdout: "1-2\n3-4\n"
    assert_eq!(
        run("set c foreach; $c {a b} {1 2 3 4} {puts \"$a-$b\"}").2,
        "1-2\n3-4\n",
    );
    // two groups, second shorter -> empty pad in the last iteration
    // tclsh stdout: "1a\n2b\n3\n"
    assert_eq!(
        run("set c foreach; $c x {1 2 3} y {a b} {puts \"$x$y\"}").2,
        "1a\n2b\n3\n",
    );
}

/// A **braced** list word is a literal in every direction.
/// `TclFindElement`'s brace semantics keep `$` and `[…]` inert both for the
/// word itself and for any braced element inside it, so the compiled
/// `foreach` header must push it verbatim. Pushing an ordinary literal
/// instead, with the VM's `subst_word` running the substitution at loop
/// entry, would raise `invalid command name "b"` for `foreach e {{a[b]c} x}`.
///
/// Every expectation below is byte-exact against real tclsh 8.6.16 and 9.0.4
/// (the answers are identical at both, the rule is unchanged across the
/// releases).
#[test]
fn braced_foreach_list_word_is_never_substituted() {
    // A braced *element* inside the braced list word.
    assert_eq!(run("foreach e {{a[b]c} x} { puts $e }").2, "a[b]c\nx\n");
    // A bare `[…]` directly in the braced list word is
    // equally inert — braces protect the whole word, not just its elements.
    assert_eq!(run("foreach e {a[b]c x} { puts $e }").2, "a[b]c\nx\n");
    // `$` and a literal backslash sequence likewise.
    assert_eq!(run("foreach e {{a$b c} x} { puts $e }").2, "a$b c\nx\n");
    // A *braced element* keeps its backslash literal…
    assert_eq!(run(r"foreach e {{a\nb} x} { puts $e }").2, "a\\nb\nx\n");
    // …while a bare element still takes `TclFindElement`'s backslash
    // substitution, which the verbatim push must not suppress.
    assert_eq!(run(r"foreach e {a\nb x} { puts $e }").2, "a\nb\nx\n");
    // Multiple variables per group, and `lmap`, take the same header path.
    assert_eq!(
        run("foreach {a b} {{p[q]r} 2 s 4} { puts \"$a=$b\" }").2,
        "p[q]r=2\ns=4\n",
    );
    assert_eq!(
        run("puts [lmap e {{a[b]c} x} { string toupper $e }]").2,
        "{A[B]C} X\n",
    );
    // Inside a proc body (the LVT path) as well as at top level.
    assert_eq!(
        run("proc f {} { foreach e {{a[b]c} x} { puts $e } }\nf").2,
        "a[b]c\nx\n",
    );
}

/// The other half of that contract: a list word that is *not* braced must
/// still substitute. Flipping the new guard on unconditionally would silence
/// the bug by breaking these, so they are asserted beside it.
#[test]
fn unbraced_foreach_list_word_still_substitutes() {
    // Quoted word: `$b` interpolates.
    assert_eq!(
        run("set b B\nforeach e \"a $b c\" { puts $e }").2,
        "a\nB\nc\n"
    );
    // Bare `$L`: the value is iterated, not the spelling.
    assert_eq!(run("set L \"p q\"\nforeach e $L { puts $e }").2, "p\nq\n");
    // A command substitution builds the list.
    assert_eq!(
        run("proc b {} { return B }\nforeach e [list x [b]] { puts $e }").2,
        "x\nB\n",
    );
    // Mixed iterators: the braced group stays literal, the quoted one does not.
    assert_eq!(
        run("set b B\nforeach i {{p[q]r}} j \"s $b\" { puts \"$i/$j\" }").2,
        "p[q]r/s\n/B\n",
    );
}

/// A whole-word array element is a dynamic list source, including when its
/// index contains substitutions.  The header must evaluate that word at loop
/// entry instead of iterating its source spelling.  These outputs match
/// Tcl 9.0.4.
#[test]
fn foreach_dynamic_array_element_list_source_matches_tcl9() {
    // The package.tcl shape that exposed #2194: the index is composed from
    // the current loop variable.
    assert_eq!(
        run("proc p {} {\n\
                 array set opts {-load {} -source foo}\n\
                 set out {}\n\
                 foreach key {load source} {\n\
                     foreach filespec $opts(-$key) {\n\
                         lappend out [list $key $filespec]\n\
                     }\n\
                 }\n\
                 return $out\n\
             }\n\
             p",)
        .1,
        "{source foo}",
    );

    // A dynamic array source also works in a nested loop.
    assert_eq!(
        run("array set a {x {one two} y {three}}\n\
             set out {}\n\
             foreach key {x y} {\n\
                 foreach item $a($key) { lappend out [list $key $item] }\n\
             }\n\
             set out",)
        .1,
        "{x one} {x two} {y three}",
    );

    // Braced list controls remain literal, even when the same spelling would
    // be a variable reference in an unbraced word.
    assert_eq!(
        run("set x {a $b [bad]}\n\
             set out {}\n\
             foreach item {$x} { lappend out $item }\n\
             set out",)
        .1,
        "{$x}",
    );

    // An index command runs once while resolving the list source.  This
    // guards the left-to-right substitution order in the canonical array
    // reference emitter.
    assert_eq!(
        run(
            "proc nextKey {name counterVar} { upvar 1 $counterVar n; incr n; return $name }\n\
             array set a {x {one two}}\n\
             set n 0\n\
             set out {}\n\
             foreach item $a([nextKey x n]) { lappend out $item }\n\
             list $n $out",
        )
        .1,
        "1 {one two}",
    );
}

/// A direct nested iterator routes the outer literal `foreach` through the
/// runtime command boundary. That gives the inner loop a fresh activation;
/// inlining both loops into one CFG would leave only the final outer item.
#[test]
fn nested_foreach_preserves_every_outer_iteration() {
    // tclsh 8.6 / 9.0: `a:HTTP TCP|b:HTTP TCP|c:HTTP TCP`
    assert_eq!(
        run("set result {}; \
             foreach outer {a b c} { \
                 foreach {key value} {profiles {HTTP TCP} steps ignored} { \
                     if {$key eq \"profiles\"} { lappend result \"$outer:$value\" } \
                 } \
             }; \
             join $result {|}",)
        .1,
        "a:HTTP TCP|b:HTTP TCP|c:HTTP TCP",
    );
}

/// `foreach`/`lmap` arity: name-stripped argv must be an odd count >= 3.
#[test]
fn foreach_lmap_runtime_arity() {
    // tclsh: `wrong # args: should be "foreach varList list ?varList list ...? command"`
    assert_eq!(
        run("set c foreach; catch {$c} m; set m").1,
        "wrong # args: should be \"foreach varList list ?varList list ...? command\"",
    );
    // even arg count
    assert_eq!(
        run("set c foreach; catch {$c a b c d} m; set m").1,
        "wrong # args: should be \"foreach varList list ?varList list ...? command\"",
    );
    // tclsh: `wrong # args: should be "lmap varList list ?varList list ...? command"`
    assert_eq!(
        run("set c lmap; catch {$c x y} m; set m").1,
        "wrong # args: should be \"lmap varList list ?varList list ...? command\"",
    );
}

/// An empty var-list for a group errors with the command name.
#[test]
fn foreach_lmap_runtime_empty_varlist() {
    // tclsh: `foreach varlist is empty`
    assert_eq!(
        run("set c foreach; catch {$c {} {1 2} {body}} m; set m").1,
        "foreach varlist is empty",
    );
    // tclsh: `lmap varlist is empty`
    assert_eq!(
        run("set c lmap; catch {$c {} {1 2} {body}} m; set m").1,
        "lmap varlist is empty",
    );
}

/// `lmap` collects each non-`continue` body result; `continue` skips collection;
/// `break` ends the loop.
#[test]
fn lmap_runtime_collects() {
    // tclsh: `2 4 6`
    assert_eq!(run("set c lmap; $c x {1 2 3} {expr {$x*2}}").1, "2 4 6");
    // continue skips the element.
    // tclsh: `2 6`
    assert_eq!(
        run("set c lmap; $c x {1 2 3} {if {$x==2} continue; expr {$x*2}}").1,
        "2 6",
    );
    // break ends collection early.
    // tclsh: `2 4`
    assert_eq!(
        run("set c lmap; $c x {1 2 3 4 5} {if {$x>2} break; expr {$x*2}}").1,
        "2 4",
    );
}

/// `break` in the runtime `foreach` body.
#[test]
fn foreach_runtime_break() {
    // tclsh: `1 2`
    assert_eq!(
        run("set c foreach; set r {}; $c x {1 2 3 4} {if {$x==3} break; lappend r $x}; set r").1,
        "1 2",
    );
}

/// A `return` out of the runtime `foreach` body propagates.
#[test]
fn foreach_runtime_return_propagates() {
    // tclsh: `early`
    assert_eq!(
        run("proc p {} { set c foreach; $c x {1 2} { return early } }; p").1,
        "early",
    );
}

/// An error in the runtime `foreach`/`lmap` body adds the body frame.
#[test]
fn foreach_runtime_body_error_adds_frame() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c foreach\n",
            "catch {$c x {1 2 3} {error boom}}\n",
            "string match {boom*(\"foreach\" body line 1)*} $::errorInfo"
        ))
        .1,
        "1",
    );
}

/// The runtime loop must log its invoking command after the body frame even
/// when `error` supplies explicit errorInfo (Tcl 9.0.4).
#[test]
fn foreach_lmap_runtime_explicit_error_info_logs_invocation() {
    for name in ["foreach", "lmap"] {
        let script = format!("set c {name}; catch {{$c x 1 {{error direct EI}}}}; set ::errorInfo");
        let expected = format!(
            "EI\n    (\"{name}\" body line 1)\n    invoked from within\n\"$c x 1 {{error direct EI}}\""
        );
        assert_eq!(run(&script).1, expected, "{name}");
    }
}

#[test]
fn tailcalled_each_loop_retains_its_entered_error_invocation() {
    // Exact errorInfo from Tcl 9.0.4, including the tailcall's replacement
    // invocation rather than the removed procedure's source instruction.
    for name in ["foreach", "lmap"] {
        let source = format!(
            "proc p {{}} {{set c {name}; tailcall $c x 1 {{error direct EI}}}}; catch {{p}}; set ::errorInfo"
        );
        let expected = format!(
            "EI\n    (\"{name}\" body line 1)\n    invoked from within\n\"{name} x 1 {{error direct EI}}\"\n    invoked from within\n\"p\""
        );
        assert_eq!(run(&source).1, expected, "{name}");
    }
}

#[test]
fn foreach_in_package_ifneeded_logs_invocation_before_loader_frame() {
    assert_eq!(
        run(concat!(
            "package ifneeded badloop 1.0 {set c foreach; $c x 1 {error direct EI}}; ",
            "catch {package require badloop}; set ::errorInfo"
        ))
        .1,
        concat!(
            "EI\n    (\"foreach\" body line 1)\n    invoked from within\n",
            "\"$c x 1 {error direct EI}\"\n",
            "    (\"package ifneeded badloop 1.0\" script)\n",
            "    invoked from within\n\"package require badloop\""
        ),
    );
}

/// A malformed list as the value list errors (the `as_list` Err arm).
#[test]
fn foreach_runtime_bad_list_value() {
    // An unbalanced brace in the value list errors.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set c foreach; $c x \"a b \\{\" {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

// cmd_control.rs / cmd_switch.rs — additional error-path coverage

/// A `foreach`/`lmap` loop-variable write that fails propagates the set error
/// (the `set_var` Err arm in `each_loop`). Binding the loop var to a constant
/// (Tcl 9) fails.
#[test]
fn foreach_runtime_var_write_failure() {
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) = run("const c 1; set f foreach; $f c {1 2 3} {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
}

/// A malformed list as a `foreach` var-list errors (the `pair[0].as_list()` Err
/// arm in `each_loop`).
#[test]
fn foreach_runtime_bad_varlist_list() {
    // An unbalanced brace makes the var-list an invalid list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set f foreach; $f \"a \\{\" {1 2} {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}
