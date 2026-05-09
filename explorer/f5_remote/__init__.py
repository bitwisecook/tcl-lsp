"""Remote-access helpers for the ``f5`` CLI.

This package provides the networking and UCS-extraction layer that
``f5 fetch``, ``f5 extract``, ``f5 pull``, and ``f5 push`` share.  All
transports are stdlib-only (``http.client`` for iControl REST,
``subprocess`` for ssh/scp) so the ``f5`` console-script keeps zero
runtime dependencies beyond the existing zipapp.

Public entry points:

- :func:`fetch_scf` — transport-agnostic facade returning a
  :class:`FetchResult` containing the SCF body (and optionally the
  raw UCS bytes) plus provenance metadata.
- :class:`FetchResult` — bundle of bytes and ``(host, transport,
  timestamp, format)`` provenance.
- :class:`Credentials` — resolved ``host``/``user``/``password`` tuple.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

from .auth import Credentials, resolve_credentials  # noqa: F401  (re-export)
from .ucs import ucs_to_scf  # noqa: F401  (re-export)


@dataclass(slots=True)
class FetchResult:
    """Bundle of the bytes pulled from a device, plus provenance.

    ``scf`` is always present (UCS inputs are converted on the fly).
    ``ucs`` is only populated when the caller asked for the UCS too.
    """

    host: str
    transport: str  # "rest" | "ssh"
    fmt: str  # "scf" | "ucs" | "both"
    fetched_at: datetime
    scf: str
    ucs: bytes | None = None
    extras: dict[str, str] = field(default_factory=dict)


def fetch_scf(
    *,
    credentials: Credentials,
    transport: str = "auto",
    fmt: str = "scf",
    insecure: bool = True,
    timeout: float = 60.0,
) -> FetchResult:
    """Pull a config from *credentials.host* and return a :class:`FetchResult`.

    *transport* must be one of ``"rest"``, ``"ssh"``, or ``"auto"``
    (try REST first, fall back to SSH on connection error).  *fmt*
    selects ``"scf"``, ``"ucs"``, or ``"both"``.  When the device only
    yields a UCS the SCF is reconstructed via :func:`ucs_to_scf`.
    """
    from . import rest, ssh  # local import to keep top-level cheap

    if transport not in {"rest", "ssh", "auto"}:
        raise ValueError(f"unknown transport: {transport!r}")
    if fmt not in {"scf", "ucs", "both"}:
        raise ValueError(f"unknown format: {fmt!r}")

    fetched_at = datetime.now(timezone.utc)

    def _via_rest() -> FetchResult:
        scf_text, ucs_bytes = rest.fetch(credentials, fmt=fmt, insecure=insecure, timeout=timeout)
        return FetchResult(
            host=credentials.host,
            transport="rest",
            fmt=fmt,
            fetched_at=fetched_at,
            scf=scf_text,
            ucs=ucs_bytes,
        )

    def _via_ssh() -> FetchResult:
        scf_text, ucs_bytes = ssh.fetch(credentials, fmt=fmt, timeout=timeout)
        return FetchResult(
            host=credentials.host,
            transport="ssh",
            fmt=fmt,
            fetched_at=fetched_at,
            scf=scf_text,
            ucs=ucs_bytes,
        )

    if transport == "rest":
        return _via_rest()
    if transport == "ssh":
        return _via_ssh()

    # auto
    try:
        return _via_rest()
    except (ConnectionError, OSError, TimeoutError):
        return _via_ssh()
