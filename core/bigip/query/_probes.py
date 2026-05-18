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

import datetime
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

    Returns ``{"status", "headers", "body", "body_json",
    "peer_cert", "error"}``.  ``body`` is the raw response body
    decoded as UTF-8 with replacement.  ``body_json`` is the
    pre-parsed JSON value when the response's ``content-type``
    header includes ``json`` and the body is valid JSON;
    otherwise ``None``.  Pre-parsing here lets callers write
    ``url_get(...) | http_body_json(.)`` without re-parsing on
    every traversal, and means a query that only needs structured
    fields doesn't pay the regex cost twice.

    ``peer_cert`` is the negotiated server certificate parsed
    through :func:`x509_parse` (same dict shape every other x509
    helper produces).  ``None`` for ``http://`` URLs, failed
    handshakes, or when the underlying SSL stack doesn't expose
    the peer cert.  Captured during the same TLS handshake that
    served the HTTP request — no extra round-trip.
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

    def _attempt(
        ctx: ssl.SSLContext | None,
    ) -> tuple[dict[str, Any], _CertCapturingHTTPSHandler | None]:
        """Run one HTTP request via the given context.

        Returns ``(out_dict, cert_handler)`` so the caller can pull
        the captured PEM off the handler.  Raises
        :class:`ssl.SSLCertVerificationError` and other
        :class:`urllib.error.URLError` instances unchanged so the
        wrapper can choose to retry."""
        handler: _CertCapturingHTTPSHandler | None
        if scheme == "https":
            effective = ctx or ssl.create_default_context()
            handler = _CertCapturingHTTPSHandler(effective)
            opener_local = urllib.request.build_opener(handler)
        else:
            handler = None
            opener_local = urllib.request.build_opener()
        try:
            with opener_local.open(req, timeout=timeout_s) as resp:
                payload = resp.read()
                return (
                    {
                        "status": resp.status,
                        "headers": {k.lower(): v for k, v in resp.headers.items()},
                        "body": payload.decode("utf-8", "replace"),
                        "error": None,
                    },
                    handler,
                )
        except urllib.error.HTTPError as exc:
            return (
                {
                    "status": exc.code,
                    "headers": {k.lower(): v for k, v in (exc.headers or {}).items()},
                    "body": exc.read().decode("utf-8", "replace") if exc.fp else "",
                    "error": exc.reason,
                },
                handler,
            )

    # Default verifying context.  When a CA bundle is pinned, trust
    # it (and only it); otherwise the platform default.
    strict_ctx = (
        ssl.create_default_context(cafile=ca_bundle) if ca_bundle else ssl.create_default_context()
    )
    reason: dict[str, Any] = {"kind": "ok", "message": "", "fatal": False}
    cert_handler: _CertCapturingHTTPSHandler | None = None
    try:
        out, cert_handler = _attempt(strict_ctx)
    except ssl.SSLCertVerificationError as exc:
        # Verification failed — record the reason and retry without
        # verification so the audit query still gets the response
        # body + peer cert.  The cert is what they want to inspect.
        reason = _classify_cert_error(exc)
        try:
            out, cert_handler = _attempt(_permissive_ssl_context())
        except (urllib.error.URLError, OSError, TimeoutError) as exc2:
            out = {"status": None, "headers": {}, "body": "", "error": str(exc2)}
            reason = {"kind": "connection_error", "message": str(exc2), "fatal": True}
    except (urllib.error.URLError, OSError, TimeoutError) as exc:
        # The URLError wrapper hides ``ssl.SSLCertVerificationError``
        # inside ``exc.reason`` — extract it so audit queries get a
        # structured reason rather than a generic connection error.
        inner = getattr(exc, "reason", None)
        if isinstance(inner, ssl.SSLCertVerificationError):
            reason = _classify_cert_error(inner)
            try:
                out, cert_handler = _attempt(_permissive_ssl_context())
            except (urllib.error.URLError, OSError, TimeoutError) as exc2:
                out = {"status": None, "headers": {}, "body": "", "error": str(exc2)}
                reason = {"kind": "connection_error", "message": str(exc2), "fatal": True}
        else:
            out = {"status": None, "headers": {}, "body": "", "error": str(exc)}
            reason = {"kind": "connection_error", "message": str(exc), "fatal": True}
    body_str = out["body"] if isinstance(out["body"], str) else ""
    hdrs = out["headers"] if isinstance(out["headers"], dict) else {}
    out["body_json"] = _maybe_parse_body_json(body_str, hdrs)
    # ``peer_cert`` is the negotiated server cert parsed via
    # :func:`x509_parse`.  ``None`` for http URLs, failed handshakes,
    # or when the certificate isn't available (extremely rare on a
    # successful TLS connection).  The dict is the same shape every
    # other x509 builtin emits so queries chain naturally.
    peer_cert: dict[str, Any] | None = None
    if cert_handler is not None and cert_handler.peer_cert_pem:
        try:
            peer_cert = x509_parse(cert_handler.peer_cert_pem)
        except BuiltinError:
            peer_cert = None
    out["peer_cert"] = peer_cert
    out["reason"] = reason
    _URL_CACHE[cache_key] = out
    return dict(out)


