"""Session fixtures: build the LSP pyz once via make, run one server, query it.

A single ``lsp_server`` is started and initialised per test session (per
xdist worker) and shared by every test in this package, so adding a test is
just "take ``lsp_server`` and send it a request".
"""

from __future__ import annotations

import os
import shutil
import subprocess
from collections.abc import Iterator
from pathlib import Path
from typing import NamedTuple

import pytest

from .harness import (
    ROOT,
    LspServerClient,
    ensure_build_info,
    native_server_bin,
    server_kind,
    server_launch_argv,
)


class _Build(NamedTuple):
    pyz: Path
    version: str


@pytest.fixture(scope="session")
def _lsp_build() -> _Build:
    """Bring the selected LSP server up to date once.

    ``TCL_LSP_SERVER_KIND=rust`` drives the native ``tcl-lsp-server`` binary
    (located via :func:`native_server_bin`); the default ``python`` mode runs
    ``make zipapp-lsp`` and drives the packaged zipapp.

    For the python backend: ``make zipapp-lsp`` regenerates
    ``shared/_build_info.py`` (a ``.FORCE`` prerequisite) and rebuilds
    ``build/tcl-lsp-server-<version>.pyz`` from current sources.  Both the file
    name and the bundled build-info carry the same ``git describe`` version, so
    we read it back and address the artifact by its exact path rather than
    guessing from a glob.

    When ``TCL_LSP_SERVER_PYZ`` points at an already-built zipapp (the runner
    builds it once before launching parallel pytest workers — see
    ``_ci-fast-pytest``), reuse it directly.  That avoids every xdist worker
    re-running ``make zipapp-lsp`` concurrently against the same output path,
    and keeps the ci-fast gate fast.
    """
    if server_kind() == "rust":
        native = native_server_bin()
        if native is None or not native.exists():
            pytest.fail(
                "TCL_LSP_SERVER_KIND=rust but no native tcl-lsp-server binary was "
                "found — set TCL_LSP_SERVER_BIN or run `make rust-server` "
                "(`cargo build -p tcl-lsp-server`)."
            )
        # The native server reports its own serverInfo.version; reuse the repo's
        # build-info string for the version fixture (the strict version-match
        # assertion is python-pyz-specific — see test_server_version).
        return _Build(native, ensure_build_info())
    prebuilt = os.environ.get("TCL_LSP_SERVER_PYZ")
    if prebuilt:
        # Resolve to absolute: the server subprocess runs with cwd set to a
        # throwaway workspace, so a relative argv path would not resolve.
        pyz = Path(prebuilt).resolve()
        if not pyz.exists():
            pytest.fail(f"TCL_LSP_SERVER_PYZ={prebuilt!r} does not exist")
        return _Build(pyz, ensure_build_info())
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
    client = LspServerClient(server_launch_argv(lsp_pyz), cwd=workspace)
    client.start()
    client.initialize(root_uri=workspace.as_uri())
    try:
        yield client
    finally:
        client.shutdown()


@pytest.fixture(scope="session")
def lsp_server_irules(
    lsp_pyz: Path, tmp_path_factory: pytest.TempPathFactory
) -> Iterator[LspServerClient]:
    """A second server dedicated to F5 iRules-dialect documents.

    The server's active command/signature pack is process-global: opening an
    iRules document auto-switches the whole server into the ``f5-irules``
    dialect, which would then resolve a plain-Tcl ``socket`` hover in iRules
    context.  Rather than fight that statefulness, dialect-sensitive iRules
    tests run against their own server so the main ``lsp_server`` stays Tcl.
    """
    workspace = tmp_path_factory.mktemp("lsp-irules-workspace")
    client = LspServerClient(server_launch_argv(lsp_pyz), cwd=workspace)
    client.start()
    client.initialize(root_uri=workspace.as_uri())
    try:
        yield client
    finally:
        client.shutdown()


@pytest.fixture(scope="session")
def lsp_server_inlay(
    lsp_pyz: Path, tmp_path_factory: pytest.TempPathFactory
) -> Iterator[LspServerClient]:
    """A server with inlay hints enabled via ``workspace/configuration``.

    Inlay hints are gated off by default (``inlay_hints_enabled``), and the
    main ``lsp_server`` deliberately pins that default-off contract.  The inlay
    *content* regressions (optional-positional labelling, issue #510) need the
    provider switched on, so they run against this dedicated server whose
    ``tclLsp`` config reply opts into ``features.inlayHints`` (keeping linked
    editing on too, matching the default fixture).
    """
    workspace = tmp_path_factory.mktemp("lsp-inlay-workspace")
    client = LspServerClient(
        server_launch_argv(lsp_pyz),
        cwd=workspace,
        tcllsp_config={"features": {"linkedEditingRange": True, "inlayHints": True}},
    )
    client.start()
    client.initialize(root_uri=workspace.as_uri())
    try:
        yield client
    finally:
        client.shutdown()


@pytest.fixture(scope="session")
def lsp_server_bigip(
    lsp_pyz: Path, tmp_path_factory: pytest.TempPathFactory
) -> Iterator[LspServerClient]:
    """A server dedicated to F5 BIG-IP ``*.conf`` documents.

    Opening a ``bigip.conf`` as a server's first document auto-switches the
    whole process into the ``f5-bigip`` signature pack (and the BIG-IP
    diagnostics/outline paths are URI-keyed on the canonical basename), so —
    exactly like ``lsp_server_irules`` — these dialect-sensitive tests run on
    their own server rather than contaminating the plain-Tcl ``lsp_server``.
    """
    workspace = tmp_path_factory.mktemp("lsp-bigip-workspace")
    client = LspServerClient(server_launch_argv(lsp_pyz), cwd=workspace)
    client.start()
    client.initialize(root_uri=workspace.as_uri())
    try:
        yield client
    finally:
        client.shutdown()


@pytest.fixture
def bigip_uri_factory(request: pytest.FixtureRequest):
    """Fresh unique ``file://`` URIs whose basename is a canonical BIG-IP name.

    The server routes BIG-IP handling on the basename (``bigip.conf``,
    ``bigip_base.conf``, …), so the basename is fixed while the *directory* is
    made unique per call — keeping the long-lived shared server from serving
    one test a buffer another test left open.
    """
    safe = "".join(ch if ch.isalnum() else "_" for ch in request.node.nodeid)
    counter = {"n": 0}

    def make(basename: str = "bigip.conf") -> str:
        counter["n"] += 1
        return f"file:///bigip/{safe}_{counter['n']}/{basename}"

    return make


@pytest.fixture
def uri_factory(request: pytest.FixtureRequest):
    """Return a callable producing a fresh, unique ``file://`` URI per call.

    Every test gets its own document URIs (namespaced by the test's node id)
    so the long-lived shared server never serves one test a buffer another
    test left open, and so version-tagged diagnostics never collide.
    """
    # Use the full node id (path::class::name), not just the function name, so
    # two same-named tests in different files/classes can never collide on a URI
    # and read each other's buffered, version-tagged diagnostics.
    safe = "".join(ch if ch.isalnum() else "_" for ch in request.node.nodeid)
    counter = {"n": 0}

    def make(suffix: str = "tcl") -> str:
        counter["n"] += 1
        return f"file:///e2e/{safe}_{counter['n']}.{suffix}"

    return make
