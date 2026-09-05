# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""A tiny standalone report web server (stdlib only, no extra dependencies).

Serves the shared builder page and generates reports server-side via
:func:`f5report.build_report`. The same page (``shared/src/pages/input.ts``)
drives this server through its ``ServerBackend``; uploaded UCS files are spilled
to a temp file so the full SSL-cert PEMs are recovered. Run with
``python -m f5report --serve`` or the ``f5-report-web`` entry point.

Forensics stays empty on this path (the UCS member inventory is a wasm/Rust-only
feature); everything else matches the CLI.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import tempfile
import uuid as _uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib import resources
from typing import Any
from urllib.parse import urlparse

from . import _engine, engine_version, load_paths
from .report import build_report

# ---- shared builder page (inlined, self-contained) -------------------------


def _input_page() -> str:
    """Render the shared builder page with the ServerBackend (no wasm payload)."""
    tmpl = (
        resources.files("f5report.templates").joinpath("input.html").read_text("utf-8")
    )
    css = resources.files("f5report.templates").joinpath("input.css").read_text("utf-8")
    js = resources.files("f5report.templates").joinpath("input.js").read_text("utf-8")
    # Same self-contained CSP as the standalone app and the generated report,
    # but `connect-src 'self'` (not 'none'): this page's ServerBackend POSTs the
    # config to this same server's /probe and /generate. No external origin is
    # ever reachable, so nothing is uploaded off-box.
    csp = (
        "default-src 'none'; script-src 'unsafe-inline' 'wasm-unsafe-eval'; "
        "style-src 'unsafe-inline'; img-src data: blob:; connect-src 'self'; "
        "base-uri 'none'; form-action 'none'"
    )
    tmpl = tmpl.replace("__CSP__", csp)
    tmpl = tmpl.replace("__STYLES__", f"<style>{css}</style>")
    tmpl = tmpl.replace("__WASM_PAYLOAD__", "")  # no wasm → the page uses ServerBackend
    tmpl = tmpl.replace("__INPUT_JS__", f"<script>{js}</script>")
    # Project marks, inlined as <svg> (the same files the report embeds).
    for token, svg in (
        ("__LOGO_F5Q__", "logo-f5q.svg"),
        ("__LOGO_TCL_LSP__", "logo-tcl-lsp.svg"),
        ("__LOGO_TCL_LSP_DARK__", "logo-tcl-lsp-dark.svg"),
    ):
        mark = resources.files("f5report.vendor").joinpath(svg).read_text("utf-8")
        tmpl = tmpl.replace(token, mark)
    return tmpl


# ---- minimal multipart/form-data parser (binary-safe) ----------------------


class Part:
    __slots__ = ("data", "filename", "name")

    def __init__(self, name: str, filename: str | None, data: bytes) -> None:
        self.name = name
        self.filename = filename
        self.data = data


def _parse_multipart(body: bytes, content_type: str) -> list[Part]:
    """Split a multipart/form-data body into parts (byte-level, so binary UCS
    uploads survive intact). Returns [] when not multipart."""
    marker = "boundary="
    idx = content_type.find(marker)
    if idx < 0:
        return []
    boundary = content_type[idx + len(marker) :].strip().strip('"')
    sep = b"--" + boundary.encode()
    parts: list[Part] = []
    for chunk in body.split(sep):
        if not chunk or chunk in (b"--", b"--\r\n", b"\r\n"):
            continue
        chunk = chunk.lstrip(b"\r\n")
        head_end = chunk.find(b"\r\n\r\n")
        if head_end < 0:
            continue
        headers = chunk[:head_end].decode("latin-1", "replace")
        data = chunk[head_end + 4 :]
        if data.endswith(b"\r\n"):
            data = data[:-2]
        name, filename = "", None
        for line in headers.split("\r\n"):
            if line.lower().startswith("content-disposition:"):
                for tok in line.split(";"):
                    tok = tok.strip()
                    if tok.startswith("name="):
                        name = tok[5:].strip('"')
                    elif tok.startswith("filename="):
                        filename = tok[9:].strip('"')
        if name:
            parts.append(Part(name, filename, data))
    return parts


_UNSAFE_UPLOAD_CHARS = re.compile(r"[^A-Za-z0-9._-]")


def _upload_basename(filename: str | None) -> str:
    """Reduce a client-supplied upload filename to a bare, inert basename.

    ``os.path.basename`` alone is not enough: on POSIX it keeps Windows
    separators and drive letters, and it lets ``.``, ``..`` and the empty name
    through. The extension survives because `load_paths` dispatches on it.
    """
    name = (filename or "").replace("\\", "/").rsplit("/", 1)[-1]
    return _UNSAFE_UPLOAD_CHARS.sub("_", name).lstrip(".") or "upload"


# ---- request handling ------------------------------------------------------