class _CertCapturingHTTPSHandler(urllib.request.HTTPSHandler):
    """``urllib`` handler that captures the negotiated server cert.

    Sits between :func:`urllib.request.build_opener` and
    :class:`http.client.HTTPSConnection`, swapping in a connection
    subclass whose ``connect()`` records the peer cert (as PEM)
    onto the handler so the caller can retrieve it after the
    response body is read.  Falls back silently — the handler
    leaves ``peer_cert_pem = None`` when the SSL stack doesn't
    expose the underlying cert (rare on a successful handshake).
    """

    def __init__(self, ctx: ssl.SSLContext) -> None:
        super().__init__(context=ctx)
        self.peer_cert_pem: str | None = None
        self._ctx = ctx

    def https_open(self, req):
        import http.client

        handler_self = self

        class _Conn(http.client.HTTPSConnection):
            def connect(self) -> None:
                super().connect()
                try:
                    der = self.sock.getpeercert(binary_form=True)
                except (AttributeError, ssl.SSLError):
                    return
                if der:
                    handler_self.peer_cert_pem = ssl.DER_cert_to_PEM_cert(der)

        return self.do_open(_Conn, req, context=self._ctx)


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
    "verify_status", "reason", "error"}``.

    Verification failures (expired cert, hostname mismatch,
    self-signed bundle, untrusted CA) do **not** abort the
    handshake — the probe retries with verification disabled so
    audit queries still get the peer cert and protocol metadata.
    The ``reason`` sub-object (``{"kind", "message", "fatal"}``)
    records what would have failed; ``fatal=True`` only when the
    connection itself couldn't complete (DNS / refused / timeout).

    Use ``reason.kind`` to filter audit results: ``select(.reason.kind
    == "expired")`` finds every endpoint with an expired cert,
    ``select(.reason.kind == "ok")`` only verified-clean endpoints.
    """
    _require_probes("tls_handshake")
    sni_value = sni if sni is not None else host
    ca_bundle = TLS_CA_BUNDLE.get()
    key = (host, int(port), sni_value, ca_bundle)
    if key in _TLS_CACHE:
        return dict(_TLS_CACHE[key])

    def _run(ctx: ssl.SSLContext) -> dict[str, Any]:
        with socket.create_connection((host, int(port)), timeout=timeout_s) as sock:
            with ctx.wrap_socket(sock, server_hostname=sni_value) as ssock:
                cipher = ssock.cipher()
                peer = ssock.getpeercert()
                peer_der = ssock.getpeercert(binary_form=True)
                alpn_selected = ssock.selected_alpn_protocol() if alpn else None
                peer_cert: Any = peer
                if peer_der is not None:
                    try:
                        pem = ssl.DER_cert_to_PEM_cert(peer_der)
                        peer_cert = x509_parse(pem)
                    except Exception:
                        peer_cert = peer
                return {
                    "protocol": ssock.version(),
                    "cipher": cipher[0] if cipher else None,
                    "peer_cert": peer_cert,
                    "alpn_selected": alpn_selected,
                }

    strict_ctx = (
        ssl.create_default_context(cafile=ca_bundle) if ca_bundle else ssl.create_default_context()
    )
    if alpn:
        strict_ctx.set_alpn_protocols(alpn)

    reason: dict[str, Any] = {"kind": "ok", "message": "", "fatal": False}
    out: dict[str, Any]
    try:
        out = _run(strict_ctx)
        out["verify_status"] = "ok"
        out["error"] = None
    except ssl.SSLCertVerificationError as exc:
        reason = _classify_cert_error(exc)
        # Retry without verification so the audit query still gets
        # the cert + protocol metadata.  Both legs go through the
        # same socket logic; only the context differs.
        permissive = _permissive_ssl_context()
        if alpn:
            permissive.set_alpn_protocols(alpn)
        try:
            out = _run(permissive)
            out["verify_status"] = reason["message"] or "verification failed"
            out["error"] = None
        except (OSError, ssl.SSLError, TimeoutError) as exc2:
            out = {
                "protocol": None,
                "cipher": None,
                "peer_cert": None,
                "alpn_selected": None,
                "verify_status": "error",
                "error": str(exc2),
            }
            reason = {"kind": "connection_error", "message": str(exc2), "fatal": True}
    except (OSError, ssl.SSLError, TimeoutError) as exc:
        out = {
            "protocol": None,
            "cipher": None,
            "peer_cert": None,
            "alpn_selected": None,
            "verify_status": "error",
            "error": str(exc),
        }
        reason = {"kind": "connection_error", "message": str(exc), "fatal": True}
    out["reason"] = reason
    _TLS_CACHE[key] = out
    return dict(out)


# ---------------------------------------------------------------------------
# Cert-error classification — shared by ``url_*`` and ``tls_handshake``
# ---------------------------------------------------------------------------


# Maps OpenSSL verify codes (the integers ``SSLCertVerificationError``
# exposes via ``.verify_code``) to the ``reason.kind`` tags the probes
# emit.  An unmatched code becomes ``"other_verification"``.  Codes
# come from ``openssl/x509_vfy.h``; we only enumerate the ones a
# typical audit query needs to filter on.
_VERIFY_CODE_TO_KIND: dict[int, str] = {
    10: "expired",  # X509_V_ERR_CERT_HAS_EXPIRED
    9: "not_yet_valid",  # X509_V_ERR_CERT_NOT_YET_VALID
    18: "self_signed",  # X509_V_ERR_DEPTH_ZERO_SELF_SIGNED_CERT
    19: "self_signed",  # X509_V_ERR_SELF_SIGNED_CERT_IN_CHAIN
    20: "untrusted_ca",  # X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT
    21: "untrusted_ca",  # X509_V_ERR_UNABLE_TO_VERIFY_LEAF_SIGNATURE
    24: "untrusted_ca",  # X509_V_ERR_INVALID_CA
    62: "hostname_mismatch",  # X509_V_ERR_HOSTNAME_MISMATCH
}


def _classify_cert_error(exc: ssl.SSLCertVerificationError) -> dict[str, Any]:
    """Return a structured ``reason`` for an SSL verification error.

    Shape: ``{"kind": str, "message": str, "fatal": bool}``.  Kind
    is one of ``ok`` / ``expired`` / ``not_yet_valid`` /
    ``self_signed`` / ``hostname_mismatch`` / ``untrusted_ca`` /
    ``other_verification``.  ``fatal=False`` for verification
    failures (we got the cert and the response body anyway);
    ``fatal=True`` is reserved for connection-level errors that
    leave us with no data.
    """
    code = getattr(exc, "verify_code", None)
    kind = (
        _VERIFY_CODE_TO_KIND.get(code, "other_verification")
        if code is not None
        else "other_verification"
    )
    message = getattr(exc, "verify_message", None) or str(exc)
    return {"kind": kind, "message": message, "fatal": False}


def _permissive_ssl_context() -> ssl.SSLContext:
    """Return an :class:`ssl.SSLContext` that accepts any peer.

    Used as the second-try context when strict verification fails
    so audit queries still collect the peer cert + response body.
    The cert lives on the captured handshake regardless of
    verification status — pinning the failure reason via
    ``_classify_cert_error`` lets consumers filter without losing
    the data they need to inspect.
    """
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


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

        # CPython's `_ssl` C extension exposes `_test_decode_cert` for ssl's
        # own test suite — the only stdlib entry point that returns a parsed
        # certificate without needing OpenSSL bindings.  Stable since 3.x.
        cert = _ssl_mod._ssl._test_decode_cert(tmp_path)  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]
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


# ---------------------------------------------------------------------------
# BIG-IP config-object cert → x509_parse-shaped dict
# ---------------------------------------------------------------------------


# Map BIG-IP ``key-type`` tokens to the same ``key_alg`` strings
# ``cryptography``'s ``type(public_key).__name__`` produces, so a
# config-object cert and an :func:`x509_parse` cert compare equal
# under ``==`` when they describe the same key material.
_KEY_TYPE_TO_KEY_ALG: dict[str, str] = {
    "rsa-public": "RSAPublicKey",
    "rsa-private": "RSAPublicKey",
    "ec-public": "EllipticCurvePublicKey",
    "ec-private": "EllipticCurvePublicKey",
    "dsa-public": "DSAPublicKey",
    "dsa-private": "DSAPublicKey",
    "ed25519-public": "Ed25519PublicKey",
    "ed25519-private": "Ed25519PublicKey",
    "ed448-public": "Ed448PublicKey",
    "ed448-private": "Ed448PublicKey",
}


def x509_from_config(cert: Any) -> dict[str, Any]:
    """Project a BIG-IP config-object cert into the same shape
    :func:`x509_parse` produces.

    Works against any config object that carries the BIG-IP cert
    metadata fields — ``subject`` / ``issuer`` / ``fingerprint``
    / ``serial-number`` / ``version`` / ``key-type`` /
    ``certificate-key-size`` / ``expiration-string`` /
    ``expiration-date`` / ``subject-alternative-name``.  The
    parsed model classes that match this shape today:

    - :class:`BigipSysFileSslCert` — ``sys file ssl-cert`` (the
      cert / chain / bundle store, used by client-ssl / server-ssl
      profiles and HTTPS monitors).
    - :class:`BigipCmCert` — ``cm cert`` (device-trust certs,
      used by ``cm device.cert`` / ``cm trust-domain.ca-cert``).

    Other config objects that reference certs by *PathRef*
    (``ltm monitor https.cert``, ``ltm profile client-ssl``
    ``cert-key-chain.cert``, ``cm device.cert``, ``cm
    trust-domain.ca-cert``) follow the indirection to one of the
    types above — pipe through ``.sys["file-ssl-cert"][ref]`` /
    ``.cm.cert[ref]`` first, then call this builtin on the
    resolved object.  Minimal projections (``sys crypto cert``)
    don't carry cert metadata; load the underlying PEM with
    :func:`cert_load` / :func:`x509_load_file` instead.

    Lets queries compare a cert imported into BIG-IP against a
    cert parsed from PEM bytes (``x509_parse`` / ``x509_load_file``
    / ``tls_handshake().peer_cert``) under plain ``==`` — every
    field is normalised:

    - ``subject`` / ``issuer`` — already RFC-4514-ish on the
      BIG-IP side; passed through.
    - ``not_after`` — parsed from ``expiration-string`` (TMSH emits
      ``"Jan 1 00:00:00 2099 GMT"``) into ISO-8601 UTC.  Falls
      back to ``expiration-date`` (epoch seconds) when the string
      form isn't available.
    - ``not_before`` — ``None``; BIG-IP doesn't expose it.
    - ``serial`` — decimal → uppercase hex (no ``0x`` prefix), the
      same shape :func:`x509_parse` emits.
    - ``fingerprint_sha256`` — ``SHA256/12:34:56:...`` → ``"1234..."``
      uppercase, prefix and separators stripped.  Other fingerprint
      algorithms come through unmodified.
    - ``sans`` — ``"DNS:foo.com,DNS:bar.com"`` → ``["foo.com",
      "bar.com"]``; the ``TYPE:`` prefix is stripped to match what
      :func:`x509_parse` emits.
    - ``key_alg`` — derived from ``key-type`` via the
      :data:`_KEY_TYPE_TO_KEY_ALG` map.
    - ``key_size`` — coerced to ``int``.
    - ``version`` — ``"3"`` → ``"v3"`` to match cryptography's
      ``CertificateVersion.name``.
    - ``sig_alg`` / ``public_key_pem`` — ``None``; BIG-IP's TMSH
      surface doesn't carry them.
    """
    if cert is None:
        raise BuiltinError("x509_from_config: cannot project None")

    def _get(name: str) -> str:
        # Accept dataclass attribute, dict key, or ObjectRef field
        # form so the helper works against the model directly AND
        # against the DSL's projected :class:`ObjectRef`.  Strip
        # surrounding ``"..."`` quotes the parser leaves on some
        # fields (``fingerprint``, ``serial-number``, …) so the
        # normalised output matches :func:`x509_parse`.
        raw: str = ""
        if hasattr(cert, name):
            value = getattr(cert, name)
            if value is not None:
                raw = str(value)
        if not raw:
            tmsh_name = name.replace("_", "-")
            if hasattr(cert, "fields"):
                field_map = cert.fields
                if tmsh_name in field_map and field_map[tmsh_name] is not None:
                    raw = str(field_map[tmsh_name])
                elif name in field_map and field_map[name] is not None:
                    raw = str(field_map[name])
        if len(raw) >= 2 and raw[0] == '"' and raw[-1] == '"':
            raw = raw[1:-1]
        return raw

    subject = _get("subject")
    issuer = _get("issuer")
    san_raw = _get("subject_alternative_name")
    sans: list[str] = []
    if san_raw:
        for chunk in san_raw.split(","):
            token = chunk.strip()
            if ":" in token:
                _, _, host = token.partition(":")
                sans.append(host.strip())
            elif token:
                sans.append(token)

    fingerprint = _get("fingerprint")
    if fingerprint.upper().startswith("SHA256/"):
        fingerprint = fingerprint[len("SHA256/") :]
    fingerprint = fingerprint.replace(":", "").upper()

    serial_raw = _get("serial_number")
    serial_hex = ""
    if serial_raw:
        try:
            serial_hex = format(int(serial_raw), "X")
        except ValueError:
            serial_hex = serial_raw.upper()

    version_raw = _get("version")
    version = (
        f"v{version_raw}"
        if version_raw and not version_raw.lower().startswith("v")
        else version_raw
    )

    key_size_raw = _get("key_size") or _get("certificate_key_size")
    try:
        key_size: int | None = int(key_size_raw) if key_size_raw else None
    except ValueError:
        key_size = None

    key_alg = _KEY_TYPE_TO_KEY_ALG.get(_get("key_type"), None)

    not_after = _parse_x509_date(_get("expiration_string"))
    if not_after is None:
        epoch_raw = _get("expiration_date")
        if epoch_raw:
            try:
                not_after = datetime.datetime.fromtimestamp(
                    int(epoch_raw), tz=datetime.timezone.utc
                ).isoformat()
            except (ValueError, OverflowError):
                not_after = None

    return {
        "subject": subject,
        "issuer": issuer,
        "not_before": None,
        "not_after": not_after,
        "serial": serial_hex,
        "fingerprint_sha256": fingerprint,
        "sans": sans,
        "key_alg": key_alg,
        "key_size": key_size,
        "sig_alg": None,
        "version": version,
        "public_key_pem": None,
    }


def _parse_x509_date(text: str) -> str | None:
    """Parse a BIG-IP ``expiration-string`` value into ISO-8601 UTC.

    TMSH emits ``"Jan 1 00:00:00 2099 GMT"``; some 13.x builds emit
    the same with a leading zero on single-digit days.  Returns
    ``None`` when the text doesn't match any known format."""
    if not text:
        return None
    formats = (
        "%b %d %H:%M:%S %Y %Z",
        "%b  %d %H:%M:%S %Y %Z",  # double-space day alignment
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S",
    )
    for fmt in formats:
        try:
            parsed = datetime.datetime.strptime(text, fmt)
        except ValueError:
            continue
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=datetime.timezone.utc)
        return parsed.astimezone(datetime.timezone.utc).isoformat()
    return None


