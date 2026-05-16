"""End-to-end TLS tests for the f5 query DSL.

Spins up a real HTTPS server in-process with a freshly generated
self-signed cert and exercises the network probes (`url_get`,
`tls_handshake`, `x509_parse`, `cert_load`) against it.  Marked
`slow` so it only runs under `make test-slow` and the comprehensive
CI gates — the cert generation, TLS handshake, and HTTPS
round-trip take noticeably longer than a unit test.

The fixture binds `TLS_CA_BUNDLE` for the duration of each query
so urllib + ssl both trust the bundle's CA, mirroring what an
operator would do when probing an endpoint signed by an internal /
private CA from inside a BIG-IP query.
"""

from __future__ import annotations

import socket
import ssl
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query
from core.bigip.query._probes import PROBES_ENABLED, TLS_CA_BUNDLE

pytestmark = pytest.mark.slow


# ---------------------------------------------------------------------------
# Self-signed CA + leaf cert helpers
# ---------------------------------------------------------------------------


def _make_self_signed(tmp_path: Path, hostname: str) -> tuple[Path, Path]:
    """Generate a self-signed CA + leaf for *hostname*.

    Returns ``(cert_pem_path, key_pem_path)``.  The cert has a SAN
    for *hostname* so urllib's strict-mode hostname check passes.
    Both CA + leaf share the same key in this simplified fixture —
    fine for handshake testing, the chain is still a single self-
    signed cert that the test pins as the trusted root.
    """
    pytest.importorskip("cryptography")
    import datetime

    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import rsa
    from cryptography.x509 import oid

    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    name = x509.Name([x509.NameAttribute(oid.NameOID.COMMON_NAME, hostname)])
    now = datetime.datetime.now(datetime.timezone.utc)
    cert = (
        x509.CertificateBuilder()
        .subject_name(name)
        .issuer_name(name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now - datetime.timedelta(minutes=1))
        .not_valid_after(now + datetime.timedelta(days=1))
        .add_extension(
            x509.SubjectAlternativeName([x509.DNSName(hostname), x509.DNSName("localhost")]),
            critical=False,
        )
        .add_extension(
            x509.BasicConstraints(ca=True, path_length=None),
            critical=True,
        )
        .sign(key, hashes.SHA256())
    )
    cert_path = tmp_path / "leaf.crt"
    key_path = tmp_path / "leaf.key"
    cert_path.write_bytes(cert.public_bytes(serialization.Encoding.PEM))
    key_path.write_bytes(
        key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.TraditionalOpenSSL,
            encryption_algorithm=serialization.NoEncryption(),
        )
    )
    return cert_path, key_path


# ---------------------------------------------------------------------------
# Mock HTTPS server
# ---------------------------------------------------------------------------


class _Handler(BaseHTTPRequestHandler):
    routes = {
        "/health": (200, "application/json", b'{"status":"ok","uptime_s":42}'),
        "/inventory": (
            200,
            "application/json",
            b'{"servers":[{"name":"web1","port":443},{"name":"api1","port":8443}]}',
        ),
        "/text": (200, "text/plain", b"hello tls"),
        "/teapot": (418, "text/plain", b"short and stout"),
    }

    def do_GET(self):  # noqa: N802 — stdlib convention
        status, ctype, body = self.routes.get(self.path, (404, "text/plain", b"not found"))
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args, **_kwargs):
        return  # silence stderr


@pytest.fixture
def https_server(tmp_path):
    """Spin a TLS-wrapped HTTPServer on a random port; yield ``(url, cert)``.

    The server is reachable as ``https://localhost:<port>`` and signed
    by the cert that the test pins as the trusted CA.  Tears down on
    fixture exit (server.shutdown + thread join).
    """
    cert_path, key_path = _make_self_signed(tmp_path, "localhost")
    server = HTTPServer(("127.0.0.1", 0), _Handler)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(certfile=str(cert_path), keyfile=str(key_path))
    server.socket = ctx.wrap_socket(server.socket, server_side=True)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        # Confirm the listener is alive before yielding — TLS sockets
        # can take a beat to become reachable on slower CI runners.
        _wait_listening("127.0.0.1", port, timeout_s=5.0)
        yield f"https://localhost:{port}", cert_path
    finally:
        server.shutdown()
        server.server_close()


def _wait_listening(host: str, port: int, *, timeout_s: float) -> None:
    """Poll a TCP listener until it accepts a connection or *timeout_s* elapses."""
    import time

    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=0.2):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"server at {host}:{port} did not start within {timeout_s}s")


# ---------------------------------------------------------------------------
# Query helpers
# ---------------------------------------------------------------------------


def _run_with_probes(query: str, *, ca_bundle: Path | None = None) -> list:
    """Run a query with probes enabled and an optional pinned CA bundle."""
    probes_token = PROBES_ENABLED.set(True)
    bundle_token = TLS_CA_BUNDLE.set(str(ca_bundle)) if ca_bundle is not None else None
    try:
        result = run_query(query, {"mem://x": ""})
    finally:
        if bundle_token is not None:
            TLS_CA_BUNDLE.reset(bundle_token)
        PROBES_ENABLED.reset(probes_token)
    flat: list = []
    for values in result.values_per_file.values():
        flat.extend(values)
    return flat


