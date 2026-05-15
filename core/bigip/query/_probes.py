"""Live network probe primitives shared by the DSL builtins.

Every probe here is opt-in via the ``probes_enabled`` contextvar
(set by the CLI's ``--enable-probes`` flag).  When the flag is
not set, each probe raises :class:`BuiltinError` so a query
written for an offline environment doesn't silently start
talking to the network.

Time bounds and result caches are encapsulated per probe so
repeated lookups inside one query don't issue duplicate calls.
The cache is keyed on the call arguments; entries persist for
the life of the Python process.
"""

from __future__ import annotations

import socket
import ssl
import subprocess
import time
import urllib.error
import urllib.request
from contextvars import ContextVar
from typing import Any
from urllib.parse import urlsplit

from .errors import BuiltinError

# Set by the CLI / runner.  Default ``False`` keeps the engine
# offline-safe — a query that hits a probe builtin with probes
# disabled gets a clear error instead of a silent network call.
PROBES_ENABLED: ContextVar[bool] = ContextVar("_bigip_query_probes_enabled", default=False)

# Optional path to a CA bundle used by ``url_request`` and
# ``tls_handshake`` for chain verification.  ``None`` falls back to
# the system trust store.  Set this from a test fixture (or the
# CLI's ``--ca-bundle`` flag) when probing an endpoint signed by an
# internal / self-signed CA.
TLS_CA_BUNDLE: ContextVar[str | None] = ContextVar("_bigip_query_tls_ca_bundle", default=None)

# Process-lifetime caches.
_PING_CACHE: dict[str, dict[str, Any]] = {}
_PORTPING_CACHE: dict[tuple[str, int, str], dict[str, Any]] = {}
_TRACEROUTE_CACHE: dict[str, list[dict[str, Any]]] = {}
_URL_CACHE: dict[tuple[str, str, frozenset, str | None], dict[str, Any]] = {}
_SOCKET_CACHE: dict[tuple[str, int, bytes, int], bytes] = {}
_TLS_CACHE: dict[tuple[str, int, str | None, str | None], dict[str, Any]] = {}


def _require_probes(name: str) -> None:
    if not PROBES_ENABLED.get():
        raise BuiltinError(
            f"{name}: network probes are disabled; pass --enable-probes "
            "to opt in for this query.  Probes always go to the network "
            "and are gated by default so an offline read-only query "
            "never accidentally reaches out."
        )


def ping(ip: str, *, timeout_s: float = 2.0) -> dict[str, Any]:
    """ICMP echo via the system ``ping`` command.

    Returns ``{"ok": bool, "rtt_ms": float | None, "error": str | None}``.
    Using subprocess (not raw sockets) sidesteps the
    ``CAP_NET_RAW`` requirement so the builtin works under a
    plain user account.
    """
    _require_probes("ping")
    if ip in _PING_CACHE:
        return dict(_PING_CACHE[ip])
    try:
        result = subprocess.run(
            ["ping", "-c", "1", "-W", str(int(timeout_s)), ip],
            capture_output=True,
            text=True,
            timeout=timeout_s + 1.0,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError) as exc:
        out = {"ok": False, "rtt_ms": None, "error": str(exc)}
        _PING_CACHE[ip] = out
        return dict(out)
    if result.returncode != 0:
        out = {
            "ok": False,
            "rtt_ms": None,
            "error": (result.stderr or result.stdout or "ping failed").strip() or None,
        }
        _PING_CACHE[ip] = out
        return dict(out)
    rtt = _parse_ping_rtt(result.stdout)
    out = {"ok": True, "rtt_ms": rtt, "error": None}
    _PING_CACHE[ip] = out
    return dict(out)


def _parse_ping_rtt(stdout: str) -> float | None:
    """Pull the ``time=NN ms`` value out of ``ping``'s first reply line."""
    for line in stdout.splitlines():
        if "time=" in line:
            try:
                bit = line.split("time=", 1)[1].split()[0]
                return float(bit.rstrip("ms"))
            except (ValueError, IndexError):
                return None
    return None


