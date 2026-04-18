#!/usr/bin/env python3
"""Query reference test results captured from real tclsh.

Reads from tests/test_reference/<version>/*.results files produced by
scripts/capture_reference_test_results.sh.

Usage:
    python3 test_results.py status
    python3 test_results.py summary [VERSION]
    python3 test_results.py suite SUITE [VERSION]
    python3 test_results.py group PATTERN [VERSION]
    python3 test_results.py lookup TEST_NAME [VERSION]
    python3 test_results.py compare SUITE [VERSION]
    python3 test_results.py source TEST_NAME [VERSION]
"""

from __future__ import annotations

import fnmatch
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent
REF_DIR = REPO_ROOT / "tests" / "test_reference"


# Data model


@dataclass
class TestResult:
    name: str
    status: str  # PASS, SKIP, FAIL
    reason: str = ""  # skip reason / constraint


@dataclass
class SuiteResult:
    file: str
    version: str
    tclsh: str = ""
    date: str = ""
    total: int = 0
    passed: int = 0
    skipped: int = 0
    failed: int = 0
    tests: list[TestResult] = field(default_factory=list)


def parse_results_file(path: Path) -> SuiteResult:
    """Parse a .results file into a SuiteResult."""
    sr = SuiteResult(file=path.stem + ".test", version=path.parent.name)
    section = "header"

    for line in path.read_text().splitlines():
        if section == "header":
            if line.startswith("TCLSH"):
                sr.tclsh = line.split(None, 1)[1] if len(line.split(None, 1)) > 1 else ""
            elif line.startswith("VERSION"):
                sr.version = line.split(None, 1)[1] if len(line.split(None, 1)) > 1 else ""
            elif line.startswith("FILE"):
                sr.file = line.split(None, 1)[1] if len(line.split(None, 1)) > 1 else ""
            elif line.startswith("DATE"):
                sr.date = line.split(None, 1)[1] if len(line.split(None, 1)) > 1 else ""
            elif line == "---":
                section = "tests"
        elif section == "tests":
            if line == "---":
                section = "summary"
            elif line.startswith("PASS "):
                name = line[5:].strip()
                sr.tests.append(TestResult(name=name, status="PASS"))
            elif line.startswith("SKIP "):
                parts = line[5:].split(None, 1)
                name = parts[0]
                reason = parts[1] if len(parts) > 1 else ""
                sr.tests.append(TestResult(name=name, status="SKIP", reason=reason))
            elif line.startswith("FAIL "):
                name = line[5:].strip()
                sr.tests.append(TestResult(name=name, status="FAIL"))
        elif section == "summary":
            if line.startswith("TOTAL"):
                sr.total = int(line.split()[1])
            elif line.startswith("PASSED"):
                sr.passed = int(line.split()[1])
            elif line.startswith("SKIPPED"):
                sr.skipped = int(line.split()[1])
            elif line.startswith("FAILED"):
                sr.failed = int(line.split()[1])

    return sr


# Discovery


def available_versions() -> list[str]:
    """Return sorted list of versions that have reference data."""
    if not REF_DIR.is_dir():
        return []
    return sorted(d.name for d in REF_DIR.iterdir() if d.is_dir() and any(d.glob("*.results")))


def available_suites(version: str) -> list[str]:
    """Return sorted list of suite names for a version."""
    vdir = REF_DIR / version
    if not vdir.is_dir():
        return []
    return sorted(p.stem for p in vdir.glob("*.results"))


def load_suite(suite: str, version: str) -> SuiteResult | None:
    """Load a suite's results for a given version."""
    # Accept "compExpr-old" or "compExpr-old.test"
    stem = suite.replace(".test", "")
    path = REF_DIR / version / f"{stem}.results"
    if not path.is_file():
        return None
    return parse_results_file(path)


def load_all_suites(version: str) -> list[SuiteResult]:
    """Load all suites for a version."""
    results = []
    for name in available_suites(version):
        sr = load_suite(name, version)
        if sr:
            results.append(sr)
    return results


