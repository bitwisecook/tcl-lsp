"""iControl REST transport for ``f5 fetch``.

Uses stdlib ``http.client`` so no third-party dependency creeps into
the zipapp.  The flow is:

1. ``POST /mgmt/tm/sys/config`` to save the running config to a named
   SCF on the device.
2. ``GET  /mgmt/shared/file-transfer/madm/<name>.scf`` (or for UCS
   ``/mgmt/shared/file-transfer/ucs-downloads/<name>.ucs``) to stream
   the file back.

The endpoints, paths, and chunked-download conventions are documented
in the F5 iControl REST guide.
"""

from __future__ import annotations

import base64
import http.client
import json
import socket
import ssl
import time
from typing import Any

from .auth import Credentials


class RestError(RuntimeError):
    """Raised for any non-2xx iControl REST response."""


def _make_connection(
    credentials: Credentials,
    *,
    insecure: bool,
    timeout: float,
) -> http.client.HTTPSConnection:
    if insecure:
        ctx = ssl._create_unverified_context()  # noqa: SLF001
    else:
        ctx = ssl.create_default_context()
    return http.client.HTTPSConnection(
        credentials.host,
        port=credentials.port,
        timeout=timeout,
        context=ctx,
    )


def _auth_header(credentials: Credentials) -> str:
    raw = f"{credentials.user}:{credentials.password}".encode()
    return "Basic " + base64.b64encode(raw).decode("ascii")


def _request(
    conn: http.client.HTTPSConnection,
    method: str,
    path: str,
    *,
    auth: str,
    body: bytes | None = None,
    headers: dict[str, str] | None = None,
) -> tuple[int, dict[str, str], bytes]:
    hdrs = {"Authorization": auth, "Accept": "application/json"}
    if body is not None:
        hdrs["Content-Type"] = "application/json"
        hdrs["Content-Length"] = str(len(body))
    if headers:
        hdrs.update(headers)
    conn.request(method, path, body=body, headers=hdrs)
    resp = conn.getresponse()
    data = resp.read()
    return resp.status, dict(resp.getheaders()), data


def _json_request(
    conn: http.client.HTTPSConnection,
    method: str,
    path: str,
    *,
    auth: str,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    body = json.dumps(payload).encode("utf-8") if payload is not None else None
    status, _hdrs, data = _request(conn, method, path, auth=auth, body=body)
    if status >= 400:
        raise RestError(f"{method} {path} -> HTTP {status}: {data[:400]!r}")
    if not data:
        return {}
    try:
        return json.loads(data)
    except json.JSONDecodeError as exc:
        raise RestError(f"{method} {path}: non-JSON response: {data[:200]!r}") from exc


def _save_scf(conn: http.client.HTTPSConnection, *, auth: str, name: str) -> None:
    """Save the running config to ``/var/local/scf/<name>`` on the device."""
    _json_request(
        conn,
        "POST",
        "/mgmt/tm/sys/config",
        auth=auth,
        payload={"command": "save", "options": [{"file": name}, {"no-passphrase": True}]},
    )


def _save_ucs(conn: http.client.HTTPSConnection, *, auth: str, name: str) -> None:
    """Trigger creation of a UCS archive on the device."""
    _json_request(
        conn,
        "POST",
        "/mgmt/tm/sys/ucs",
        auth=auth,
        payload={"command": "save", "name": name},
    )


def _download(
    conn: http.client.HTTPSConnection,
    *,
    auth: str,
    path: str,
    poll_interval: float = 0.5,
    poll_timeout: float = 60.0,
) -> bytes:
    """GET *path* on the device, retrying briefly while the server prepares the file.

    iControl REST occasionally returns 404 right after a save command
    while the file is still being flushed; we poll for a short window.
    """
    deadline = time.monotonic() + poll_timeout
    last_status = 0
    last_body = b""
    while time.monotonic() < deadline:
        status, _hdrs, data = _request(
            conn, "GET", path, auth=auth, headers={"Accept": "application/octet-stream"}
        )
        if status == 200:
            return data
        last_status, last_body = status, data
        if status in (404, 503):
            time.sleep(poll_interval)
            continue
        break
    raise RestError(f"GET {path} -> HTTP {last_status}: {last_body[:200]!r}")


def fetch(
    credentials: Credentials,
    *,
    fmt: str = "scf",
    insecure: bool = True,
    timeout: float = 60.0,
    name: str | None = None,
) -> tuple[str, bytes | None]:
    """Pull config from the device.  Returns ``(scf_text, ucs_bytes_or_None)``.

    When *fmt* is ``"ucs"`` the SCF is reconstructed from the UCS via
    :func:`explorer.f5_remote.ucs.ucs_to_scf`.  When ``"both"``, both
    artefacts are returned.
    """
    from .ucs import ucs_to_scf

    if name is None:
        name = f"f5_fetch_{int(time.time())}"

    auth = _auth_header(credentials)
    conn = _make_connection(credentials, insecure=insecure, timeout=timeout)
    try:
        scf_text: str = ""
        ucs_bytes: bytes | None = None

        if fmt in {"scf", "both"}:
            _save_scf(conn, auth=auth, name=name)
            scf_data = _download(
                conn, auth=auth, path=f"/mgmt/shared/file-transfer/madm/{name}.scf"
            )
            scf_text = scf_data.decode("utf-8", errors="replace")

        if fmt in {"ucs", "both"}:
            _save_ucs(conn, auth=auth, name=name)
            ucs_bytes = _download(
                conn,
                auth=auth,
                path=f"/mgmt/shared/file-transfer/ucs-downloads/{name}.ucs",
            )
            if not scf_text:
                scf_text = ucs_to_scf(ucs_bytes)

        return scf_text, ucs_bytes
    except (socket.gaierror, ConnectionRefusedError, TimeoutError) as exc:
        raise ConnectionError(f"REST connection to {credentials.host}: {exc}") from exc
    finally:
        conn.close()
