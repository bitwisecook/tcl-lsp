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

"""SSL certificate inventory & expiry.

Answers the everyday "which certs are expiring / already expired, and what do
they front?" question (see e.g. the r/f5networks certificate-expiry automation
threads) straight from a config export or UCS backup — no device access, no
OpenSSL.

Two data sources are combined, file-first:

* The **real certificate** parsed out of the UCS filestore
  (:func:`f5report._engine.sys_file_ssl_certs_x509`). A modern BIG-IP export
  carries a *metadata-free* ``sys file ssl-cert`` stanza — the subject / issuer
  / validity / SANs live only in the PEM, not in the config — so the report can
  only fill the row (and recover the **issue date** and the certificate
  **chain**) by reading the archived cert bytes. When a source is a UCS on disk
  this is where subject, issuer, not-before, not-after, serial, SANs, key- and
  signature-algorithm come from.
* The ``sys file ssl-cert`` **config metadata** (``expiration-string``,
  ``fingerprint``, ``key-type``…), used as a fallback when the file isn't
  reachable (e.g. a bare ``bigip.conf`` with no filestore).

Each cert is cross-referenced against the SSL profiles (and, through them, the
virtual servers) that use it, and its trust chain is reconstructed by walking
issuer → subject across every cert (leaf, intermediates in a bundle, and the
referenced CA-chain file). Days-until-expiry is computed live in the browser
against the viewer's clock (see ``templates/certs.js``), so the tab is accurate
whenever the report is opened.
"""

from __future__ import annotations

import json
from datetime import datetime
from typing import Any

from . import _engine

Sources = list[tuple[str, str]]


def _unquote(s: str) -> str:
    """Strip one layer of surrounding double quotes (a TMSH metadata artifact)."""
    s = (s or "").strip()
    if len(s) >= 2 and s[0] == '"' and s[-1] == '"':
        return s[1:-1]
    return s


def _cn(dn: str) -> str:
    """Pull the CN out of an RFC 4514 distinguished name (``CN=…,O=…``)."""
    dn = _unquote(dn)
    if not dn:
        return ""
    # RFC 4514 uses ``,`` separators; a literal comma in a value is escaped as
    # ``\,`` — split on unescaped commas only.
    parts: list[str] = []
    buf = ""
    esc = False
    for ch in dn:
        if esc:
            buf += ch
            esc = False
        elif ch == "\\":
            buf += ch
            esc = True
        elif ch == ",":
            parts.append(buf)
            buf = ""
        else:
            buf += ch
    parts.append(buf)
    for p in parts:
        p = p.strip()
        if p[:3].upper() == "CN=":
            return p[3:]
    return ""


def _epoch(iso: str) -> str:
    """ISO-8601 (``2026-08-13T12:47:21+00:00``) → unix-seconds string, or ``""``."""
    if not iso:
        return ""
    try:
        return str(int(datetime.fromisoformat(iso).timestamp()))
    except ValueError:
        return ""


def _human_date(iso: str) -> str:
    """ISO-8601 → ``Aug 13 2026 12:47 UTC`` for display, or ``""``."""
    if not iso:
        return ""
    try:
        dt = datetime.fromisoformat(iso)
    except ValueError:
        return ""
    return dt.strftime("%b %d %Y %H:%M UTC")


def _key_type_from_x509(key_alg: str) -> str:
    """``RSAPublicKey`` → ``rsa``; ``EC`` → ``ec`` — matches the config wording."""
    a = (key_alg or "").lower()
    if "rsa" in a:
        return "rsa"
    if "ec" in a or "elliptic" in a:
        return "ec"
    if "dsa" in a:
        return "dsa"
    return key_alg or ""


def _pool_entry(x: dict[str, Any], source: str) -> dict[str, Any]:
    """One node of the trust graph, keyed later by its subject DN."""
    subject = x.get("subject", "")
    issuer = x.get("issuer", "")
    return {
        "subject": subject,
        "issuer": issuer,
        "cn": _cn(subject),
        "issuerCn": _cn(issuer),
        "notBefore": x.get("not_before", ""),
        "notAfter": x.get("not_after", ""),
        "serial": x.get("serial", ""),
        "fingerprint": x.get("fingerprint_sha256", ""),
        "selfSigned": bool(subject) and subject == issuer,
        "source": source,
    }