def resolve_version(version: str | None) -> str:
    """Resolve version argument, defaulting to best available."""
    if version:
        return version
    versions = available_versions()
    # Prefer 9.0 > 8.6 > 8.5 > 8.4
    for pref in ["9.0", "8.6", "8.5", "8.4"]:
        if pref in versions:
            return pref
    if versions:
        return versions[-1]
    print("ERROR: No reference test data found.", file=sys.stderr)
    print(f"  Expected in: {REF_DIR}/", file=sys.stderr)
    print("  Run: ./scripts/capture_reference_test_results.sh", file=sys.stderr)
    sys.exit(1)


# Test source extraction


def find_tests_dir(version: str) -> Path | None:
    """Find the Tcl test source directory for a major.minor version.

    Searches the same locations as the capture scripts:
    1. /usr/src/tcl<version>*/tests/
    2. REPO_ROOT/tmp/tcl<version>*/tests/
    3. ~/src/tcl<version>*/tests/
    """
    import glob as globmod

    home = Path.home()

    # /usr/src/tcl<version>*/tests/
    for d in sorted(globmod.glob(f"/usr/src/tcl{version}*/tests")):
        p = Path(d)
        if p.is_dir():
            return p

    # REPO_ROOT/tmp/tcl<version>*/tests/
    for d in sorted((REPO_ROOT / "tmp").glob(f"tcl{version}*/tests")):
        if d.is_dir():
            return d

    # ~/src/tcl<version>*/tests/
    for d in sorted(home.glob(f"src/tcl{version}*/tests")):
        if d.is_dir():
            return d

    return None


def _tcl_brace_depth_delta(line: str) -> int:
    """Count net brace depth change in a line, ignoring escaped braces."""
    delta = 0
    i = 0
    while i < len(line):
        ch = line[i]
        if ch == "\\" and i + 1 < len(line):
            i += 2  # skip escaped char
            continue
        if ch == "{":
            delta += 1
        elif ch == "}":
            delta -= 1
        i += 1
    return delta


def extract_test_source(test_file: Path, test_name: str) -> dict | None:
    """Extract a single test command from a .test file.

    Returns a dict with keys:
    - raw: verbatim test command text
    - line: 1-based line number where the test starts
    Or None if the test is not found.
    """
    content = test_file.read_text()
    lines = content.splitlines()

    # Find the line that starts the test command.
    # Patterns: "test <name> " or "test <name>\t" at start of line
    # (possibly with leading whitespace).
    start_idx = None
    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if stripped.startswith(f"test {test_name} ") or stripped.startswith(f"test {test_name}\t"):
            start_idx = i
            break

    if start_idx is None:
        return None

    # Accumulate lines until brace depth returns to 0.
    # We track depth from the start of the test command.
    collected: list[str] = []
    depth = 0
    for i in range(start_idx, len(lines)):
        line = lines[i]
        collected.append(line)
        depth += _tcl_brace_depth_delta(line)

        # The command is complete when depth returns to 0 and we have
        # at least the description (which means at least 1 opening brace
        # was seen and closed).
        if depth <= 0 and len(collected) > 1:
            break
        # Also handle single-line tests (depth goes up and back to 0)
        if depth <= 0 and len(collected) == 1 and "{" in line:
            break

    raw = "\n".join(collected)
    return {"raw": raw, "line": start_idx + 1}


