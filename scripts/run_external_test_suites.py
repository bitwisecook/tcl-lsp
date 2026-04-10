#!/usr/bin/env python3
"""Download pure-Tcl projects and run their test suites through the VM.

Evaluates what percentage of each project's test suite passes, fails,
or crashes, producing a detailed report of missing/stubbed functionality.

Usage:
    python scripts/run_external_test_suites.py [OPTIONS] [PROJECT...]

    # Run all projects
    python scripts/run_external_test_suites.py

    # Run specific projects
    python scripts/run_external_test_suites.py tcllib exercism

    # List available projects
    python scripts/run_external_test_suites.py --list

    # JSON report
    python scripts/run_external_test_suites.py --json -o report.json

    # Only show failures
    python scripts/run_external_test_suites.py --failures-only
"""

from __future__ import annotations

import argparse
import io
import json
import os
import re
import shutil
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

# Ensure repo root is on sys.path so we can import vm/ and core/
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

from vm.commands import tcltest_cmds  # noqa: E402
from vm.commands.test_support_cmds import setup_test_support  # noqa: E402
from vm.interp import TclInterp  # noqa: E402

# ---------------------------------------------------------------------------
# Project definitions
# ---------------------------------------------------------------------------

DOWNLOAD_DIR = REPO_ROOT / "tmp" / "external_projects"


@dataclass
class Project:
    """An external Tcl project to test against our VM."""

    name: str
    description: str
    git_url: str
    git_ref: str  # branch or tag
    sparse_paths: list[str]  # directories to sparse-checkout
    test_glob: str  # glob pattern for test files relative to checkout root
    setup_script: str | None = None  # Tcl to eval before each test file
    pkgindex_dirs: list[str] = field(default_factory=list)  # dirs with pkgIndex.tcl
    skip_files: list[str] = field(default_factory=list)  # files to skip entirely
    source_init: bool = False  # whether to source Tcl's init.tcl


PROJECTS: dict[str, Project] = {
    "tcllib": Project(
        name="tcllib",
        description="Tcl Standard Library — 200+ pure-Tcl modules",
        git_url="https://github.com/tcltk/tcllib.git",
        git_ref="tcllib-2-0",
        sparse_paths=["modules", "devtools"],
        test_glob="modules/*/*.test",
        source_init=True,
        pkgindex_dirs=["modules"],
        skip_files=[
            # These need network access
            "modules/ftp/ftp.test",
            "modules/http/http.test",
            "modules/smtp/smtp.test",
            "modules/pop3/pop3.test",
            "modules/imap4/imap4.test",
            "modules/websocket/websocket.test",
            # These need Tk
            "modules/tkpiechart/tkpiechart.test",
            "modules/widget/widget.test",
            "modules/tooltip/tooltip.test",
            "modules/crosshair/crosshair.test",
            "modules/ntext/ntext.test",
            "modules/ctext/ctext.test",
            "modules/chatwidget/chatwidget.test",
            "modules/controlwidget/controlwidget.test",
            "modules/diagrams/diagrams.test",
            "modules/plotchart/plotchart.test",
            "modules/tablelist/tablelist.test",
        ],
    ),
    "tcl-tests": Project(
        name="tcl-tests",
        description="Tcl core test suite (231 .test files)",
        git_url="https://github.com/tcltk/tcl.git",
        git_ref="core-9-0-3",
        sparse_paths=["tests", "library"],
        test_glob="tests/*.test",
        source_init=True,
        setup_script="package require tcltest 2.5; namespace import ::tcltest::*",
        skip_files=[
            # These need the C test harness commands heavily
            "tests/io.test",
            "tests/socket.test",
            "tests/chanio.test",
            "tests/thread.test",
            "tests/load.test",
            "tests/unload.test",
            "tests/encoding.test",
            "tests/event.test",
            "tests/timer.test",
            "tests/async.test",
            "tests/notify.test",
            "tests/pid.test",
            # Needs child processes
            "tests/exec.test",
            "tests/safe.test",
            "tests/interp.test",
        ],
    ),
    "exercism": Project(
        name="exercism",
        description="Exercism Tcl track — 80+ exercises with tcltest",
        git_url="https://github.com/exercism/tcl.git",
        git_ref="main",
        sparse_paths=["exercises/practice"],
        test_glob="exercises/practice/*/*.test",
        source_init=True,
    ),
    "tclssg": Project(
        name="tclssg",
        description="Static site generator in pure Tcl",
        git_url="https://github.com/tclssg/tclssg.git",
        git_ref="master",
        sparse_paths=["."],
        test_glob="tests.tcl",
    ),
    "nagelfar": Project(
        name="nagelfar",
        description="Tcl static analysis / linter — pure Tcl",
        git_url="https://git.code.sf.net/p/nagelfar/code",
        git_ref="master",
        sparse_paths=["."],
        test_glob="tests/*.test",
    ),
    "testcl": Project(
        name="testcl",
        description="F5 iRules test framework — pure Tcl",
        git_url="https://github.com/landro/TesTcl.git",
        git_ref="master",
        sparse_paths=["src", "test", "examples"],
        test_glob="test/*.test",
    ),
}


# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------


def download_project(project: Project) -> Path:
    """Clone a project via sparse checkout. Returns the checkout directory."""
    target = DOWNLOAD_DIR / project.name
    marker = target / ".download_complete"

    if marker.exists():
        return target

    # Clean up partial downloads
    if target.exists():
        shutil.rmtree(target)

    DOWNLOAD_DIR.mkdir(parents=True, exist_ok=True)

    print(f"  Downloading {project.name} from {project.git_url} ...")

    for attempt in range(1, 5):
        try:
            subprocess.run(
                [
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "--filter=blob:none",
                    "--sparse",
                    "--branch",
                    project.git_ref,
                    project.git_url,
                    str(target),
                ],
                check=True,
                capture_output=True,
                timeout=120,
            )
            break
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            if attempt < 4:
                wait = 2**attempt
                print(f"    Retry {attempt} (waiting {wait}s): {exc}")
                shutil.rmtree(target, ignore_errors=True)
                time.sleep(wait)
            else:
                print(f"    FAILED after 4 attempts: {exc}")
                return target

    # Set sparse checkout paths
    if project.sparse_paths != ["."]:
        try:
            subprocess.run(
                ["git", "sparse-checkout", "set"] + project.sparse_paths,
                cwd=target,
                check=True,
                capture_output=True,
                timeout=60,
            )
        except subprocess.CalledProcessError as exc:
            print(f"    sparse-checkout failed: {exc}")
            shutil.rmtree(target, ignore_errors=True)
            return target

    marker.touch()
    return target


# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------


@dataclass
class TestFileResult:
    """Result of running a single .test file."""

    file: str
    total: int = 0
    passed: int = 0
    skipped: int = 0
    failed: int = 0
    crashed: bool = False
    crash_error: str = ""
    crash_type: str = ""
    failed_tests: list[str] = field(default_factory=list)
    missing_commands: list[str] = field(default_factory=list)
    duration_ms: float = 0
    stdout: str = ""


@dataclass
class ProjectReport:
    """Aggregated report for a project."""

    name: str
    description: str
    files_tested: int = 0
    files_crashed: int = 0
    files_skipped: int = 0
    total_tests: int = 0
    total_passed: int = 0
    total_failed: int = 0
    total_skipped: int = 0
    total_duration_ms: float = 0
    missing_commands: Counter = field(default_factory=Counter)
    crash_categories: Counter = field(default_factory=Counter)
    file_results: list[TestFileResult] = field(default_factory=list)


def _extract_missing_command(error_msg: str) -> str | None:
    """Extract the missing command name from a TclError message."""
    m = re.search(r'invalid command name "([^"]+)"', error_msg)
    if m:
        return m.group(1)
    return None


