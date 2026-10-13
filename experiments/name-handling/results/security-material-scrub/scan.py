# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

import base64
import bisect
import bz2
import collections
import concurrent.futures
import gzip
import hashlib
import io
import json
import lzma
import re
import subprocess
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
PATTERNS = {
    "certificate": re.compile(rb"-----BEGIN (?:TRUSTED )?CERTIFICATE-----"),
    "public-key": re.compile(rb"-----BEGIN (?:RSA )?PUBLIC KEY-----"),
    "private-key": re.compile(
        rb"-----BEGIN (?:[A-Z0-9 ]*PRIVATE KEY|PGP PRIVATE KEY BLOCK)-----"
    ),
    "provider-token": re.compile(
        rb"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,}|sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{25,}|xox[baprs]-[A-Za-z0-9-]{20,}|AIza[\w-]{30,}|(?:AKIA|ASIA)[A-Z0-9]{16}|SG\.[\w-]{20,}\.[\w-]{20,})\b"
    ),
    "password-hash": re.compile(
        rb"\$(?:[1256]|2[aby]|argon2id?)\$[./A-Za-z0-9$=,+-]{12,}"
    ),
    "f5-encrypted-secret": re.compile(rb"\$[MS]\$[A-Za-z0-9+/=]{12,}"),
    "authorization": re.compile(
        rb'(?i)\b(?:authorization|proxy-authorization)\s*[:=]\s*["\']?(?:Basic|Bearer)\s+([A-Za-z0-9+/_.=-]{8,})'
    ),
    "jwt": re.compile(
        rb"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{12,}\b"
    ),
    "credential-assignment": re.compile(
        rb'(?i)(?:["\']?\b(?:[A-Za-z_]*password|passwd|passphrase|client[_-]secret|api[_-]key|access[_-]token|refresh[_-]token|auth[_-]token|secret[_-]key|shared[_-]secret|privacy[_-]password|auth[_-]password|community[_-]name|encrypted[_-]password)\b["\']?\s*(?:[:=]\s*|\s+))["\']?([^\s"\'{}\\<>;,]{3,})'
    ),
    "cookie": re.compile(rb"(?im)^\s*(?:set-cookie|cookie)\s*:\s*([^\r\n]+)"),
    "credential-url": re.compile(
        rb'(?i)\b(?:https?|ssh|ftp|postgres(?:ql)?|mysql|redis|mongodb(?:\+srv)?)://[^\s/:"\'<>]+:[^\s/@"\'<>]+@'
    ),
    "credential-cli": re.compile(
        rb'(?i)(?:--password|-password|--token|--api-key|--client-secret|sshpass\s+-p|curl[^\r\n]{0,80}\s-u)\s+["\']?([^\s"\'{}<>\\]{3,})'
    ),
}
HEX = re.compile(rb"(?<![\w])(?:[0-9a-fA-F]{2}){24,}(?![\w])")
B64 = re.compile(
    rb"(?<![\w+/])(?:[A-Za-z0-9+/]{4}){8,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?(?![\w+/])"
)
KNOWN = []
# Optional private comparison file: one credential value per line, outside the checkout.
import os

comparison = os.environ.get("TCL_SCRUB_COMPARISON_FILE")
if comparison:
    for secret in Path(comparison).read_bytes().splitlines():
        if secret:
            KNOWN.extend([secret, secret.hex().encode(), base64.b64encode(secret)])


# No candidate values or snippets are written to reports.
def scan_text(data, label, encoding="plain"):
    out = []
    for marker in KNOWN:
        start = data.find(marker)
        if start >= 0:
            out.append(
                {
                    "path": label,
                    "encoding": encoding,
                    "category": "known-lab-credential",
                    "offset": start,
                    "length": len(marker),
                }
            )
    newlines = None
    for category, pattern in PATTERNS.items():
        if category == "credential-assignment" and not any(
            n in data.lower()
            for n in [
                b"password",
                b"passwd",
                b"passphrase",
                b"secret",
                b"key",
                b"token",
                b"community",
            ]
        ):
            continue
        if encoding == "binary":
            if category in [
                "credential-assignment",
                "credential-cli",
                "cookie",
                "credential-url",
            ]:
                continue
            needles = {
                "certificate": [b"BEGIN CERTIFICATE", b"BEGIN TRUSTED CERTIFICATE"],
                "public-key": [b"PUBLIC KEY-----"],
                "private-key": [b"PRIVATE KEY-----", b"PGP PRIVATE KEY BLOCK"],
                "provider-token": [
                    b"ghp_",
                    b"gho_",
                    b"ghu_",
                    b"ghs_",
                    b"ghr_",
                    b"github_pat_",
                    b"sk-",
                    b"xox",
                    b"AIza",
                    b"AKIA",
                    b"ASIA",
                    b"SG.",
                ],
                "password-hash": [
                    b"$1$",
                    b"$2$",
                    b"$5$",
                    b"$6$",
                    b"$2a$",
                    b"$2b$",
                    b"$2y$",
                    b"$argon2",
                ],
                "f5-encrypted-secret": [b"$M$", b"$S$"],
                "authorization": [b"Authorization", b"authorization"],
                "jwt": [b"eyJ"],
            }[category]
            if not any(needle in data for needle in needles):
                continue
        for match in pattern.finditer(data):
            value = match.group(1) if match.lastindex else match.group()
            if newlines is None:
                newlines = [m.start() for m in re.finditer(b"\n", data)]
            out.append(
                {
                    "path": label,
                    "encoding": encoding,
                    "category": category,
                    "line": bisect.bisect_left(newlines, match.start()) + 1,
                    "offset": match.start(),
                    "length": len(value),
                }
            )
    if encoding == "plain":
        seen = set()
        for kind, pattern in [("hex", HEX), ("base64", B64)]:
            for match in pattern.finditer(data):
                token = match.group()
                if len(token) > 4 * 1024 * 1024:
                    continue
                try:
                    decoded = (
                        bytes.fromhex(token.decode())
                        if kind == "hex"
                        else base64.b64decode(token, validate=True)
                    )
                except (ValueError, base64.binascii.Error):
                    continue
                if len(decoded) < 20 or b"\x00" in decoded[:256]:
                    continue
                sample = decoded[:512]
                if (
                    sum(32 <= c < 127 or c in (9, 10, 13) for c in sample)
                    < len(sample) * 0.8
                ):
                    continue
                digest = hashlib.sha256(decoded).digest()
                if digest in seen:
                    continue
                seen.add(digest)
                nested = scan_text(decoded, label, kind)
                if nested and newlines is None:
                    newlines = [m.start() for m in re.finditer(b"\n", data)]
                for row in nested:
                    row["carrier_line"] = (
                        bisect.bisect_left(newlines, match.start()) + 1
                    )
                out.extend(nested)
    return out