def parse_test_options(raw: str) -> dict:
    """Parse a test command's raw text into structured options.

    Handles both forms:
    - Short: test name desc body result
    - Long:  test name desc -option value ...

    Returns dict with keys: name, description, body, result, match,
    returnCodes, setup, cleanup, constraints, output, errorOutput.
    """
    opts: dict[str, str] = {
        "name": "",
        "description": "",
        "body": "",
        "result": "",
        "match": "exact",
        "returnCodes": "ok",
        "setup": "",
        "cleanup": "",
        "constraints": "",
        "output": "",
        "errorOutput": "",
    }

    # Extract the test name from the first token after "test"
    stripped = raw.lstrip()
    if not stripped.startswith("test "):
        return opts

    # Parse using brace-aware tokenisation.
    # Skip "test " prefix, then extract tokens.
    rest = stripped[5:]
    tokens = _tcl_tokenise(rest)

    if len(tokens) < 2:
        return opts

    opts["name"] = tokens[0]
    opts["description"] = tokens[1]

    # Determine form: if token[2] starts with "-", it's the long form
    remaining = tokens[2:]
    if remaining and remaining[0].startswith("-"):
        # Long form: -option value pairs
        i = 0
        while i < len(remaining) - 1:
            key = remaining[i].lstrip("-")
            val = remaining[i + 1]
            if key in opts:
                opts[key] = val
            i += 2
    elif len(remaining) >= 1:
        # Short form: body [result]
        # Could also be: constraints body result (3 positional args)
        if len(remaining) == 3:
            opts["constraints"] = remaining[0]
            opts["body"] = remaining[1]
            opts["result"] = remaining[2]
        elif len(remaining) == 2:
            opts["body"] = remaining[0]
            opts["result"] = remaining[1]
        elif len(remaining) == 1:
            opts["body"] = remaining[0]

    return opts


def _tcl_tokenise(text: str) -> list[str]:
    """Tokenise Tcl-like text into brace/quote-delimited tokens.

    Handles brace-delimited ({...}), quote-delimited ("..."), and bare words.
    Returns the content inside delimiters (braces/quotes stripped).
    """
    tokens: list[str] = []
    i = 0
    text = text.strip()

    while i < len(text):
        # Skip whitespace
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break

        if text[i] == "{":
            # Brace-delimited: find matching close brace
            depth = 0
            start = i
            while i < len(text):
                ch = text[i]
                if ch == "\\" and i + 1 < len(text):
                    i += 2
                    continue
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                i += 1
            # Strip outer braces
            token = text[start + 1 : i - 1]
            tokens.append(token)
        elif text[i] == '"':
            # Quote-delimited
            i += 1
            start = i
            while i < len(text):
                if text[i] == "\\" and i + 1 < len(text):
                    i += 2
                    continue
                if text[i] == '"':
                    break
                i += 1
            tokens.append(text[start:i])
            i += 1  # skip closing quote
        elif text[i] == "-":
            # Option flag: read until whitespace
            start = i
            while i < len(text) and text[i] not in " \t\n\r":
                i += 1
            tokens.append(text[start:i])
        else:
            # Bare word
            start = i
            while i < len(text) and text[i] not in " \t\n\r":
                i += 1
            tokens.append(text[start:i])

    return tokens


def extract_file_context(test_file: Path, test_raw: str) -> str:
    """Extract relevant file-level setup from a .test file.

    Scans from the top of the file until the first 'test ' command,
    collecting proc definitions, testConstraint calls, and variable
    setup. Only includes items whose identifiers appear in test_raw.
    """
    lines = test_file.read_text().splitlines()
    preamble_end = 0
    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if stripped.startswith("test ") and not stripped.startswith("testConstraint"):
            preamble_end = i
            break
    else:
        preamble_end = len(lines)

    preamble = lines[:preamble_end]

    # Extract blocks: proc definitions, testConstraint, set/unset at top level.
    # Skip tcltest loading boilerplate and comments.
    blocks: list[str] = []
    i = 0
    while i < len(preamble):
        line = preamble[i]
        stripped = line.lstrip()

        # Skip comments and blank lines
        if not stripped or stripped.startswith("#"):
            i += 1
            continue

        # Skip tcltest boilerplate
        if any(
            kw in stripped
            for kw in [
                "package require tcltest",
                "namespace import",
                "namespace children",
                "::tcltest::loadTestedCommands",
                "package require -exact",
                "package require Tcltest",
            ]
        ):
            # Skip this block (may span multiple lines)
            depth = _tcl_brace_depth_delta(line)
            i += 1
            while depth > 0 and i < len(preamble):
                depth += _tcl_brace_depth_delta(preamble[i])
                i += 1
            continue

        # Collect this block (may span multiple lines via braces)
        block_lines = [line]
        depth = _tcl_brace_depth_delta(line)
        i += 1
        while depth > 0 and i < len(preamble):
            block_lines.append(preamble[i])
            depth += _tcl_brace_depth_delta(preamble[i])
            i += 1

        blocks.append("\n".join(block_lines))

    # Filter: only include blocks whose key identifiers appear in the test
    relevant: list[str] = []
    for block in blocks:
        stripped = block.lstrip()
        # Extract identifier from proc/testConstraint/set/unset
        ident = None
        if stripped.startswith("proc "):
            parts = stripped.split(None, 2)
            if len(parts) >= 2:
                ident = parts[1]
        elif stripped.startswith("testConstraint "):
            parts = stripped.split(None, 2)
            if len(parts) >= 2:
                ident = parts[1]
        elif stripped.startswith("set ") or stripped.startswith("unset "):
            parts = stripped.split(None, 2)
            if len(parts) >= 2:
                ident = parts[1]

        if ident and ident in test_raw:
            relevant.append(block)
        elif ident is None:
            # Can't determine identifier; include if any word from
            # the block appears in the test (conservative)
            pass

    return "\n\n".join(relevant)


