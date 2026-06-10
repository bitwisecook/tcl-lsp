"""Typed BIG-IP cert-key-chain entry.

Client- and server-SSL profiles attach certificates as a keyed-block
list under ``cert-key-chain``:

    cert-key-chain {
        cert_a {
            cert /Common/cert_a.crt
            key /Common/cert_a.key
            chain /Common/ca_bundle.crt
        }
        cert_b {
            cert /Common/cert_b.crt
            key /Common/cert_b.key
        }
    }

Each item carries up to three sub-references (cert + key + optional
chain).  Modelling this typed lets the graph layer find every
profile that references a given certificate, and lets queries
filter chains by which CA bundle they trust.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class CertKeyChain:
    """One ``cert-key-chain`` keyed-block entry.

    ``name`` is the entry's bare name (the key inside the
    ``cert-key-chain { ... }`` block).  ``cert`` / ``key`` /
    ``chain`` are the optional sub-references; empty when the
    corresponding line was omitted in the source.  ``raw``
    preserves the original body for round trips.
    """

    name: str = ""
    cert: str = ""
    key: str = ""
    chain: str = ""
    passphrase: str = ""
    raw: str = ""

    @classmethod
    def from_raw(cls, name: str, body: str) -> "CertKeyChain":
        """Build an entry from the keyed-block parser output."""
        cert = key = chain = passphrase = ""
        tokens = body.split()
        for i, tok in enumerate(tokens):
            if i + 1 >= len(tokens):
                continue
            value = tokens[i + 1]
            if tok == "cert":
                cert = value
            elif tok == "key":
                key = value
            elif tok == "chain":
                chain = value
            elif tok == "passphrase":
                passphrase = value
        return cls(
            name=name,
            cert=cert,
            key=key,
            chain=chain,
            passphrase=passphrase,
            raw=body.strip(),
        )

    def references(self) -> tuple[tuple[str, str], ...]:
        """Yield ``(kind, full-path)`` pairs for every sub-reference.

        The graph layer uses these to compute edges from the
        containing profile into the SSL cert / key / CA bundle.
        Order is stable so re-emission preserves the user's intent.
        """
        out: list[tuple[str, str]] = []
        if self.cert:
            out.append(("sys file ssl-cert", self.cert))
        if self.key:
            out.append(("sys file ssl-key", self.key))
        if self.chain:
            out.append(("sys file ssl-cert", self.chain))
        return tuple(out)

    def __str__(self) -> str:
        if self.raw:
            return f"{self.name} {{ {self.raw} }}"
        parts: list[str] = []
        if self.cert:
            parts.append(f"cert {self.cert}")
        if self.key:
            parts.append(f"key {self.key}")
        if self.chain:
            parts.append(f"chain {self.chain}")
        if self.passphrase:
            parts.append(f"passphrase {self.passphrase}")
        body = " ".join(parts)
        return f"{self.name} {{ {body} }}" if body else f"{self.name} {{ }}"


__all__ = ["CertKeyChain"]