class _Handler(BaseHTTPRequestHandler):
    server_version = "f5report-web"

    def _send(
        self, code: int, body: bytes, ctype: str, extra: dict | None = None
    ) -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (extra or {}).items():
            self.send_header(k, str(v))
        self.end_headers()
        self.wfile.write(body)

    def _text(
        self, code: int, text: str, ctype: str = "text/plain; charset=utf-8", extra=None
    ):
        self._send(code, text.encode("utf-8"), ctype, extra)

    def _read_body(self) -> bytes:
        length = int(self.headers.get("Content-Length", 0))
        return self.rfile.read(length) if length else b""

    @staticmethod
    def _sources_from_parts(parts: list[Part], passphrase: str):
        """Spill uploaded files to a temp dir and load them (UCS extract +
        full cert PEMs), returning (sources, tmpdir) — caller cleans tmpdir."""
        files = [p for p in parts if p.filename]
        tmp = tempfile.mkdtemp(prefix="f5report-")
        try:
            # `tmp` itself may sit behind a symlink (/var -> /private/var on
            # macOS), so both sides of the containment check are resolved.
            root = os.path.realpath(tmp)
            paths = []
            for i, p in enumerate(files):
                # One directory per upload. Two distinct names can sanitise to
                # the same basename ("prod east.ucs" and "prod_east.ucs"), and
                # in a shared directory the second would overwrite the first —
                # dropping one device from the report and duplicating another.
                # Renaming instead would not do: the basename is what
                # `_device_name` falls back to for a config with no hostname,
                # and the extension is how the engine spots a UCS.
                spill = os.path.join(root, str(i))
                os.mkdir(spill)
                dest = os.path.realpath(
                    os.path.join(spill, _upload_basename(p.filename))
                )
                if not dest.startswith(spill + os.sep):
                    raise ValueError(f"unsafe upload filename: {p.filename!r}")
                with open(dest, "wb") as fh:
                    fh.write(p.data)
                paths.append(dest)
            sources = load_paths(paths, passphrase=passphrase or None) if paths else []
        except Exception:
            _rmtree(tmp)
            raise
        return sources, tmp

    @staticmethod
    def _fields(parts: list[Part]) -> dict[str, str]:
        return {
            p.name: p.data.decode("utf-8", "replace") for p in parts if not p.filename
        }

    def do_GET(self) -> None:
        route = urlparse(self.path)
        if route.path in ("/", "/index.html"):
            self._text(200, _input_page(), "text/html; charset=utf-8")
        elif route.path == "/version":
            self._text(200, engine_version())
        elif route.path == "/manual":
            self._text(200, _engine.manual())
        else:
            self._text(404, "not found")

    def do_POST(self) -> None:
        route = urlparse(self.path)
        ctype = self.headers.get("Content-Type", "")
        body = self._read_body()
        parts = _parse_multipart(body, ctype)
        fields = self._fields(parts)
        try:
            if route.path == "/probe":
                self._probe(parts, fields)
            elif route.path == "/generate":
                self._generate(parts, fields)
            elif route.path == "/architecture":
                self._architecture(fields)
            elif route.path == "/enrich":
                self._enrich(fields)
            else:
                self._text(404, "not found")
        except Exception as exc:  # noqa: BLE001 — surface a clean 400 to the client
            self._text(400, str(exc))

    def _probe(self, parts, fields) -> None:
        sources, tmp = self._sources_from_parts(parts, fields.get("passphrase", ""))
        try:
            count = sum(
                len(json.loads(_engine.list_secrets(text))) for _uri, text in sources
            )
        finally:
            _rmtree(tmp)
        self._text(200, json.dumps({"secretCount": count}), "application/json")

    def _generate(self, parts, fields) -> None:
        sources, tmp = self._sources_from_parts(parts, fields.get("passphrase", ""))
        try:
            if not sources:
                self._text(400, "no config files uploaded")
                return
            embed = fields.get("embedConsole", "true").lower() != "false"
            secret_total = sum(
                len(json.loads(_engine.list_secrets(t))) for _u, t in sources
            )
            html = build_report(
                sources,
                title=fields.get("title") or "F5 BIG-IP Configuration Report",
                embed_console=embed,
                master_key=fields.get("masterKey") or None,
                report_id=fields.get("reportId") or str(_uuid.uuid4()),
                manifest=fields.get("manifest") or None,
            )
        finally:
            _rmtree(tmp)
        locked = "1" if (secret_total > 0 and not fields.get("masterKey")) else "0"
        self._text(
            200,
            html,
            "text/html; charset=utf-8",
            {"X-Device-Count": len(sources), "X-Secrets-Locked": locked},
        )

    def _architecture(self, fields) -> None:
        out = _engine.build_architecture(
            fields.get("devices", "[]"), fields.get("manifest", "") or None
        )
        self._text(200, out, "application/json")

    def _enrich(self, fields) -> None:
        out = _engine.build_enrichment(
            fields.get("devices", "[]"), fields.get("architecture", "{}")
        )
        self._text(200, out, "application/json")

    def log_message(self, format: str, *args: Any) -> None:  # keep the console quiet
        # Signature must match BaseHTTPRequestHandler.log_message, which callers
        # may invoke with `format` as a keyword.
        pass


def _rmtree(path: str) -> None:
    import shutil

    shutil.rmtree(path, ignore_errors=True)


def serve(host: str = "127.0.0.1", port: int = 8080) -> None:
    """Run the report web server until interrupted."""
    httpd = ThreadingHTTPServer((host, port), _Handler)
    print(f"f5report web server on http://{host}:{port}  (Ctrl-C to stop)")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        # Ctrl-C is the advertised way to stop; `finally` closes the socket.
        pass
    finally:
        httpd.server_close()


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        prog="f5-report-web", description="Serve the BIG-IP report builder."
    )
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8080)
    args = ap.parse_args(argv)
    serve(args.host, args.port)
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