def _find_test_suite(test_name: str, version: str) -> str | None:
    """Find which suite file a test belongs to, using reference results."""
    suites = load_all_suites(version)
    for sr in suites:
        for t in sr.tests:
            if t.name == test_name:
                return sr.file
    return None


# Commands


def cmd_status() -> None:
    """Show what reference data is available."""
    versions = available_versions()
    if not versions:
        print("No reference test data found.")
        print(f"  Expected in: {REF_DIR}/")
        print("  Run: ./scripts/capture_reference_test_results.sh")
        return

    print("Reference test data:")
    print()
    for v in versions:
        suites = available_suites(v)
        total_tests = 0
        total_pass = 0
        total_skip = 0
        total_fail = 0
        for name in suites:
            sr = load_suite(name, v)
            if sr:
                total_tests += sr.total
                total_pass += sr.passed
                total_skip += sr.skipped
                total_fail += sr.failed
        print(
            f"  Tcl {v}:  {len(suites)} suites, "
            f"{total_tests} tests ({total_pass} passed, "
            f"{total_skip} skipped, {total_fail} failed)"
        )


def cmd_summary(version: str) -> None:
    """Show summary table for all suites in a version."""
    suites = load_all_suites(version)
    if not suites:
        print(f"No results for Tcl {version}")
        return

    print(f"Reference test results — Tcl {version}")
    print()
    print(f"{'Suite':<30s} {'Total':>7s} {'Pass':>7s} {'Skip':>7s} {'Fail':>7s}  {'Rate':>6s}")
    print(f"{'─' * 30:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s}  {'─' * 6:s}")

    grand_total = grand_pass = grand_skip = grand_fail = 0

    for sr in suites:
        rate = f"{sr.passed / sr.total * 100:.1f}%" if sr.total else "—"
        print(
            f"{sr.file:<30s} {sr.total:>7d} {sr.passed:>7d} "
            f"{sr.skipped:>7d} {sr.failed:>7d}  {rate:>6s}"
        )
        grand_total += sr.total
        grand_pass += sr.passed
        grand_skip += sr.skipped
        grand_fail += sr.failed

    grand_rate = f"{grand_pass / grand_total * 100:.1f}%" if grand_total else "—"
    print(f"{'─' * 30:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s}  {'─' * 6:s}")
    print(
        f"{'TOTAL':<30s} {grand_total:>7d} {grand_pass:>7d} "
        f"{grand_skip:>7d} {grand_fail:>7d}  {grand_rate:>6s}"
    )


def cmd_suite(suite: str, version: str) -> None:
    """Show full results for a test suite."""
    sr = load_suite(suite, version)
    if not sr:
        print(f"No results for {suite} in Tcl {version}")
        # Suggest available suites
        available = available_suites(version)
        if available:
            matches = [s for s in available if suite.lower() in s.lower()]
            if matches:
                print(f"  Did you mean: {', '.join(matches)}")
        return

    print(f"{sr.file} — Tcl {sr.version}")
    if sr.tclsh:
        print(f"  tclsh: {sr.tclsh}")
    if sr.date:
        print(f"  date:  {sr.date}")
    print(f"  Total: {sr.total}  Passed: {sr.passed}  Skipped: {sr.skipped}  Failed: {sr.failed}")
    print()

    # Group by status
    passes = [t for t in sr.tests if t.status == "PASS"]
    skips = [t for t in sr.tests if t.status == "SKIP"]
    fails = [t for t in sr.tests if t.status == "FAIL"]

    if fails:
        print(f"FAILED ({len(fails)}):")
        for t in fails:
            print(f"  {t.name}")
        print()

    if skips:
        print(f"SKIPPED ({len(skips)}):")
        for t in skips:
            reason = f"  ({t.reason})" if t.reason else ""
            print(f"  {t.name}{reason}")
        print()

    if passes:
        print(f"PASSED ({len(passes)}):")
        for t in passes:
            print(f"  {t.name}")


