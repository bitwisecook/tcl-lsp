"""Measure our ``clock format`` against upstream clock.test expected results.

We bypass tcltest entirely: the upstream harness hits a chain of
pre-existing runtime issues unrelated to the clock work
(``namespace import`` interaction with uplevel, ``lsearch -exact``
mis-comparing list-vs-scalar, ``"" is not writable`` from tcltest's
init).  Instead we parse each ``test NAME DESC body result`` block
out of clock.test, extract the ``clock format <epoch> -format <fmt>
-gmt true ?-locale en_US_roman?`` call from the body, and run that
single command through our WASM runtime — comparing the printed
output to the upstream expected string in pure Python.

Two test-block shapes are recognised:

1. ``test NAME DESC BODY EXPECTED`` — the conversion-table form (the
   bulk of ``clock-2`` / ``clock-3`` / ``clock-4``).
2. ``test NAME DESC -body BODY -result EXPECTED ?-match KIND? ...`` —
   the option-flag form used by ``clock-5`` / ``clock-7`` and a few
   stragglers in the conversion tables.  ``-match glob`` / ``-match
   regexp`` are honoured (vs. plain string equality); other tcltest
   options (``-setup`` / ``-cleanup`` / ``-constraints`` / ``-returnCodes``)
   are skipped.

Output: per-section pass/fail count plus the first 10 mismatched
expectations for triage.
"""

from __future__ import annotations

import fnmatch
import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

CLOCK_TEST = REPO / "tmp" / "tcl9.0.3" / "tests" / "clock.test"


def _strip_tcl_quoting(s: str) -> str:
    """Strip one layer of Tcl word quoting from *s*.

    Tcl's ``test`` body and result fields may be wrapped in either
    ``{...}`` (brace word — verbatim, no substitution) or ``"..."``
    (quoted word — substitution applied, but for our static parsing
    we treat it as plain text).  Either form's outermost pair is
    optional and tells us nothing about the *contents*, so we strip
    one matching outer pair off the input.
    """
    s = s.strip()
    if len(s) >= 2 and s[0] == "{" and s[-1] == "}":
        return s[1:-1]
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _balance_segment(text: str, start: int, opener: str, closer: str) -> int:
    """Return the index just past the matching ``closer`` for ``text[start]``.

    ``text[start]`` must be ``opener``.  Walks forward matching nested
    ``{...}`` (or whatever the pair is) and returns the index of the
    character one past the closing brace.  Used to extract the brace-
    delimited body and result fields of an option-flag ``test``
    invocation.
    """
    depth = 0
    i = start
    while i < len(text):
        c = text[i]
        if c == opener:
            depth += 1
        elif c == closer:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return -1


def _read_brace_word(text: str, start: int) -> tuple[str, int] | None:
    """Read one ``{...}`` brace-quoted word starting at ``text[start]``.

    Returns ``(contents, end)`` where ``end`` is the index just past
    the closing brace.  The outer braces are stripped from
    ``contents`` (the inner text is returned verbatim).
    """
    if start >= len(text) or text[start] != "{":
        return None
    end = _balance_segment(text, start, "{", "}")
    if end < 0:
        return None
    return text[start + 1 : end - 1], end


def _read_word(text: str, start: int) -> tuple[str, int] | None:
    """Read one Tcl-style word (brace-quoted, double-quoted, or bare).

    Returns ``(contents, end)`` or null if no word starts at ``start``.
    """
    if start >= len(text):
        return None
    c = text[start]
    if c == "{":
        return _read_brace_word(text, start)
    if c == '"':
        # Double-quoted: stop at the next unescaped ``"``.  Tcl backslash
        # escapes inside the quoted word are preserved verbatim — we
        # just need the bytes between the quotes.
        i = start + 1
        while i < len(text):
            if text[i] == "\\":
                i += 2
                continue
            if text[i] == '"':
                return text[start + 1 : i], i + 1
            i += 1
        return None
    # Bare word — stop at whitespace.
    i = start
    while i < len(text) and not text[i].isspace():
        i += 1
    return text[start:i], i