def _categorise_crash(error_msg: str, error_type: str) -> str:
    """Categorise a crash into a bucket for the report."""
    if "invalid command name" in error_msg:
        cmd = _extract_missing_command(error_msg)
        return f"missing_command:{cmd}" if cmd else "missing_command:unknown"
    if "can't find package" in error_msg:
        m = re.search(r"can't find package (\S+)", error_msg)
        pkg = m.group(1) if m else "unknown"
        return f"missing_package:{pkg}"
    if "wrong # args" in error_msg:
        return "wrong_args"
    if "no such variable" in error_msg or "can't read" in error_msg:
        return "variable_error"
    if "syntax error" in error_msg.lower():
        return "syntax_error"
    if error_type == "CompilationError" or "compile" in error_type.lower():
        return "compilation_error"
    if "RecursionError" in error_type:
        return "recursion_limit"
    if "NotImplementedError" in error_type:
        return "not_implemented"
    return f"other:{error_type}"


def _collect_tcltest_results(interp: TclInterp | None) -> dict:
    """Read tcltest results from the interpreter.

    Tries multiple approaches to find the results:
    1. Eval the Tcl expression to read the namespace variable
    2. Read from the global frame (Python tcltest sync)
    3. Parse stdout output for the summary line
    4. Fall back to the Python-internal ``_results`` dict
    """
    results = {"Total": 0, "Passed": 0, "Skipped": 0, "Failed": 0}

    if interp is None:
        for key in results:
            results[key] = tcltest_cmds._results.get(key, 0)
        return results

    # Method 1: Try reading via Tcl eval (works for real tcltest)
    for key in results:
        try:
            val = interp.eval(f"set ::tcltest::numTests({key})")
            results[key] = int(val.value)
        except Exception:
            pass

    # If we got results, return them
    if results["Total"] > 0:
        return results

    # Method 2: Try global frame variables (works for Python tcltest)
    for key in results:
        try:
            val = interp.global_frame.get_var(f"::tcltest::numTests({key})")
            results[key] = int(val)
        except Exception:
            pass

    if results["Total"] > 0:
        return results

    # Method 3: Fall back to Python internal state
    for key in results:
        results[key] = tcltest_cmds._results.get(key, 0)

    return results


def _parse_tcltest_stdout(output: str) -> dict | None:
    """Parse tcltest summary from stdout as last resort.

    Looks for the line: ``filename:\tTotal\tN\tPassed\tN\tSkipped\tN\tFailed\tN``
    Also handles the case where \\t is rendered as literal 't' due to
    a backslash substitution bug in the VM.
    """
    for line in reversed(output.split("\n")):
        # Standard format with real tabs or whitespace
        m = re.search(
            r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)",
            line,
        )
        if m:
            return {
                "Total": int(m.group(1)),
                "Passed": int(m.group(2)),
                "Skipped": int(m.group(3)),
                "Failed": int(m.group(4)),
            }
        # Workaround: \t rendered as literal 't' character
        m = re.search(
            r"Total.?(\d+).?Passed.?(\d+).?Skipped.?(\d+).?Failed.?(\d+)",
            line,
        )
        if m:
            return {
                "Total": int(m.group(1)),
                "Passed": int(m.group(2)),
                "Skipped": int(m.group(3)),
                "Failed": int(m.group(4)),
            }
    return None


