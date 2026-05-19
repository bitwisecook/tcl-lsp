"""Tests for the M5 round-trip verbs: pull, push, irule trace, irule extract."""

from __future__ import annotations

import json
import ssl
import sys
import textwrap
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from tooling.explorer.f5_remote.auth import Credentials
from tooling.explorer.f5_remote.object_io import (
    _encode_path,
    object_to_scf_stanza,
    pull_object,
    push_object,
)
from tooling.f5.main import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _write(tmp_path, name, body):
    path = tmp_path / name
    path.write_text(body)
    return path


SAMPLE = (
    textwrap.dedent(
        """
    ltm rule /Common/r_http {
        when HTTP_REQUEST {
            log local0. "incoming"
            pool /Common/p1
            HTTP::redirect /healthy
        }
        when HTTP_RESPONSE {
            log local0. "outgoing"
        }
    }
    ltm rule /Common/r_other {
        when CLIENT_ACCEPTED {
            persist source_addr
        }
    }
    """
    ).strip()
    + "\n"
)


# ── object_io unit tests ─────────────────────────────────────────────


def test_encode_path_translates_slashes():
    assert _encode_path("/Common/foo") == "~Common~foo"
    assert _encode_path("/Common2/bar.baz") == "~Common2~bar.baz"


def test_object_to_scf_virtual():
    obj = {
        "fullPath": "/Common/vs",
        "destination": "/Common/10.0.0.5:80",
        "pool": "/Common/p1",
        "rules": ["/Common/r1"],
    }
    text = object_to_scf_stanza("virtual", obj)
    assert "ltm virtual /Common/vs" in text
    assert "destination /Common/10.0.0.5:80" in text
    assert "pool /Common/p1" in text
    assert "/Common/r1" in text


def test_object_to_scf_pool_with_members():
    obj = {
        "fullPath": "/Common/p",
        "members": [{"fullPath": "/Common/n:80", "address": "10.0.0.1"}],
        "monitor": "/Common/http",
        "loadBalancingMode": "least-connections-member",
    }
    text = object_to_scf_stanza("pool", obj)
    assert "ltm pool /Common/p" in text
    assert "/Common/n:80" in text
    assert "address 10.0.0.1" in text
    assert "least-connections-member" in text


def test_object_to_scf_rule():
    obj = {"fullPath": "/Common/r", "apiAnonymous": "when HTTP_REQUEST { return }"}
    text = object_to_scf_stanza("rule", obj)
    assert "ltm rule /Common/r" in text
    assert "when HTTP_REQUEST" in text


# ── pull / push against a stub server ────────────────────────────────


class _Stub(BaseHTTPRequestHandler):
    response_payload: dict = {}
    last_method: str | None = None
    last_path: str | None = None
    last_body: bytes = b""

    def log_message(self, *_args, **_kwargs):
        pass

    def _respond(self, status, body):
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):  # noqa: N802
        type(self).last_method = "GET"
        type(self).last_path = self.path
        self._respond(200, json.dumps(type(self).response_payload).encode())

    def do_PUT(self):  # noqa: N802
        type(self).last_method = "PUT"
        type(self).last_path = self.path
        n = int(self.headers.get("Content-Length", "0"))
        type(self).last_body = self.rfile.read(n) if n else b""
        self._respond(200, b'{"ok":true}')

    def do_POST(self):  # noqa: N802
        type(self).last_method = "POST"
        type(self).last_path = self.path
        n = int(self.headers.get("Content-Length", "0"))
        type(self).last_body = self.rfile.read(n) if n else b""
        self._respond(200, b'{"created":true}')


def _generate_self_signed(tmp_path):
    import shutil
    import subprocess

    if shutil.which("openssl") is None:
        pytest.skip("openssl not available")

    out = tmp_path / "certs"
    out.mkdir()
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            str(out / "key.pem"),
            "-out",
            str(out / "cert.pem"),
            "-days",
            "1",
            "-subj",
            "/CN=127.0.0.1",
            "-addext",
            "subjectAltName=IP:127.0.0.1",
        ],
        check=True,
        capture_output=True,
    )
    return out


@pytest.fixture
def stub_server(tmp_path):
    cert_dir = _generate_self_signed(tmp_path)
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), _Stub)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(certfile=cert_dir / "cert.pem", keyfile=cert_dir / "key.pem")
    httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    try:
        yield httpd.server_address
    finally:
        httpd.shutdown()
        thread.join(timeout=5)


def test_pull_object_returns_payload(stub_server):
    host, port = stub_server
    _Stub.response_payload = {"fullPath": "/Common/p", "loadBalancingMode": "round-robin"}
    creds = Credentials(host=host, user="u", password="p", port=port)

    obj = pull_object(creds, kind="pool", full_path="/Common/p", insecure=True, timeout=10.0)

    assert obj["fullPath"] == "/Common/p"
    assert _Stub.last_path == "/mgmt/tm/ltm/pool/~Common~p"


