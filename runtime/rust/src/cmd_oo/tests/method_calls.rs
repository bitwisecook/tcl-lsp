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

//! Calling a method: filters, unknown handlers, private scope, the built-in `variable`/`varname`/`eval` methods, and the frames a method body reports.

use super::*;

#[test]
fn filters_wrap_method_calls() {
    leak_free(|i| {
        ok(i, b"set ::r {}");
        ok(i, b"oo::class create C");
        ok(
            i,
            b"oo::define C method foo args {global r; lappend r foo; return done}",
        );
        ok(i, b"oo::define C method log args {global r; lappend r [self method]; return [next {*}$args]}");
        ok(i, b"oo::define C filter log");
        ok(i, b"C create o");
        // The filter runs first (self method = the filter name), then `next`
        // reaches the real method.
        assert_eq!(ok(i, b"o foo"), b"done");
        assert_eq!(ok(i, b"set ::r"), b"log foo");
        i.eval_str(b"C destroy; unset -nocomplain ::r");
    });
}

#[test]
fn private_variable_cross_class_mangling() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // clsA/clsB private x, clsC public x — three independent storages
        // (oo-38.1).
        ok(
            i,
            b"oo::class create clsA { superclass parent; private variable x; \
                  constructor {} {set x 1}; method getA {} {return $x} }",
        );
        ok(
            i,
            b"oo::class create clsB { superclass clsA; private variable x; \
                  constructor {} {set x 2; next}; method getB {} {return $x} }",
        );
        ok(
            i,
            b"oo::class create clsC { superclass clsB; variable x; \
                  constructor {} {set x 3; next}; method getC {} {return $x} }",
        );
        ok(i, b"clsC create o");
        assert_eq!(ok(i, b"o getA"), b"1");
        assert_eq!(ok(i, b"o getB"), b"2");
        assert_eq!(ok(i, b"o getC"), b"3");
        // `my varname` of a private var returns its mangled storage (oo-38.3).
        ok(
            i,
            b"oo::class create V { superclass parent; private variable p; \
                  method name {} {my varname p} }",
        );
        ok(i, b"V create v");
        let n = ok(i, b"v name");
        assert!(n.ends_with(b" : p"), "mangled storage, got {n:?}");
    });
}

#[test]
fn my_variable_rejects_array_element() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C { method bar {} { my variable a(b) } }",
        );
        ok(i, b"C create o");
        assert_eq!(i.eval_str(b"o bar"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't define \"a(b)\": name refers to an element in an array"
        );
    });
}

// Needs the numeric tower: `$obj eval` runs `expr`.
#[cfg(have_tommath)]
#[test]
fn object_builtin_variable_varname_eval() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C {
                    method s {v} { my variable x; set x $v }
                    method g {} { my variable x; return $x }
                    method vn {} { my varname x }
                    method e {} { my eval {set y 9; return $y} }
                }",
        );
        ok(i, b"set o [C new]");
        ok(i, b"$o s hi");
        assert_eq!(ok(i, b"$o g"), b"hi");
        let mut variable = ok(i, b"info object namespace $o");
        variable.extend_from_slice(b"::x");
        assert_eq!(ok(i, b"$o vn"), variable);
        assert_eq!(ok(i, b"$o e"), b"9");
        // Built-ins are unexported: an external call is an unknown method.
        assert_eq!(i.eval_str(b"$o variable x"), Code::Error);
        // ... but `export` promotes a built-in to a public method.
        ok(i, b"oo::class create E { export eval }; set e [E new]");
        assert_eq!(ok(i, b"$e eval {expr 3+4}"), b"7");
        // `variable` rejects a namespace-qualified name.
        assert_eq!(
            i.eval_str(b"oo::class create D {method z {} {my variable a::b}}; [D new] z"),
            Code::Error,
        );
    });
}

/// The declaring scope of an unknown-method miss is stable OO identity.
/// Renaming the provider inside its user `unknown` method therefore does
/// not hide that provider's private methods from the final diagnostic.
#[test]
fn unknown_scope_survives_provider_rename() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"oo::class create C {
                            private method secret {} {return SECRET}
                            method probe {} {my missing}
                            method unknown args {
                                rename ::C ::D
                                next {*}$args
                            }
                        }
                        C create o
                        catch {o probe} m
                        list $m [info object class o]"#,
                ),
                br#"{unknown method "missing": must be <cloned>, destroy, eval, probe, secret, unknown, variable or varname} ::D"#,
            );
    });
}