def run_test_file(
    project: Project,
    checkout_dir: Path,
    test_file: Path,
) -> TestFileResult:
    """Run a single .test file through the VM and collect results."""
    result = TestFileResult(
        file=str(test_file.relative_to(checkout_dir)),
    )

    start = time.monotonic()
    interp: TclInterp | None = None
    stdout_buf: io.StringIO | None = None
    saved_cwd = os.getcwd()

    try:
        interp = TclInterp(source_init=project.source_init)
        setup_test_support(interp)
        tcltest_cmds._reset_state()

        # Capture output
        stdout_buf = io.StringIO()
        stderr_buf = io.StringIO()
        interp.channels["stdout"] = stdout_buf
        interp.channels["stderr"] = stderr_buf

        # Set up test directory context
        interp.global_frame.set_var("argv0", str(test_file))
        interp.global_frame.set_var("argv", "")
        interp.global_frame.set_var("argc", "0")
        interp.script_file = str(test_file)

        # Register package ifneeded scripts for project packages
        for pkg_dir in project.pkgindex_dirs:
            pkgindex = checkout_dir / pkg_dir / "pkgIndex.tcl"
            if pkgindex.exists():
                try:
                    interp.eval(pkgindex.read_text())
                except Exception:
                    pass  # Best effort

        # Set up tcltest (uses real tcltest.tcl if available)
        from vm.commands.tcltest_cmds import setup_tcltest

        setup_tcltest(interp)

        # Run any setup script
        if project.setup_script:
            try:
                interp.eval(project.setup_script)
            except Exception:
                pass

        # Change to the test file's directory so relative `source`
        # commands resolve correctly
        os.chdir(test_file.parent)

        # Source the test file
        script = test_file.read_text(errors="replace")

        try:
            interp.eval(script)
        except SystemExit:
            # Some test helpers call `exit 1` on failure — catch it
            pass
        finally:
            os.chdir(saved_cwd)

        # Collect results from Tcl variables or Python fallback
        result.stdout = stdout_buf.getvalue() if stdout_buf else ""
        tcl_results = _collect_tcltest_results(interp)
        if tcl_results["Total"] == 0:
            # Try parsing stdout as last resort
            parsed = _parse_tcltest_stdout(result.stdout)
            if parsed:
                tcl_results = parsed
        result.total = tcl_results["Total"]
        result.passed = tcl_results["Passed"]
        result.skipped = tcl_results["Skipped"]
        result.failed = tcl_results["Failed"]
        result.failed_tests = list(tcltest_cmds._failed_tests)

    except SystemExit:
        # Catch exit calls that escape the inner handler
        try:
            os.chdir(saved_cwd)
        except Exception:
            pass
        result.stdout = stdout_buf.getvalue() if stdout_buf else ""
        tcl_results = _collect_tcltest_results(interp)
        if tcl_results["Total"] == 0:
            parsed = _parse_tcltest_stdout(result.stdout)
            if parsed:
                tcl_results = parsed
        result.total = tcl_results["Total"]
        result.passed = tcl_results["Passed"]
        result.skipped = tcl_results["Skipped"]
        result.failed = tcl_results["Failed"]
        result.failed_tests = list(tcltest_cmds._failed_tests)

    except Exception as exc:
        try:
            os.chdir(saved_cwd)
        except Exception:
            pass
        result.crashed = True
        result.crash_error = str(exc)
        result.crash_type = type(exc).__name__

        # Still capture any partial results
        result.stdout = stdout_buf.getvalue() if stdout_buf else ""
        try:
            tcl_results = _collect_tcltest_results(interp)
            if tcl_results["Total"] == 0:
                parsed = _parse_tcltest_stdout(result.stdout)
                if parsed:
                    tcl_results = parsed
            result.total = tcl_results["Total"]
            result.passed = tcl_results["Passed"]
            result.skipped = tcl_results["Skipped"]
            result.failed = tcl_results["Failed"]
        except Exception:
            pass
        result.failed_tests = list(tcltest_cmds._failed_tests)

        # Extract missing commands from the error
        cmd = _extract_missing_command(str(exc))
        if cmd:
            result.missing_commands.append(cmd)

    finally:
        result.duration_ms = (time.monotonic() - start) * 1000

    return result


