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

//! String, `regexp`, `format`, `scan` and `binary` builtins, and the substitution of command words: escapes, brackets, braces and `subst`.

use super::*;

#[test]
fn string_ops() {
    out_eq("puts [string length hello]\n", "5\n");
    out_eq("puts [string index hello 1]\n", "e\n");
    out_eq("puts [string range hello 1 3]\n", "ell\n");
    out_eq("puts [string toupper aBc]\n", "ABC\n");
    out_eq("puts [string trim {  hi  }]\n", "hi\n");
    out_eq("puts [string map {a A o O} banana]\n", "bAnAnA\n");
    out_eq("puts [string match f* foo]\n", "1\n");
    out_eq("puts [string equal abc abc]\n", "1\n");
    out_eq("puts [string repeat ab 3]\n", "ababab\n");
    out_eq("puts [string is integer 42]\n", "1\n");
    out_eq("puts [string is integer 4x]\n", "0\n");
}

#[test]
fn append_builtin() {
    out_eq("set s foo\nappend s bar baz\nputs $s\n", "foobarbaz\n");
}

#[test]
fn subst_command() {
    out_eq("set x 5\nputs [subst {x is $x}]\n", "x is 5\n");
    out_eq("set x 5\nputs [subst {sum [expr {$x+1}]}]\n", "sum 6\n");
    out_eq("set a(k) v\nputs [subst {got $a(k)}]\n", "got v\n");
    out_eq("puts [subst -novariables {keep $x}]\n", "keep $x\n");
    out_eq("puts [subst -nocommands {keep [cmd]}]\n", "keep [cmd]\n");
    // Per-bracket control flow (subst-8.x/10.x, matched against tclsh 9.0.4):
    // `break` finalises with the text so far, `continue` drops the bracket's
    // value, and `return` (any non-error code) substitutes its result. These
    // exercise the yieldable subst frame's rules via the non-coroutine path.
    out_eq("puts [subst {a[break]b}]\n", "a\n");
    out_eq("puts [subst {a[continue]b}]\n", "ab\n");
    out_eq("puts [subst {a[return -level 0 X]b}]\n", "aXb\n");
    // `-nobackslashes` leaves an escape verbatim; the default decodes it.
    out_eq("puts [subst -nobackslashes {a\\tb}]\n", "a\\tb\n");
    out_eq("puts [subst {x[expr 1][expr 2]y}]\n", "x12y\n");
}

#[test]
fn regexp_regsub() {
    out_eq("puts [regexp {[0-9]+} abc123]\n", "1\n");
    out_eq("puts [regexp {xyz} abc123]\n", "0\n");
    out_eq(
        "regexp {([a-z]+)([0-9]+)} abc123 m a b\nputs \"$m|$a|$b\"\n",
        "abc123|abc|123\n",
    );
    out_eq("puts [regexp -all {[0-9]} a1b2c3]\n", "3\n");
    out_eq("puts [regexp -inline {[0-9]+} abc123def]\n", "123\n");
    out_eq("puts [regsub {[0-9]+} abc123 X]\n", "abcX\n");
    out_eq("puts [regsub -all {[0-9]} a1b2 _]\n", "a_b_\n");
    out_eq(
        "set n [regsub -all o foo 0 out]\nputs \"$n|$out\"\n",
        "2|f00\n",
    );
}

