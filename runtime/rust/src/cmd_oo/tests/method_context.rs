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

//! Commands inside a method: `self`, `my`, `next`, `nextto` and `classvariable`.

use super::*;

#[test]
fn class_methods_via_self() {
    leak_free(|i| {
        // `oo::define … self method` defines a method on the class object.
        ok(i, b"oo::class create C");
        ok(
            i,
            b"oo::define C self method greet {} {return \"hi from [self]\"}",
        );
        assert_eq!(ok(i, b"C greet"), b"hi from ::C");
        // `self method` inside the class body, and `self` alone returns it.
        assert_eq!(
            ok(
                i,
                b"oo::class create D {self method who {} {return [self]}}; D who"
            ),
            b"::D"
        );
        // Instance methods still resolve (the class-as-object regression guard).
        ok(i, b"oo::class create E {method m {} {return inst}}");
        assert_eq!(ok(i, b"E create e; e m"), b"inst");
        i.eval_str(b"C destroy; D destroy; E destroy");
    });
}

#[test]
fn my_command_invoked_directly() {
    leak_free(|i| {
        // The per-object `my`, invoked directly (not from a method), still
        // dispatches on its own object (oo-16.13).
        ok(i, b"oo::object create foo");
        ok(i, b"oo::objdefine foo method Bar {} {return {ok in foo}}");
        assert_eq!(ok(i, b"[info object namespace foo]::my Bar"), b"ok in foo");
    });
}

#[test]
fn nextto_resumes_chain_at_class() {
    leak_free(|i| {
        ok(i, b"oo::class create root");
        ok(
            i,
            b"oo::class create A { superclass root; method x args {lappend ::r ==A==} }",
        );
        ok(
            i,
            b"oo::class create B { superclass A; method x args {lappend ::r ==B==; nextto A} }",
        );
        ok(
            i,
            b"oo::class create C { superclass A; method x args {lappend ::r ==C==} }",
        );
        ok(
            i,
            b"oo::class create D { superclass B C; method x args {lappend ::r ==D==; next} }",
        );
        ok(i, b"set ::r {}; [D new] x");
        // D -> B (next) -> B does `nextto A`, skipping C, to A.
        assert_eq!(ok(i, b"set ::r"), b"==D== ==B== ==A==");
        // nextto to an unrelated/behind class errors.
        ok(i, b"oo::class create Z { superclass root }");
        ok(
            i,
            b"oo::class create E { superclass root; method x {} {nextto Z} }",
        );
        ok(i, b"E create e");
        assert_eq!(i.eval_str(b"e x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"method has no non-filter implementation by \"Z\""
        );
    });
}

#[test]
fn class_method_constructor_instance_var() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create Animal {
                    variable sound
                    constructor {s} { set sound $s }
                    method speak {} { return \"I say $sound\" }
                    method describe {} { return \"[self] speaks\" }
                }",
        );
        ok(i, b"set a [Animal new woof]");
        assert_eq!(ok(i, b"$a speak"), b"I say woof");
        ok(i, b"Animal create ::bessie moo");
        assert_eq!(ok(i, b"bessie speak"), b"I say moo");
        ok(i, b"bessie destroy");
        assert_eq!(i.eval_str(b"bessie speak"), Code::Error);
    });
}

#[test]
fn classvariable_shares_state_per_declaring_class() {
    leak_free(|i| {
        // A class variable is shared across all instances of the class, and
        // lives in the class object's namespace.
        ok(
            i,
            b"oo::class create Counter {
                    constructor {} { classvariable count; incr count }
                    method count {} { classvariable count; return $count }
                }",
        );
        ok(i, b"Counter create a; Counter create b; Counter create c");
        assert_eq!(ok(i, b"a count"), b"3");
        assert_eq!(ok(i, b"b count"), b"3");
        // It really lives in the class's own namespace.
        assert_eq!(ok(i, b"set [info object namespace Counter]::count"), b"3");
        // A method reached via `next` links to *its own* declaring class, not
        // the leaf object's — so Base and Derived keep separate stores.
        ok(
            i,
            b"oo::class create Base {
                    method tag {v} { classvariable t; set t $v }
                    method get {} { classvariable t; return $t }
                }",
        );
        ok(
            i,
            b"oo::class create Sub {
                    superclass Base
                    method tag {v} { classvariable t; set t sub-$v; next $v }
                }",
        );
        ok(i, b"Sub create s; s tag hi");
        assert_eq!(ok(i, b"set [info object namespace Base]::t"), b"hi");
        assert_eq!(ok(i, b"set [info object namespace Sub]::t"), b"sub-hi");
        assert_eq!(ok(i, b"s get"), b"hi");
    });
}