def _scf_with_https_endpoint(url: str) -> str:
    """Build a tiny LTM SCF that names an HTTPS endpoint in `description`.

    Putting the URL in the VS ``description`` (with a label) is the
    natural way to thread external metadata through to a query that
    probes the endpoint and writes back per-VS status.  Matches the
    GTM-from-LTM example pattern: human-readable metadata on the VS
    that the engine can lift out and act on.
    """
    return (
        "ltm node /Common/web1 { address 10.0.0.10 }\n"
        "ltm pool /Common/web_pool {\n"
        "    members { /Common/web1:443 { address 10.0.0.10 } }\n"
        "    monitor /Common/https\n"
        "}\n"
        f"ltm virtual /Common/web_vs {{\n"
        f"    destination /Common/10.0.0.10:443\n"
        f"    ip-protocol tcp\n"
        f'    description "health-url={url}/health"\n'
        f"    pool /Common/web_pool\n"
        f"}}\n"
        f"ltm virtual /Common/inventory_vs {{\n"
        f"    destination /Common/10.0.0.10:8443\n"
        f"    ip-protocol tcp\n"
        f'    description "inventory-url={url}/inventory"\n'
        f"    pool /Common/web_pool\n"
        f"}}\n"
    )


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_url_get_validates_against_pinned_ca(https_server):
    """`url_get` honours TLS_CA_BUNDLE.  When the bundle is pinned
    ``reason.kind`` is ``"ok"``; without it, the call still
    collects the body + peer cert but ``reason.kind`` flags
    ``"self_signed"`` so audit queries can filter to the
    unverified results."""
    url, cert = https_server
    # Without the CA bundle, the self-signed cert fails strict
    # verification.  We still collect the response (audit-mode
    # default) and the reason tells the consumer why.
    [unverified] = _run_with_probes(f'url_get("{url}/health")')
    assert unverified["status"] == 200
    assert unverified["reason"]["kind"] == "self_signed"
    assert unverified["reason"]["fatal"] is False
    assert unverified["peer_cert"] is not None
    # With the CA pinned the same call verifies clean.
    [resp] = _run_with_probes(f'url_get("{url}/health")', ca_bundle=cert)
    assert resp["status"] == 200
    assert resp["headers"]["content-type"] == "application/json"
    assert resp["reason"]["kind"] == "ok"
    assert resp["reason"]["fatal"] is False
    # body_json is pre-parsed because the response declared JSON.
    assert resp["body_json"] == {"status": "ok", "uptime_s": 42}


def test_url_get_body_json_traverses_inline(https_server):
    """The pre-parsed `body_json` field flows through a path chain."""
    url, cert = https_server
    [count] = _run_with_probes(
        f'url_get("{url}/inventory") | http_body_json(.).servers | count',
        ca_bundle=cert,
    )
    assert count == 2


def test_url_get_captures_peer_cert_on_https(https_server):
    """``url_get`` captures the negotiated server cert on every
    ``https://`` request — same dict shape every ``x509_*``
    helper produces.  No extra round-trip; the cert comes off
    the same handshake that served the HTTP body."""
    url, cert = https_server
    [resp] = _run_with_probes(f'url_get("{url}/health")', ca_bundle=cert)
    assert resp["status"] == 200
    peer = resp["peer_cert"]
    assert peer is not None
    assert "localhost" in peer["sans"]
    assert peer["key_size"] == 2048
    # The pre-parsed cert is the SAME dict shape ``x509_parse`` /
    # ``x509_from_config`` produce, so an equality check works
    # across the three sources.
    [matches] = _run_with_probes(
        f'x509_eq(url_get("{url}/health").peer_cert, url_get("{url}/health").peer_cert)',
        ca_bundle=cert,
    )
    assert matches is True


def test_url_get_peer_cert_is_null_for_http(https_server):
    """Plain ``http://`` URLs leave ``peer_cert`` as ``null`` so
    consumers can branch on ``select(.peer_cert != null)``."""
    # The fixture is https-only — use a small local HTTP server.
    import http.server
    import threading

    server = http.server.HTTPServer(("127.0.0.1", 0), http.server.BaseHTTPRequestHandler)
    port = server.server_address[1]
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        [resp] = _run_with_probes(f'url_get("http://127.0.0.1:{port}/")')
        # ``url_get`` returns either a status (when the dummy
        # handler responds) or ``status=None`` with an error; in
        # either case ``peer_cert`` is null for plain http.
        assert resp["peer_cert"] is None
    finally:
        server.shutdown()