def _split_words(text: str) -> list[str]:
    """Split a single line's worth of Tcl words at the top brace level.

    Used to scan the ``test ... -opt val -opt val ...`` option list
    after the body has been consumed.
    """
    out: list[str] = []
    i = 0
    while i < len(text):
        # Skip whitespace and structural newlines.
        while i < len(text) and text[i] in " \t\n\\":
            i += 1
        if i >= len(text):
            break
        w = _read_word(text, i)
        if w is None:
            break
        word, i = w
        out.append(word)
    return out


class TestBlock:
    __slots__ = ("name", "body", "expected", "match_kind")

    def __init__(self, name: str, body: str, expected: str, match_kind: str = "exact"):
        self.name = name
        self.body = body
        self.expected = expected
        self.match_kind = match_kind  # "exact" | "glob" | "regexp"


def _references_outer_variable(body: str) -> bool:
    """Heuristic: does *body* reference a non-locally-defined variable?

    Tests inside ``foreach`` / ``set`` blocks (e.g. ``clock-6.10a$sign``)
    rely on a loop variable bound in the surrounding scope.  Our slice
    runner runs each body in isolation so those references would trap
    with ``no such variable``.  Filter them out at parse time.

    The check is intentionally narrow — it only flags ``$sign`` /
    ``${sign}`` and similar one-letter-or-name globals that aren't
    set anywhere in the body itself.  False positives would just
    drop a test from the slice; false negatives reintroduce the
    runtime trap.
    """
    # Find every variable reference $name / ${name}.
    refs = set(re.findall(r"\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?", body))
    if not refs:
        return False
    # Variables introduced by the body (set / foreach / lassign / ...).
    introduced: set[str] = set()
    for m in re.finditer(r"\b(?:set|lassign|foreach|incr)\s+([A-Za-z_][A-Za-z0-9_]*)", body):
        introduced.add(m.group(1))
    # Common loop / proc-arg names commonly used at the test-file
    # toplevel — refs to these are the trap-trigger.
    suspects = {"sign", "i", "j", "n", "x", "y", "z", "mon", "shm", "e", "s", "t", "res", "months"}
    leaked = refs - introduced
    return bool(leaked & suspects)


def extract_test_blocks(content: str, name_pattern: str) -> list[TestBlock]:
    """Pull test blocks from the upstream file.

    Returns a list of :class:`TestBlock` covering both the implicit
    four-arg form (``test NAME DESC BODY EXPECTED``) and the option-
    flag form (``test NAME DESC -body BODY -result EXPECTED ...``).

    Tests whose name itself depends on a substitution
    (``test clock-6.10a$sign``) are skipped — their bodies typically
    reference loop-bound variables that would trap our isolated
    runner.
    """
    out: list[TestBlock] = []
    # Match ``test NAME DESC ...``.  After the description (a brace
    # word) we either see ``{`` (implicit body) or ``-`` (option list).
    # The ``(?![A-Za-z0-9._-$])`` lookahead anchors the name boundary
    # so ``clock-6\.[0-9]+`` doesn't also catch ``clock-6.10a$sign``-
    # style names whose suffix is a Tcl substitution.  The trailing
    # ``\s+`` ensures we land on the description's first non-space
    # character.
    header_rx = re.compile(rf"^test\s+({name_pattern})(?![A-Za-z0-9._\-$])\s+")
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = header_rx.match(line)
        if not m:
            i += 1
            continue
        name = m.group(1)
        # Glue subsequent lines into one chunk via brace counting so
        # we can re-parse the test invocation as a single string.
        chunk_lines = [line]
        # Track braces only (Tcl doesn't allow ``{`` inside a comment
        # to count, but the test files don't use that pattern).
        depth = line.count("{") - line.count("}")
        j = i
        while depth != 0 and j + 1 < len(lines):
            j += 1
            chunk_lines.append(lines[j])
            depth += lines[j].count("{") - lines[j].count("}")
        chunk = "\n".join(chunk_lines)
        # Strip the leading ``test NAME DESC`` to leave just the
        # remainder (body+result OR ``-body ... -result ...``).
        rest = chunk[m.end() :]
        # Skip the description word.
        desc = _read_word(rest, 0)
        if desc is None:
            i = j + 1
            continue
        rest = rest[desc[1] :].lstrip()
        block = _parse_remainder(name, rest)
        if block is not None and not _references_outer_variable(block.body):
            out.append(block)
        i = j + 1
    return out


