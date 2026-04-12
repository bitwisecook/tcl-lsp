#!/usr/bin/env python3
"""Generate self-signed certificates for BigIP event-ordering tests.

Generates a CA plus two signed certificates:

- **clientssl**: For the BIG-IP clientssl profile (client → BIG-IP TLS).
- **serverssl**: For pool member HTTPS servers (BIG-IP → server TLS).

All certs are RSA-2048, valid for 365 days.

Usage::

    python cert_gen.py [--output-dir certs]
"""

from __future__ import annotations

import argparse
import os
import subprocess
from dataclasses import dataclass


@dataclass
class CertBundle:
    """Paths to generated certificate files."""

    ca_cert: str
    ca_key: str
    clientssl_cert: str
    clientssl_key: str
    serverssl_cert: str
    serverssl_key: str


def generate_certs(
    output_dir: str,
    *,
    cn_prefix: str = "evt-order",
) -> CertBundle:
    """Generate CA + clientssl + serverssl certificates in *output_dir*.

    Returns a :class:`CertBundle` with absolute paths to all files.
    """
    os.makedirs(output_dir, exist_ok=True)

    bundle = CertBundle(
        ca_cert=os.path.join(output_dir, "ca.crt"),
        ca_key=os.path.join(output_dir, "ca.key"),
        clientssl_cert=os.path.join(output_dir, "clientssl.crt"),
        clientssl_key=os.path.join(output_dir, "clientssl.key"),
        serverssl_cert=os.path.join(output_dir, "serverssl.crt"),
        serverssl_key=os.path.join(output_dir, "serverssl.key"),
    )

    # 1. Self-signed CA
    _openssl_self_signed(
        bundle.ca_key,
        bundle.ca_cert,
        f"{cn_prefix}-ca",
    )

    # 2. ClientSSL cert signed by CA
    _openssl_signed(
        bundle.ca_cert,
        bundle.ca_key,
        bundle.clientssl_key,
        bundle.clientssl_cert,
        f"{cn_prefix}-clientssl",
    )

    # 3. ServerSSL cert signed by CA
    _openssl_signed(
        bundle.ca_cert,
        bundle.ca_key,
        bundle.serverssl_key,
        bundle.serverssl_cert,
        f"{cn_prefix}-serverssl",
    )

    return bundle


def _openssl_self_signed(
    key_path: str,
    cert_path: str,
    cn: str,
) -> None:
    """Generate a self-signed CA certificate."""
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key_path,
            "-out",
            cert_path,
            "-days",
            "365",
            "-nodes",
            "-subj",
            f"/CN={cn}",
        ],
        check=True,
        capture_output=True,
    )


def _openssl_signed(
    ca_cert: str,
    ca_key: str,
    key_path: str,
    cert_path: str,
    cn: str,
) -> None:
    """Generate a certificate signed by the CA."""
    csr_path = cert_path + ".csr"
    try:
        # Generate key + CSR
        subprocess.run(
            [
                "openssl",
                "req",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                key_path,
                "-out",
                csr_path,
                "-subj",
                f"/CN={cn}",
            ],
            check=True,
            capture_output=True,
        )
        # Sign with CA
        subprocess.run(
            [
                "openssl",
                "x509",
                "-req",
                "-in",
                csr_path,
                "-CA",
                ca_cert,
                "-CAkey",
                ca_key,
                "-CAcreateserial",
                "-out",
                cert_path,
                "-days",
                "365",
            ],
            check=True,
            capture_output=True,
        )
    finally:
        # Clean up CSR
        if os.path.exists(csr_path):
            os.unlink(csr_path)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate self-signed certificates for BigIP tests",
    )
    parser.add_argument(
        "--output-dir",
        "-o",
        default="certs",
        help="Directory for generated certificates (default: certs)",
    )
    parser.add_argument(
        "--cn-prefix",
        default="evt-order",
        help="Common name prefix (default: evt-order)",
    )
    args = parser.parse_args()

    bundle = generate_certs(args.output_dir, cn_prefix=args.cn_prefix)
    print(f"Generated certificates in {args.output_dir}/:")
    print(f"  CA:        {bundle.ca_cert}")
    print(f"  ClientSSL: {bundle.clientssl_cert}")
    print(f"  ServerSSL: {bundle.serverssl_cert}")


if __name__ == "__main__":
    main()
