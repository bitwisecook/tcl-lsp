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

"""Drift test: verify the inline rgxg-generated regexes match what rgxg
currently produces.

The IPv4/IPv6 literal regexes in :mod:`dialects.f5.bigip.redact_map` are
hand-pasted from ``rgxg cidr 0.0.0.0/0`` and ``rgxg cidr ::/0`` so
they accept exactly the strings :mod:`ipaddress` would parse.  If a
later rgxg release tightens or relaxes the grammar we want to know;
this test re-runs rgxg (when installed) and checks the inline
strings byte-for-byte.

Skipped silently when rgxg isn't on PATH — CI runners may not have it.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.redact_map import _RGXG_V4, _RGXG_V6

pytestmark = pytest.mark.skipif(shutil.which("rgxg") is None, reason="rgxg not on PATH")


def _run_rgxg(cidr: str) -> str:
    proc = subprocess.run(["rgxg", "cidr", cidr], capture_output=True, text=True, check=True)
    return proc.stdout.rstrip("\n")


def test_inline_v4_matches_rgxg_output():
    expected = _run_rgxg("0.0.0.0/0")
    assert _RGXG_V4 == expected, (
        "dialects.f5.bigip.redact_map._RGXG_V4 has drifted from `rgxg cidr 0.0.0.0/0`. "
        "Regenerate with that command and update the inline string."
    )


def test_inline_v6_matches_rgxg_output():
    expected = _run_rgxg("::/0")
    assert _RGXG_V6 == expected, (
        "dialects.f5.bigip.redact_map._RGXG_V6 has drifted from `rgxg cidr ::/0`. "
        "Regenerate with that command and update the inline string."
    )
