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


def test_initialize_reports_packaged_build_version(lsp_server, lsp_full_version):
    """serverInfo.version from the live server is the build version, not 'dev'."""
    info = lsp_server.server_info
    assert info is not None, "initialize result had no serverInfo — cannot read the version banner"
    reported = info.get("version")
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
