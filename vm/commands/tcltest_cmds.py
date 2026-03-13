"""Minimal tcltest package — enough to run Tcl .test files in the VM.

Implements the ``test``, ``testConstraint``, ``cleanupTests``,
``loadTestedCommands``, ``makeFile``, ``removeFile``, ``makeDirectory``,
``removeDirectory``, ``interpreter``, ``customMatch``, and
``configure`` commands that the Tcl test suite relies on.
"""

from __future__ import annotations

import fnmatch
import os
import re
import sys
import tempfile
from typing import TYPE_CHECKING

from ..machine import _list_escape, _split_list
from ..types import TclBreak, TclContinue, TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from ..commands import CommandHandler
    from ..interp import TclInterp

# State

_constraints: dict[str, bool] = {}
_results: dict[str, int] = {
    "Total": 0,
    "Passed": 0,
    "Skipped": 0,
    "Failed": 0,
}
_verbose: str = "body"
_match_patterns: list[str] = []
_skip_patterns: list[str] = []
_custom_matchers: dict[str, str] = {}
_tmpdir: str = ""
_testdir: str = ""
_output_channel: str = "stdout"
_error_channel: str = "stderr"
_failed_tests: list[str] = []  # Track names of failed tests


def _get_tmpdir() -> str:
    global _tmpdir
    if not _tmpdir:
        _tmpdir = tempfile.mkdtemp(prefix="tcltest_")
    return _tmpdir


def _reset_state() -> None:
    """Reset tcltest state for a fresh run."""
    global _constraints, _results, _verbose, _match_patterns, _skip_patterns
    global _custom_matchers, _tmpdir, _testdir, _failed_tests
    _constraints.clear()
    _constraints.update(
        {
            "emptyTest": True,
            "knownBug": False,
        }
    )
    _results.update({"Total": 0, "Passed": 0, "Skipped": 0, "Failed": 0})
    _verbose = "body"
    _match_patterns.clear()
    _skip_patterns.clear()
    _custom_matchers.clear()
    _failed_tests.clear()
    _tmpdir = ""
    _testdir = ""


def _write_output(interp: TclInterp, msg: str) -> None:
    """Write to the tcltest output channel."""
    ch = interp.channels.get(_output_channel, sys.stdout)
    ch.write(msg)
    ch.flush()


def _write_error(interp: TclInterp, msg: str) -> None:
    """Write to the tcltest error channel."""
    ch = interp.channels.get(_error_channel, sys.stderr)
    ch.write(msg)
    ch.flush()


# Commands


def _cmd_test(interp: TclInterp, args: list[str]) -> TclResult:
    """test name description ?options...?

    Supports both old-style and new-style syntax:
    - Old: test name desc ?constraints? body result
    - New: test name desc -body body -result result ?-setup setup? ...
    """
    if len(args) < 2:
        raise TclError('wrong # args: should be "test name description ..."')

    name = args[0]
    # description = args[1]  # not used programmatically
    rest = args[2:]

    # Check match/skip patterns
    if _match_patterns and not any(fnmatch.fnmatch(name, p) for p in _match_patterns):
        return TclResult()
    if _skip_patterns and any(fnmatch.fnmatch(name, p) for p in _skip_patterns):
        return TclResult()

    # Parse options — detect new-style (has -body) vs old-style
    if any(a == "-body" for a in rest):
        return _run_test_new_style(interp, name, rest)
    return _run_test_old_style(interp, name, rest)


def _check_constraints(constraint_str: str) -> bool:
    """Check if all constraints in *constraint_str* are satisfied."""
    if not constraint_str:
        return True
    parts = _split_list(constraint_str)
    for c in parts:
        if c.startswith("!"):
            if _constraints.get(c[1:], False):
                return False
        elif not _constraints.get(c, False):
            return False
    return True


def _run_test_old_style(interp: TclInterp, name: str, rest: list[str]) -> TclResult:
    """Old-style: test name desc ?constraints? body result"""
    constraints = ""
    body = ""
    expected = ""

    if len(rest) == 2:
        body, expected = rest[0], rest[1]
    elif len(rest) >= 3:
        constraints, body, expected = rest[0], rest[1], rest[2]

    _results["Total"] += 1

    if not _check_constraints(constraints):
        _results["Skipped"] += 1
        return TclResult()

    try:
        result = interp.eval(body)
        actual = result.value
    except TclError as e:
        actual = e.message
    except TclReturn as ret:
        actual = ret.value
    except TclBreak:
        actual = ""
    except TclContinue:
        actual = ""
    except Exception as exc:
        actual = str(exc)

    if actual == expected:
        _results["Passed"] += 1
    else:
        _results["Failed"] += 1
        _failed_tests.append(name)
        _write_output(
            interp,
            f"\n==== {name} FAILED\n"
            f"---- Result was:\n{actual}\n"
            f"---- Result should have been:\n{expected}\n"
            f"==== {name} FAILED\n",
        )

    return TclResult()


