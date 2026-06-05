"""Session fixtures: build the LSP pyz once via make, run one server, query it.

A single ``lsp_server`` is started and initialised per test session (per
xdist worker) and shared by every test in this package, so adding a test is
just "take ``lsp_server`` and send it a request".
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import NamedTuple

import pytest

from .harness import ROOT, LspServerClient, ensure_build_info


class _Build(NamedTuple):
    pyz: Path
    version: str


@pytest.fixture(scope="session")
def _lsp_build() -> _Build:
    """Bring the packaged LSP server zipapp up to date once, via ``make``.

    ``make zipapp-lsp`` regenerates ``shared/_build_info.py`` (a ``.FORCE``
    prerequisite) and rebuilds ``build/tcl-lsp-server-<version>.pyz`` from
    current sources.  Both the file name and the bundled build-info carry the
    same ``git describe`` version, so we read it back and address the artifact
    by its exact path rather than guessing from a glob.
    """
    if shutil.which("make") is None:
        pytest.skip("make is required to build the LSP zipapp")
    proc = subprocess.run(
        ["make", "zipapp-lsp"],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        pytest.fail(
            "`make zipapp-lsp` failed:\n"
            f"--- stdout ---\n{proc.stdout}\n--- stderr ---\n{proc.stderr}"
        )
    version = ensure_build_info()  # make just (re)generated shared/_build_info.py
    pyz = ROOT / "build" / f"tcl-lsp-server-{version}.pyz"
    if not pyz.exists():
        available = sorted(p.name for p in (ROOT / "build").glob("tcl-lsp-server-*.pyz"))
        pytest.fail(
            f"`make zipapp-lsp` finished but {pyz.name} is missing; build/ has: {available}"
        )
    return _Build(pyz, version)


@pytest.fixture(scope="session")
def lsp_pyz(_lsp_build: _Build) -> Path:
    """Path to the freshly built LSP server zipapp."""
    return _lsp_build.pyz


@pytest.fixture(scope="session")
def lsp_full_version(_lsp_build: _Build) -> str:
    """The build version the running server is expected to report back."""
    return _lsp_build.version


@pytest.fixture(scope="session")
def lsp_server(
    lsp_pyz: Path, tmp_path_factory: pytest.TempPathFactory
) -> Iterator[LspServerClient]:
    """Start and initialise one server from the pyz; tear it down at the end."""
    workspace = tmp_path_factory.mktemp("lsp-workspace")
    client = LspServerClient([sys.executable, str(lsp_pyz)], cwd=workspace)
    client.start()
    client.initialize(root_uri=workspace.as_uri())
    try:
        yield client
    finally:
        client.shutdown()


@pytest.fixture
def uri_factory(request: pytest.FixtureRequest):
    """Return a callable producing a fresh, unique ``file://`` URI per call.

    Every test gets its own document URIs (namespaced by the test's node id)
    so the long-lived shared server never serves one test a buffer another
    test left open, and so version-tagged diagnostics never collide.
    """
    safe = "".join(ch if ch.isalnum() else "_" for ch in request.node.name)
    counter = {"n": 0}

    def make(suffix: str = "tcl") -> str:
        counter["n"] += 1
        return f"file:///e2e/{safe}_{counter['n']}.{suffix}"

    return make