/// Features the VM gained by sharing `tcl_cmd_core::regex`'s plumbing (it had
/// only `-nocase`/`-all`/`-inline`/`-line` before). Pinned against tclsh 9.0.
#[test]
fn regexp_shared_features() {
    // `-indices` reports char-offset {start end} pairs.
    assert_eq!(run("regexp -indices {bc} abcd m; set m").1, "1 2");
    // `-start` resumes the search at a char offset.
    assert_eq!(run("regexp -start 3 {a} {a a a}").1, "1");
    assert_eq!(run("regsub -start 2 -all {a} aaaa X").1, "aaXX");
    assert_eq!(run("regexp -start 1+1 {a} aaaa").1, "1");
    assert_eq!(run("regsub -start 0x2 {a} aaaa X").1, "aaXa");
    let (ok, msg, _) = run("regexp -start bogus {a} aaaa");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"bogus\""), "got: {msg}");
    let (ok, msg, _) = run("regsub -start {end - 2} {a} aaaa X");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"end - 2\""), "got: {msg}");
    // `-inline -all` with submatches flattens whole+subs per match.
    assert_eq!(
        run("regexp -inline -all {(\\d)(\\d)} 1234").1,
        "12 1 2 34 3 4"
    );
    // A failed match leaves the match variables untouched (was: set to empty).
    assert_eq!(run("set m PRESET; regexp {z} abc m; set m").1, "PRESET");
    // The tclsh compile-error prefix (was: "couldn't compile…").
    let (ok, msg, _) = run("regexp {a(} b");
    assert!(!ok);
    assert!(
        msg.starts_with("cannot compile regular expression pattern"),
        "got: {msg}"
    );
    // The per-command bad-option message.
    let (ok, msg, _) = run("regexp -bogus {a} b");
    assert!(!ok);
    assert!(
        msg.starts_with("bad option \"-bogus\": must be -all, -about"),
        "got: {msg}"
    );
    // Options are exact-only (C's TCL_EXACT): even a unique prefix is a bad
    // option. tclsh: `regexp -no a A x` errors identically; only tclsh's
    // compiled no-match-var fast path (`regexp -no a A`) abbreviates
    // `-nocase`, and the VM implements the runtime semantics everywhere
    // (`TclCompileRegexpCmd`).
    let (ok, msg, _) = run("regexp -no {a} A x");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-no\": must be -all, -about, -indices, -inline, \
         -expanded, -line, -linestop, -lineanchor, -nocase, -start, or --"
    );
    // regsub is exact-only in every context (tclsh compiles only a literal
    // `-all`); the enumeration is tclsh9.0's table (which has `-command`).
    let (ok, msg, _) = run("regsub -no {a} A b");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-no\": must be -all, -command, -expanded, -line, \
         -linestop, -lineanchor, -nocase, -start, or --"
    );
}

#[test]
fn cmd_subst_substitutes_proc_param() {
    // A bare `$param` arg to a *generic* command inside a command substitution
    // must load the variable, not push the literal `$param`.
    out_eq(
        "proc f {a b} { return [file join $a $b] }\nputs [f /x y]\n",
        "/x/y\n",
    );
}

#[test]
fn format_command() {
    out_eq("puts [format %05d 42]\n", "00042\n");
    out_eq("puts [format \"%d-%s\" 5 hi]\n", "5-hi\n");
    out_eq("puts [format %x 255]\n", "ff\n");
    out_eq("puts [format %.2f 3.14159]\n", "3.14\n");
    out_eq("puts [format %-5d| 42]\n", "42   |\n");
    out_eq("set n 3\nputs [format \"n=%d\" $n]\n", "n=3\n");
}

#[test]
fn braced_literal_not_substituted() {
    out_eq("set x {a [b] $c}\nputs $x\n", "a [b] $c\n");
}

#[test]
fn scan_command() {
    out_eq("puts [scan 4 %c]\n", "52\n");
    out_eq("puts [scan \"42 7\" \"%d %d\"]\n", "42 7\n");
    out_eq("scan \"0xff\" 0x%x n\nputs $n\n", "255\n");
    out_eq(
        "scan \"hello 99\" {%s %d} word num\nputs \"$word $num\"\n",
        "hello 99\n",
    );
    out_eq("puts [scan abc %d]\n", "{}\n");
}

#[test]
fn string_map_nocase() {
    out_eq(
        "puts [string map -nocase {ok 0 error 1} {OK ERROR ok}]\n",
        "0 1 0\n",
    );
    out_eq("puts [string map {a A b B} abcab]\n", "ABcAB\n");
}

/// `-failindex` must be written. The inline `string is` codegen gates on arity
/// alone, so gating on arity without checking the option name would accept
/// `CLASS -failindex var value`, take the last word as the value, and
/// silently drop the option — the class answer would be right but the
/// variable would never be assigned. Pinned against tclsh 8.6.16 / 9.0.4,
/// which report index 1 here.
#[test]
fn string_is_failindex_is_written() {
    out_eq(
        "set fi UNSET
set r [string is integer -failindex fi 1.5]
puts \"$r $fi\"\n",
        "0 1\n",
    );
    // Through a variable, and inside a proc body (a different codegen path).
    out_eq(
        "proc p {v} {\n set fi UNSET\n set r [string is integer -failindex fi $v]\n return \"$r $fi\"\n}\nputs [p 12x]\n",
        "0 2\n",
    );
    // A member leaves the variable untouched.
    out_eq(
        "set fi UNSET
set r [string is integer -failindex fi 42]
puts \"$r $fi\"\n",
        "1 UNSET\n",
    );
    // `-strict` still specialises inline and stays correct.
    out_eq("puts [string is alpha -strict \"\"]\n", "0\n");
}