def _run_test_new_style(interp: TclInterp, name: str, rest: list[str]) -> TclResult:
    """New-style: test name desc -body body -result result ..."""
    opts: dict[str, str] = {}
    i = 0
    while i < len(rest):
        if rest[i].startswith("-") and i + 1 < len(rest):
            key = rest[i][1:]  # strip leading -
            opts[key] = rest[i + 1]
            i += 2
        else:
            i += 1

    body = opts.get("body", "")
    expected = opts.get("result", "")
    constraints = opts.get("constraints", "")
    setup = opts.get("setup", "")
    cleanup = opts.get("cleanup", "")
    return_codes_str = opts.get("returnCodes", "ok return")
    match_mode = opts.get("match", "exact")

    _results["Total"] += 1

    if not _check_constraints(constraints):
        _results["Skipped"] += 1
        return TclResult()

    # Run setup
    if setup:
        try:
            interp.eval(setup)
        except TclError:
            pass

    # Run body
    actual = ""
    actual_code = 0
    try:
        result = interp.eval(body)
        actual = result.value
        actual_code = int(result.code)
    except TclError as e:
        actual = e.message
        actual_code = 1
    except TclReturn as ret:
        actual = ret.value
        actual_code = 2
    except TclBreak:
        actual = ""
        actual_code = 3
    except TclContinue:
        actual = ""
        actual_code = 4

    # Check return codes
    code_ok = _check_return_code(actual_code, return_codes_str)

    # Check result match
    result_ok = _match_result(actual, expected, match_mode, interp)

    if code_ok and result_ok:
        _results["Passed"] += 1
    else:
        _results["Failed"] += 1
        _failed_tests.append(name)
        msg = f"\n==== {name} FAILED\n"
        if not code_ok:
            msg += f"---- Return code was: {actual_code}\n"
            msg += f"---- Return code should have been: {return_codes_str}\n"
        if not result_ok:
            msg += f"---- Result was:\n{actual}\n"
            msg += f"---- Result should have been ({match_mode} matching):\n{expected}\n"
        msg += f"==== {name} FAILED\n"
        _write_output(interp, msg)

    # Run cleanup
    if cleanup:
        try:
            interp.eval(cleanup)
        except TclError:
            pass

    return TclResult()


def _check_return_code(actual: int, expected_str: str) -> bool:
    """Check if *actual* return code matches *expected_str*."""
    code_map = {"ok": 0, "error": 1, "return": 2, "break": 3, "continue": 4}
    parts = _split_list(expected_str)
    for p in parts:
        p_lower = p.lower()
        if p_lower in code_map:
            if actual == code_map[p_lower]:
                return True
        else:
            try:
                if actual == int(p):
                    return True
            except ValueError:
                pass
    return False


def _match_result(actual: str, expected: str, mode: str, interp: TclInterp) -> bool:
    """Compare actual vs expected using the given match mode."""
    match mode:
        case "exact":
            return actual == expected
        case "glob":
            return fnmatch.fnmatch(actual, expected)
        case "regexp":
            return re.search(expected, actual) is not None
        case _:
            # Custom match?
            if mode in _custom_matchers:
                try:
                    result = interp.eval(
                        f"{_custom_matchers[mode]} {_list_escape(expected)} {_list_escape(actual)}"
                    )
                    return result.value in ("1", "true", "yes")
                except TclError:
                    return False
            return actual == expected


def _cmd_test_constraint(interp: TclInterp, args: list[str]) -> TclResult:
    """testConstraint constraint ?value?"""
    if not args:
        raise TclError('wrong # args: should be "testConstraint constraint ?value?"')

    constraint = args[0]
    if len(args) < 2:
        return TclResult(value="1" if _constraints.get(constraint, False) else "0")

    val = args[1]
    _constraints[constraint] = val.lower() not in ("0", "false", "no", "off", "")
    return TclResult(value=val)


def _cmd_cleanup_tests(interp: TclInterp, args: list[str]) -> TclResult:
    """cleanupTests"""
    total = _results["Total"]
    passed = _results["Passed"]
    skipped = _results["Skipped"]
    failed = _results["Failed"]
    _write_output(
        interp,
        f"\nTotal\t{total}\tPassed\t{passed}\tSkipped\t{skipped}\tFailed\t{failed}\n",
    )
    if _tmpdir and os.path.isdir(_tmpdir):
        import shutil

        try:
            shutil.rmtree(_tmpdir, ignore_errors=True)
        except OSError:
            pass
    return TclResult()


def _cmd_load_tested_commands(interp: TclInterp, args: list[str]) -> TclResult:
    """::tcltest::loadTestedCommands — no-op for the VM."""
    return TclResult()


def _cmd_configure(interp: TclInterp, args: list[str]) -> TclResult:
    """tcltest::configure ?option value ...?"""
    global _verbose, _testdir
    if not args:
        return TclResult()
    i = 0
    while i < len(args):
        opt = args[i]
        val = args[i + 1] if i + 1 < len(args) else ""
        match opt:
            case "-verbose":
                _verbose = val
            case "-match":
                _match_patterns.clear()
                _match_patterns.extend(_split_list(val))
            case "-skip":
                _skip_patterns.clear()
                _skip_patterns.extend(_split_list(val))
            case "-testdir":
                _testdir = val
            case "-tmpdir":
                global _tmpdir
                _tmpdir = val
        i += 2
    return TclResult()