def portping(
    ip: str, port: int, *, protocol: str = "tcp", timeout_s: float = 2.0
) -> dict[str, Any]:
    """TCP connect (or UDP send-receive) to *(ip, port)*.

    Returns ``{"ok": bool, "rtt_ms": float | None, "error": str | None}``.
    UDP is best-effort: a successful ``sendto`` reports ``ok=True``
    without a confirmed response (no reply doesn't imply
    unreachable for UDP).
    """
    _require_probes("portping")
    key = (ip, int(port), protocol)
    if key in _PORTPING_CACHE:
        return dict(_PORTPING_CACHE[key])
    proto = protocol.lower()
    if proto not in ("tcp", "udp"):
        raise BuiltinError(f"portping: protocol must be 'tcp' or 'udp', got {protocol!r}")
    family = socket.AF_INET6 if ":" in ip else socket.AF_INET
    sock_type = socket.SOCK_STREAM if proto == "tcp" else socket.SOCK_DGRAM
    start = time.monotonic()
    try:
        with socket.socket(family, sock_type) as s:
            s.settimeout(timeout_s)
            if proto == "tcp":
                s.connect((ip, int(port)))
            else:
                s.sendto(b"", (ip, int(port)))
        rtt = (time.monotonic() - start) * 1000
        out = {"ok": True, "rtt_ms": rtt, "error": None}
    except (OSError, socket.timeout) as exc:
        out = {"ok": False, "rtt_ms": None, "error": str(exc)}
    _PORTPING_CACHE[key] = out
    return dict(out)


def traceroute(ip: str, *, max_hops: int = 30, timeout_s: float = 2.0) -> list[dict[str, Any]]:
    """Subprocess ``traceroute`` invocation.

    Returns a list of ``{"hop": int, "ip": str | None, "rtt_ms":
    float | None}`` records.  Hops the routing layer didn't
    answer for show up with ``ip=None``.
    """
    _require_probes("traceroute")
    if ip in _TRACEROUTE_CACHE:
        return [dict(h) for h in _TRACEROUTE_CACHE[ip]]
    try:
        result = subprocess.run(
            ["traceroute", "-n", "-m", str(max_hops), "-w", str(int(timeout_s)), ip],
            capture_output=True,
            text=True,
            timeout=timeout_s * max_hops + 5.0,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError) as exc:
        return [{"hop": 0, "ip": None, "rtt_ms": None, "error": str(exc)}]
    hops: list[dict[str, Any]] = []
    for line in result.stdout.splitlines():
        line = line.strip()
        if not line or not line[0].isdigit():
            continue
        parts = line.split()
        try:
            hop = int(parts[0])
        except ValueError:
            continue
        hop_ip: str | None = None
        rtt: float | None = None
        for token in parts[1:]:
            if "." in token or ":" in token:
                # First IP-looking token wins.
                if hop_ip is None and any(ch.isdigit() for ch in token) and token != "ms":
                    hop_ip = token
            elif token.replace(".", "", 1).isdigit():
                rtt = float(token)
        hops.append({"hop": hop, "ip": hop_ip, "rtt_ms": rtt})
    _TRACEROUTE_CACHE[ip] = hops
    return [dict(h) for h in hops]


