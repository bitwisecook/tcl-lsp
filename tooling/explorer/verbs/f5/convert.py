"""``f5 convert`` — UCS↔SCF and SCF→AS3 conversions."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from dialects.f5.bigip.convert import scf_to_as3
from dialects.f5.bigip.parser import parse_bigip_conf
from tooling.explorer.f5_remote.ucs import is_ucs_bytes, ucs_to_scf

from ._paths import read_path
from ._registry import verb


@verb(
    "convert",
    aliases=(),
    help="Convert between UCS / SCF / AS3 declaration formats.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Format conversion between BIG-IP serialisation formats.\n"
        "  ucs2scf  unpack a UCS archive (same shape as `f5 extract`).\n"
        "  scf2as3  convert a bigip.conf to an AS3 (Application Services 3)\n"
        "           declaration.  AS3 conversion only covers common LTM\n"
        "           objects; unmapped objects are listed in the coverage\n"
        "           report so nothing silently disappears."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 convert ucs2scf prod.ucs -o prod.scf\n"
        "  f5 convert scf2as3 bigip.conf -o app.as3.json\n"
        "  f5 convert scf2as3 bigip.conf --tenant Prod --application web\n"
        "  f5 convert scf2as3 bigip.conf --report   # coverage report on stderr\n"
    )
    p.add_argument(
        "format",
        choices=("ucs2scf", "scf2as3"),
        help="Conversion to perform.",
    )
    p.add_argument("path", help="Input file (`-` for stdin; UCS must be a real file).")
    p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    p.add_argument(
        "--tenant",
        default="Common",
        help="(scf2as3) Tenant name in the AS3 declaration (default: Common).",
    )
    p.add_argument(
        "--application",
        default="app",
        help="(scf2as3) Application name in the AS3 declaration (default: app).",
    )
    p.add_argument(
        "--report",
        action="store_true",
        help="(scf2as3) Print a coverage report on stderr listing unmapped objects.",
    )
    p.set_defaults(handler=_run_convert)


def _run_convert(args: argparse.Namespace) -> int:
    if args.format == "ucs2scf":
        return _run_ucs2scf(args)
    if args.format == "scf2as3":
        return _run_scf2as3(args)
    print(f"error: unknown format {args.format!r}", file=sys.stderr)
    return 2


def _run_ucs2scf(args: argparse.Namespace) -> int:
    if args.path == "-":
        print("error: UCS input must be a file path, not stdin", file=sys.stderr)
        return 2
    try:
        data = Path(args.path).read_bytes()
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if not is_ucs_bytes(data):
        print(f"error: {args.path}: not a UCS archive", file=sys.stderr)
        return 2
    text = ucs_to_scf(data)
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    return 0


def _run_scf2as3(args: argparse.Namespace) -> int:
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    cfg = parse_bigip_conf(source)
    report = scf_to_as3(cfg, tenant=args.tenant, application=args.application)
    output = json.dumps(report.declaration, indent=2) + "\n"
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    if args.report:
        print(
            f"as3: mapped {report.mapped} object(s); {len(report.unmapped)} unmapped",
            file=sys.stderr,
        )
        for entry in report.unmapped:
            print(f"  unmapped: {entry}", file=sys.stderr)
    return 0
