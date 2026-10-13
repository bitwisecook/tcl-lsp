# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Create verified certificate UCS fixtures outside the checkout using OpenSSL."""

import argparse
import io
import json
import subprocess
import tarfile
import tempfile
from pathlib import Path


def openssl(*arguments):
    result = subprocess.run(
        ["openssl", *map(str, arguments)], capture_output=True, check=True
    )
    return result.stdout.decode()


def generate(template, output, chain):
    with tempfile.TemporaryDirectory(prefix="tcl-ucs-keys-") as directory:
        work = Path(directory)
        names = ["root", "intermediate", "leaf"] if chain else ["leaf"]
        for name in names:
            openssl(
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
                work / f"{name}.key",
            )
            openssl("pkey", "-in", work / f"{name}.key", "-check", "-noout")
        if chain:
            openssl(
                "req",
                "-new",
                "-x509",
                "-key",
                work / "root.key",
                "-out",
                work / "root.crt",
                "-days",
                "2",
                "-sha256",
                "-subj",
                "/CN=Acme Root CA",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-addext",
                "keyUsage=critical,keyCertSign,cRLSign",
            )
            openssl(
                "req",
                "-new",
                "-key",
                work / "intermediate.key",
                "-out",
                work / "intermediate.csr",
                "-subj",
                "/CN=Acme Intermediate CA",
            )
            (work / "intermediate.ext").write_text(
                "basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\n"
            )
            openssl(
                "x509",
                "-req",
                "-in",
                work / "intermediate.csr",
                "-CA",
                work / "root.crt",
                "-CAkey",
                work / "root.key",
                "-set_serial",
                "2",
                "-days",
                "2",
                "-sha256",
                "-extfile",
                work / "intermediate.ext",
                "-out",
                work / "intermediate.crt",
            )
            openssl(
                "req",
                "-new",
                "-key",
                work / "leaf.key",
                "-out",
                work / "leaf.csr",
                "-subj",
                "/CN=www.acme.example",
            )
            (work / "index").touch()
            (work / "serial").write_text("03\n")
            (work / "ca.conf").write_text(
                f"[ca]\ndefault_ca=fixture\n[fixture]\ndatabase={work}/index\nserial={work}/serial\nnew_certs_dir={work}\ndefault_md=sha256\npolicy=policy\nx509_extensions=leaf\n[policy]\ncommonName=supplied\n[leaf]\nbasicConstraints=critical,CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:www.acme.example,DNS:api.acme.example,IP:192.0.2.5\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\n"
            )
            openssl(
                "ca",
                "-config",
                work / "ca.conf",
                "-cert",
                work / "intermediate.crt",
                "-keyfile",
                work / "intermediate.key",
                "-in",
                work / "leaf.csr",
                "-out",
                work / "leaf.crt",
                "-batch",
                "-notext",
                "-startdate",
                "20200101000000Z",
                "-enddate",
                "20200102000000Z",
            )
            openssl(
                "verify",
                "-no_check_time",
                "-CAfile",
                work / "root.crt",
                "-untrusted",
                work / "intermediate.crt",
                work / "leaf.crt",
            )
            expired = subprocess.run(
                [
                    "openssl",
                    "verify",
                    "-CAfile",
                    str(work / "root.crt"),
                    "-untrusted",
                    str(work / "intermediate.crt"),
                    str(work / "leaf.crt"),
                ],
                capture_output=True,
            )
            assert expired.returncode != 0 and b"expired" in expired.stderr, (
                "expired leaf verification boundary"
            )
            replacements = {
                "config/filestore/files_d/Common_d/certificate_d/:Common:www.acme.example.crt_1": (
                    work / "leaf.crt"
                ).read_bytes(),
                "config/filestore/files_d/Common_d/certificate_d/:Common:acme-chain.crt_1": (
                    work / "intermediate.crt"
                ).read_bytes()
                + (work / "root.crt").read_bytes(),
                "config/filestore/files_d/Common_d/certificate_key_d/:Common:www.acme.example.key_1": (
                    work / "leaf.key"
                ).read_bytes(),
            }
        else:
            openssl(
                "req",
                "-new",
                "-x509",
                "-key",
                work / "leaf.key",
                "-out",
                work / "leaf.crt",
                "-days",
                "2",
                "-sha256",
                "-subj",
                "/CN=example.test",
            )
            openssl("verify", "-CAfile", work / "leaf.crt", work / "leaf.crt")
            replacements = {
                "config/filestore/files_d/Common_d/certificate_d/:Common:t.crt_1": (
                    work / "leaf.crt"
                ).read_bytes()
            }
        certificate_public = openssl(
            "x509", "-in", work / "leaf.crt", "-pubkey", "-noout"
        )
        key_public = openssl("pkey", "-in", work / "leaf.key", "-pubout")
        assert certificate_public == key_public, (
            "generated certificate/key correspondence"
        )
        with tarfile.open(template) as source, tarfile.open(output, "w:gz") as target:
            for member in source.getmembers():
                if member.isfile():
                    data = replacements.get(member.name)
                    if data is None:
                        data = source.extractfile(member).read()
                    member.size = len(data)
                    target.addfile(member, io.BytesIO(data))
                else:
                    target.addfile(member)
        fingerprint = (
            openssl(
                "x509", "-in", work / "leaf.crt", "-noout", "-fingerprint", "-sha256"
            )
            .split("=", 1)[1]
            .strip()
            .replace(":", "")
        )
        return {"fingerprint_sha256": fingerprint}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("template", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--chain", action="store_true")
    args = parser.parse_args()
    print(json.dumps(generate(args.template, args.output, args.chain)))
