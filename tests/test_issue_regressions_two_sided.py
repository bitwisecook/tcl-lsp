"""Two-sided regression tests for historically-reported false positives.

Each closed issue here reported a *false positive* (or a missing-recognition
bug).  For every one we assert **both** sides, so a future change can't quietly
re-break either:

* **fix holds** -- the original, legitimate code no longer raises the bogus
  diagnostic; and
* **true positive still fires** -- a semantically-similar but genuinely-wrong
  variant *is* still flagged.

The "genuinely wrong" variants are ground-truthed against real C ``tclsh``
(8.6 and 9.0, both on PATH in CI); the exact interpreter error each one
produces is quoted in the test so the assertion's intent is auditable.  Run
the snippet through ``tclsh`` if you ever need to re-confirm.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from compiler.registry.dialect import dialect_scope


def _codes(source: str) -> set[str]:
    return {d.code for d in analyse(source).diagnostics}


def _has(source: str, code: str) -> bool:
    return code in _codes(source)


def _w214_unused(source: str) -> set[str]:
    """Parameter names flagged W214 (unused proc parameter)."""
    return {
        d.message.split("'")[1]
        for d in analyse(source).diagnostics
        if d.code == "W214"
    }


class TestE102UnmatchedBrace:
    """#130 / #527 -- ``Unmatched '}'`` (E102).

    The reporter's ``if`` one-liner with a nested empty ``{}`` was wrongly
    flagged as a stray close-brace.
    """

    def test_fix_holds_nested_empty_braces(self):
        # tclsh: runs fine -- the inner {} is an empty list literal.
        assert not _has("if {[llength $domain] != 2} {return {}}\n", "E102")

    def test_true_positive_stray_close_brace(self):
        # tclsh 8.6/9.0:  invalid command name "}"
        # A "}" with no matching "{" begins a command word, which is invalid.
        assert _has("set x 1\n}\n", "E102")


class TestE204ExtraAfterCloseBrace:
    """#57 -- ``Extra chars after '}'`` (E204).

    A brace word followed immediately by a backslash-newline continuation was
    wrongly flagged; the continuation joins the next line, so nothing actually
    follows the ``}``.
    """

    def test_fix_holds_brace_then_backslash_continuation(self):
        # The "}" is followed by "\<newline>", a continuation -- the two
        # physical lines form one command "tt {x} {y}" (valid; tclsh: ok).
        src = "proc tt {a b} {return}\ntt {x}\\\n   {y}\n"
        assert not _has(src, "E204")

    def test_true_positive_chars_glued_after_brace(self):
        # tclsh 8.6/9.0:  extra characters after close-brace
        assert _has("set x {a b}c\n", "E204")


class TestE002TooFewArguments:
    """#129 / #308 -- ``Too few arguments`` (E002)."""

    def test_fix_holds_word_expansion_makes_arity_unknown(self):
        # #129: {*}$rgb expands to an unknown number of words at runtime, so
        # the static arity check must not fire.  tclsh: runs fine.
        src = "proc rgbToLab {r g b} {return}\nset rgb {255 255 255}\nrgbToLab {*}$rgb\n"
        assert not _has(src, "E002")

    def test_fix_holds_proc_name_passed_as_argument(self):
        # #308: passing a proc *name* as a callback argument is not a call of
        # that proc, so its arity must not be checked here.
        src = "proc cb {x} {return}\nproc run {a b} {return}\nrun [list 1] cb\n"
        assert "E002" not in {
            d.code for d in analyse(src).diagnostics if "cb" in d.message
        }

    def test_true_positive_literal_too_few(self):
        # tclsh 8.6/9.0:  wrong # args: should be "f r g b"
        assert _has("proc f {r g b} {return}\nf 1\n", "E002")


class TestE003TooManyArguments:
    """#84 -- ``Too many arguments`` (E003).

    ``puts`` accepts optional ``-nonewline`` and a channel before the string;
    those forms were wrongly counted as excess arguments.
    """

    def test_fix_holds_puts_nonewline(self):
        assert not _has('puts -nonewline "msg"\n', "E003")

    def test_fix_holds_puts_channel(self):
        assert not _has('puts stderr "msg"\n', "E003")

    def test_true_positive_genuinely_too_many(self):
        # tclsh 8.6/9.0:  wrong # args: should be "puts ?-nonewline? ..."
        assert _has('puts a b c d\n', "E003")

    def test_true_positive_user_proc_too_many(self):
        # tclsh 8.6/9.0:  wrong # args: should be "f a"
        assert _has("proc f {a} {return}\nf 1 2 3\n", "E003")


class TestW210ReadBeforeSet:
    """#23 / #62 / #87 / #500 -- ``Variable read before set`` (W210).

    Loop / iteration variables bound by a command, and ``info exists`` probes,
    were wrongly treated as reads of an unset variable.
    """

    def test_fix_holds_dict_for_binds_key_value(self):  # #87
        src = 'set d [dict create a 1]\ndict for {k v} $d {puts "$k $v"}\n'
        assert not _has(src, "W210")

    def test_fix_holds_foreach_loop_variable(self):  # #23
        assert not _has("set l {1 2}\nforeach x $l {puts $x}\n", "W210")

    def test_fix_holds_info_exists_probe(self):  # #500
        # info exists never reads the variable's value -- no W210.
        assert not _has("proc p {} {\n if {[info exists z]} {return}\n}\n", "W210")

    def test_true_positive_read_of_unset_variable(self):
        # tclsh 8.6/9.0:  can't read "neverset": no such variable
        assert _has("proc p {} {puts $neverset}\n", "W210")


