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

"""Regression test for the ``return -options`` value-ownership leak.

``return -options DICT`` is lowered through the eval fallback to the
runtime ``eval_return`` builtin (``runtime/zig/cmds/flow.zig``), which
expands the dict via ``apply_return_options_dict`` /
``apply_one_return_option``.  ``ret_opt_elem_to_obj`` mints a fresh +1
``val_obj`` for every ``KEY VALUE`` pair, but the expander only released
``key_obj`` — every value leaked:

* ``-code`` / ``-level`` values (read as a string, never stored),
* a nested ``-options`` dict (recursed into, never released),
* arbitrary extras after ``dict_set`` copies the bytes it needs, and
* the ``-errorcode`` / ``-errorinfo`` slots (the +1 was stored but the
  consumers — ``tcl_cmd_error_full`` / ``global_set`` — *retain* a borrow
  rather than consume the caller's reference, so it was never freed),
  including the overwritten-slot case (``-errorcode A -errorcode B``).

The fix makes ownership explicit: ``apply_one_return_option`` reports
whether it took ownership of ``val_obj`` (the ``-error*`` slots do), the
loop releases every transient value, and ``eval_return`` frees the owned
``-error*`` slots via a ``defer`` after consumption.

**Isolating the leak from baselines.**  The leak-check ``residual`` counter
carries two non-leak contributions that confound an absolute
``residual == 0`` check: a per-iteration *baseline* unrelated to
``-options`` (a bare ``catch {proc-that-returns}`` retains several objects
per call in this harness), and a *constant* retained-state offset (the
last ``return``'s ``-options`` extras dict survives as
``last_return_extras``, sized by its key count).  A genuine per-call leak
is the only contribution that grows with **both** the value-key count and
the iteration count, so we compare the residual *growth slope* (residual
at a high vs a low iteration count) between two dicts that differ only in
their number of value-bearing keys.  The baseline and the retained-state
offset cancel; a per-key leak makes the many-key slope exceed the few-key
slope by ``extra_keys * (HI - LO)``.  With the fix the slopes match
exactly.  ``double_free`` / ``pending`` guard the added releases against
over-release.

Requires the leak-check runtime (``make build-runtime-leakcheck``);
skips otherwise.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from tests.test_wasm_switch_overrelease import (  # noqa: E402
    _has_leakcheck_exports,
    _run_under_leakcheck,
)

pytestmark = pytest.mark.skipif(
    not _has_leakcheck_exports(),
    reason="leak-check runtime exports missing — run 'make build-runtime-leakcheck'",
)

_LO, _HI = 100, 500


def _src(body: str, iters: int) -> str:
    """Compile to ``::top``: a loop that catches *body* *iters* times."""
    return (
        f"proc boom {{}} {{\n    {body}\n}}\n"
        f"for {{set i 0}} {{$i < {iters}}} {{incr i}} {{ catch {{boom}} }}\n"
    )


def _growth(body: str) -> tuple[int, int, int]:
    """Return (residual_slope, double_free, pending) for *body*.

    ``residual_slope`` is ``residual(HI) - residual(LO)`` — a per-call leak
    scales it with the iteration span; constant baselines/retained-state do
    not contribute.
    """
    lo = _run_under_leakcheck(_src(body, _LO))
    hi = _run_under_leakcheck(_src(body, _HI))
    return (
        hi["residual"] - lo["residual"],
        max(lo["double_free"], hi["double_free"]),
        max(lo["pending"], hi["pending"]),
    )


# (few, many) — same 4-word command (``return -options DICT boom``) and the
# same standard ``-code error`` slot, differing ONLY in the count of
# transient value-bearing keys.  Each tuple targets one release path.
_PAIRS = {
    # Arbitrary extras → dict_set copies the bytes; val_obj must be freed.
    "extras": (
        "return -options {-code error -k0 a} boom",
        "return -options {-code error -k0 a -k1 a -k2 a -k3 a -k4 a -k5 a} boom",
    ),
    # Repeated -errorcode → each prior owned slot must be freed on overwrite.
    "errorcode_replacement": (
        "return -options {-code error -errorcode {A 0}} boom",
        "return -options "
        "{-code error -errorcode {A 0} -errorcode {A 1} -errorcode {A 2} "
        "-errorcode {A 3} -errorcode {A 4}} boom",
    ),
    # Repeated -errorinfo → same overwrite-release path for the info slot.
    "errorinfo_replacement": (
        "return -options {-code error -errorinfo i0} boom",
        "return -options "
        "{-code error -errorinfo i0 -errorinfo i1 -errorinfo i2 -errorinfo i3 -errorinfo i4} boom",
    ),
    # Nested -options dicts (recursed into) — the nested dict val_obj itself
    # must be freed after the recursion reads it.
    "nested_options": (
        "return -options {-code error -options {-x 1}} boom",
        "return -options "
        "{-code error -options {-x 1} -options {-x 2} -options {-x 3} -options {-x 4}} boom",
    ),
}


@pytest.mark.parametrize("name", sorted(_PAIRS))
def test_return_options_value_keys_do_not_leak(name):
    """The residual *growth slope* must not depend on the number of
    value-bearing keys in a ``-options`` dict.

    Pre-fix: each extra key leaked one ``val_obj`` per call, so the many-key
    slope exceeded the few-key slope by ``extra_keys * (HI - LO)``.
    Post-fix: the slopes are equal.
    """
    few_body, many_body = _PAIRS[name]
    few_slope, few_df, few_pend = _growth(few_body)
    many_slope, many_df, many_pend = _growth(many_body)
    assert many_slope == few_slope, (
        f"return -options leaks a TclObj per value key per call ({name}): "
        f"residual slope {few_slope} (few) vs {many_slope} (many) over "
        f"{_LO}->{_HI} iters — a val_obj is not released in "
        f"apply_return_options_dict / apply_one_return_option."
    )
    # The added releases must not over-release.
    assert few_df == 0 and many_df == 0, (name, few_df, many_df)
    assert few_pend == 0 and many_pend == 0, (name, few_pend, many_pend)