def _build_pool(raw: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    """Map subject-DN → cert node across every leaf, embedded and chain cert."""
    pool: dict[str, dict[str, Any]] = {}
    for f in raw:
        src = f.get("full_path", "")
        x = f.get("x509")
        if x:
            ent = _pool_entry(x, src)
            pool.setdefault(ent["subject"], ent)
        for e in f.get("x509_embedded", []) or []:
            ent = _pool_entry(e, src)
            pool.setdefault(ent["subject"], ent)
    return pool


def _resolve_chain(
    leaf: dict[str, Any], pool: dict[str, dict[str, Any]]
) -> list[dict[str, Any]]:
    """Walk issuer → subject from ``leaf`` to a self-signed root (or a gap)."""
    if not leaf or not leaf.get("subject"):
        return []
    chain: list[dict[str, Any]] = []
    seen: set[str] = set()
    cur = _pool_entry(leaf, "")
    while cur and cur["subject"] not in seen:
        seen.add(cur["subject"])
        chain.append(cur)
        if cur["selfSigned"]:
            break
        cur = pool.get(cur["issuer"])
    complete = bool(chain) and chain[-1]["selfSigned"]
    out: list[dict[str, Any]] = []
    for i, link in enumerate(chain):
        if i == 0:
            role = "leaf"
        elif link["selfSigned"]:
            role = "root"
        else:
            role = "intermediate"
        out.append(
            {
                "role": role,
                "cn": link["cn"] or link["subject"],
                "subject": link["subject"],
                "issuerCn": link["issuerCn"] or link["issuer"],
                "notAfter": _human_date(link["notAfter"]),
                "selfSigned": link["selfSigned"],
            }
        )
    # A trailing non-self-signed link means we couldn't reach the root.
    if out and not complete:
        out.append(
            {
                "role": "gap",
                "cn": chain[-1]["issuerCn"] or chain[-1]["issuer"],
                "subject": "",
                "issuerCn": "",
                "notAfter": "",
                "selfSigned": False,
            }
        )
    return out


def collect_certs(sources: Sources, device: dict[str, Any]) -> list[dict[str, Any]]:
    """Build the certificate inventory for one already-shaped device.

    ``device`` must already hold the shaped ``profiles`` and ``virtuals`` (used
    for the cross-reference); ``sources`` is the single ``(uri, scf)`` for the
    device.
    """
    raw = json.loads(_engine.sys_file_ssl_certs_x509(sources))
    keys = {
        k.get("full_path", ""): k
        for k in json.loads(_engine.sys_file_ssl_keys(sources))
    }
    pool = _build_pool(raw)

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
        x = f.get("x509") or {}

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

        # File-first, config-fallback for every field the real cert carries.
        subject = x.get("subject") or _unquote(f.get("subject", ""))
        issuer = x.get("issuer") or _unquote(f.get("issuer", ""))
        sans_list = x.get("sans")
        if sans_list:
            san = ", ".join(sans_list)
        else:
            san = _unquote(f.get("subject_alternative_name", ""))
        not_after_iso = x.get("not_after", "")
        exp_string = _unquote(f.get("expiration_string", "")) or _human_date(
            not_after_iso
        )
        exp_epoch = f.get("expiration_date", "") or _epoch(not_after_iso)
        serial = x.get("serial") or f.get("serial_number", "")
        key_type = f.get("key_type", "") or _key_type_from_x509(x.get("key_alg", ""))
        key_size = f.get("key_size", "") or (
            str(x.get("key_size")) if x.get("key_size") else ""
        )
        fingerprint = x.get("fingerprint_sha256") or f.get("fingerprint", "")

        chain = _resolve_chain(x, pool) if x else []

        certs.append(
            {
                "name": name,
                "fullPath": full_path,
                "subject": subject,
                "cn": _cn(subject),
                "issuer": issuer,
                "issuerCn": _cn(issuer),
                "notBefore": _human_date(x.get("not_before", "")),
                "expirationString": exp_string,
                "expirationDate": exp_epoch,
                "fingerprint": fingerprint,
                "keyType": key_type,
                "keySize": key_size,
                "sigAlg": x.get("sig_alg", ""),
                "serialNumber": serial,
                "subjectAlternativeName": san,
                "isBundle": f.get("is_bundle", "")
                or ("true" if f.get("x509_embedded") else ""),
                "sourcePath": f.get("source_path", ""),
                "fromCertFile": bool(x),
                "chain": chain,
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
