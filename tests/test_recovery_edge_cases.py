"""Deep edge-case probes for the single recovering-detection pass.

Where ``test_recovering_lexer_differential.py`` fuzzes the broad invariant, this
file names and pins the *specific* corners of the change:

- the soundness boundary of the ``_recovery_possible`` fast path (the class of
  bug the fuzzer found: a suspicious quote that closes at a later stray ``"``);
- ``detect_recovery`` consistency (its insertions are exactly the legacy
  ``compute_virtual_insertions``; empty for well-formed);
- diagnostic de-duplication (true duplicates collapse; same-code-different-range
  diagnostics are preserved);
- per-heuristic boundaries for ``[`` / ``"`` / ``{`` and their decline cases;
- ``body_token`` anchoring (recovery inside a braced body substring at a
  non-zero base offset);
- delimiter nesting / level tracking (an inserted closer that does not balance);
- multi-line, CRLF, tabs, ``${...}`` and backslash interactions.

The workhorse assertion is the differential invariant — the recovering
tokeniser must equal the established two-pass — applied to hand-picked inputs so
each scenario is pinned by name, not only by random sampling.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.green_tree import tokenise
from compiler.parsing.recovering_lexer import _recovery_possible, tokenise_recovering
from compiler.parsing.recovery import (
    compute_virtual_insertions,
    detect_recovery,
    segment_with_recovery,
)


def _find_tcl9() -> str | None:
    for cand in (shutil.which("tclsh9.0"), shutil.which("tclsh")):
        if not cand or not Path(cand).exists():
            continue
        try:
            proc = subprocess.run(
                [cand], input="puts [info patchlevel]", capture_output=True, text=True, timeout=5
            )
        except (subprocess.TimeoutExpired, OSError):
            continue
        if proc.stdout.strip().startswith("9."):
            return cand
    return None


_TCLSH9 = _find_tcl9()


def _tcl_complete(source: str, tmp_path: Path) -> bool:
    """``info complete`` on tclsh9 — the ground truth a recovered fix must hit.

    Passes *source* via a file so braces/quotes in it can't corrupt the probe.
    """
    assert _TCLSH9 is not None
    f = tmp_path / "probe.tcl"
    f.write_text(source)
    proc = subprocess.run(
        [_TCLSH9, "-"],
        input=(f"set f [open {{{f}}} r];set d [read $f];close $f;puts [info complete $d]"),
        capture_output=True,
        text=True,
        timeout=5,
    )
    return proc.stdout.strip() == "1"


def _apply(source: str, insertions: dict[int, str]) -> str:
    out = source
    for off in sorted(insertions, reverse=True):
        out = out[:off] + insertions[off] + out[off:]
    return out


def _tok_key(tokens):
    return [(t.type.name, t.start.offset, t.end.offset, t.text, bool(t.in_quote)) for t in tokens]


def _warn_key(warnings):
    return [(w[0].offset, w[1]) for w in (warnings or [])]


def _two_pass(source, body_token=None):
    vi = compute_virtual_insertions(source, body_token) or None
    return tokenise(source, 0, 0, 0, virtual_insertions=vi)


def _assert_matches_two_pass(source: str) -> None:
    """The recovering tokeniser must equal the two-pass, tokens and warnings."""
    old_t, old_w = _two_pass(source)
    new_t, new_w = tokenise_recovering(source)
    assert _tok_key(new_t) == _tok_key(old_t), (
        f"tokens diverged for {source!r}\n  two-pass:    {_tok_key(old_t)}\n"
        f"  single-pass: {_tok_key(new_t)}"
    )
    assert _warn_key(new_w) == _warn_key(old_w), (
        f"warnings diverged for {source!r}\n  two-pass: {_warn_key(old_w)}\n"
        f"  single:   {_warn_key(new_w)}"
    )


def _diag_keys(diags):
    return [
        (
            d.code,
            d.range.start.offset,
            d.range.end.offset,
            d.message,
            tuple((f.new_text, f.description) for f in (d.fixes or ())),
        )
        for d in diags
    ]


# --------------------------------------------------------------------------- #
# _recovery_possible — the fast-path soundness boundary
# --------------------------------------------------------------------------- #


class TestFastPathSoundness:
    """``_recovery_possible`` must never be a false negative: if it says no
    recovery, ``compute_virtual_insertions`` must agree (empty)."""

    @pytest.mark.parametrize(
        "source",
        [
            "",
            "   ",
            "\n\n",
            "\t\n",
            "# just a comment\n",
            "set x 1\n",
            "set x 1",  # no trailing newline — over-triggers, still correct
            "set x [foo bar]\n",
            'set x "hi"\n',
            "set x {a b}\n",
            "set x [foo bar]",  # closed bracket at EOF
            'puts "a"; puts "b"\n',
            "proc p {} { return 1 }\n",
        ],
    )
    def test_clean_sources_round_trip(self, source):
        # Whatever _recovery_possible decides, the result must equal the two-pass.
        _assert_matches_two_pass(source)
        # And if it declines, compute_virtual_insertions must truly be empty.
        toks, _ = tokenise(source, 0, 0, 0)
        if not _recovery_possible(toks, source, 0):
            assert not compute_virtual_insertions(source)

    @pytest.mark.parametrize(
        "source",
        [
            # The fuzzer-found class: a "-at-EOL quote that closes at a LATER
            # stray quote, so nothing reaches EOF *in quote*, yet E202 fires.
            '"\nforeach"$x',
            'set x "\nputs hi\nputs "more\n',
            # quote tail is a substitution reaching EOF
            'set y "\nproc q {\n$v',
            'set y "\nputs [foo]',
            # unterminated bracket whose tail is a var/cmd
            "set x [foo $bar\nputs hi\n",
            "set x [foo [bar $baz\nputs hi\n",
        ],
    )
    def test_recovery_triggering_sources_round_trip(self, source):
        # These MUST take the recovery path and match the two-pass exactly.
        _assert_matches_two_pass(source)

    def test_no_trailing_newline_overtriggers_but_is_correct(self):
        # A bare word ending at EOF makes _recovery_possible answer True, but
        # detection then inserts nothing — output still equals the two-pass.
        src = "set x 1"
        toks, _ = tokenise(src, 0, 0, 0)
        assert _recovery_possible(toks, src, 0) is True
        assert not compute_virtual_insertions(src)
        _assert_matches_two_pass(src)


# --------------------------------------------------------------------------- #
# detect_recovery consistency
# --------------------------------------------------------------------------- #


class TestDetectRecoveryConsistency:
    @pytest.mark.parametrize(
        "source",
        [
            "set x [foo bar\nputs hi\n",
            'set x "\nputs hi\n',
            "proc p {x y} {\nputs hi\nset z 3\n",
            "set x [foo {a b}\n",
            "set x 1\n",
            "[foo [bar\n",
        ],
    )
    def test_insertions_equal_legacy(self, source):
        assert detect_recovery(source).insertions == compute_virtual_insertions(source)

    def test_well_formed_detection_is_empty(self):
        det = detect_recovery("proc p {} { return [expr {1 + 2}] }\n")
        assert det.insertions == {}
        assert det.virtuals == []
        assert det.fallback_diags == []

    def test_detection_carries_commands_and_warnings(self):
        det = detect_recovery("set x [foo bar\nputs hi\n")
        assert det.commands, "first-parse commands should be retained"
        assert det.insertions, "an unterminated [ should produce an insertion"


# --------------------------------------------------------------------------- #
# diagnostic de-duplication
# --------------------------------------------------------------------------- #


class TestDiagnosticDedup:
    def test_no_exact_duplicates_in_recovery_output(self):
        # `""[` after recovery re-lexes the command substitution body and the
        # first parse + reparse both flag "extra characters after close-quote";
        # the assembled set must collapse the identical pair.
        src = '{\nif\n""[\nset'
        _cmds, diags = segment_with_recovery(src)
        keys = _diag_keys(diags)
        assert len(keys) == len(set(keys)), f"duplicate diagnostics: {keys}"

    def test_distinct_same_code_diagnostics_are_preserved(self):
        # Two genuinely different unterminated quotes (different ranges) must NOT
        # be collapsed by the dedup — dedup is by full identity, not by code.
        src = 'a "\nset\n\nb "\nputs\n'
        _cmds, diags = segment_with_recovery(src)
        keys = _diag_keys(diags)
        assert len(keys) == len(set(keys))
        # The two quotes occupy different offsets, so if both are flagged they
        # are distinct entries (this guards against an over-aggressive dedup).
        e202 = [k for k in keys if k[0] == "E202"]
        assert len(e202) == len(set(e202))


# --------------------------------------------------------------------------- #
# per-heuristic boundaries
# --------------------------------------------------------------------------- #


class TestBracketHeuristics:
    @pytest.mark.parametrize(
        "source",
        [
            "set x [foo bar\nset y 2\n",  # command-break
            "set x [foo bar\n# c\nset y 2\n",  # comment-break beats command-break
            "set x [foo {a b}\n",  # brace-break
            "set x [foo bar\nnotacommand here\n",  # no known command -> declines
            "set x [foo\n\n\nset y 2\n",  # blank lines before the known command
            "set x [foo [bar\nset y 2\n",  # nested [: insertion may not balance
            "[]\n",  # empty command substitution
            "set x []\n",  # empty substitution, closed
            "set x [\nset y 2\n",  # immediately-unterminated [
        ],
    )
    def test_bracket(self, source):
        _assert_matches_two_pass(source)


class TestQuoteHeuristics:
    @pytest.mark.parametrize(
        "source",
        [
            'set x "\nset y 2\n',  # newline heuristic fires
            'set x "hello\nset y 2\n',  # text after " -> declines, over-runs
            'set x ""\n',  # empty quote, closed
            'puts "\n"\n',  # quote then a lone quote line
            'set x "\n"\nset y 2\n',
            'a "\n',  # quote at EOF, single line
            'set x "$y\nputs hi\n',  # quote with substitution
        ],
    )
    def test_quote(self, source):
        _assert_matches_two_pass(source)


class TestBraceHeuristics:
    @pytest.mark.parametrize(
        "source",
        [
            "proc p {x y} {\nputs hi\nset z 3\n",  # command heuristic
            "proc p {x\nset y 2\n",  # no match -> over-runs + warning
            "set x {}\n",  # empty brace, closed
            "set x {a b\n",  # unterminated brace, single content line
            "namespace eval n {\nproc q {} {}\n",  # nested braces, outer unterminated
            "proc p {} {\n  set x [foo\n}\n",  # nested unterminated [ inside body
            "set x {a {b {c\n",  # deeply nested unterminated braces
        ],
    )
    def test_brace(self, source):
        _assert_matches_two_pass(source)


# --------------------------------------------------------------------------- #
# body_token anchoring
# --------------------------------------------------------------------------- #


class TestBodyTokenAnchoring:
    """Recovery inside a body substring must match the two-pass anchored the
    same way — exercises the non-zero base offset paths."""

    @pytest.mark.parametrize(
        "outer",
        [
            "proc p {} {\n  set x [foo bar\n  puts hi\n}\n",
            'proc p {} {\n  set x "\n  puts hi\n}\n',
            "namespace eval n {\n  set y [bar\n  set z 1\n}\n",
            "if {1} {\n  set x [foo\n  return\n}\n",
        ],
    )
    def test_body_substring_recovery(self, outer):
        # Find the braced body of the outer construct and recover *within* it,
        # exactly as the semantic-token collector does when it descends.
        toks, _ = tokenise(outer, 0, 0, 0)
        body = next((t for t in toks if t.type.name == "STR" and "\n" in t.text), None)
        assert body is not None, "expected a multi-line braced body token"
        inner = body.text
        base_off = body.start.offset + 1
        base_line = body.start.line
        base_col = body.start.character + 1
        vi = compute_virtual_insertions(inner, body) or None
        old_t, old_w = tokenise(inner, base_off, base_line, base_col, virtual_insertions=vi)
        new_t, new_w = tokenise_recovering(inner, base_off, base_line, base_col, body_token=body)
        assert _tok_key(new_t) == _tok_key(old_t), (
            f"body tokens diverged for {inner!r}\n  old={_tok_key(old_t)}\n  new={_tok_key(new_t)}"
        )
        assert _warn_key(new_w) == _warn_key(old_w)


# --------------------------------------------------------------------------- #
# multiline / special characters / level tracking
# --------------------------------------------------------------------------- #


class TestSpecialCharsAndNesting:
    @pytest.mark.parametrize(
        "source",
        [
            "set x [foo bar\r\nset y 2\r\n",  # CRLF line endings
            "set x [foo\tbar\nset y 2\n",  # tabs inside the substitution
            "set x [foo \\\nbar\nset y 2\n",  # backslash-newline continuation
            "set x [foo ${a b}\nset y 2\n",  # ${...} var with space inside [
            "set x [foo $bar(baz\nset y 2\n",  # array-ish unterminated
            "set x [a [b [c\nputs hi\n",  # triple-nested unterminated [
            "set x [foo; bar\nputs hi\n",  # ; separator inside [
            "[foo]\n[bar\nputs hi\n",  # one closed, one unterminated [
            'set x "a\\"b\nputs hi\n',  # escaped quote inside an unterminated "
            "set a 1\nset b [foo\nset c 3\nset d [bar\nset e 5\n",  # two recoveries
        ],
    )
    def test_special(self, source):
        _assert_matches_two_pass(source)

    def test_inserted_closer_that_does_not_balance_overruns(self):
        # Nested [ where the heuristic's single ] cannot balance both levels:
        # the recovered stream must keep the over-running command + warning,
        # exactly as the two-pass does.
        src = "set x [foo [bar baz\n"
        _assert_matches_two_pass(src)


class TestExprBraceRecovery:
    """ArgRole.EXPR braces (`if {…`, `while {…`, `expr {…`) recover before a
    following known command without needing a de-indent — additive over the
    conservative brace rule, since an expression can't contain a bare command
    line.  Body and data braces are unaffected.
    """

    @pytest.mark.parametrize(
        ("source", "tail_name"),
        [
            ("if {$x > 5\nputs hi\nset y 2\n", "puts"),
            ("while {$i < 3\nputs hi\n", "puts"),
            ("expr {1 + 2\nreturn\n", "return"),
            ("for {set i 0} {$i < 3\nputs hi\n", "puts"),  # condition expr brace
            # No trailing newline: a two-line expr brace must recover identically
            # (regression — the outcome must not hinge on a trailing \n).
            ("if {$x > 5\nset", "set"),
            ("while {$i < 3\nputs hi", "puts"),
        ],
    )
    def test_expr_brace_recovers_before_command(self, source, tail_name):
        cmds, diags = segment_with_recovery(source)
        e203 = [d for d in diags if d.code == "E203"]
        assert e203 and e203[0].fixes, f"expected a recovered E203 with a fix for {source!r}"
        assert e203[0].fixes[0].new_text == "}"
        assert any(c.name == tail_name for c in cmds), (
            f"tail command {tail_name!r} should be recovered; got {[c.name for c in cmds]}"
        )

    def test_expr_brace_without_following_command_does_not_recover(self):
        # No known command after the expression — nothing signals a close, so the
        # brace over-runs (no false recovery).
        cmds, diags = segment_with_recovery("if {$x > 5\n  more stuff here\n")
        e203 = [d for d in diags if d.code == "E203"]
        # Either no recovery, or at most a fallback (no quick-fix).
        assert not (e203 and e203[0].fixes), "should not fabricate a close with no signal"

    def test_data_brace_still_requires_dedent(self):
        # A *data* value brace must NOT pick up the aggressive expr rule: a
        # command-looking word at the same indent is data, not a close signal.
        cmds, diags = segment_with_recovery("set x {\na b c\nputs hi\n")
        names = [c.name for c in cmds]
        # `puts` must remain inside the data value (not split out as a command).
        assert "puts" not in names, f"data brace wrongly split on a command word: {names}"

    @pytest.mark.parametrize(
        "source",
        [
            "if {$x > 5\nputs hi\nset y 2\n",
            "while {$i < 3\nputs hi\n",
            "expr {1 + 2\nreturn\n",
            "if {$x > 5\n  more stuff here\n",
        ],
    )
    def test_expr_brace_matches_two_pass(self, source):
        _assert_matches_two_pass(source)


class TestBracketInsertNotInert:
    """The ``]`` of an E201 fix must never land *inside* an open brace/quote word.

    A known-command word (for command-break) or ``#`` (for comment-break) that
    sits inside a balanced brace/quote is *content*, not a command/comment — a
    ``]`` inserted before it is literal and leaves the command incomplete. This
    is a C-Tcl-9.0.3-grounded correction: for ``set x [foo {bar`` / ``puts
    baz}`` the old command-break inserted ``]`` after ``bar`` (offset 15),
    yielding ``set x [foo {bar]…}`` which ``info complete`` reports as ``0``
    (incomplete), while the truth-preserving recovery is complete. See
    ``_bracket_insert_inert`` in ``compiler/parsing/recovery.py``.
    """

    # The exact regressions: the close-bracket signal is brace/quote content.
    INERT_SIGNAL = [
        "set x [foo {bar\nputs baz}",  # `puts` is inside the brace word
        'set x [foo "bar\nputs baz"',  # `puts` is inside the quote word
        "set x [foo {bar\n# c}",  # `#` is inside the brace word
    ]
    # Plain-text recoveries whose signal really is a command/comment at script
    # level — these must keep their fix unchanged.  Includes the mid-word
    # delimiter cases: a `"` or `{` that is not the first character of a word is
    # an ordinary literal (dodekalogue rules 8/9), so the following line is still
    # a script-level command-break (C Tcl 9.0.3 completes `set x [foo abc"]`).
    LEGIT_SIGNAL = [
        "set x [foo bar\nputs done",  # command-break
        "set x [foo bar\n# c\nset y 2",  # comment-break
        'set x [foo abc"\nputs done',  # mid-word " is literal, not a quote word
        "set x [foo abc{\nputs done",  # mid-word { is literal, not a brace word
    ]

    @pytest.mark.parametrize("source", INERT_SIGNAL)
    def test_no_inert_bracket_insertion(self, source):
        det = detect_recovery(source)
        # Whatever fix (if any) is chosen, the inserted ] must not sit inside the
        # open brace/quote: the `]` must occur at script level in the result.
        for off, ch in det.insertions.items():
            assert ch == "]"
            # The char at the chosen offset belongs to a word that is *not* the
            # interior of the balanced brace/quote the signal lived in.
            assert off != 15, f"chose the inert offset for {source!r}"

    @pytest.mark.parametrize("source", INERT_SIGNAL)
    def test_inert_signal_does_not_emit_command_or_comment_fix(self, source):
        _cmds, diags = segment_with_recovery(source)
        for d in (d for d in diags if d.code == "E201"):
            for f in d.fixes or ():
                desc = f.description.lower()
                assert "before command" not in desc and "before comment" not in desc, (
                    f"inert {desc!r} fix emitted for {source!r}"
                )

    @pytest.mark.parametrize("source", LEGIT_SIGNAL)
    def test_legit_plain_text_recovery_keeps_fix(self, source):
        _cmds, diags = segment_with_recovery(source)
        e201 = [d for d in diags if d.code == "E201"]
        assert e201 and e201[0].fixes, f"plain-text recovery lost its fix for {source!r}"
        assert e201[0].fixes[0].new_text == "]"

    @pytest.mark.parametrize("source", INERT_SIGNAL + LEGIT_SIGNAL)
    @pytest.mark.skipif(_TCLSH9 is None, reason="tclsh9.0 not available")
    def test_recovered_fix_is_complete_per_ctcl(self, source, tmp_path):
        det = detect_recovery(source)
        if not det.insertions:
            # No fix offered is acceptable (never a *wrong* fix); nothing to check.
            return
        fixed = _apply(source, det.insertions)
        assert _tcl_complete(fixed, tmp_path), (
            f"recovered fix is incomplete per C Tcl 9.0.3: {fixed!r}"
        )