def cmd_group(pattern: str, version: str) -> None:
    """Show all tests matching a glob pattern across suites."""
    suites = load_all_suites(version)
    if not suites:
        print(f"No results for Tcl {version}")
        return

    # If pattern doesn't have wildcards, add * suffix
    if "*" not in pattern and "?" not in pattern:
        pattern = f"{pattern}*"

    matches: list[tuple[str, TestResult]] = []
    for sr in suites:
        for t in sr.tests:
            if fnmatch.fnmatch(t.name, pattern):
                matches.append((sr.file, t))

    if not matches:
        print(f"No tests matching '{pattern}' in Tcl {version}")
        return

    print(f"Tests matching '{pattern}' — Tcl {version}")
    print(f"  {len(matches)} matches")
    print()

    passes = [(f, t) for f, t in matches if t.status == "PASS"]
    skips = [(f, t) for f, t in matches if t.status == "SKIP"]
    fails = [(f, t) for f, t in matches if t.status == "FAIL"]

    if fails:
        print(f"FAILED ({len(fails)}):")
        for suite_file, t in fails:
            print(f"  {t.name}  ({suite_file})")
        print()

    if skips:
        print(f"SKIPPED ({len(skips)}):")
        for suite_file, t in skips:
            reason = f"  ({t.reason})" if t.reason else ""
            print(f"  {t.name}  ({suite_file}){reason}")
        print()

    if passes:
        print(f"PASSED ({len(passes)}):")
        for suite_file, t in passes:
            print(f"  {t.name}  ({suite_file})")


def cmd_lookup(test_name: str, version: str) -> None:
    """Look up a specific test by name."""
    suites = load_all_suites(version)
    found: list[tuple[str, TestResult]] = []

    for sr in suites:
        for t in sr.tests:
            if t.name == test_name:
                found.append((sr.file, t))

    if not found:
        # Try prefix match
        for sr in suites:
            for t in sr.tests:
                if t.name.startswith(test_name):
                    found.append((sr.file, t))
        if found:
            print(f"No exact match for '{test_name}'; prefix matches:")
        else:
            print(f"Test '{test_name}' not found in Tcl {version}")
            return

    print()
    for suite_file, t in found:
        status_str = t.status
        if t.reason:
            status_str += f" ({t.reason})"
        print(f"  {t.name}  {status_str}  in {suite_file}")

    # If looking across all versions
    if len(found) == 1:
        print()
        print("Across all versions:")
        for v in available_versions():
            for name in available_suites(v):
                sr = load_suite(name, v)
                if not sr:
                    continue
                for t in sr.tests:
                    if t.name == found[0][1].name:
                        status_str = t.status
                        if t.reason:
                            status_str += f" ({t.reason})"
                        print(f"  Tcl {v}: {status_str}")