#[test]
fn string_is_classes() {
    out_eq("puts [string is print 4]\n", "1\n");
    out_eq("puts [string is print \"hello world\"]\n", "1\n");
    out_eq("puts [string is graph 4]\n", "1\n");
    out_eq("puts [string is graph \" \"]\n", "0\n"); // space is not graph
    // A real control char (tab/bell) is not printable; build it with `format`.
    out_eq("puts [string is print [format %c 9]]\n", "0\n");
    out_eq("puts [string is control [format %c 7]]\n", "1\n");
    out_eq("puts [string is wordchar foo_1]\n", "1\n");
    out_eq("puts [string is wordchar foo-1]\n", "0\n");
}

#[test]
fn backslash_substituted_command_arguments() {
    // Non-braced arguments decode escapes once; escaped sigils remain literal data.
    out_eq(
        "proc p {a} { puts [string length $a]:$a }\n\
         p \\{\n\
         p a\\{b\n\
         p x\\$y\n\
         p f\\[g\n",
        "1:{\n3:a{b\n3:x$y\n3:f[g\n",
    );
    // A real `$var` substitution still resolves (escaped markers don't disable
    // interpolation of the rest of the word).
    out_eq("set y hi\nproc p {a} { puts $a }\np \\$=$y\n", "$=hi\n");
}

/// An escaped bracket in an **expr** operand: the codegen decodes `"\["` to a
/// bare `[`, so the compiled word reaches the VM with an unclosed bracket and
/// nothing left to close it. That is data, not C's `missing close-bracket` —
/// tclsh 8.6.16 and 9.0.4 both answer the comparison. Treating it as a parse
/// error broke every `if {$ch eq "\["}` in a guest script.
#[test]
fn an_escaped_bracket_in_an_expr_operand_is_data() {
    out_eq("set ch {[}\nputs [expr {$ch eq \"\\[\"}]\n", "1\n");
    out_eq("puts [expr {\"\\[\" eq \"\\[\"}]\n", "1\n");
    out_eq(
        "set ch {[}\nif {$ch eq \"\\[\"} { puts hit } else { puts miss }\n",
        "hit\n",
    );
    // The closing half still behaves, and a real substitution still runs.
    out_eq("set ch {]}\nputs [expr {$ch eq \"\\]\"}]\n", "1\n");
    out_eq("puts [expr {[string length ab] + 1}]\n", "3\n");
}

#[test]
fn escaped_brackets_not_double_substituted() {
    // A word carrying an escaped bracket (`\[` / `\]`) must be backslash-decoded
    // exactly once. Pre-decoding `\[` to a bare `[` made the runtime word-subst
    // re-read it as a command substitution and drop the backslash before the
    // matching `]` (`"x\[y z\\]"` → `x[y z]` instead of `x[y z\]`).
    out_eq("set a \"x\\[y z\\\\]\"\nputs $a\n", "x[y z\\]\n");
    out_eq("set b \"{a\\[} b\\\\]\"\nputs $b\n", "{a[} b\\]\n");
    // An escaped `\${…}` is the literal `${…}`, not a variable substitution.
    out_eq("set x 9\nputs \"\\${x}\"\n", "${x}\n");
    // Same word as a (non-braced) proc argument.
    out_eq(
        "proc plen {s} { return [string length $s]:$s }\nputs [plen a\\[b\\]c]\n",
        "5:a[b]c\n",
    );
    // And surviving a `[list …]` → `uplevel` round-trip: the escaped brackets
    // and trailing braces must reach the evaluated `list` intact (the list.test
    // `invalid command name "}"` abort).
    out_eq(
        "proc run {s} { return [uplevel 1 $s] }\nputs [run {list a\\[ b\\]}]\n",
        "{a[} b\\]\n",
    );
}

#[test]
fn bare_dollar_literal_decodes() {
    // A deferred literal carrying a *bare* `$` (`f\$}` — `$}` is not a variable)
    // must still have its backslash escapes decoded once, not left raw.
    out_eq(
        "set body \"list e\\\\n} f\\\\$} \"\nputs [string length $body]\n",
        "15\n",
    );
    out_eq("set x \"a\\\\nq$\"\nputs $x\n", "a\\nq$\n");
    // Real variables still interpolate.
    out_eq("set z hi\nputs \"a $z b\"\n", "a hi b\n");
}