def run_test_file_wasm(
    project: Project,
    checkout_dir: Path,
    test_file: Path,
) -> TestFileResult:
    """Compile a single .test file to WASM and validate the output.

    This does not *execute* the WASM — it validates that the Tcl source
    compiles to a valid WASM module.  Full execution requires a WASM
    runtime linked against the Zig Tcl runtime.
    """
    from core.compiler.cfg import build_cfg
    from core.compiler.codegen.wasm import WasmOp, wasm_codegen_module
    from core.compiler.lowering import lower_to_ir

    result = TestFileResult(
        file=str(test_file.relative_to(checkout_dir)),
    )
    start = time.monotonic()

    try:
        script = test_file.read_text(errors="replace")
        ir_module = lower_to_ir(script)
        cfg_module = build_cfg(ir_module)
        wasm_module = wasm_codegen_module(cfg_module, ir_module)
        wasm_bytes = wasm_module.to_bytes()

        # Validate WASM binary header
        assert wasm_bytes[:4] == b"\x00asm"

        # Collect stats
        total_instrs = sum(len(f.body) for f in wasm_module.functions)
        nop_count = sum(1 for f in wasm_module.functions for i in f.body if i.op == WasmOp.NOP)
        nop_pct = 100 * nop_count // max(total_instrs, 1)

        result.total = 1  # 1 "test" = successful compilation
        result.passed = 1
        result.stdout = (
            f"WASM: {len(wasm_bytes)} bytes, {len(wasm_module.functions)} funcs, "
            f"{total_instrs} instrs, {nop_count} NOPs ({nop_pct}%), "
            f"{len(wasm_module.imports)} imports"
        )

    except Exception as e:
        result.crashed = True
        result.crash_error = str(e)
        result.crash_type = type(e).__name__

    result.duration_ms = (time.monotonic() - start) * 1000
    return result


def run_project(
    project: Project, checkout_dir: Path, *, max_files: int = 0, mode: str = "vm"
) -> ProjectReport:
    """Run all test files for a project and build a report."""
    report = ProjectReport(
        name=project.name,
        description=project.description,
    )

    # Find test files
    test_files = sorted(checkout_dir.glob(project.test_glob))
    if not test_files:
        print(f"  WARNING: No test files matched '{project.test_glob}'")
        return report

    # Filter out skipped files
    skip_set = {str(checkout_dir / s) for s in project.skip_files}
    test_files = [f for f in test_files if str(f) not in skip_set]
    skipped_count = len(skip_set)

    # Apply max-files limit
    if max_files > 0 and len(test_files) > max_files:
        test_files = test_files[:max_files]

    print(f"  Found {len(test_files)} test files ({skipped_count} pre-skipped)")
    report.files_skipped = skipped_count

    for i, test_file in enumerate(test_files, 1):
        rel = test_file.relative_to(checkout_dir)
        sys.stdout.write(f"\r  [{i}/{len(test_files)}] {rel}".ljust(80))
        sys.stdout.flush()

        if mode == "wasm":
            file_result = run_test_file_wasm(project, checkout_dir, test_file)
        else:
            file_result = run_test_file(project, checkout_dir, test_file)
        report.file_results.append(file_result)
        report.files_tested += 1

        if file_result.crashed:
            report.files_crashed += 1
            cat = _categorise_crash(file_result.crash_error, file_result.crash_type)
            report.crash_categories[cat] += 1

        report.total_tests += file_result.total
        report.total_passed += file_result.passed
        report.total_failed += file_result.failed
        report.total_skipped += file_result.skipped
        report.total_duration_ms += file_result.duration_ms

        for cmd in file_result.missing_commands:
            report.missing_commands[cmd] += 1

    print()  # clear the progress line
    return report


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def print_report(report: ProjectReport, *, failures_only: bool = False) -> None:
    """Print a human-readable report for a project."""
    width = 72
    print("=" * width)
    print(f"  {report.name}: {report.description}")
    print("=" * width)

    # Summary
    if report.total_tests > 0:
        pass_pct = report.total_passed / report.total_tests * 100
    else:
        pass_pct = 0

    print(f"\n  Files tested:  {report.files_tested}")
    print(f"  Files crashed: {report.files_crashed}")
    print(f"  Files skipped: {report.files_skipped}")
    print(f"\n  Total tests:   {report.total_tests}")
    print(f"  Passed:        {report.total_passed} ({pass_pct:.1f}%)")
    print(f"  Failed:        {report.total_failed}")
    print(f"  Skipped:       {report.total_skipped}")
    print(f"  Duration:      {report.total_duration_ms / 1000:.1f}s")

    # Missing commands (top 20)
    if report.missing_commands:
        print("\n  Missing commands (caused crashes):")
        for cmd, count in report.missing_commands.most_common(20):
            print(f"    {cmd:40s}  {count:3d} file(s)")

    # Crash categories
    if report.crash_categories:
        print("\n  Crash categories:")
        for cat, count in report.crash_categories.most_common(20):
            print(f"    {cat:40s}  {count:3d} file(s)")

    # Per-file details
    if not failures_only:
        # Show files that ran successfully
        ok_files = [
            r for r in report.file_results if not r.crashed and r.failed == 0 and r.total > 0
        ]
        if ok_files:
            print(f"\n  Clean files ({len(ok_files)}):")
            for r in ok_files[:30]:
                print(f"    {r.file:50s}  {r.total:4d} tests, {r.passed} passed")
            if len(ok_files) > 30:
                print(f"    ... and {len(ok_files) - 30} more")

    # Show files with failures
    fail_files = [r for r in report.file_results if r.failed > 0 and not r.crashed]
    if fail_files:
        print(f"\n  Files with test failures ({len(fail_files)}):")
        for r in sorted(fail_files, key=lambda r: r.failed, reverse=True)[:30]:
            print(f"    {r.file:50s}  {r.total:4d} tests, {r.passed} passed, {r.failed} FAILED")
            for t in r.failed_tests[:5]:
                print(f"      - {t}")
            if len(r.failed_tests) > 5:
                print(f"      ... and {len(r.failed_tests) - 5} more")

    # Show crashed files
    crash_files = [r for r in report.file_results if r.crashed]
    if crash_files:
        print(f"\n  Crashed files ({len(crash_files)}):")
        for r in sorted(crash_files, key=lambda r: r.file)[:40]:
            # Truncate error to one line
            err = r.crash_error.split("\n")[0][:80]
            partial = ""
            if r.total > 0:
                partial = f" (partial: {r.passed}/{r.total} before crash)"
            print(f"    {r.file:50s}  {r.crash_type}: {err}{partial}")
        if len(crash_files) > 40:
            print(f"    ... and {len(crash_files) - 40} more")

    print()


