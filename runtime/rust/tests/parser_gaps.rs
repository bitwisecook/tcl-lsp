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

//! Oracle-pinned regression coverage for the r4-parser-gaps lane
//! (#1576, #1586, #1577) and the two lenient cases r10-word-parts closed on
//! top of it (`missing "`, `missing close-bracket`). Every expected
//! message/errorCode/value below was taken verbatim from `tclsh9.0` (9.0.4)
//! and/or `tclsh8.6` (8.6.16); a reader can paste the sheet into a real
//! `tclsh` and re-derive the expectation without this harness.

use tcl_runtime::interp::{Code, Interp};

/// Evaluate `sheet` and return `(code, result, errorCode)`. `errorCode` is
/// read from `::errorCode`, which the outermost `eval_str` publishes on an
/// uncaught error — matching how `catch ... opts; dict get $opts -errorcode`
/// observes it in real Tcl.
fn run(sheet: &str) -> (Code, String, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(sheet.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    let error_code = if code == Code::Error {
        interp.eval_str(b"set ::errorCode");
        String::from_utf8_lossy(&interp.result_bytes()).into_owned()
    } else {
        String::new()
    };
    (code, result, error_code)
}

// ---------------------------------------------------------------------------
// #1576 — an unterminated `{` word must raise `missing close-brace`, not
// tokenize best-effort. Oracle: tclsh 8.6.16/9.0.4 both raise `missing
// close-brace` with `-errorcode NONE` for every repro in the issue (`list`,
// `set`, `string length`).
// ---------------------------------------------------------------------------

#[test]
fn unterminated_brace_word_raises_missing_close_brace() {
    let (code, result, error_code) = run(r#"eval "list a \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
    assert_eq!(error_code, "NONE");
}

#[test]
fn unterminated_brace_word_raises_for_set_too() {
    let (code, result, _) = run(r#"eval "set x \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
}

#[test]
fn unterminated_brace_word_raises_for_string_length_too() {
    let (code, result, _) = run(r#"eval "string length \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
}

/// A properly closed braced word — including one glued to a second braced
/// word (`{*}{b c}`, no unterminated construct) — is unaffected by the
/// #1576 fix.
#[test]
fn well_formed_braced_words_still_parse() {
    let (code, result, _) = run("list a {hello world} c");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a {hello world} c");

    let (code, result, _) = run("list a {*}{b c} d");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a b c d");
}

// ---------------------------------------------------------------------------
// #1586 — an unterminated `${` inside a script word (or a script word nested
// in a command substitution) must raise `missing close-brace for variable
// name`, the same message `subst` already raises for the identical
// construct — not the lenient lexer recovery's `${a{` == name `a{` reading
// (which then fails downstream as `can't read "a{"`). Oracle: tclsh
// 8.6.16/9.0.4 both raise `missing close-brace for variable name` with
// `-errorcode NONE`.
// ---------------------------------------------------------------------------

#[test]
fn unterminated_braced_var_in_a_script_word_raises_missing_close_brace_for_var() {
    // eval "set x \"${a{\""
    let (code, result, error_code) = run("eval \"set x \\\"\\$\\{a\\{\\\"\"");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace for variable name");
    assert_eq!(error_code, "NONE");
}

/// The same gap, nested inside a command substitution reached through
/// `subst` — `subst $t` where `t` is `[set y ${a{b]` — matching the second
/// row of the issue's table (the C parser reports the same error at every
/// nesting depth, not `can't read "a{b"`).
#[test]
fn unterminated_braced_var_nested_in_a_command_subst_raises_the_same_error() {
    // `t`'s value is the 14-byte string `[set y ${a{b]` — built through
    // `format` rather than a literal `{...}` word, since its raw text has
    // two unmatched `{` and would itself be an unterminated *outer* brace
    // word (this file's own #1576 sibling gap, not what this test targets).
    let (code, result, _) = run(r#"set t [format {[set y $%sa%sb]} "{" "{"]
subst $t"#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace for variable name");
}

/// A well-formed `${name}` (including one immediately followed by more
/// substitution) is unaffected by the #1586 fix.
#[test]
fn well_formed_braced_var_still_substitutes() {
    let (code, result, _) = run("set a hi\nsubst {x${a}y}");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "xhiy");
}

// ---------------------------------------------------------------------------
// #1577 — `lassign`, `catch` (result/options vars), `regexp` match vars,
// `scan`, `binary scan`, and `foreach`/`lmap` loop vars must write `arr(a)`
// as the array *element*, like `set` already does, not as a literal scalar
// named `arr(a)`. Oracle: tclsh 8.6.16/9.0.4 all agree `array get arr` comes
// back `a p b q` (etc.) after each of these commands targets `arr(a)`/
// `arr(b)`.
// ---------------------------------------------------------------------------

#[test]
fn lassign_writes_array_elements() {
    let (code, result, _) = run("lassign {p q} arr(a) arr(b)\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a p b q");
}

#[test]
fn lassign_into_an_existing_array_scalar_target_still_errors_like_set() {
    let (code, result, _) = run("array set arr {x 1}\ncatch {lassign {1} arr} e\nset e");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "can't set \"arr\": variable is array");
}

#[test]
fn catch_writes_result_and_options_array_elements() {
    let (code, result, _) =
        run("catch {error boom} arr(a) opts(o)\nlist [array get arr] [dict get $opts(o) -code]");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "{a boom} 1");
}

#[test]
fn regexp_writes_match_vars_as_array_elements() {
    let (code, result, _) = run("regexp {(a)(b)} xabx m arr(1) arr(2)\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "1 a 2 b");
}

#[test]
fn scan_writes_vars_as_array_elements() {
    let (code, result, _) = run(r#"scan "12 ab" "%d %s" arr(1) arr(2)
array get arr"#);
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "1 12 2 ab");
}

#[test]
fn binary_scan_writes_vars_as_array_elements() {
    let (code, result, _) = run("binary scan AB cc arr(1) arr(2)\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "1 65 2 66");
}

#[test]
fn foreach_writes_loop_var_as_array_element() {
    let (code, result, _) = run("foreach arr(a) {1 2} {}\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a 2");
}

#[test]
fn lmap_writes_loop_var_as_array_element_too() {
    let (code, result, _) = run("lmap arr(a) {1 2} {set arr(a)}\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a 2");
}

/// The zero-length-array-name spelling `(k)` — base name `""` — routes
/// through the same `split_array_ref` owner as every other element write, so
/// it comes along for free (tracked separately as #1458 for whether the
/// *owner itself* handles every edge of that spelling; this only pins that
/// these six sites don't bypass it).
#[test]
fn zero_length_array_name_spelling_routes_through_the_same_owner() {
    let (code, result, _) = run("lassign {v} (k)\narray get {}");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "k v");
}

// ---------------------------------------------------------------------------
// R10 — the two cases R4 left lenient. Both are word-delimiter failures the
// lexer recovers from (it is shared with the LSP and must keep tokenizing
// half-typed source), so the eval-facing parser is the one that must fail
// closed. Before the shared `tcl_lexer::word_parts` owner landed, this
// runtime read `list a "b` as the two-word list `a b` and evaluated the `b`
// of `list a [b` as a command.
//
// Oracle (both interpreters):
//   % eval {list a "b}    => missing "                -errorcode NONE
//   % eval {list a [b}    => missing close-bracket    -errorcode NONE
// ---------------------------------------------------------------------------

#[test]
fn unterminated_quoted_word_raises_missing_quote() {
    let (code, result, error_code) = run(r#"eval {list a "b}"#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing \"");
    assert_eq!(error_code, "NONE");
}

#[test]
fn unterminated_quoted_word_raises_for_other_commands_too() {
    for sheet in [r#"eval {puts "a}"#, r#"eval {set x "a}"#] {
        let (code, result, _) = run(sheet);
        assert_eq!(code, Code::Error, "{sheet}");
        assert_eq!(result, "missing \"", "{sheet}");
    }
}

#[test]
fn unterminated_command_substitution_raises_missing_close_bracket() {
    let (code, result, error_code) = run("eval {list a [b}");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-bracket");
    assert_eq!(error_code, "NONE");
}

/// The same failure through `subst`, where C reports it *after* the earlier
/// bracket in the template has already run and kept its side effects — the
/// left-to-right order `WordPart::ParseError` exists to preserve.
///
/// Oracle: `proc side {} {puts ran; return S}; subst {[side][b}` prints `ran`
/// and then raises `missing close-bracket` on 8.6.16 and 9.0.4.
#[test]
fn subst_reports_the_missing_bracket_after_running_the_earlier_one() {
    let (code, result, _) = run("set ::ran 0\nproc side {} {set ::ran 1; return S}\ncatch {subst {[side][b}} e\nlist $::ran $e");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "1 {missing close-bracket}");
}

/// An unterminated `$name(` array index is C's `missing )` — the third
/// delimiter failure the same owner now spells, previously read as a scalar
/// literally named `x(`.
///
/// Oracle: `eval {list a $x(}` and `subst {$x(}` both give `missing )`.
#[test]
fn unterminated_array_index_raises_missing_paren() {
    for sheet in ["eval {list a $x(}", "subst {$x(}"] {
        let (code, result, _) = run(sheet);
        assert_eq!(code, Code::Error, "{sheet}");
        assert_eq!(result, "missing )", "{sheet}");
    }
}

/// Well-formed quoted, bracketed and array-index words are unaffected: the
/// close checks must not fire on a `"` inside a command substitution, a `]`
/// inside a braced or quoted word of the substituted script, or a `)` closing
/// a nested index.
#[test]
fn well_formed_quoted_and_bracketed_words_still_parse() {
    for (sheet, want) in [
        (r#"list a "b c" d"#, "a {b c} d"),
        (r#"set x [string cat "a" "b"]"#, "ab"),
        (r#"subst {[list {a]b}]}"#, r"a\]b"),
        (r#"subst {[list "a]b"]}"#, r"a\]b"),
        (
            "set c(1) inner\nset b(inner) mid\nset a(mid) outer\nsubst {$a($b($c(1)))}",
            "outer",
        ),
    ] {
        let (code, result, _) = run(sheet);
        assert_eq!(code, Code::Ok, "{sheet}: {result}");
        assert_eq!(result, want, "{sheet}");
    }
}

// ---------------------------------------------------------------------------
// r5b-leftovers follow-up — `regsub`'s target variable has the identical
// #1577 shape (flagged, not fixed, by r4-parser-gaps): `arr(k)` must write
// the array *element*, not a literal scalar named `arr(k)`. Oracle: tclsh
// 8.6.16/9.0.4 both give `array get arr` => `k xbx` after `regsub -all a $s
// b arr(k)` on `s = xax`.
// ---------------------------------------------------------------------------

#[test]
fn regsub_writes_its_target_variable_as_an_array_element() {
    let (code, result, _) = run("set s xax\nregsub -all a $s b arr(k)\narray get arr");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "k xbx");
}

// ---------------------------------------------------------------------------
// #1786 — text welded straight onto a `{…}` word's close-brace. The boundary
// question moved to `tcl_lexer::script::group_commands`, which records the
// weld as `WordSpan::welded_after_close`; this eval-facing engine turns it
// into C's hard error, closing a gap both Rust groupers had (and disagreed
// about: this crate welded `{a}` and `$b` into one `Bare` word, the compiler's
// segmenter split them into two, and C accepts neither).
//
// Oracle, measured on tclsh 8.4.20 / 8.5.19 / 8.6.16 / 9.0.4 / 9.1b0 — every
// row identical on every release:
//
//   {a}b        {a}$b       {a}[b]      {}x        {a}{b}      {a}{*}$b
//   set x {a}b              set x {a}{*}$y         list {a}{*}$b
//     -> code 1, `extra characters after close-brace`, -errorcode NONE
//
// and it is a *parse*-time failure of the command: `set x {a}[side]` leaves
// `x` unset without running `side`. Earlier commands of the same script have
// already run, since `Tcl_EvalEx` parses one command at a time — pinned below
// by `sfx pre`.
//
// The ordering nuance this header once recorded as pending — C parses every
// word of a command before substituting any, so `list [side] {a}b` never runs
// `side` — is closed by #1787 and pinned below by
// `a_command_parses_whole_before_any_of_its_words_substitute`.
//
// Not welded, and still accepted (measured the same on 8.6.16 / 9.0.4):
// `a{b}c`, `x{a}y`, `$a{b}c`, `[b]{c}d` — a `{` away from word start is an
// ordinary literal byte, and the lexer never emits a `Str` token for it — and
// `{a} {b}` / `list a {*}{b c} d`, which have a real separator between them.
// ---------------------------------------------------------------------------

/// Every welded shape raises C's message, with `-errorcode NONE`.
#[test]
fn text_welded_onto_a_close_brace_raises_extra_characters() {
    for sheet in [
        "{a}b",
        "{a}$b",
        "{a}[b]",
        "{}x",
        "{a}{b}",
        "{a}{*}$b",
        "set x {a}b",
        "set x {a}{*}$y",
        "list {a}{*}$b",
        "list [list a] {a}b",
    ] {
        let (code, result, error_code) = run(sheet);
        assert_eq!(code, Code::Error, "{sheet}");
        assert_eq!(result, "extra characters after close-brace", "{sheet}");
        assert_eq!(error_code, "NONE", "{sheet}");
    }
}

/// The failure is the *command's*, not the word's: nothing the command would
/// have substituted runs, and the variable it would have written stays unset —
/// while a command earlier in the same script has already run and kept its
/// effect.
#[test]
fn a_welded_close_brace_fails_the_command_before_it_substitutes_anything() {
    let (code, result, _) = run("set ::ran {}\n\
         proc sfx {t} { lappend ::ran $t; return S$t }\n\
         sfx pre\n\
         set ::x {a}[sfx inner]");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "extra characters after close-brace");

    // The same sheet under `catch`, so the interpreter survives to be asked
    // what actually ran: `pre` (an earlier command) yes, `inner` (a word of
    // the failing command) no, and `::x` was never written.
    let (code, result, _) = run("set ::ran {}\n\
         proc sfx {t} { lappend ::ran $t; return S$t }\n\
         sfx pre\n\
         catch {set ::x {a}[sfx inner]} e\n\
         list $e $::ran [info exists ::x]");
    assert_eq!(code, Code::Ok);
    assert_eq!(
        result, "{extra characters after close-brace} pre 0",
        "tclsh 8.6.16/9.0.4: the error, only `pre` ran, `x` unset"
    );
}

/// A `{` that is not at the start of a word is an ordinary literal byte, and a
/// real separator between two braced words is not a weld — neither may trip
/// the new error.
#[test]
fn a_brace_away_from_word_start_or_after_a_separator_is_not_a_weld() {
    for (sheet, want) in [
        ("list a{b}c", "a{b}c"),
        ("list x{a}y", "x{a}y"),
        ("set a A\nlist $a{b}c", "A{b}c"),
        ("list [list q]{c}d", "q{c}d"),
        ("list {a} {b}", "a b"),
        ("list a {*}{b c} d", "a b c d"),
        ("list {a}", "a"),
    ] {
        let (code, result, _) = run(sheet);
        assert_eq!(code, Code::Ok, "{sheet}: {result}");
        assert_eq!(result, want, "{sheet}");
    }
}

// ---------------------------------------------------------------------------
// #1787 — C's script-parsing ORDER. `Tcl_EvalEx` parses one whole command
// (`Tcl_ParseCommand`, every word of it) and only then substitutes and
// dispatches, so a parse failure anywhere in a command is the *command's*
// failure: nothing that command would have substituted runs, however early in
// the command it sits. And `Tcl_ParseCommand`'s `ParseTokens` recurses into a
// `[…]`, parsing the substituted script's own commands, so an error nested
// inside a later bracket also stops an earlier bracket — and stops the earlier
// commands *inside* the failing bracket too.
//
// Property (1) of the same issue — earlier commands of the script have already
// run — this engine has had since it was written (`parse.rs`'s `next`-offset
// `Command` and the per-command loop in `interp.rs`); every row below pins it
// as well, through the `sfx pre` that always shows up in `ran`.
//
// Oracle: `PARSE_ORDER_SHEET` below, saved to a file and run as
// `tclsh sheet.tcl` (FROM A FILE — a stdin-fed tclsh swallows the error text)
// on tclsh 8.6.16 and tclsh 9.0.4. Both shells produced `PARSE_ORDER_ORACLE`
// byte for byte; paste the sheet into either shell to re-derive it without this
// harness. The sheet builds its `${abc` rows with `format` because a sheet that
// spelled `${` inside a braced `catch`/`probe` word would itself be an
// unbalanced brace group.
// ---------------------------------------------------------------------------

/// The oracle sheet, verbatim. `$ob` is a literal `{`.
const PARSE_ORDER_SHEET: &str = r#"set ob "\173"
set ::out {}
proc sfx {t} { lappend ::ran $t; return S$t }
proc probe {name script} {
    set ::ran {}
    set verb [expr {[catch {uplevel 1 $script} e] ? "error" : "ok"}]
    lappend ::out "$name -> $verb {$e} ran {$::ran}"
}
probe welded        {sfx pre; list [sfx inner] {a}b}
probe quote         {sfx pre; list [sfx inner] "unterminated}
probe bracket       {sfx pre; list [sfx inner] [sfx two}
probe braced-var    [format {sfx pre; list [sfx inner]pre$%sabc} $ob]
probe nested-quote  {sfx pre; list [sfx inner] [list "oops]}
probe nested-var    [format {sfx pre; list [sfx inner] [set y "a$%sbcd"]} $ob]
probe nested-weld   {sfx pre; list [sfx inner] [set y {a}b]}
probe two-cmd-brkt  [format {sfx pre; list [sfx one; set y "a$%sbcd"]} $ob]
probe var-first     [format {sfx pre; list $nosuchvar [set y "a$%sbcd"]} $ob]
probe runtime-err   {sfx pre; list [sfx inner] [set y $nosuchvar]}
probe all-well      {sfx pre; list [sfx inner] [list ok]}
join $::out \n"#;

/// tclsh 8.6.16 and tclsh 9.0.4, identical.
const PARSE_ORDER_ORACLE: &str = r#"welded -> error {extra characters after close-brace} ran {pre}
quote -> error {missing "} ran {pre}
bracket -> error {missing close-bracket} ran {pre}
braced-var -> error {missing close-brace for variable name} ran {pre}
nested-quote -> error {missing "} ran {pre}
nested-var -> error {missing close-brace for variable name} ran {pre}
nested-weld -> error {extra characters after close-brace} ran {pre}
two-cmd-brkt -> error {missing close-brace for variable name} ran {pre}
var-first -> error {missing close-brace for variable name} ran {pre}
runtime-err -> error {can't read "nosuchvar": no such variable} ran {pre inner}
all-well -> ok {Sinner ok} ran {pre inner}"#;

/// Every row of the sheet, against the recorded shells.
///
/// The two rows that are *not* parse failures are the guard against
/// over-rejecting: `runtime-err` (a well-formed later bracket that fails at run
/// time) and `all-well` both still run the earlier `[sfx inner]`, so the
/// pre-scan may not turn a runtime error — or a healthy command — into a
/// parse-time abort.
#[test]
fn a_command_parses_whole_before_any_of_its_words_substitute() {
    let (code, result, _) = run(PARSE_ORDER_SHEET);
    assert_eq!(code, Code::Ok, "{result}");
    assert_eq!(result, PARSE_ORDER_ORACLE);
}

/// `subst` is the deliberate non-consumer: it is not a command parse, so C
/// substitutes its template left to right and keeps the side effects of every
/// `[…]` that ran before the failure. Pinned separately above by
/// `subst_reports_the_missing_bracket_after_running_the_earlier_one`; repeated
/// here next to its opposite so the pair cannot drift apart.
///
/// Oracle (8.6.16 / 9.0.4): `1 {missing close-bracket}` — `side` ran.
#[test]
fn subst_is_not_a_command_parse_and_still_runs_the_earlier_bracket() {
    let (code, result, _) = run("set ::ran 0\n\
         proc side {} {set ::ran 1; return S}\n\
         catch {subst {[side][b}} e\n\
         list $::ran $e");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "1 {missing close-bracket}");
}