class TestW214UnusedParameter:
    """#236 / #239 / #307 / #471 -- ``Unused proc parameter`` (W214).

    Reads hidden inside ``switch`` subjects, ``dict with`` / ``dict for``
    bodies, and mandatory-but-unused trace-callback parameters were missed,
    so used parameters were wrongly reported unused.

    (W214 is a lint with no ``tclsh`` analogue -- Tcl never errors on an unused
    parameter -- so the "true positive" is grounded in the plain fact that the
    parameter is never referenced in the body.)
    """

    def test_fix_holds_switch_subject_is_a_use(self):  # #471
        src = (
            "proc editStartCmd {tbl row col text} {\n"
            "    switch -- $col {1 {return} default {return $text}}\n"
            "}\n"
        )
        assert "col" not in _w214_unused(src)

    def test_fix_holds_dict_with_imports_keys(self):  # #307
        src = "proc d {xall pdata args} {\n dict with pdata {}\n return $xall\n}\n"
        # pdata is used by `dict with`; it must not be reported unused.
        assert "pdata" not in _w214_unused(src)

    def test_fix_holds_trace_callback_mandatory_args(self):  # #239
        # A `trace add variable` callback must accept (name1 name2 op); those
        # parameters are mandated by the API even when the body ignores them.
        src = "proc onchg {name1 name2 op} {puts changed}\ntrace add variable ::x write onchg\n"
        assert _w214_unused(src) == set()

    def test_true_positive_genuinely_unused_parameter(self):
        # `b` is declared but never read anywhere in the body.
        assert "b" in _w214_unused("proc p {a b} {return $a}\n")


class TestW301UplevelInjection:
    """#1 -- ``uplevel with string-built script`` (W301).

    The ``[list ...]`` idiom protects against double substitution, so it must
    not be flagged; a string-built script genuinely risks it.
    """

    def test_fix_holds_list_protected_uplevel(self):
        # tclsh: [list ...] yields a proper, substitution-safe command list.
        assert not _has("set ns [uplevel 1 [list ::namespace current]]\n", "W301")

    def test_true_positive_string_built_uplevel(self):
        # Building the script by string interpolation re-substitutes $v in the
        # caller's frame -- the injection risk W301 exists to catch (issue #1
        # demonstrates the double-substitution behaviour in tclsh).
        assert _has('proc p {} {set v x; uplevel 1 "puts $v"}\n', "W301")


class TestW304MissingOptionTerminator:
    """#235 -- ``Missing option terminator '--'`` (W304) / W306.

    ``dict filter ... script {k v} {...}`` was wrongly flagged (W306,
    substitution-in-literal); meanwhile a variable pattern handed to ``regexp``
    without ``--`` is a real option-injection risk and *should* warn.
    """

    def test_fix_holds_dict_filter_script_form(self):
        src = 'set d [dict create a 1]\ndict filter $d script {k v} {string match x $v}\n'
        assert not _has(src, "W306")

    def test_fix_holds_regexp_with_terminator(self):
        assert not _has("proc f {pattern str} {return [regexp -- $pattern $str]}\n", "W304")

    def test_true_positive_regexp_without_terminator(self):
        # tclsh: a $pattern beginning with '-' is consumed as a regexp option
        # (e.g. -nocase), not matched literally -- W304 guards against that.
        assert _has("proc f {pattern str} {return [regexp $pattern $str]}\n", "W304")


class TestW123CommandRecognition:
    """#105 / #109 / #110 / #111 / #112 / #455 -- builtin command recognition.

    Core builtins were reported as unknown (W123) / mis-analysed; they must be
    recognised, while a genuinely-undefined command is still flagged.
    """

    def test_fix_holds_core_builtins_recognised(self):
        src = (
            "set l [list a b c]\n"
            "lappend l d\n"
            "set i [lsearch $l b]\n"
            "switch $i {1 {puts one} default {puts other}}\n"
            'regsub -all x "xxx" y out\n'
            "package require Tcl\n"
        )
        assert not _has(src, "W123")

    def test_fix_holds_tcl9_foreachline_under_tcl9_dialect(self):  # #429
        # foreachLine is a Tcl 9.0 builtin (TIP 670); recognised in that dialect.
        src = "set f [open data.txt r]\nforeachLine line $f {puts $line}\n"
        with dialect_scope("tcl9.0"):
            assert not _has(src, "W123")

    def test_true_positive_undefined_command(self):
        # tclsh 8.6/9.0:  invalid command name "frobnicate"
        assert _has("frobnicate $x\n", "W123")


class TestE206BraceVarParsing:
    """#137 -- ``${var}`` and ``{*}$var`` must parse cleanly.

    Brace-delimited variable names and ``{*}`` expansion of a ``$var`` were
    mis-tokenised, which broke parsing/highlighting for the rest of the line.
    """

    def test_fix_holds_brace_var_reference(self):
        assert not any(c.startswith("E") for c in _codes("set foo bar\nputs ${foo}\n"))

    def test_fix_holds_expansion_of_var(self):
        assert not any(
            c.startswith("E") for c in _codes("set lst {a b c}\nputs [llength {*}$lst]\n")
        )

    def test_true_positive_unterminated_brace_var(self):
        # tclsh 8.6/9.0:  missing close-brace for variable name
        # E206 maps 1:1 to that lexer message (compiler/parsing/recovery.py).
        assert _has("puts ${foo\n", "E206")
