"""First end-to-end probe: ask the running server what version it is.

The version is deliberately the smallest real thing to query — it comes
straight back in the ``initialize`` response's ``serverInfo`` — but it is a
genuine round-trip against the live, packaged server, not a static check.

It guards a real regression: ``server/server.py`` once imported
``FULL_VERSION`` via ``from ._build_info`` (a module the build never
generates), so the packaged server silently fell back to ``"dev"`` and
advertised ``vdev`` during ``initialize``.  Booting the shipped pyz and
reading the version it actually reports catches that — and any other way
the banner could regress to the fallback.

More substantive conformance tests (hover, completion, diagnostics, …)
should be added alongside this one against the same ``lsp_server``.
"""

from __future__ import annotations

from pathlib import Path

import tomllib

from .harness import server_kind


def _rust_crate_version() -> str:
    """The ``[workspace.package] version`` the native binary reports back.

    The Rust server advertises ``env!("CARGO_PKG_VERSION")``, which
    ``rust/tcl-lsp-server`` inherits from the workspace root — so this reads the
    same source of truth the compiled banner comes from.
    """
    root = Path(__file__).resolve().parents[2]
    with (root / "Cargo.toml").open("rb") as f:
        return tomllib.load(f)["workspace"]["package"]["version"]


def test_initialize_reports_version(lsp_server, request):
    """serverInfo.version from the live server is the real version, not 'dev'.

    Python packaged ``.pyz``: the banner is ``v{FULL_VERSION}`` (a regression to
    ``vdev`` means build-info was imported from a path the build never
    generates).  Native Rust: the banner is the workspace crate version
    (``CARGO_PKG_VERSION``).  Either way it must be a real, non-fallback value.
    """
    info = lsp_server.server_info
    assert info is not None, "initialize result had no serverInfo — cannot read the version banner"
    reported = info.get("version")
    assert reported, "server reported no version banner"
    assert reported not in ("vdev", "dev"), (
        f"server fell back to a dev version banner: {reported!r}"
    )
    if server_kind() == "rust":
        # The native binary reports the workspace crate version verbatim (no
        # leading ``v``), so match it against the same Cargo source of truth.
        expected = _rust_crate_version()
        assert reported == expected, (
            f"native server reported version {reported!r}, expected the workspace "
            f"crate version {expected!r} (CARGO_PKG_VERSION)"
        )
        return
    # The pyz banner is f"v{FULL_VERSION}"; a broken build-info import yields "vdev".
    lsp_full_version = request.getfixturevalue("lsp_full_version")
    assert reported == f"v{lsp_full_version}", (
        f"running server reported version {reported!r}, expected "
        f"{'v' + lsp_full_version!r}. A regression to 'vdev' means a module "
        f"imported build-info from a path the build never generates, so the "
        f"server fell back to the 'dev' default."
    )


def test_server_info_name(lsp_server):
    """The server identifies itself as tcl-lsp (sanity for the harness)."""
    info = lsp_server.server_info
    assert info is not None
    assert info.get("name") == "tcl-lsp"