def cmd_compare(suite: str, version: str) -> None:
    """Compare reference results across all available versions for a suite."""
    versions = available_versions()
    if not versions:
        print("No reference data available.")
        return

    results: dict[str, SuiteResult] = {}
    for v in versions:
        sr = load_suite(suite, v)
        if sr:
            results[v] = sr

    if not results:
        print(f"No results for {suite} in any version")
        return

    print(f"Cross-version comparison: {suite}")
    print()
    print(f"{'Version':<10s} {'Total':>7s} {'Pass':>7s} {'Skip':>7s} {'Fail':>7s}  {'Rate':>6s}")
    print(f"{'─' * 10:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s} {'─' * 7:s}  {'─' * 6:s}")

    for v in versions:
        if v not in results:
            print(f"{'Tcl ' + v:<10s} {'—':>7s} {'—':>7s} {'—':>7s} {'—':>7s}  {'—':>6s}")
            continue
        sr = results[v]
        rate = f"{sr.passed / sr.total * 100:.1f}%" if sr.total else "—"
        print(
            f"{'Tcl ' + v:<10s} {sr.total:>7d} {sr.passed:>7d} "
            f"{sr.skipped:>7d} {sr.failed:>7d}  {rate:>6s}"
        )

    # Show tests that differ between versions
    if len(results) > 1:
        all_test_names: set[str] = set()
        for sr in results.values():
            for t in sr.tests:
                all_test_names.add(t.name)

        diffs: list[tuple[str, dict[str, str]]] = []
        for name in sorted(all_test_names):
            statuses: dict[str, str] = {}
            for v, sr in results.items():
                for t in sr.tests:
                    if t.name == name:
                        statuses[v] = t.status
                        break
                else:
                    statuses[v] = "—"
            # Check if all the same
            status_values = set(statuses.values())
            if len(status_values) > 1:
                diffs.append((name, statuses))

        if diffs:
            print()
            print(f"Tests that differ between versions ({len(diffs)}):")
            header = f"  {'Test':<35s}"
            for v in versions:
                if v in results:
                    header += f" {v:>6s}"
            print(header)
            for name, statuses in diffs[:50]:  # Cap at 50
                line = f"  {name:<35s}"
                for v in versions:
                    if v in results:
                        line += f" {statuses.get(v, '—'):>6s}"
                print(line)
            if len(diffs) > 50:
                print(f"  ... and {len(diffs) - 50} more")


def cmd_source(test_name: str, version: str) -> None:
    """Show the source code of a specific test from the .test file."""
    # 1. Find which suite the test belongs to
    suite_file = _find_test_suite(test_name, version)
    if not suite_file:
        # Try prefix match
        suites = load_all_suites(version)
        candidates: list[str] = []
        for sr in suites:
            for t in sr.tests:
                if t.name.startswith(test_name):
                    candidates.append(t.name)
        if candidates:
            print(f"Test '{test_name}' not found. Similar names:")
            for c in candidates[:10]:
                print(f"  {c}")
        else:
            print(f"Test '{test_name}' not found in Tcl {version}")
        return

    # 2. Find the test source directory
    tests_dir = find_tests_dir(version)
    if not tests_dir:
        print(f"No Tcl {version} source tree found.")
        print("  Searched: /usr/src/tcl{v}*/tests/")
        print(f"           {REPO_ROOT}/tmp/tcl{version}*/tests/")
        print(f"           ~/src/tcl{version}*/tests/")
        print()
        print("  Run: /test-results fetch-tcl-source (or fetch manually)")
        return

    test_path = tests_dir / suite_file
    if not test_path.is_file():
        print(f"Test file not found: {test_path}")
        return

    # 3. Extract the test source
    result = extract_test_source(test_path, test_name)
    if not result:
        print(f"Could not find test '{test_name}' in {test_path}")
        return

    raw = result["raw"]
    line_num = result["line"]

    # 4. Look up reference result
    ref_result = None
    sr = load_suite(suite_file.replace(".test", ""), version)
    if sr:
        for t in sr.tests:
            if t.name == test_name:
                ref_result = t
                break

    # 5. Parse options
    opts = parse_test_options(raw)

    # 6. Extract file-level context
    context = extract_file_context(test_path, raw)

    # 7. Output
    print(f"=== {test_name} ({suite_file}:{line_num} — Tcl {version}) ===")
    print()

    if ref_result:
        status_str = ref_result.status
        if ref_result.reason:
            status_str += f" ({ref_result.reason})"
        print(f"Reference result: {status_str}")
        print()

    if context:
        print("--- File-level setup ---")
        print(context)
        print()

    print("--- Test ---")
    print(raw)
    print()

    print("--- Parsed ---")
    print(f"Name:        {opts['name']}")
    print(f"Description: {opts['description']}")
    if opts["constraints"]:
        print(f"Constraints: {opts['constraints']}")
    if opts["setup"]:
        print(f"Setup:       {_oneline(opts['setup'])}")
    print(f"Body:        {_oneline(opts['body'])}")
    print(f"Result:      {_oneline(opts['result'])}")
    if opts["match"] != "exact":
        print(f"Match:       {opts['match']}")
    if opts["returnCodes"] != "ok":
        print(f"ReturnCodes: {opts['returnCodes']}")
    if opts["cleanup"]:
        print(f"Cleanup:     {_oneline(opts['cleanup'])}")
    if opts["output"]:
        print(f"Output:      {_oneline(opts['output'])}")
    if opts["errorOutput"]:
        print(f"ErrorOutput: {_oneline(opts['errorOutput'])}")


