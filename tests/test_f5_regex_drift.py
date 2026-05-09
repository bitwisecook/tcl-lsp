"""Drift test: verify the inline rgxg-generated regexes match what rgxg
currently produces.

The IPv4/IPv6 literal regexes in :mod:`core.bigip.redact_map` are
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

from core.bigip.redact_map import _RGXG_V4, _RGXG_V6

pytestmark = pytest.mark.skipif(shutil.which("rgxg") is None, reason="rgxg not on PATH")


def _run_rgxg(cidr: str) -> str:
    proc = subprocess.run(["rgxg", "cidr", cidr], capture_output=True, text=True, check=True)
    return proc.stdout.rstrip("\n")


def test_inline_v4_matches_rgxg_output():
    expected = _run_rgxg("0.0.0.0/0")
    assert _RGXG_V4 == expected, (
        "core.bigip.redact_map._RGXG_V4 has drifted from `rgxg cidr 0.0.0.0/0`. "
        "Regenerate with that command and update the inline string."
    )


def test_inline_v6_matches_rgxg_output():
    expected = _run_rgxg("::/0")
    assert _RGXG_V6 == expected, (
        "core.bigip.redact_map._RGXG_V6 has drifted from `rgxg cidr ::/0`. "
        "Regenerate with that command and update the inline string."
    )
