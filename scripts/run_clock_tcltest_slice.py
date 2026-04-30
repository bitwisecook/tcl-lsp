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

This is a strict subset of what tcltest does (no -setup / -cleanup
/ -returnCodes machinery) but it covers the vast majority of
clock.test which is plain conversion-table assertions.

Output: per-section pass/fail count plus the first 10 mismatched
expectations for triage.
"""

from __future__ import annotations

import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

CLOCK_TEST = REPO / "tmp" / "tcl9.0.3" / "tests" / "clock.test"


def extract_test_blocks(content: str, name_pattern: str) -> list[tuple[str, str, str]]:
    """Pull ``(name, body, expected)`` triples from the upstream file.

    Only the *implicit* two-arg ``test NAME DESC BODY EXPECTED`` form
    is recognised (which is what the conversion-table tests use).
    Tests that use ``-body`` / ``-result`` option flags are skipped
    here — they're typically setup / cleanup-heavy and don't fit
    the bypass approach.
    """
    out: list[tuple[str, str, str]] = []
    rx = re.compile(rf"^test\s+({name_pattern})\s+\{{[^\n]*}}\s+\{{")
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = rx.match(line)
        if not m:
            i += 1
            continue
        name = m.group(1)
        # Body starts at the trailing ``{`` of this line.
        body_lines: list[str] = []
        depth = line.count("{") - line.count("}")
        body_start_idx = line.index("{", line.index("}") + 1)
        body_lines.append(line[body_start_idx + 1 :])
        i += 1
        while i < len(lines) and depth > 0:
            depth += lines[i].count("{") - lines[i].count("}")
            if depth > 0:
                body_lines.append(lines[i])
            else:
                # Closing brace line — emit prefix and break.
                end = lines[i].rindex("}")
                body_lines.append(lines[i][:end])
                break
            i += 1
        # Expected result is whatever follows the closing ``}`` on
        # the same line as the body terminator.  May be a literal
        # bare list or wrapped in ``{...}`` — strip one matching
        # outer pair if present.
        tail = lines[i][lines[i].rindex("}") + 1 :].strip()
        if tail.startswith("{") and tail.endswith("}"):
            tail = tail[1:-1]
        body = "\n".join(body_lines).strip()
        out.append((name, body, tail))
        i += 1
    return out


def make_runner(bodies: list[tuple[str, str]]) -> str:
    """Build a Tcl program that runs every ``(name, body)`` and prints
    ``NAME\tRESULT\n`` (errors prefixed with ``!``).
    """
    src_parts: list[str] = []
    for name, body in bodies:
        src_parts.append(
            "set _name {" + name + "}\n"
            "if {[catch {" + body + "} _r]} {\n"
            "    puts \"$_name\\t!$_r\"\n"
            "} else {\n"
            "    puts \"$_name\\t$_r\"\n"
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

    bodies = [(n, b) for n, b, _ in blocks]
    expected = {n: e for n, _, e in blocks}
    src = make_runner(bodies)
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
        want = expected[name]
        if got.startswith("!"):
            errored += 1
            failed += 1
            if len(mismatches) < 10:
                mismatches.append((name, want, got))
            continue
        if got == want:
            passed += 1
        else:
            failed += 1
            if len(mismatches) < 10:
                mismatches.append((name, want, got))

    missing = [n for n, _ in bodies if n not in seen]
    if missing:
        # Tests beyond the trap point — count as fail.
        failed += len(missing)

    total = len(blocks)
    pct = 100.0 * passed / max(total, 1)
    print(f"  {elapsed:.1f}s  Total {total} Passed {passed} Failed {failed} (errors {errored})  → {pct:.1f}%")
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
        print(f"  grand total: {grand_passed}/{grand_total} pass ({grand_failed} fail)  → {pct:.1f}%")
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