def url_request(
    method: str,
    url: str,
    *,
    body: bytes | str | None = None,
    headers: dict[str, str] | None = None,
    timeout_s: float = 5.0,
) -> dict[str, Any]:
    """Generic HTTP request via :mod:`urllib`.

    Returns ``{"status", "headers", "body", "body_json", "error"}``.
    ``body`` is the raw response body decoded as UTF-8 with
    replacement.  ``body_json`` is the pre-parsed JSON value when the
    response's ``content-type`` header includes ``json`` and the body
    is valid JSON; otherwise ``None``.  Pre-parsing here lets callers
    write ``url_get(...) | http_body_json(.)`` without re-parsing on
    every traversal, and means a query that only needs structured
    fields doesn't pay the regex cost twice.
    """
    _require_probes(f"url_{method.lower()}")
    scheme = urlsplit(url).scheme.lower()
    if scheme not in {"http", "https"}:
        out = {
            "status": None,
            "headers": {},
            "body": "",
            "body_json": None,
            "error": f"unsupported URL scheme: {scheme or '<none>'}",
        }
        return out
    ca_bundle = TLS_CA_BUNDLE.get()
    hdr_items = frozenset((headers or {}).items())
    cache_key = (method.upper(), url, hdr_items, ca_bundle)
    if cache_key in _URL_CACHE:
        return dict(_URL_CACHE[cache_key])
    data: bytes | None
    if body is None:
        data = None
    elif isinstance(body, str):
        data = body.encode("utf-8")
    else:
        data = body
    req = urllib.request.Request(url, data=data, method=method.upper(), headers=headers or {})
    # When a custom CA bundle is bound, build a verifying context
    # that trusts it (and only it).  Otherwise let urllib fall back
    # to the platform default.
    opener_kwargs: dict[str, Any] = {"timeout": timeout_s}
    if ca_bundle:
        opener_kwargs["context"] = ssl.create_default_context(cafile=ca_bundle)
    try:
        with urllib.request.urlopen(req, **opener_kwargs) as resp:
            payload = resp.read()
            out = {
                "status": resp.status,
                "headers": {k.lower(): v for k, v in resp.headers.items()},
                "body": payload.decode("utf-8", "replace"),
                "error": None,
            }
    except urllib.error.HTTPError as exc:
        out = {
            "status": exc.code,
            "headers": {k.lower(): v for k, v in (exc.headers or {}).items()},
            "body": exc.read().decode("utf-8", "replace") if exc.fp else "",
            "error": exc.reason,
        }
    except (urllib.error.URLError, OSError, TimeoutError) as exc:
        out = {"status": None, "headers": {}, "body": "", "error": str(exc)}
    out["body_json"] = _maybe_parse_body_json(out["body"], out["headers"])
    _URL_CACHE[cache_key] = out
    return dict(out)


def _maybe_parse_body_json(body: str, headers: dict[str, Any]) -> Any:
    """Parse *body* as JSON when *headers* declare a JSON content-type.

    Returns the parsed value or ``None``.  The content-type sniff
    matches any ``content-type`` whose value contains ``json``
    (covers ``application/json``, ``application/problem+json``,
    ``application/vnd.api+json``, etc.) — gives the
    pre-parsed-body ergonomics without locking the heuristic to one
    spelling.  Falls back to ``None`` on a parse error rather than
    raising so a bad-shape body still leaves ``body`` available for
    the caller to inspect.
    """
    if not body:
        return None
    ctype = headers.get("content-type", "") if isinstance(headers, dict) else ""
    if "json" not in str(ctype).lower():
        return None
    import json as _json

    try:
        return _json.loads(body)
    except _json.JSONDecodeError:
        return None


def socket_get(
    host: str, port: int, *, send: bytes | str = b"", recv_max: int = 4096, timeout_s: float = 5.0
) -> str:
    """Connect to *(host, port)*, optionally send *send*, read up to
    *recv_max* bytes, and return the response as UTF-8 (replacement).

    Useful for grabbing protocol banners: ``socket_get("server", 22)``
    returns the SSH version string; ``socket_get("smtp.example.com", 25)``
    the SMTP greeting.
    """
    _require_probes("socket_get")
    if isinstance(send, str):
        send_bytes = send.encode("utf-8")
    else:
        send_bytes = send
    key = (host, int(port), send_bytes, int(recv_max))
    if key in _SOCKET_CACHE:
        return _SOCKET_CACHE[key].decode("utf-8", "replace")
    family = socket.AF_INET6 if ":" in host else socket.AF_INET
    try:
        with socket.socket(family, socket.SOCK_STREAM) as s:
            s.settimeout(timeout_s)
            s.connect((host, int(port)))
            if send_bytes:
                s.sendall(send_bytes)
            data = s.recv(recv_max)
    except (OSError, socket.timeout) as exc:
        raise BuiltinError(f"socket_get: {exc}") from exc
    _SOCKET_CACHE[key] = data
    return data.decode("utf-8", "replace")