def build_json_report(reports: list[ProjectReport]) -> dict:
    """Build a JSON-serializable report."""
    return {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "projects": [
            {
                "name": r.name,
                "description": r.description,
                "summary": {
                    "files_tested": r.files_tested,
                    "files_crashed": r.files_crashed,
                    "files_skipped": r.files_skipped,
                    "total_tests": r.total_tests,
                    "passed": r.total_passed,
                    "failed": r.total_failed,
                    "skipped": r.total_skipped,
                    "pass_rate": (
                        round(r.total_passed / r.total_tests * 100, 1) if r.total_tests > 0 else 0
                    ),
                    "duration_ms": round(r.total_duration_ms, 1),
                },
                "missing_commands": dict(r.missing_commands.most_common(50)),
                "crash_categories": dict(r.crash_categories.most_common(50)),
                "files": [
                    {
                        "file": fr.file,
                        "total": fr.total,
                        "passed": fr.passed,
                        "failed": fr.failed,
                        "skipped": fr.skipped,
                        "crashed": fr.crashed,
                        "crash_error": fr.crash_error if fr.crashed else None,
                        "crash_type": fr.crash_type if fr.crashed else None,
                        "missing_commands": fr.missing_commands or None,
                        "failed_tests": fr.failed_tests or None,
                        "duration_ms": round(fr.duration_ms, 1),
                    }
                    for fr in r.file_results
                ],
            }
            for r in reports
        ],
    }


