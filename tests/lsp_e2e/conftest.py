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

import pytest

from .harness import ROOT, LspServerClient, ensure_build_info

# Matches the Makefile's ZIPAPP_LSP := $(BUILD_DIR)/tcl-lsp-server-$(VERSION).pyz
_PYZ_GLOB = "tcl-lsp-server-*.pyz"


@pytest.fixture(scope="session")
def lsp_pyz() -> Path:
    """Bring the packaged LSP server zipapp up to date once, via ``make``.

    ``make zipapp-lsp`` regenerates ``shared/_build_info.py`` (a ``.FORCE``
    prerequisite) and rebuilds ``build/tcl-lsp-server-<version>.pyz`` from
    current sources, so the server we then launch reflects the tree under
    test and reports its real build version.
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
    built = sorted(
        (ROOT / "build").glob(_PYZ_GLOB), key=lambda p: p.stat().st_mtime
    )
    assert built, f"`make zipapp-lsp` succeeded but no {_PYZ_GLOB} in build/"
    return built[-1]


@pytest.fixture(scope="session")
def lsp_full_version(lsp_pyz: Path) -> str:
    """The build version the running server is expected to report back.

    Read after ``make`` has (re)generated ``shared/_build_info.py`` so it
    matches the copy bundled into ``lsp_pyz``.
    """
    return ensure_build_info()


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