def _parse_remainder(name: str, rest: str) -> TestBlock | None:
    """Parse the ``BODY EXPECTED`` or ``-body B -result R ...`` tail."""
    if rest.startswith("-"):
        # Option-flag form.  Walk the option list collecting -body /
        # -result / -match.
        words = _split_words(rest)
        body = ""
        result = ""
        match_kind = "exact"
        idx = 0
        while idx + 1 < len(words):
            opt = words[idx]
            val = words[idx + 1]
            if opt == "-body":
                body = val
            elif opt == "-result":
                result = val
            elif opt == "-match":
                match_kind = val
            elif opt in {
                "-setup",
                "-cleanup",
                "-constraints",
                "-returnCodes",
                "-output",
                "-errorOutput",
            }:
                pass  # ignored
            idx += 2
        if not body:
            return None
        return TestBlock(name, body.strip(), result, match_kind)
    # Implicit form: ``BODY EXPECTED`` (4-arg) or
    # ``CONSTRAINTS BODY EXPECTED`` (5-arg).  Read the first brace
    # word; if it's a constraint-style list (single line, no leading
    # whitespace, no command keywords) and a second brace word
    # follows, the first was the constraints field.  Otherwise the
    # first is the body and whatever follows is the expected result.
    first_rd = _read_brace_word(rest, 0)
    if first_rd is None:
        return None
    first, after = first_rd
    next_rest = rest[after:].lstrip()
    if next_rest.startswith("{") and _looks_like_constraints(first):
        body_rd = _read_brace_word(next_rest, 0)
        if body_rd is None:
            return None
        body, after2 = body_rd
        tail = next_rest[after2:].strip()
        expected = _strip_tcl_quoting(tail)
        return TestBlock(name, body.strip(), expected, "exact")
    expected = _strip_tcl_quoting(next_rest)
    return TestBlock(name, first.strip(), expected, "exact")


def _looks_like_constraints(s: str) -> bool:
    """Heuristic: is *s* a tcltest constraints list rather than a body?

    Constraints are a single-line whitespace-separated list of
    identifier-like tokens (``detroit y2038``, ``unix pcOnly``).  A
    body, by contrast, almost always spans multiple lines and
    contains substitution / command syntax.  We use newline-presence
    as the discriminator with a tighter shape check on top — every
    token must start with an ASCII letter.
    """
    if "\n" in s:
        return False
    s = s.strip()
    if not s:
        return False
    for tok in s.split():
        if not tok or not (tok[0].isalpha() or tok[0] == "_"):
            return False
        for ch in tok:
            if not (ch.isalnum() or ch in "_-."):
                return False
    return True


def _eval_expected(expected: str) -> str:
    """Evaluate simple ``[list ...]`` / ``[lrepeat N ...]`` substitutions.

    The slice driver compares verbatim text against the captured
    runtime result.  When the upstream expected field is itself a
    Tcl construction (``[list -1 -2 -3]`` / ``[lrepeat 4 1 ...]``)
    a verbatim compare fails because the runtime emits the
    *substituted* value.  Recognise the two common patterns at
    parse time and pre-evaluate them in Python — anything else
    falls through unchanged.
    """
    s = expected.strip()
    if not (s.startswith("[") and s.endswith("]")):
        return expected
    inner = s[1:-1].strip()
    parts = inner.split(None, 1)
    if not parts:
        return expected
    cmd = parts[0]
    rest = parts[1] if len(parts) > 1 else ""
    if cmd == "list":
        elems = _split_tcl_words(rest)
        return " ".join(_list_quote(e) for e in elems)
    if cmd == "lrepeat":
        words = _split_tcl_words(rest)
        if len(words) < 2:
            return expected
        try:
            n = int(words[0])
        except ValueError:
            return expected
        body = words[1:]
        return " ".join(_list_quote(e) for e in body * n)
    return expected


