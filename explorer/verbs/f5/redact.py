"""``f5 redact`` — strip secrets and remap real IPs for sharing."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.rewrite import redact_secrets

from ._paths import read_path
from ._registry import verb


@verb(
    "redact",
    aliases=("sanitize",),
    help="Replace passwords/PEM blocks/community strings and remap public IPs to RFC1918.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Produce a copy of the bigip.conf / SCF safe to share externally: "
        "strip values from secret-bearing keys (passphrase, password, "
        "secret, community, encrypted-password, ...), replace PEM-encoded "
        "certs/keys with `<REDACTED>`, and consistently remap any public "
        "IPv4 addresses into the 10.0.0.0/8 range so cross-references "
        "remain valid."
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument(
        "-o", "--output", metavar="FILE", help="Write redacted config here (default: stdout)."
    )
    p.add_argument(
        "--keep-ips",
        action="store_true",
        help="Preserve original IP addresses (only redact secrets and PEM blocks).",
    )
    p.set_defaults(handler=_run_redact)


def _run_redact(args: argparse.Namespace) -> int:
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    report = redact_secrets(source, remap_ips=not args.keep_ips)

    if args.output:
        Path(args.output).write_text(report.new_source, encoding="utf-8")
    else:
        sys.stdout.write(report.new_source)

    print(
        f"redacted: {report.secrets_replaced} secret(s), "
        f"{report.pem_blocks_replaced} PEM block(s), "
        f"{report.ips_remapped} IP(s) remapped",
        file=sys.stderr,
    )
    return 0