def tls_handshake(
    host: str,
    port: int,
    *,
    sni: str | None = None,
    alpn: list[str] | None = None,
    timeout_s: float = 5.0,
) -> dict[str, Any]:
    """Open a TLS connection and report what the peer offered.

    Returns ``{"protocol", "cipher", "peer_cert", "alpn_selected",
    "verify_status", "error"}``.  The handshake is verified against
    the host's default trust store; ``verify_status`` is ``"ok"`` or
    the verification error.
    """
    _require_probes("tls_handshake")
    sni_value = sni if sni is not None else host
    ca_bundle = TLS_CA_BUNDLE.get()
    key = (host, int(port), sni_value, ca_bundle)
    if key in _TLS_CACHE:
        return dict(_TLS_CACHE[key])
    ctx = (
        ssl.create_default_context(cafile=ca_bundle) if ca_bundle else ssl.create_default_context()
    )
    if alpn:
        ctx.set_alpn_protocols(alpn)
    out: dict[str, Any]
    try:
        with socket.create_connection((host, int(port)), timeout=timeout_s) as sock:
            with ctx.wrap_socket(sock, server_hostname=sni_value) as ssock:
                cipher = ssock.cipher()
                peer = ssock.getpeercert()
                peer_der = ssock.getpeercert(binary_form=True)
                alpn_selected = ssock.selected_alpn_protocol() if alpn else None
                # Pre-parse the peer cert into the structured shape
                # ``x509_parse`` produces so callers can navigate it
                # directly: ``tls_handshake(h, p).peer_cert.subject``.
                # When the DER is unavailable (very old Pythons /
                # alpine variants) fall back to the raw ssl dict.
                peer_cert: Any = peer
                if peer_der is not None:
                    try:
                        pem = ssl.DER_cert_to_PEM_cert(peer_der)
                        peer_cert = x509_parse(pem)
                    except Exception:
                        peer_cert = peer
                out = {
                    "protocol": ssock.version(),
                    "cipher": cipher[0] if cipher else None,
                    "peer_cert": peer_cert,
                    "alpn_selected": alpn_selected,
                    "verify_status": "ok",
                    "error": None,
                }
    except ssl.SSLCertVerificationError as exc:
        out = {
            "protocol": None,
            "cipher": None,
            "peer_cert": None,
            "alpn_selected": None,
            "verify_status": exc.verify_message,
            "error": str(exc),
        }
    except (OSError, ssl.SSLError, TimeoutError) as exc:
        out = {
            "protocol": None,
            "cipher": None,
            "peer_cert": None,
            "alpn_selected": None,
            "verify_status": "error",
            "error": str(exc),
        }
    _TLS_CACHE[key] = out
    return dict(out)