def _list_quote(s: str) -> str:
    """Re-quote *s* as a single Tcl list element when it needs braces.

    Mirrors Tcl's ``Tcl_ConvertElement`` minimal form: an empty
    string becomes ``{}``, a string containing whitespace or special
    list-quoting characters gets brace-wrapped, otherwise the bare
    text is returned.  This is the exact transform the runtime
    applies when rendering a list, so pre-substituting expected
    text matches the captured stdout.
    """
    if not s:
        return "{}"
    if any(c in s for c in ' \t\n\r{}\\$[];"'):
        return "{" + s + "}"
    return s


def _split_tcl_words(text: str) -> list[str]:
    """Split *text* into Tcl words, stripping outer braces from each."""
    out: list[str] = []
    i = 0
    while i < len(text):
        while i < len(text) and text[i].isspace():
            i += 1
        if i >= len(text):
            break
        if text[i] == "{":
            depth = 1
            j = i + 1
            while j < len(text) and depth > 0:
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                j += 1
            out.append(text[i + 1 : j - 1])
            i = j
        elif text[i] == '"':
            j = i + 1
            while j < len(text) and text[j] != '"':
                if text[j] == "\\":
                    j += 2
                    continue
                j += 1
            out.append(text[i + 1 : j])
            i = j + 1
        else:
            j = i
            while j < len(text) and not text[j].isspace():
                j += 1
            out.append(text[i:j])
            i = j
    return out


def _matches(want: str, got: str, kind: str) -> bool:
    """Apply tcltest ``-match`` semantics."""
    if kind == "glob":
        return fnmatch.fnmatchcase(got, want)
    if kind == "regexp":
        try:
            return re.search(want, got) is not None
        except re.error:
            return False
    return got == want


def _normalise_body(body: str) -> str:
    """Trim trailing line-continuation backslashes that the brace
    extractor leaves behind.

    The implicit four-arg form ``test NAME DESC { body } expected``
    pulls the body text up to the ``}`` token; if the upstream
    formatting put a ``\\`` immediately before the closing brace
    (``... \\`` then ``}`` on the next line) the extracted body
    ends with a hanging ``\\`` which Tcl reads as "command
    continues on next line" — a fatal "incomplete command" trap
    in the slice runner.  Strip those tails before emitting.
    """
    s = body.rstrip()
    while s.endswith("\\"):
        s = s[:-1].rstrip()
    return s


def make_runner(blocks: list["TestBlock"]) -> str:
    """Build a Tcl program that runs each ``TestBlock`` and prints
    ``NAME\tRESULT\n`` (errors prefixed with ``!``).
    """
    src_parts: list[str] = []
    for blk in blocks:
        src_parts.append(
            "set _name {" + blk.name + "}\n"
            "if {[catch {" + _normalise_body(blk.body) + "} _r]} {\n"
            '    puts "$_name\\t!$_r"\n'
            "} else {\n"
            '    puts "$_name\\t$_r"\n'
            "}\n"
        )
    return "\n".join(src_parts)