def test_push_object_replace(stub_server):
    host, port = stub_server
    creds = Credentials(host=host, user="u", password="p", port=port)
    payload = {"fullPath": "/Common/p", "loadBalancingMode": "least-connections-member"}

    result = push_object(creds, kind="pool", payload=payload, insecure=True, timeout=10.0)

    assert result == {"ok": True}
    assert _Stub.last_method == "PUT"
    assert _Stub.last_path == "/mgmt/tm/ltm/pool/~Common~p"
    assert json.loads(_Stub.last_body)["loadBalancingMode"] == "least-connections-member"


def test_push_object_create(stub_server):
    host, port = stub_server
    creds = Credentials(host=host, user="u", password="p", port=port)
    payload = {"name": "p_new", "loadBalancingMode": "round-robin"}

    result = push_object(
        creds, kind="pool", payload=payload, create=True, insecure=True, timeout=10.0
    )

    assert result == {"created": True}
    assert _Stub.last_method == "POST"
    assert _Stub.last_path == "/mgmt/tm/ltm/pool"


# ── pull / push verbs ────────────────────────────────────────────────


def test_pull_verb_emits_scf(stub_server, capsys, monkeypatch):
    host, port = stub_server
    _Stub.response_payload = {"fullPath": "/Common/vs", "destination": "/Common/10.0.0.5:80"}
    monkeypatch.setenv("F5_HOST", host)
    monkeypatch.setenv("F5_USER", "u")
    monkeypatch.setenv("F5_PASSWORD", "p")
    monkeypatch.setenv("F5_PORT", str(port))

    code, out, _err = _run(
        ["pull", "virtual", "/Common/vs", "--no-prompt", "--timeout", "10"], capsys
    )
    assert code == 0
    assert "ltm virtual /Common/vs" in out
    assert "destination /Common/10.0.0.5:80" in out


def test_pull_verb_json_format(stub_server, capsys, monkeypatch):
    host, port = stub_server
    _Stub.response_payload = {"fullPath": "/Common/vs", "destination": "10.0.0.5:80"}
    monkeypatch.setenv("F5_HOST", host)
    monkeypatch.setenv("F5_USER", "u")
    monkeypatch.setenv("F5_PASSWORD", "p")
    monkeypatch.setenv("F5_PORT", str(port))

    code, out, _err = _run(
        ["pull", "virtual", "/Common/vs", "--no-prompt", "--json", "--timeout", "10"], capsys
    )
    assert code == 0
    payload = json.loads(out)
    assert payload["fullPath"] == "/Common/vs"


def test_push_verb_dry_run(tmp_path, capsys):
    payload_path = _write(
        tmp_path,
        "p.json",
        json.dumps({"fullPath": "/Common/p", "loadBalancingMode": "round-robin"}),
    )
    code, out, err = _run(["push", "pool", str(payload_path), "--dry-run"], capsys)
    assert code == 0
    assert "would PUT" in err
    assert json.loads(out)["fullPath"] == "/Common/p"


def test_push_verb_replace(stub_server, tmp_path, capsys, monkeypatch):
    host, port = stub_server
    payload_path = _write(
        tmp_path,
        "p.json",
        json.dumps({"fullPath": "/Common/p", "loadBalancingMode": "round-robin"}),
    )
    monkeypatch.setenv("F5_HOST", host)
    monkeypatch.setenv("F5_USER", "u")
    monkeypatch.setenv("F5_PASSWORD", "p")
    monkeypatch.setenv("F5_PORT", str(port))

    code, out, _err = _run(
        ["push", "pool", str(payload_path), "--no-prompt", "--timeout", "10"], capsys
    )
    assert code == 0
    assert json.loads(out) == {"ok": True}
    assert _Stub.last_method == "PUT"


# ── irule trace ──────────────────────────────────────────────────────


def test_irule_trace_text(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, out, _err = _run(["irule", "trace", "HTTP_REQUEST", str(p)], capsys)
    assert code == 0
    assert "/Common/r_http" in out
    assert "log" in out
    assert "pool" in out
    assert "HTTP::redirect" in out
    # r_other has no HTTP_REQUEST event — must not appear
    assert "/Common/r_other" not in out


def test_irule_trace_json(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, out, _err = _run(["irule", "trace", "HTTP_REQUEST", str(p), "--json"], capsys)
    assert code == 0
    payload = json.loads(out)
    assert payload["event"] == "HTTP_REQUEST"
    assert len(payload["traces"]) == 1
    assert payload["traces"][0]["rule"] == "/Common/r_http"
    assert payload["traces"][0]["commandCount"] >= 3


def test_irule_trace_no_match_exits_one(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    code, _out, _err = _run(["irule", "trace", "NOT_AN_EVENT", str(p)], capsys)
    assert code == 1


# ── irule extract ────────────────────────────────────────────────────


def test_irule_extract_writes_one_file_per_rule(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SAMPLE)
    out_dir = tmp_path / "rules"
    code, _out, err = _run(["irule", "extract", str(p), str(out_dir)], capsys)
    assert code == 0
    assert "extracted 2" in err
    files = sorted(f.name for f in out_dir.iterdir())
    assert "Common__r_http.tcl" in files
    assert "Common__r_other.tcl" in files
    body = (out_dir / "Common__r_http.tcl").read_text()
    assert "HTTP::redirect" in body
