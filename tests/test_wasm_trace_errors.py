# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""WASM variable-trace error semantics.

Pins the contract from ``tmp/tcl9.0.3/generic/tclTrace.c``
(``TclCallVarTraces``, ``done:`` label) and ``tclVar.c``:

* When a **read** trace callback returns an error, the operation fails
  with ``can't read "NAME": <callback msg>``.
* When a **write** trace callback returns an error, it fails with
  ``can't set "NAME": <callback msg>`` (the verb is ``set``, not
  ``write``).
* ``NAME`` is the user-facing variable name — ``arr(key)`` for an array
  element, even when the matching trace was installed on the whole array.
* Errors raised by **unset** trace callbacks are *ignored*: the unset
  still succeeds (``catch`` sees code 0) and any later callback in the
  same fire pass still runs.

Pre-fix the runtime evaluated the trace callback but discarded its
result code, so the raw callback message (e.g. ``trace returned error``)
surfaced unwrapped, and an erroring unset trace aborted the unset.  This
took out the whole ``trace`` stem's error-return group
(trace-0.0 / trace-8.1 / 8.2 / 8.3 / 8.4 / 8.5 / 8.6) in
``tests/baselines/tcl9-tcltest-wasm/``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


def test_read_trace_error_wrapped() -> None:
    """A read-trace error → ``can't read "x": <msg>``."""
    out = _run(
        """
proc te {args} {error "boom"}
set x 1
trace add variable x read te
puts [catch {set x} m]|$m
"""
    )
    assert out == '1|can\'t read "x": boom'


def test_write_trace_error_wrapped_with_set_verb() -> None:
    """A write-trace error → ``can't set "x": <msg>`` (verb is ``set``)."""
    out = _run(
        """
proc te {args} {error "boom"}
set x 1
trace add variable x write te
puts [catch {set x 2} m]|$m
"""
    )
    assert out == '1|can\'t set "x": boom'


def test_append_write_trace_error_wrapped() -> None:
    """``append`` fires the write trace too — same ``can't set`` wrap
    (upstream trace-8.3)."""
    out = _run(
        """
proc te {args} {error "boom"}
set x 1
trace add variable x write te
puts [catch {append x 44} m]|$m
"""
    )
    assert out == '1|can\'t set "x": boom'


def test_array_element_read_trace_error_uses_full_name() -> None:
    """An element read-trace error names the full ``arr(key)``."""
    out = _run(
        """
proc te {args} {error "boom"}
set x(0) 1
trace add variable x(0) read te
puts [catch {set x(0)} m]|$m
"""
    )
    assert out == '1|can\'t read "x(0)": boom'


def test_whole_array_read_trace_error_names_element() -> None:
    """A trace on the whole array still reports the accessed element's
    full name when it errors (upstream trace-8.5 shape)."""
    out = _run(
        """
proc te {args} {error "boom"}
set x(0) 1
trace add variable x read te
puts [catch {set x(0)} m]|$m
"""
    )
    assert out == '1|can\'t read "x(0)": boom'


def test_unset_trace_error_is_ignored() -> None:
    """An erroring unset trace does not fail the unset (upstream
    trace-8.6): ``catch`` sees code 0 and an empty result."""
    out = _run(
        """
proc te {args} {error "boom"}
set x 1
trace add variable x unset te
puts [catch {unset x} m]|$m
"""
    )
    assert out == "0|"


def test_unset_trace_error_does_not_stop_later_traces() -> None:
    """After an erroring unset trace is ignored, the remaining unset
    traces in the fire pass still run (upstream trace-8.4 shape)."""
    out = _run(
        """
set info {}
proc terr {args} {error "boom"}
proc ttag {args} {global info; lappend info tagged}
set x 1
trace add variable x unset ttag
trace add variable x unset terr
puts [catch {unset x} m]|$m|$info
"""
    )
    # Traces fire newest-first: terr (ignored) then ttag.
    assert out == "0||tagged"


def test_unset_trace_error_ignored_without_enclosing_catch() -> None:
    """An erroring unset trace must be ignored even when the triggering
    ``unset`` is NOT wrapped in a script-level ``catch`` — the callback is
    fired under a temporary saved interp state, so it cannot trap the
    interpreter. (Regression for PR #554 review: relying on an existing
    catch depth let the callback's ``error`` trap before the ignore ran.)"""
    out = _run(
        """
proc te {args} {error "boom"}
set x 1
trace add variable x unset te
unset x
puts done
"""
    )
    assert out == "done"


def test_read_trace_no_error_unaffected() -> None:
    """A read trace that succeeds returns the value normally."""
    out = _run(
        """
proc te {args} {}
set x 42
trace add variable x read te
puts [set x]
"""
    )
    assert out == "42"