def run_slice(name_pattern: str, label: str, *, max_tests: int = 0) -> dict:
    print(f"\n=== {label} ===")
    content = CLOCK_TEST.read_text(encoding="utf-8")
    blocks = extract_test_blocks(content, name_pattern)
    if max_tests > 0:
        blocks = blocks[:max_tests]
    if not blocks:
        print("  no tests matched")
        return {"label": label, "matched": 0}
    print(f"  matched {len(blocks)} two-arg test blocks")

    expected: dict[str, tuple[str, str]] = {
        blk.name: (blk.expected, blk.match_kind) for blk in blocks
    }
    src = make_runner(blocks)
    t0 = time.time()
    try:
        wasm = _compile_tcl(src)
    except Exception as exc:
        print(f"  COMPILE FAIL: {exc}")
        return {"label": label, "compile_error": str(exc)}
    print(f"  compiled in {time.time() - t0:.1f}s, wasm = {len(wasm):,} bytes")

    t0 = time.time()
    try:
        result = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
    except Exception as trap:
        elapsed = time.time() - t0
        stderr_text = getattr(trap, "tcl_stderr", "") or ""
        print(f"  TRAPPED after {elapsed:.1f}s")
        print("    " + stderr_text[-500:].replace("\n", "\n    "))
        return {"label": label, "trap": stderr_text[-200:]}
    elapsed = time.time() - t0
    stdout = result[1] if len(result) >= 2 else ""

    passed = 0
    failed = 0
    errored = 0
    mismatches: list[tuple[str, str, str]] = []
    seen = set()
    for line in stdout.splitlines():
        if "\t" not in line:
            continue
        name, _, got = line.partition("\t")
        if name not in expected:
            continue
        seen.add(name)
        want, kind = expected[name]
        if got.startswith("!"):
            errored += 1
            failed += 1
            if len(mismatches) < 10:
                mismatches.append((name, want, got))
            continue
        # Pre-evaluate ``[list …]`` / ``[lrepeat …]`` expected texts
        # so the comparison sees the substituted value (matching what
        # tcltest's ``-result`` does internally).
        want = _eval_expected(want)
        if _matches(want, got, kind):
            passed += 1
        else:
            failed += 1
            if len(mismatches) < 10:
                mismatches.append((name, want, got))

    missing = [blk.name for blk in blocks if blk.name not in seen]
    if missing:
        # Tests beyond the trap point — count as fail.
        failed += len(missing)

    total = len(blocks)
    pct = 100.0 * passed / max(total, 1)
    print(
        f"  {elapsed:.1f}s  Total {total} Passed {passed} Failed {failed} (errors {errored})  → {pct:.1f}%"
    )
    if mismatches:
        print("  first 10 mismatches:")
        for name, want, got in mismatches:
            w = want[:60] + ("…" if len(want) > 60 else "")
            g = got[:60] + ("…" if len(got) > 60 else "")
            print(f"    {name}\n      want: {w}\n      got : {g}")
    return {
        "label": label,
        "total": total,
        "passed": passed,
        "failed": failed,
        "errored": errored,
        "missing": len(missing),
    }


def main() -> None:
    slices = [
        # Pure conversion tables — these are the bulk of clock.test.
        ("clock-2\\.[0-9]+", "clock-2: gregorian conversion"),
        ("clock-3\\.[0-9]+", "clock-3: fiscal year/week/dow"),
        ("clock-4\\.[0-9]+", "clock-4: time-of-day"),
        ("clock-5\\.[0-9]+", "clock-5: DST + %z/%Z"),
        ("clock-6\\.[0-9]+", "clock-6: scan seconds"),
        ("clock-7\\.[0-9]+", "clock-7: scan Julian day"),
        ("clock-8\\.[0-9]+", "clock-8: scan ccyymmdd"),
    ]
    summary = []
    for pat, lbl in slices:
        # Cap each slice at 200 tests so the harness finishes in
        # < 5 minutes total.  Random-sampling by index would be
        # better; 200 is enough to characterise the pass-rate.
        summary.append(run_slice(pat, lbl, max_tests=200))
    print("\n=== Summary ===")
    grand_total = sum(s.get("total", 0) for s in summary)
    grand_passed = sum(s.get("passed", 0) for s in summary)
    grand_failed = sum(s.get("failed", 0) for s in summary)
    if grand_total:
        pct = 100.0 * grand_passed / grand_total
        print(
            f"  grand total: {grand_passed}/{grand_total} pass ({grand_failed} fail)  → {pct:.1f}%"
        )
    for s in summary:
        if "trap" in s:
            print(f"  {s['label']}: TRAPPED")
        elif "compile_error" in s:
            print(f"  {s['label']}: compile error")
        elif s.get("matched") == 0:
            print(f"  {s['label']}: 0 tests matched")
        else:
            t = s.get("total", 0)
            p = s.get("passed", 0)
            f = s.get("failed", 0)
            print(f"  {s['label']}: {p}/{t} pass, {f} fail")


if __name__ == "__main__":
    main()