def print_summary_table(reports: list[ProjectReport]) -> None:
    """Print a compact summary table across all projects."""
    print("\n" + "=" * 80)
    print("  SUMMARY")
    print("=" * 80)
    print(
        f"  {'Project':15s}  {'Files':>6s}  {'Crash':>6s}  "
        f"{'Tests':>7s}  {'Pass':>7s}  {'Fail':>6s}  {'Rate':>6s}  {'Time':>7s}"
    )
    print("-" * 80)

    grand_tests = 0
    grand_passed = 0
    grand_failed = 0

    for r in reports:
        rate = f"{r.total_passed / r.total_tests * 100:.1f}%" if r.total_tests else "N/A"
        t = f"{r.total_duration_ms / 1000:.1f}s"
        print(
            f"  {r.name:15s}  {r.files_tested:6d}  {r.files_crashed:6d}  "
            f"{r.total_tests:7d}  {r.total_passed:7d}  {r.total_failed:6d}  "
            f"{rate:>6s}  {t:>7s}"
        )
        grand_tests += r.total_tests
        grand_passed += r.total_passed
        grand_failed += r.total_failed

    print("-" * 80)
    grand_rate = f"{grand_passed / grand_tests * 100:.1f}%" if grand_tests else "N/A"
    print(
        f"  {'TOTAL':15s}  {'':6s}  {'':6s}  "
        f"{grand_tests:7d}  {grand_passed:7d}  {grand_failed:6d}  {grand_rate:>6s}"
    )

    # Global missing commands across all projects
    all_missing: Counter = Counter()
    all_crashes: Counter = Counter()
    for r in reports:
        all_missing.update(r.missing_commands)
        all_crashes.update(r.crash_categories)

    if all_missing:
        print("\n  Top missing commands (across all projects):")
        for cmd, count in all_missing.most_common(25):
            print(f"    {cmd:45s}  {count:3d} file(s)")

    if all_crashes:
        print("\n  Top crash categories:")
        for cat, count in all_crashes.most_common(15):
            print(f"    {cat:45s}  {count:3d} file(s)")

    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run external Tcl project test suites through the VM",
        prog="run_external_test_suites",
    )
    parser.add_argument(
        "projects",
        nargs="*",
        help="Projects to test (default: all)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List available projects and exit",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON report",
    )
    parser.add_argument(
        "-o",
        "--output",
        help="Output file for JSON report",
    )
    parser.add_argument(
        "--failures-only",
        action="store_true",
        help="Only show failures in the report",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=0,
        help="Limit number of test files per project (0 = unlimited)",
    )
    parser.add_argument(
        "--mode",
        choices=["vm", "wasm"],
        default="vm",
        help="Execution mode: 'vm' runs in bytecode VM (default), 'wasm' compiles to WASM and validates",
    )

    args = parser.parse_args()

    if args.list:
        print("Available projects:\n")
        for name, p in PROJECTS.items():
            print(f"  {name:15s}  {p.description}")
        print(f"\nProjects download to: {DOWNLOAD_DIR}/")
        return 0

    # Select projects
    if args.projects:
        selected = []
        for name in args.projects:
            if name not in PROJECTS:
                print(f"Unknown project: {name}")
                print(f"Available: {', '.join(PROJECTS.keys())}")
                return 1
            selected.append(name)
    else:
        selected = list(PROJECTS.keys())

    print(f"Running test suites for: {', '.join(selected)}\n")

    reports: list[ProjectReport] = []

    for name in selected:
        project = PROJECTS[name]
        print(f"\n{'=' * 60}")
        print(f"  Project: {project.name}")
        print(f"{'=' * 60}")

        # Download
        checkout_dir = download_project(project)
        if not checkout_dir.exists():
            print("  SKIP: download failed")
            continue

        # Run tests
        report = run_project(project, checkout_dir, max_files=args.max_files, mode=args.mode)
        reports.append(report)

        # Print per-project report
        print_report(report, failures_only=args.failures_only)

    # Print summary
    if len(reports) > 1:
        print_summary_table(reports)

    # JSON output
    if args.json:
        json_data = build_json_report(reports)
        if args.output:
            Path(args.output).write_text(json.dumps(json_data, indent=2))
            print(f"JSON report written to {args.output}")
        else:
            print(json.dumps(json_data, indent=2))

    return 0


if __name__ == "__main__":
    sys.exit(main())