def _cmd_custom_match(interp: TclInterp, args: list[str]) -> TclResult:
    """customMatch mode command"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "customMatch mode command"')
    _custom_matchers[args[0]] = args[1]
    return TclResult()


def _cmd_make_file(interp: TclInterp, args: list[str]) -> TclResult:
    """makeFile contents name ?directory?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "makeFile contents name ?directory?"')
    contents = args[0]
    name = args[1]
    directory = args[2] if len(args) > 2 else _get_tmpdir()
    filepath = os.path.join(directory, name)
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    with open(filepath, "w") as f:
        f.write(contents)
    return TclResult(value=filepath)


def _cmd_remove_file(interp: TclInterp, args: list[str]) -> TclResult:
    """removeFile name ?directory?"""
    if not args:
        raise TclError('wrong # args: should be "removeFile name ?directory?"')
    name = args[0]
    directory = args[1] if len(args) > 1 else _get_tmpdir()
    filepath = os.path.join(directory, name)
    try:
        os.unlink(filepath)
    except OSError:
        pass
    return TclResult()


def _cmd_make_directory(interp: TclInterp, args: list[str]) -> TclResult:
    """makeDirectory name ?directory?"""
    if not args:
        raise TclError('wrong # args: should be "makeDirectory name ?directory?"')
    name = args[0]
    directory = args[1] if len(args) > 1 else _get_tmpdir()
    dirpath = os.path.join(directory, name)
    os.makedirs(dirpath, exist_ok=True)
    return TclResult(value=dirpath)


def _cmd_remove_directory(interp: TclInterp, args: list[str]) -> TclResult:
    """removeDirectory name ?directory?"""
    if not args:
        raise TclError('wrong # args: should be "removeDirectory name ?directory?"')
    import shutil

    name = args[0]
    directory = args[1] if len(args) > 1 else _get_tmpdir()
    dirpath = os.path.join(directory, name)
    try:
        shutil.rmtree(dirpath, ignore_errors=True)
    except OSError:
        pass
    return TclResult()


def _cmd_interpreter(interp: TclInterp, args: list[str]) -> TclResult:
    """interpreter — return path to the Tcl interpreter."""
    return TclResult(value=sys.executable)


def _cmd_output_channel(interp: TclInterp, args: list[str]) -> TclResult:
    """outputChannel ?channelID?"""
    global _output_channel
    if args:
        _output_channel = args[0]
    return TclResult(value=_output_channel)


def _cmd_error_channel(interp: TclInterp, args: list[str]) -> TclResult:
    """errorChannel ?channelID?"""
    global _error_channel
    if args:
        _error_channel = args[0]
    return TclResult(value=_error_channel)


# Registration


_initialised = False


def setup_tcltest(interp: TclInterp) -> None:
    """Register tcltest as a loadable package in the interpreter.

    Called during interpreter initialisation so that
    ``package require tcltest`` succeeds.
    """
    from ..scope import ensure_namespace

    global _initialised
    if not _initialised:
        _reset_state()
        _initialised = True

    # Create the ::tcltest namespace
    ns = ensure_namespace(interp.root_namespace, "::tcltest")

    # Register package so ``package require tcltest 2.5`` works
    interp.packages["tcltest"] = {
        "version": "2.5.10",
        "loaded": True,
        "ifneeded": {},
    }

    # Map command short names to handlers
    cmds: dict[str, CommandHandler] = {
        "test": _cmd_test,
        "testConstraint": _cmd_test_constraint,
        "cleanupTests": _cmd_cleanup_tests,
        "loadTestedCommands": _cmd_load_tested_commands,
        "configure": _cmd_configure,
        "customMatch": _cmd_custom_match,
        "makeFile": _cmd_make_file,
        "removeFile": _cmd_remove_file,
        "makeDirectory": _cmd_make_directory,
        "removeDirectory": _cmd_remove_directory,
        "interpreter": _cmd_interpreter,
        "outputChannel": _cmd_output_channel,
        "errorChannel": _cmd_error_channel,
    }

    # Register in the namespace
    for cmd_name, handler in cmds.items():
        ns.register_command(cmd_name, handler)

    # Also set export patterns
    ns._export_patterns = list(cmds.keys())

    # Register fully-qualified names in the command registry
    for cmd_name, handler in cmds.items():
        interp.register_command(f"::tcltest::{cmd_name}", handler)

    # Also import into the root namespace so the commands are
    # available without an explicit ``namespace import``.  Real tclsh
    # relies on the test file calling ``namespace import -force
    # ::tcltest::*``, but many test files skip this when the
    # ``::tcltest`` namespace already exists.
    for cmd_name, handler in cmds.items():
        interp.root_namespace.register_command(cmd_name, handler)
