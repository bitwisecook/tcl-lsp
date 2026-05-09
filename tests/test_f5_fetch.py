"""Tests for ``f5 fetch`` against an in-process iControl REST stub."""

from __future__ import annotations

import json
import ssl
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from explorer.f5_cli import main
from explorer.f5_remote import fetch_scf
from explorer.f5_remote.auth import Credentials
from explorer.f5_remote.ucs import make_test_ucs


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


_FAKE_SCF = "ltm pool /Common/p {\n    members { /Common/n:80 { } }\n}\n"


class _RestStub(BaseHTTPRequestHandler):
    """Minimal iControl REST surface that satisfies our client.

    Routes:
      POST /mgmt/tm/sys/config        -> 200 {} (save SCF)
      POST /mgmt/tm/sys/ucs           -> 200 {} (save UCS)
      GET  /mgmt/shared/file-transfer/madm/<n>.scf  -> 200 SCF body
      GET  /mgmt/shared/file-transfer/ucs-downloads/<n>.ucs -> 200 UCS body
    """

    saved_scf_name: str | None = None
    saved_ucs_name: str | None = None

    def log_message(self, *_args, **_kwargs):  # noqa: D401 — silence test output
        pass

    def _read_json(self) -> dict:
        length = int(self.headers.get("Content-Length", "0") or "0")
        raw = self.rfile.read(length) if length else b""
        if not raw:
            return {}
        return json.loads(raw)

    def do_POST(self):  # noqa: N802
        if self.path == "/mgmt/tm/sys/config":
            payload = self._read_json()
            for opt in payload.get("options", []):
                if "file" in opt:
                    type(self).saved_scf_name = opt["file"]
            self._respond(200, b"{}")
        elif self.path == "/mgmt/tm/sys/ucs":
            payload = self._read_json()
            type(self).saved_ucs_name = payload.get("name", "")
            self._respond(200, b"{}")
        else:
            self._respond(404, b'{"error":"unknown"}')

    def do_GET(self):  # noqa: N802
        if self.path.startswith("/mgmt/shared/file-transfer/madm/"):
            self._respond(200, _FAKE_SCF.encode("utf-8"))
        elif self.path.startswith("/mgmt/shared/file-transfer/ucs-downloads/"):
            ucs = make_test_ucs({"config/bigip.conf": _FAKE_SCF})
            self._respond(200, ucs)
        else:
            self._respond(404, b'{"error":"unknown"}')

    def _respond(self, status: int, body: bytes):
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body)


@pytest.fixture
def rest_server(tmp_path):
    """Spin up an HTTPS server with a self-signed cert for the lifetime of one test."""
    cert_dir = _generate_self_signed(tmp_path)
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), _RestStub)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(certfile=cert_dir / "cert.pem", keyfile=cert_dir / "key.pem")
    httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    try:
        yield httpd.server_address  # (host, port)
    finally:
        httpd.shutdown()
        thread.join(timeout=5)


def _generate_self_signed(tmp_path: Path) -> Path:
    """Generate a self-signed cert/key pair using the system ``openssl`` CLI."""
    import shutil
    import subprocess

    if shutil.which("openssl") is None:
        pytest.skip("openssl CLI not available")

    out = tmp_path / "certs"
    out.mkdir()
    key = out / "key.pem"
    cert = out / "cert.pem"
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            str(key),
            "-out",
            str(cert),
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


def test_fetch_scf_via_rest(rest_server):
    host, port = rest_server
    creds = Credentials(host=host, user="admin", password="pw", port=port)
    result = fetch_scf(credentials=creds, transport="rest", fmt="scf", insecure=True, timeout=10.0)
    assert result.transport == "rest"
    assert result.fmt == "scf"
    assert "ltm pool /Common/p" in result.scf
    assert result.ucs is None


def test_fetch_ucs_via_rest_includes_bytes(rest_server):
    host, port = rest_server
    creds = Credentials(host=host, user="admin", password="pw", port=port)
    result = fetch_scf(credentials=creds, transport="rest", fmt="ucs", insecure=True, timeout=10.0)
    assert result.ucs is not None
    assert result.ucs[:2] == b"\x1f\x8b"  # gzip magic
    assert "ltm pool /Common/p" in result.scf  # reconstructed from UCS


def test_fetch_both_via_rest(rest_server):
    host, port = rest_server
    creds = Credentials(host=host, user="admin", password="pw", port=port)
    result = fetch_scf(credentials=creds, transport="rest", fmt="both", insecure=True, timeout=10.0)
    assert result.ucs is not None
    assert result.scf  # SCF endpoint, not the reconstructed one


def test_fetch_verb_writes_cache(rest_server, tmp_path, capsys, monkeypatch):
    host, port = rest_server
    monkeypatch.setenv("F5_HOST", host)
    monkeypatch.setenv("F5_USER", "admin")
    monkeypatch.setenv("F5_PASSWORD", "pw")
    monkeypatch.setenv("F5_PORT", str(port))
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))

    code, _out, err = _run(
        ["fetch", "--transport", "rest", "--no-prompt", "--print-path", "--timeout", "10"],
        capsys,
    )
    assert code == 0, err
    cache_root = tmp_path / "cache" / "f5"
    host_dirs = list(cache_root.iterdir())
    assert len(host_dirs) == 1
    runs = [p for p in host_dirs[0].iterdir() if p.is_dir()]
    assert runs
    scf_text = (runs[0] / "config.scf").read_text()
    assert "ltm pool /Common/p" in scf_text


def test_fetch_verb_to_stdout(rest_server, capsys, monkeypatch, tmp_path):
    host, port = rest_server
    monkeypatch.setenv("F5_HOST", host)
    monkeypatch.setenv("F5_USER", "admin")
    monkeypatch.setenv("F5_PASSWORD", "pw")
    monkeypatch.setenv("F5_PORT", str(port))
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))

    code, out, err = _run(
        ["fetch", "--transport", "rest", "--no-prompt", "-o", "-", "--timeout", "10"],
        capsys,
    )
    assert code == 0, err
    assert "ltm pool /Common/p" in out


def test_fetch_no_prompt_without_creds(capsys, monkeypatch, tmp_path):
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "cfg"))
    for var in ("F5_HOST", "F5_USER", "F5_PASSWORD"):
        monkeypatch.delenv(var, raising=False)

    code, _out, err = _run(["fetch", "--no-prompt"], capsys)
    assert code == 2
    assert "host" in err