// Needs the numeric tower: method bodies compute via `expr`.
#[cfg(have_tommath)]
#[test]
fn private_method_scope_and_unknown_listing() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // A class's private method is invisible to a different scope's `my`
        // call: the object's private `step` does not shadow the class's
        // public `step` when the caller is a class method (oo-39.4/39.5).
        ok(
            i,
            b"oo::class create clsA { superclass parent; variable x; \
                  constructor {} {set x 1}; method act {} {my step; return}; \
                  method step {} {incr x}; method x {} {return $x} }",
        );
        ok(i, b"clsA create obj; obj act");
        assert_eq!(ok(i, b"obj x"), b"2");
        ok(
            i,
            b"oo::objdefine obj { variable x; private { method step {} {incr x 2} } }",
        );
        ok(i, b"obj act"); // class act -> class step (public), not obj private
        assert_eq!(ok(i, b"obj x"), b"3");
        // The unknown-method error lists in-scope private methods (oo-39.6).
        ok(
            i,
            b"oo::class create cls { superclass parent; variable x; \
                  constructor {val} {set x $val}; private method x {} {return $x}; \
                  method equal {other} {expr {$x == [$other y]}} }",
        );
        ok(i, b"cls create a 1; cls create b 2");
        assert_eq!(i.eval_str(b"a equal b"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown method \"y\": must be destroy, equal or x"
        );
        // An internal (`my`) miss also lists the unexported oo::object
        // built-ins and unexported methods (oo-39.8).
        ok(i, b"oo::class create P { method m {} { my nonesuch } }");
        ok(i, b"P create p");
        assert_eq!(i.eval_str(b"p m"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"unknown method \"nonesuch\": must be <cloned>, destroy, eval, m, unknown, variable or varname"
            );
    });
}

#[test]
fn method_body_errorinfo_frames() {
    leak_free(|i| {
        ok(i, b"oo::class create cls { constructor {} {} }");
        ok(i, b"cls create obj");
        // A per-object method: `(object "::obj" method "bar" line N)`.
        ok(i, b"oo::objdefine obj method bar {} {error foo}");
        assert_eq!(i.eval_str(b"obj bar"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(object \"::obj\" method \"bar\" line 1)"),
            "got {ei:?}"
        );
        // A class method: `(class "::cls" method "cm" line N)`.
        ok(i, b"oo::define cls method cm {} {error boom}");
        assert_eq!(i.eval_str(b"obj cm"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(class \"::cls\" method \"cm\" line 1)"),
            "got {ei:?}"
        );
        // `my eval {…}` adds `(in "my eval" script line N)`.
        ok(i, b"oo::objdefine obj method ev {} {my eval {error e}}");
        assert_eq!(i.eval_str(b"obj ev"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(in \"my eval\" script line 1)"),
            "got {ei:?}"
        );

        // The shared proc runner uses the full external invocation for
        // both `info level 0` and TIP 348's procedure-boundary `CALL`.
        ok(i, b"oo::define cls method stack {arg} {error stacked}");
        ok(i, b"catch {obj stack value} m o");
        assert_eq!(
            ok(i, b"lindex [dict get $o -errorstack] end"),
            b"obj stack value"
        );
        ok(i, b"catch {::obj stack rooted} m o");
        assert_eq!(
            ok(i, b"lindex [dict get $o -errorstack] end"),
            b"::obj stack rooted"
        );
    });
}

#[test]
fn info_frame_method_body_lines_are_relative() {
    leak_free(|i| {
        // A method body defined while sourcing reports file-absolute lines,
        // so a body command's `info frame` line tracks the source — even
        // when the body opens on a later line than the `method` command.
        let script = b"oo::class create C {}\n\
                oo::define C {\n\
                  method a {} {info frame 0}\n\
                  method b {\n\
                  } {info frame 0}\n\
                }\n\
                oo::define C method c {} {info frame 0}\n\
                C create o\n\
                list [dict get [o a] line] [dict get [o b] line] \
                     [dict get [o c] line] [dict get [o a] type] \
                     [dict get [o a] file]";
        let r = i.eval_sourced(script, b"/tmp/x.tcl");
        assert_eq!(r, Code::Ok);
        // `method a` body line 3; `method b` body line 5; `method c` line 7.
        assert_eq!(i.result_bytes(), b"3 5 7 source /tmp/x.tcl");
    });
}

#[test]
fn info_frame_reports_method_context() {
    leak_free(|i| {
        // `info frame 0` in a class-defined method reports method + class;
        // an object-defined method reports method + object (not `proc`).
        ok(i, b"oo::class create c");
        ok(i, b"c create i");
        ok(i, b"oo::define c method m {} { info frame 0 }");
        ok(i, b"oo::objdefine i method n {} { info frame 0 }");
        let cm = ok(i, b"c create o; o m");
        assert!(cm.windows(13).any(|w| w == b"method m clas"));
        assert!(cm.ends_with(b"class ::c level 0"));
        let inf = ok(i, b"i n");
        assert!(inf.ends_with(b"object ::i level 0"));
        // A plain proc still reports `proc`, not method/class.
        ok(i, b"proc p {} { info frame 0 }");
        let pf = ok(i, b"p");
        assert!(pf.windows(7).any(|w| w == b"proc ::"));
        assert!(!pf.windows(6).any(|w| w == b"method"));
    });
}

#[test]
fn no_method_forces_user_unknown() {
    leak_free(|i| {
        // A no-method invocation `$obj` with a user `unknown` runs it with
        // empty args (C's FORCE_UNKNOWN); a plain object reports usage.
        ok(i, b"set o [oo::object new]");
        assert_eq!(i.eval_str(b"$o"), Code::Error);
        assert!(i.result_bytes().starts_with(b"wrong # args: should be"));
        ok(
            i,
            b"oo::objdefine $o method unknown args { return \"u:>>$args<<\" }",
        );
        assert_eq!(ok(i, b"$o"), b"u:>><<");
        assert_eq!(ok(i, b"$o foo bar"), b"u:>>foo bar<<");
    });
}