def test_http_helpers_against_live_endpoint(https_server):
    url, cert = https_server
    [ok] = _run_with_probes(f'url_get("{url}/health") | http_ok(.)', ca_bundle=cert)
    assert ok is True
    [teapot] = _run_with_probes(f'url_get("{url}/teapot") | http_client_error(.)', ca_bundle=cert)
    assert teapot is True
    [text_type] = _run_with_probes(
        f'url_get("{url}/text") | http_header(., "content-type")', ca_bundle=cert
    )
    assert text_type == "text/plain"


def test_tls_handshake_returns_structured_peer_cert(https_server):
    """`tls_handshake` pre-parses the peer cert into the same
    structured dict `x509_parse` produces."""
    url, cert = https_server
    host, port = url.replace("https://", "").split(":")
    [info] = _run_with_probes(f'tls_handshake("{host}", {int(port)})', ca_bundle=cert)
    assert info["verify_status"] == "ok"
    assert info["protocol"].startswith("TLS")
    peer = info["peer_cert"]
    assert isinstance(peer, dict)
    assert "localhost" in peer.get("subject", "")
    assert "localhost" in peer.get("sans", [])
    assert peer["key_size"] == 2048


def test_cert_load_round_trip(https_server):
    """`cert_load` reads the same PEM the server is using and parses
    it without any network call."""
    _url, cert = https_server
    [parsed] = _run_with_probes(f'cert_load("{cert}")')
    assert "localhost" in parsed["subject"]
    assert "localhost" in parsed["sans"]
    assert parsed["key_size"] == 2048


def test_x509_eq_across_every_source(https_server):
    """The point of unified shape + ``x509_eq``: a cert reaches an
    audit pipeline from many sources (PEM on disk, live TLS
    handshake, captured ``url_get`` peer cert), and the four
    builtins below all produce dicts that ``x509_eq`` recognises
    as the same certificate.  Test pins every cross-source
    combination."""
    import urllib.parse

    url, cert = https_server
    parsed = urllib.parse.urlsplit(url)
    host = parsed.hostname
    port = parsed.port
    # Three pairwise comparisons across the three real sources — a
    # cert on disk, a cert captured during ``url_get``'s TLS
    # handshake, and a cert from an explicit ``tls_handshake``
    # probe.  Every cell must come back ``true`` because they're
    # the same self-signed cert; ``x509_eq`` collapses field-by-
    # field differences (``not_before`` may render different ISO
    # offsets, etc.) into cert-identity semantics.
    [results] = _run_with_probes(
        f"""[
            x509_eq(cert_load("{cert}"),
                    url_get("{url}/health").peer_cert),
            x509_eq(cert_load("{cert}"),
                    tls_handshake("{host}", {port}).peer_cert),
            x509_eq(url_get("{url}/health").peer_cert,
                    tls_handshake("{host}", {port}).peer_cert)
        ]""",
        ca_bundle=cert,
    )
    assert results == [True, True, True]


def test_scf_referenced_endpoint_is_probed(https_server):
    """End-to-end: an SCF describes virtual servers whose
    ``description`` field embeds an external URL.  The query lifts
    the URL out, hits the endpoint, and emits a per-VS health row
    using `if-then-else` for the verdict.
    """
    url, cert = https_server
    scf = _scf_with_https_endpoint(url)
    # The description is shaped as ``label=value``; split on ``=``
    # to recover the URL.  ``contains(., "url=")`` filters VSes
    # that advertise an external endpoint.
    query = """
    .ltm.virtual[]
    | select(contains(.description, "url="))
    | . as $vs
    | (split(.description, "=") | last) as $url
    | url_get($url) as $resp
    | { vs: basename($vs.name)
      , url: $url
      , status: http_status($resp)
      , verdict: (if http_ok($resp) then "healthy" elif http_client_error($resp) then "client-fail" elif http_server_error($resp) then "server-fail" else "unreachable" end)
      , body: http_body_json($resp)
      }
    """
    probes_token = PROBES_ENABLED.set(True)
    bundle_token = TLS_CA_BUNDLE.set(str(cert))
    try:
        result = run_query(query, {"ltm.conf": scf})
    finally:
        TLS_CA_BUNDLE.reset(bundle_token)
        PROBES_ENABLED.reset(probes_token)
    rows = result.values_per_file["ltm.conf"]
    by_vs = {r["vs"]: r for r in rows}
    assert by_vs["web_vs"]["status"] == 200
    assert by_vs["web_vs"]["verdict"] == "healthy"
    assert by_vs["web_vs"]["body"] == {"status": "ok", "uptime_s": 42}
    assert by_vs["inventory_vs"]["status"] == 200
    assert by_vs["inventory_vs"]["verdict"] == "healthy"
    assert by_vs["inventory_vs"]["body"]["servers"][0]["name"] == "web1"


def test_comma_stream_for_multi_endpoint_probe(https_server):
    """The new ``,`` stream-concat operator lets a single query
    probe several endpoints in one go without a let-binding chain."""
    url, cert = https_server
    [statuses] = _run_with_probes(
        f'[url_get("{url}/health"), url_get("{url}/inventory"), url_get("{url}/teapot")] '
        "| map(http_status(.))",
        ca_bundle=cert,
    )
    assert statuses == [200, 200, 418]