def _oneline(text: str) -> str:
    """Collapse multiline text to a single line for display."""
    text = text.strip()
    if "\n" in text:
        lines = text.splitlines()
        if len(lines) <= 3:
            return " \\n ".join(line.strip() for line in lines)
        return " \\n ".join(line.strip() for line in lines[:2]) + f" ... ({len(lines)} lines)"
    return text


# Main

USAGE = """\
Usage: python3 test_results.py <command> [args...]

Commands:
  status                      Show available reference data
  summary [VERSION]           Summary table for all suites
  suite SUITE [VERSION]       Full results for a test suite
  group PATTERN [VERSION]     All tests matching a glob pattern
  lookup TEST_NAME [VERSION]  Look up a specific test
  compare SUITE               Cross-version comparison for a suite
  source TEST_NAME [VERSION]  Show test source code from .test file

VERSION defaults to best available (9.0 > 8.6 > 8.5 > 8.4).
SUITE accepts "compExpr-old" or "compExpr-old.test".
PATTERN accepts globs: "compExpr-old-1.*", "expr-old-2?.*"

Examples:
  python3 test_results.py summary 9.0
  python3 test_results.py suite compExpr-old 9.0
  python3 test_results.py group "expr-old-29.*"
  python3 test_results.py lookup expr-old-29.1
  python3 test_results.py compare expr-old
  python3 test_results.py source incr-1.15 8.6
"""


def main() -> None:
    if len(sys.argv) < 2:
        print(USAGE)
        sys.exit(1)

    command = sys.argv[1]

    if command == "status":
        cmd_status()
    elif command == "summary":
        version = resolve_version(sys.argv[2] if len(sys.argv) > 2 else None)
        cmd_summary(version)
    elif command == "suite":
        if len(sys.argv) < 3:
            print("Usage: test_results.py suite SUITE [VERSION]")
            sys.exit(1)
        suite = sys.argv[2]
        version = resolve_version(sys.argv[3] if len(sys.argv) > 3 else None)
        cmd_suite(suite, version)
    elif command == "group":
        if len(sys.argv) < 3:
            print("Usage: test_results.py group PATTERN [VERSION]")
            sys.exit(1)
        pattern = sys.argv[2]
        version = resolve_version(sys.argv[3] if len(sys.argv) > 3 else None)
        cmd_group(pattern, version)
    elif command == "lookup":
        if len(sys.argv) < 3:
            print("Usage: test_results.py lookup TEST_NAME [VERSION]")
            sys.exit(1)
        test_name = sys.argv[2]
        version = resolve_version(sys.argv[3] if len(sys.argv) > 3 else None)
        cmd_lookup(test_name, version)
    elif command == "compare":
        if len(sys.argv) < 3:
            print("Usage: test_results.py compare SUITE")
            sys.exit(1)
        suite = sys.argv[2]
        cmd_compare(suite, resolve_version(None))
    elif command == "source":
        if len(sys.argv) < 3:
            print("Usage: test_results.py source TEST_NAME [VERSION]")
            sys.exit(1)
        test_name = sys.argv[2]
        version = resolve_version(sys.argv[3] if len(sys.argv) > 3 else None)
        cmd_source(test_name, version)
    else:
        print(f"Unknown command: {command}")
        print()
        print(USAGE)
        sys.exit(1)


if __name__ == "__main__":
    main()
