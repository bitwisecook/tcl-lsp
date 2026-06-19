"""Zipapp build packs marker-gated transitive deps for the minimum Python.

Regression for #657: the release ``.pyz`` is built on a newer interpreter
(3.11+), but must run on the ``requires-python`` floor (3.10).  ``uv pip
install --target`` evaluates environment markers against the *build*
interpreter, so a version-gated dep like ``exceptiongroup; python_version <
"3.11"`` (a cattrs requirement reached via pygls) was silently dropped, and
the LSP server died on a 3.10 host with ``ModuleNotFoundError: No module named
'exceptiongroup'``.

The fix pins resolution to ``MIN_PYTHON`` via ``--python-version``.  These
tests assert the flag is wired through (fast, no install) and — when ``uv`` is
available — that the floor actually pulls the marker-gated dep (slow, network).
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from scripts.build import zipapps


def test_min_python_matches_requires_python_floor():
    """MIN_PYTHON must track the pyproject ``requires-python`` floor."""
    pyproject = (Path(zipapps.ROOT) / "pyproject.toml").read_text()
    assert f'requires-python = ">={zipapps.MIN_PYTHON}"' in pyproject


def test_pip_install_resolves_for_min_python(monkeypatch, tmp_path):
    """The install command pins marker resolution to the minimum Python."""
    captured: list[list[str]] = []

    def fake_check_call(cmd, *args, **kwargs):
        captured.append(cmd)

    monkeypatch.setattr(zipapps.subprocess, "check_call", fake_check_call)

    zipapps._pip_install_pure(tmp_path, ("pygls>=2.0",))

    assert len(captured) == 1
    cmd = captured[0]
    assert "--python-version" in cmd
    assert cmd[cmd.index("--python-version") + 1] == zipapps.MIN_PYTHON
    # Resolution version sits before the requested packages.
    assert cmd.index("--python-version") < cmd.index("pygls>=2.0")


def test_pip_install_noop_for_empty_packages(monkeypatch, tmp_path):
    """No subprocess is spawned when a profile bundles no pip packages."""
    called = False

    def fake_check_call(cmd, *args, **kwargs):
        nonlocal called
        called = True

    monkeypatch.setattr(zipapps.subprocess, "check_call", fake_check_call)
    zipapps._pip_install_pure(tmp_path, ())
    assert not called


@pytest.mark.skipif(shutil.which("uv") is None, reason="uv not available")
def test_min_python_floor_bundles_exceptiongroup(tmp_path):
    """End-to-end: the minimum-Python floor pulls in the gated cattrs dep.

    Guards the actual #657 failure mode — on a 3.11+ build host, the default
    marker resolution drops ``exceptiongroup`` from the pygls/cattrs chain.
    """
    zipapps._pip_install_pure(tmp_path, ("pygls>=2.0",))
    assert (tmp_path / "exceptiongroup").is_dir(), (
        "exceptiongroup must be bundled for Python 3.10 hosts (see #657)"
    )