def x509_parse(pem: str) -> dict[str, Any]:
    """Best-effort X.509 parsing.

    Uses the :mod:`cryptography` package when available (richer
    fields, full SAN list, signature algorithm names).  Falls
    back to :mod:`ssl`'s built-in PEM-to-DER + ``DER_cert_to_PEM``
    + ``_test_decode_cert`` path when ``cryptography`` is not
    installed — that path returns a strict subset (``subject``,
    ``issuer``, ``notBefore``, ``notAfter``, ``serialNumber``,
    ``subjectAltName``).  Raises :class:`BuiltinError` for input
    that doesn't decode as a certificate.
    """
    try:
        from cryptography import x509 as _x509
        from cryptography.hazmat.primitives import hashes, serialization
    except ImportError:
        return _x509_parse_ssl_fallback(pem)
    pem_bytes = pem.encode("utf-8") if isinstance(pem, str) else pem
    try:
        cert = _x509.load_pem_x509_certificate(pem_bytes)
    except (ValueError, TypeError) as exc:
        raise BuiltinError(f"x509_parse: not a PEM certificate ({exc})") from exc
    sans: list[str] = []
    try:
        san_ext = cert.extensions.get_extension_for_class(_x509.SubjectAlternativeName)
        sans = [str(n.value) for n in san_ext.value]
    except _x509.ExtensionNotFound:
        pass
    public_key = cert.public_key()
    key_size = getattr(public_key, "key_size", None)
    fingerprint = cert.fingerprint(hashes.SHA256()).hex().upper()
    return {
        "subject": cert.subject.rfc4514_string(),
        "issuer": cert.issuer.rfc4514_string(),
        "not_before": cert.not_valid_before_utc.isoformat()
        if hasattr(cert, "not_valid_before_utc")
        else cert.not_valid_before.isoformat(),
        "not_after": cert.not_valid_after_utc.isoformat()
        if hasattr(cert, "not_valid_after_utc")
        else cert.not_valid_after.isoformat(),
        "serial": format(cert.serial_number, "x").upper(),
        "fingerprint_sha256": fingerprint,
        "sans": sans,
        "key_alg": type(public_key).__name__,
        "key_size": key_size,
        "sig_alg": cert.signature_algorithm_oid._name,
        "version": cert.version.name,
        "public_key_pem": public_key.public_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PublicFormat.SubjectPublicKeyInfo,
        ).decode("ascii"),
    }


def _x509_parse_ssl_fallback(pem: str) -> dict[str, Any]:
    """Lightweight stdlib-only X.509 inspection.

    Reads the limited subset the :mod:`ssl` module exposes —
    enough for "subject / issuer / validity / SANs" but not for
    key sizes or signature algorithms.
    """
    import hashlib
    import tempfile

    pem_text = pem if isinstance(pem, str) else pem.decode("utf-8")
    try:
        der = ssl.PEM_cert_to_DER_cert(pem_text)
    except (ValueError, ssl.SSLError) as exc:
        raise BuiltinError(f"x509_parse: not a PEM certificate ({exc})") from exc
    fingerprint = hashlib.sha256(der).hexdigest().upper()
    # ``ssl._ssl._test_decode_cert`` reads from a file path.
    try:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".pem", delete=False) as tmp:
            tmp.write(pem_text)
            tmp_path = tmp.name
        import ssl as _ssl_mod

        cert = _ssl_mod._ssl._test_decode_cert(tmp_path)  # type: ignore[attr-defined]
    except Exception as exc:  # noqa: BLE001 — fallback is best-effort
        raise BuiltinError(f"x509_parse: cannot decode cert ({exc})") from exc
    sans = [val for typ, val in cert.get("subjectAltName", ()) if typ in ("DNS", "IP Address")]
    return {
        "subject": _x509_name_to_str(cert.get("subject", ())),
        "issuer": _x509_name_to_str(cert.get("issuer", ())),
        "not_before": cert.get("notBefore"),
        "not_after": cert.get("notAfter"),
        "serial": cert.get("serialNumber"),
        "fingerprint_sha256": fingerprint,
        "sans": sans,
        "key_alg": None,
        "key_size": None,
        "sig_alg": None,
        "version": cert.get("version"),
    }


def _x509_name_to_str(name_tuples: tuple) -> str:
    parts: list[str] = []
    for rdn in name_tuples:
        for typ, val in rdn:
            parts.append(f"{typ}={val}")
    return ", ".join(parts)