def x509_eq(left: Any, right: Any) -> bool:
    """Compare two :func:`x509_parse`-shaped dicts for cert identity.

    Two certs are considered equal when they describe the same
    public key material.  Comparison order, strongest → weakest:

    1. ``fingerprint_sha256`` — the canonical identity.  Two certs
       with the same SHA-256 fingerprint are the same cert.
    2. ``subject`` + ``issuer`` + ``serial`` — the X.509-defined
       primary key.  Used when one side is a BIG-IP ``sys file
       ssl-cert`` that doesn't carry a SHA-256 fingerprint.

    Plain ``==`` on the dicts compares every field including
    ``not_before`` / ``sig_alg`` / ``public_key_pem`` which the
    BIG-IP side leaves ``None``.  Use this helper instead when
    you want "same cert" rather than "same projection"."""
    if not isinstance(left, dict) or not isinstance(right, dict):
        return left == right
    lfp = left.get("fingerprint_sha256") or ""
    rfp = right.get("fingerprint_sha256") or ""
    if lfp and rfp:
        return lfp.upper() == rfp.upper()
    # Fallback: X.509 issuer + serial uniquely identify a cert
    # within an issuer's namespace; pair with subject to catch
    # mismatched root vs intermediate vs leaf.
    return (
        (left.get("subject") or "") == (right.get("subject") or "")
        and (left.get("issuer") or "") == (right.get("issuer") or "")
        and (left.get("serial") or "").upper() == (right.get("serial") or "").upper()
    )
