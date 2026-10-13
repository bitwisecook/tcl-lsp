# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

import base64
import concurrent.futures
import gzip
import io
import json
import re
import subprocess
import tarfile
import zipfile
from pathlib import Path
from cryptography import x509
from cryptography.exceptions import UnsupportedAlgorithm
from cryptography.hazmat.primitives import serialization

ROOT = Path(__file__).resolve().parents[4]
B64 = re.compile(rb"(?<![A-Za-z0-9+/])MII[A-Za-z0-9+/]{150,}={0,2}")
DER = re.compile(rb"\x30\x82([\x01-\x7f])([\x00-\xff])(?=\x30|\x02)")


def identity(data):
    try:
        x509.load_der_x509_certificate(data)
        return "der-certificate"
    except (ValueError, UnsupportedAlgorithm):
        pass
    try:
        serialization.load_der_private_key(data, None)
        return "der-private-key"
    except (ValueError, TypeError, UnsupportedAlgorithm):
        pass
    try:
        serialization.load_der_public_key(data)
        return "der-public-key"
    except (ValueError, UnsupportedAlgorithm):
        return None


def scan(data, label):
    found = []
    errors = []
    try:
        if data[:2] == b"\x1f\x8b":
            return scan(gzip.decompress(data), label + "!gzip")
        if data[:4] == b"PK\x03\x04":
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for i in z.infolist():
                    if not i.is_dir():
                        a, b = scan(z.read(i), label + "!" + i.filename)
                        found += a
                        errors += b
            return found, errors
        if len(data) > 512 and data[257:262] == b"ustar":
            with tarfile.open(fileobj=io.BytesIO(data)) as t:
                for i in t.getmembers():
                    if i.isfile():
                        a, b = scan(t.extractfile(i).read(), label + "!" + i.name)
                        found += a
                        errors += b
            return found, errors
        for m in B64.finditer(data):
            try:
                raw = base64.b64decode(m.group(), validate=True)
            except (ValueError, UnsupportedAlgorithm):
                continue
            category = identity(raw)
            if category:
                found.append(
                    {
                        "path": label,
                        "encoding": "base64",
                        "category": category,
                        "offset": m.start(),
                        "length": len(m.group()),
                    }
                )
        if b"\x00" in data[:256] or data.startswith(b"\x7fELF"):
            for m in DER.finditer(data):
                n = 4 + int.from_bytes(m.group(1) + m.group(2), "big")
                raw = data[m.start() : m.start() + n]
                if len(raw) != n:
                    continue
                category = identity(raw)
                if category:
                    found.append(
                        {
                            "path": label,
                            "encoding": "DER",
                            "category": category,
                            "offset": m.start(),
                            "length": n,
                        }
                    )
    except Exception as e:
        errors.append(type(e).__name__ + ":" + label)
    return found, errors


def one(path):
    try:
        return scan((ROOT / path).read_bytes(), path)
    except Exception as e:
        return [], [type(e).__name__ + ":" + path]


if __name__ == "__main__":
    paths = [
        x.decode()
        for x in subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).split(
            b"\0"
        )
        if x
    ]
    found = []
    errors = []
    with concurrent.futures.ProcessPoolExecutor(max_workers=4) as pool:
        futures = [pool.submit(one, p) for p in paths]
        for n, f in enumerate(concurrent.futures.as_completed(futures), 1):
            a, b = f.result()
            found += a
            errors += b
            if n % 5000 == 0:
                print("DER scanned", n, "candidates", len(found), flush=True)
    Path("/tmp/tcl-scrub-der-candidates.json").write_text(
        json.dumps(
            {"files": len(paths), "candidates": found, "errors": errors}, indent=2
        )
        + "\n"
    )
    print(
        "DER scan complete",
        len(paths),
        "files",
        len(found),
        "candidates",
        len(errors),
        "errors",
        flush=True,
    )