#[test]
fn brace_wrapped_command_substitution_runs() {
    // A quoted `{…}`-wrapped value with a command substitution runs the command
    // and keeps the literal braces (rather than being stripped as a braced
    // literal). `string is list` / `string is dict` validate list-ness.
    out_eq("set s \"{[list a b]}\"\nputs $s\n", "{a b}\n");
    out_eq("puts [string is list {a b c}]\n", "1\n");
    out_eq("puts [string is list \"a \\{b\"]\n", "0\n");
    out_eq("puts [string is dict {a 1 b 2}]\n", "1\n");
    out_eq("puts [string is dict {a 1 b}]\n", "0\n");
}

#[test]
fn binary_format_rejects_invalid_integer() {
    // A malformed integer is an error, not a silent zero byte.
    let (ok1, _r1, _o1) = run("binary format c bad\n");
    assert!(!ok1, "binary format c bad should error");
    let (ok2, _r2, _o2) = run("binary format c* {1 bad 3}\n");
    assert!(!ok2, "binary format c* with a bad element should error");
    // Valid radix lists still work.
    out_eq(
        "puts [string length [binary format I* {0x50515253 0x52}]]\n",
        "8\n",
    );
}

#[test]
fn format_dynamic_decimal_and_float_generic_fallback_preserves_dispatch() {
    out_eq(
        "set seen {}\nproc side {v} {global seen; lappend seen $v; return $v}\nputs [list [format %d [side 42]] [format %f [side 2]] $seen]\nputs [list [catch {format %d [error boom]} msg] $msg]\nrename format oldformat\nproc format args {return [list override {*}$args]}\nputs [format %d [side 7]]\n",
        "42 2.000000 {42 2}\n1 boom\noverride %d 7\n",
    );
}

#[test]
fn private_string_is_dynamic_class_decline_preserves_dispatch() {
    out_eq(
        "set seen {}\nproc val {tag value} {global seen; lappend seen $tag; return $value}\nputs [list [::tcl::string::is [val class ascii] -strict [val value abc]] $seen]\nset seen {}\nputs [list [catch {::tcl::string::is [val class digit] -failindex index [val value abc]} result] $result $index $seen]\nset seen {}\nputs [list [catch {::tcl::string::is [error boom] [val value abc]} result] $result $seen]\nset seen {}\nproc rebind {} {global seen; lappend seen class; rename ::tcl::string::is ::tcl::string::oldis; proc ::tcl::string::is args {return OVERRIDE}; return ascii}\nputs [list [::tcl::string::is [rebind] -strict [val value abc]] $seen]\n",
        "1 {class value}\n0 0 0 {class value}\n1 boom {}\nOVERRIDE {class value}\n",
    );
}

#[test]
fn dynamic_format_word_decline_preserves_dispatch_and_operand_order() {
    out_eq(
        "set seen {}\nproc value {} {global seen; lappend seen value; return 7}\nproc fmt {} {global seen; lappend seen fmt; return %d}\nputs [list [format [fmt] [value]] $seen]\nset seen {}\nputs [list [catch {format [error boom] [value]} result] $result $seen]\nset seen {}\nrename fmt oldfmt\nproc fmt {} {global seen; lappend seen fmt; rename format oldformat; proc format args {return OVERRIDE}; return %d}\nputs [list [format [fmt] [value]] $seen]\n",
        "7 {fmt value}\n1 boom {}\nOVERRIDE {fmt value}\n",
    );
}

#[test]
fn dynamic_format_word_zero_and_two_values_keep_order_and_rebinding() {
    out_eq(
        "set seen {}\nproc value {tag v} {lappend ::seen $tag; return $v}\nputs [list [format [value fmt %%]] $seen]\nset seen {}\nputs [list [format [value fmt {%s:%s}] [value a A] [value b B]] $seen]\nset seen {}\nputs [list [catch {format [error boom] [value a A] [value b B]} result] $result $seen]\nset seen {}\nproc rebind {} {lappend ::seen fmt; rename format oldformat; proc format args {return OVERRIDE}; return {%s:%s}}\nputs [list [format [rebind] [value a A] [value b B]] $seen]\n",
        "% fmt\nA:B {fmt a b}\n1 boom {}\nOVERRIDE {fmt a b}\n",
    );
}
