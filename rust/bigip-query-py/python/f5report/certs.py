"""SSL certificate inventory & expiry.

Answers the everyday "which certs are expiring / already expired, and what do
they front?" question (see e.g. the r/f5networks certificate-expiry automation
threads) straight from a config export or UCS backup — no device access, no
OpenSSL.

BIG-IP stores certificate metadata *in the config itself*: every
``sys file ssl-cert`` stanza carries ``expiration-string``, ``expiration-date``
(epoch seconds), ``subject``, ``issuer``, ``fingerprint``, ``key-type`` /
``key-size`` and the subject-alternative-name list.

The ``f5-query`` DSL only projects the ``ltm`` module, so — unlike the rest of
the report — the certificate list comes from the parsed model directly, via the
native :func:`f5report._engine.sys_file_ssl_certs` (the same ``parse_bigip_conf``
the query engine is built on). Each cert is cross-referenced against the SSL
profiles (and, through them, the virtual servers) that use it. Days-until-expiry
is computed live in the browser against the viewer's clock (see
``templates/certs.js``), so the tab is accurate whenever the report is opened.
"""

from __future__ import annotations

import json
from typing import Any

from . import _engine

Sources = list[tuple[str, str]]


def collect_certs(sources: Sources, device: dict[str, Any]) -> list[dict[str, Any]]:
    """Build the certificate inventory for one already-shaped device.

    ``device`` must already hold the shaped ``profiles`` and ``virtuals`` (used
    for the cross-reference); ``sources`` is the single ``(uri, scf)`` for the
    device.
    """
    raw = json.loads(_engine.sys_file_ssl_certs(sources))
    keys = {k.get("full_path", ""): k for k in json.loads(_engine.sys_file_ssl_keys(sources))}

    # profile full-path -> [virtual names] using it, for the reverse map.
    profile_to_virtuals: dict[str, list[str]] = {}
    for v in device.get("virtuals", []):
        for p in v.get("profiles", []) or []:
            profile_to_virtuals.setdefault(p, []).append(v["name"])

    profiles = device.get("profiles", [])

    certs: list[dict[str, Any]] = []
    for f in raw:
        full_path = f.get("full_path", "")
        name = f.get("name") or full_path.rsplit("/", 1)[-1]

        used_profiles: set[str] = set()
        used_virtuals: set[str] = set()
        key_candidates: list[str] = []
        for p in profiles:
            if p.get("cert") == full_path or p.get("chain") == full_path:
                used_profiles.add(p["name"])
                if p.get("key"):
                    key_candidates.append(p["key"])
                for vn in profile_to_virtuals.get(p.get("fullPath", ""), []):
                    used_virtuals.add(vn)
        # Fallback: pair `foo.crt` with `foo.key` by name.
        if full_path.endswith(".crt"):
            key_candidates.append(full_path[: -len(".crt")] + ".key")

        key = next((keys[kp] for kp in key_candidates if kp in keys), None)
        key_pass = (key or {}).get("passphrase", "")

        certs.append(
            {
                "name": name,
                "fullPath": full_path,
                "subject": f.get("subject", ""),
                "issuer": f.get("issuer", ""),
                "expirationString": f.get("expiration_string", ""),
                "expirationDate": f.get("expiration_date", ""),
                "fingerprint": f.get("fingerprint", ""),
                "keyType": f.get("key_type", ""),
                "keySize": f.get("key_size", ""),
                "serialNumber": f.get("serial_number", ""),
                "subjectAlternativeName": f.get("subject_alternative_name", ""),
                "isBundle": f.get("is_bundle", ""),
                "sourcePath": f.get("source_path", ""),
                "hasKey": key is not None,
                "keyPath": (key or {}).get("full_path", ""),
                "keySecurityType": (key or {}).get("security_type", ""),
                "keyPassphrase": key_pass,
                "keyPassphraseEncrypted": key_pass.startswith("$M$"),
                "usedByProfiles": sorted(used_profiles),
                "usedByVirtuals": sorted(used_virtuals),
            }
        )
    return certs