#[test]
fn classvariable_context_and_name_errors() {
    leak_free(|i| {
        // Outside a method it is unusable (helpers are call-stack-gated).
        assert_eq!(i.eval_str(b"classvariable foo"), Code::Error);
        assert!(contains(
            &i.result_bytes(),
            b"classvariable may only be called from inside a method"
        ));
        // A per-object / class-side method has no class namespace to share.
        ok(
            i,
            b"oo::object create solo; oo::objdefine solo {method m {} {classvariable x}}",
        );
        assert_eq!(i.eval_str(b"solo m"), Code::Error);
        assert_eq!(i.result_bytes(), b"method not defined by a class");
        // Name validation mirrors C: array look-alike, then namespace sep.
        ok(
            i,
            b"oo::class create C {
                    method arr {} { classvariable a(b) }
                    method ns {} { classvariable ::foo }
                    method none {} { classvariable }
                }",
        );
        ok(i, b"C create o");
        assert_eq!(i.eval_str(b"o arr"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"bad variable name \"a(b)\": can't create a scalar variable that looks like an array element"
            );
        assert_eq!(i.eval_str(b"o ns"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"bad variable name \"::foo\": can't create a local variable with a namespace separator in it"
            );
        assert_eq!(i.eval_str(b"o none"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"classvariable name ...\""
        );
    });
}

#[test]
fn inheritance_and_next() {
    leak_free(|i| {
        ok(i, b"oo::class create Animal { variable s; constructor {x} {set s $x}; method speak {} {return \"I say $s\"} }");
        ok(i, b"oo::class create Dog { superclass Animal; constructor {} {next bark}; method speak {} {return \"Dog: [next]\"} }");
        ok(i, b"set d [Dog new]");
        assert_eq!(ok(i, b"$d speak"), b"Dog: I say bark");
        ok(i, b"oo::define Dog method legs {} { return 4 }");
        assert_eq!(ok(i, b"$d legs"), b"4");
    });
}

#[test]
fn self_class_in_object_method_errors() {
    leak_free(|i| {
        // `self class` in a method declared directly on the object has no
        // declaring class.
        ok(i, b"oo::object create obj");
        ok(i, b"oo::objdefine obj method demo {} { self class }");
        assert_eq!(i.eval_str(b"obj demo"), Code::Error);
        assert_eq!(i.result_bytes(), b"method not defined by a class");
        // A class instance method still reports its declaring class.
        ok(i, b"oo::class create C { method d {} { self class } }");
        assert_eq!(ok(i, b"[C new] d"), b"::C");
    });
}

#[test]
fn next_shares_caller_frame_level() {
    leak_free(|i| {
        // Every method in a `next` chain runs at the level of the original
        // invocation, so `info level` is the same and `upvar 1` from a
        // next-invoked method reaches the original caller's frame.
        ok(
                i,
                b"oo::class create A { method incr {var step} { upvar 1 $var v; ::incr v $step; return [info level] } }",
            );
        ok(
                i,
                b"oo::class create B { superclass A; method incr {var {step 1}} { list [info level] [next $var $step] } }",
            );
        ok(i, b"B create b");
        ok(i, b"set x 10");
        // Both methods report level 1; A's upvar bumps the global x to 11.
        assert_eq!(ok(i, b"b incr x"), b"1 1");
        assert_eq!(ok(i, b"set x"), b"11");
    });
}
