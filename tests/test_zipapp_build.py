"""Zipapp builds pack marker-gated transitive deps for the minimum Python.

Regression for #657: the release ``.pyz`` is built on a newer interpreter
(3.11+), but must run on the ``requires-python`` floor (3.10).  ``uv pip
install --target`` evaluates environment markers against the *build*
interpreter, so version-gated deps were silently dropped and the server died
on a 3.10 host with ``ModuleNotFoundError``:

* ``exceptiongroup`` (a cattrs requirement reached via pygls / lsprotocol,
  gated on ``python_version < "3.11"``) — the symptom reported in #657.
* ``tomli`` (the ``tomllib`` fallback ``import``ed by shipped ``dialects`` /
  ``tooling`` modules on < 3.11) — never a transitive dep of a bundled
  package, so each profile must list it explicitly.

The fix pins marker resolution to ``MIN_PYTHON`` via ``--python-version`` and
adds ``tomli`` to every profile.  These tests stay offline (no PyPI access):
they assert the resolution flag is wired through and that ``tomli`` is bundled
everywhere, without performing a real install.
"""

from __future__ import annotations

import sys
from pathlib import Path

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


def test_every_profile_bundles_tomli_fallback():
    """Each profile ships ``dialects`` (and ``tooling``), which import ``tomli``
    as the ``tomllib`` fallback on Python < 3.11.  ``tomli`` is not a
    transitive dep of any bundled package, so every profile must list it
    explicitly or 3.10 hosts hit ``ModuleNotFoundError: No module named
    'tomli'`` (e.g. parsing an F5 iRule trailer)."""
    for name, profile in zipapps.PROFILES.items():
        # ``dialects`` carries the tomllib-fallback modules; assert the
        # invariant the bundling relies on, then that tomli is listed.
        assert "dialects" in profile.packages, name
        assert any(pkg.startswith("tomli") for pkg in profile.pip_packages), (
            f"profile {name!r} ships dialects/tooling (which import tomli on "
            f"Python < 3.11) but does not bundle tomli"
        )
