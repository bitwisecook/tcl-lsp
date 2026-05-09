"""iControl REST transport for ``f5 fetch``.

Uses stdlib ``http.client`` so no third-party dependency creeps into
the zipapp.

Only the UCS download path is exposed over REST.  The flow:

1. ``POST /mgmt/tm/sys/ucs`` with ``{"command":"save","name":<n>}`` to
   create a UCS archive on the device.
2. ``GET /mgmt/shared/file-transfer/ucs-downloads/<n>.ucs`` to stream
   it back.

If the caller asked for SCF, we extract it from the downloaded UCS
locally via :func:`explorer.f5_remote.ucs.ucs_to_scf`.  Native SCF
download over REST exists on real devices but the path varies by TMOS
version and typically involves a manual file move into a downloadable
directory; the SSH transport (``f5_remote.ssh``) handles that case
cleanly via ``tmsh save sys config file ... ; scp``, so REST sticks
to the well-defined UCS endpoint.
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
    """Pull config from the device via REST.  Returns ``(scf_text, ucs_bytes_or_None)``.

    Always downloads a UCS; the SCF text is reconstructed locally with
    :func:`explorer.f5_remote.ucs.ucs_to_scf`.

    - ``fmt="scf"``  -> ``(scf_text, None)`` (UCS is consumed and discarded)
    - ``fmt="ucs"``  -> ``(scf_text, ucs_bytes)``
    - ``fmt="both"`` -> ``(scf_text, ucs_bytes)``
    """
    from .ucs import ucs_to_scf

    if name is None:
        name = f"f5_fetch_{int(time.time())}"

    auth = _auth_header(credentials)
    conn = _make_connection(credentials, insecure=insecure, timeout=timeout)
    try:
        _save_ucs(conn, auth=auth, name=name)
        ucs_data = _download(
            conn,
            auth=auth,
            path=f"/mgmt/shared/file-transfer/ucs-downloads/{name}.ucs",
        )
        scf_text = ucs_to_scf(ucs_data)
        keep_ucs = fmt in {"ucs", "both"}
        return scf_text, (ucs_data if keep_ucs else None)
    except (socket.gaierror, ConnectionRefusedError, TimeoutError) as exc:
        raise ConnectionError(f"REST connection to {credentials.host}: {exc}") from exc
    finally:
        conn.close()
