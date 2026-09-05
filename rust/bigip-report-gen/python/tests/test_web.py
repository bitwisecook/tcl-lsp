# tcl-lsp — a language server and toolchain for Tcl
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for the stdlib f5report web server."""

from __future__ import annotations

import os
import threading
import time
import urllib.request

import pytest
from f5report import web


@pytest.fixture()
def server():
    srv = web.ThreadingHTTPServer(("127.0.0.1", 0), web._Handler)
    port = srv.server_address[1]
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    time.sleep(0.2)
    try:
        yield f"http://127.0.0.1:{port}"
    finally:
        srv.shutdown()
        srv.server_close()


def _get(url: str) -> str:
    return urllib.request.urlopen(url, timeout=10).read().decode()


def test_builder_page_and_version(server):
    page = _get(server + "/")
    assert "selectBackend" in page  # the shared input controller
    # The __CSP__ placeholder is substituted, and the server page's policy allows
    # only same-origin connections (its ServerBackend POSTs to /generate) — never
    # an external origin, and never a leftover unsubstituted placeholder.
    assert "__CSP__" not in page
    assert "Content-Security-Policy" in page and "connect-src 'self'" in page
    assert _get(server + "/version").strip()  # engine version string
    assert "F5 QUERY DSL" in _get(server + "/manual")  # embedded manual


def test_generate_plaintext_config(server):
    boundary = "BND"
    scf = b"ltm virtual /Common/vs { destination 10.0.0.1:443 }\n"
    body = (
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="files"; '
            f'filename="bigip.conf"\r\n\r\n'
        ).encode()
        + scf
        + b"\r\n"
    )
    for name, val in (("title", "My Report"), ("reportId", "abc-123")):
        body += (
            f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n{val}\r\n'
        ).encode()
    body += f"--{boundary}--\r\n".encode()
    req = urllib.request.Request(
        server + "/generate",
        data=body,
        headers={"Content-Type": f"multipart/form-data; boundary={boundary}"},
    )
    resp = urllib.request.urlopen(req, timeout=30)
    html = resp.read().decode()
    assert resp.headers.get("X-Device-Count") == "1"
    assert 'data-report-id="abc-123"' in html
    assert "vs" in html  # the virtual server made it into the report


def test_uploads_with_colliding_sanitised_names_stay_distinct():
    """Two filenames that sanitise to one basename must not overwrite each
    other: the report would lose a device and show another twice."""
    parts = [
        web.Part("files", "prod east.ucs", b"ltm virtual /Common/east { }\n"),
        web.Part("files", "prod_east.ucs", b"ltm virtual /Common/west { }\n"),
    ]
    sources, tmp = web._Handler._sources_from_parts(parts, "")
    try:
        assert len({uri for uri, _ in sources}) == 2
        # The basename still has to survive the spill — `_device_name` falls
        # back to it, and the engine reads the extension to spot a UCS.
        assert [os.path.basename(uri) for uri, _ in sources] == [
            "prod_east.ucs",
            "prod_east.ucs",
        ]
        assert "/Common/east" in sources[0][1]
        assert "/Common/west" in sources[1][1]
    finally:
        web._rmtree(tmp)