def scan_blob(data, label, depth=0):
    if depth > 3:
        return [], ["depth-limit:" + label]
    errors = []
    try:
        if data[:2] == b"\x1f\x8b":
            dec = gzip.decompress(data)
            if tarfile.is_tarfile(io.BytesIO(dec)):
                return scan_tar(dec, label, depth)
            return scan_blob(dec, label + "!gzip", depth + 1)
        if data[:3] == b"BZh":
            return scan_blob(bz2.decompress(data), label + "!bzip2", depth + 1)
        if data[:6] == b"\xfd7zXZ\x00":
            return scan_blob(lzma.decompress(data), label + "!xz", depth + 1)
        if data[:4] == b"PK\x03\x04":
            out = []
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for item in z.infolist():
                    if item.is_dir():
                        continue
                    if item.file_size > 268435456:
                        errors.append(
                            "member-size-limit:" + label + "!" + item.filename
                        )
                        continue
                    found, problem = scan_blob(
                        z.read(item), label + "!" + item.filename, depth + 1
                    )
                    out += found
                    errors += problem
            return out, errors
        if len(data) > 512 and data[257:262] == b"ustar":
            return scan_tar(data, label, depth)
        binary = data.startswith(b"\x7fELF") or b"\x00" in data[:256]
        return scan_text(data, label, "binary" if binary else "plain"), []
    except Exception as e:
        return [], [type(e).__name__ + ":" + label]


def scan_tar(data, label, depth):
    out = []
    errors = []
    with tarfile.open(fileobj=io.BytesIO(data)) as t:
        for item in t.getmembers():
            if not item.isfile():
                continue
            if item.size > 268435456:
                errors.append("member-size-limit:" + label + "!" + item.name)
                continue
            found, problem = scan_blob(
                t.extractfile(item).read(), label + "!" + item.name, depth + 1
            )
            out += found
            errors += problem
    return out, errors


def one(path):
    try:
        data = (ROOT / path).read_bytes()
        found, errors = scan_blob(data, path)
        return found, errors, len(data)
    except Exception as e:
        return [], [type(e).__name__ + ":" + path], 0


def main():
    paths = [
        p.decode()
        for p in subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).split(
            b"\0"
        )
        if p
    ]
    extra = [
        p.decode()
        for p in subprocess.check_output(
            ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=ROOT
        ).split(b"\0")
        if p
    ]
    paths += extra
    findings = []
    errors = []
    size = 0
    journal = Path("/tmp/tcl-scrub-candidates.jsonl").open("w")
    with concurrent.futures.ProcessPoolExecutor(max_workers=4) as pool:
        futures = [pool.submit(one, path) for path in paths]
        for index, future in enumerate(concurrent.futures.as_completed(futures), 1):
            rows, issues, n = future.result()
            findings += rows
            errors += issues
            size += n
            for row in rows:
                journal.write(json.dumps(row) + "\n")
            journal.flush()
            if index % 500 == 0:
                print("Scanned", index, "files; candidates", len(findings), flush=True)
    report = {
        "scope": "Current tracked files plus nonignored untracked files; archives recursively expanded; hex/base64 textual projections scanned. No values or snippets retained.",
        "files": len(paths),
        "source_bytes": size,
        "candidates": findings,
        "errors": errors,
        "counts": dict(collections.Counter(r["category"] for r in findings)),
    }
    Path("/tmp/tcl-scrub-candidates.json").write_text(
        json.dumps(report, indent=2) + "\n"
    )
    print(
        json.dumps(
            {k: v for k, v in report.items() if k not in ["candidates", "errors"]}
        )
    )
    print("Errors", len(errors), flush=True)


if __name__ == "__main__":
    main()
