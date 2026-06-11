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

from .harness import server_kind


def test_initialize_reports_packaged_build_version(lsp_server, lsp_full_version):
    """serverInfo.version from the live server is the build version, not 'dev'."""
    info = lsp_server.server_info
    assert info is not None, "initialize result had no serverInfo — cannot read the version banner"
    reported = info.get("version")
    if server_kind() == "rust":
        # The native binary reports its own (Cargo) version, not the Python
        # zipapp build string, so the exact ``v{FULL_VERSION}`` match is
        # pyz-specific.  Still guard the regression class the test exists for:
        # a real, non-empty, non-fallback version banner.
        assert reported, "native server reported no version banner"
        assert reported not in ("vdev", "dev"), (
            f"native server fell back to a dev version banner: {reported!r}"
        )
        return
    # The banner is f"v{FULL_VERSION}"; a broken build-info import yields "vdev".
    assert reported == f"v{lsp_full_version}", (
        f"running server reported version {reported!r}, expected "
        f"{'v' + lsp_full_version!r}. A regression to 'vdev' means a module "
        f"imported build-info from a path the build never generates, so the "
        f"server fell back to the 'dev' default."
    )
    assert reported != "vdev", "version banner fell back to the 'dev' default"


def test_server_info_name(lsp_server):
    """The server identifies itself as tcl-lsp (sanity for the harness)."""
    info = lsp_server.server_info
    assert info is not None
    assert info.get("name") == "tcl-lsp"
