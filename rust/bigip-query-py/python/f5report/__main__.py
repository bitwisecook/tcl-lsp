"""Command-line entry point: ``f5-report`` / ``python -m f5report``.

Turns one or more BIG-IP configs or UCS archives into a single self-contained
HTML report, using the native query engine for every fact it shows.
"""

from __future__ import annotations

import argparse
import json
import sys

from . import __version__, engine_version, load_paths
from .report import build_report, collect_model


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="f5-report",
        description="Generate an interactive HTML report from F5 BIG-IP configs "
        "or UCS archives, powered by the f5-query engine (via PyO3).",
    )
    parser.add_argument("inputs", nargs="+", metavar="PATH",
                        help="bigip.conf / SCF files or .ucs archives (plain or encrypted)")
    parser.add_argument("-o", "--output", default="bigip-report.html",
                        help="output HTML path (default: %(default)s; '-' for stdout)")
    parser.add_argument("-t", "--title", default="F5 BIG-IP Configuration Report",
                        help="report title")
    parser.add_argument("--passphrase", default=None,
                        help="passphrase for an encrypted UCS (or set $F5_UCS_PASSPHRASE)")
    parser.add_argument("--f5mku", metavar="KEY", default=None,
                        help="base64 unit master key (f5mku -K) — decrypts the config's "
                        "$M$ secrets (SSL key passphrases, monitor/RADIUS/SNMP secrets)")
    parser.add_argument("--f5mku-file", metavar="FILE", default=None,
                        help="read the base64 master key from FILE (e.g. f5mku -K > key.txt)")
    parser.add_argument("--include-extras", action="store_true",
                        help="fold every config/*.conf UCS member (partitions, GTM) into the report")
    parser.add_argument("--no-console", action="store_true",
                        help="omit the in-browser WASM query console (smaller page; "
                        "suitable for hosting where a strict CSP blocks WebAssembly)")
    parser.add_argument("--json", action="store_true",
                        help="emit the report model as JSON instead of HTML")
    parser.add_argument("--version", action="version",
                        version=f"f5-report {__version__} (engine {engine_version()})")
    args = parser.parse_args(argv)

    master_key = args.f5mku
    if args.f5mku_file:
        try:
            master_key = open(args.f5mku_file, encoding="utf-8").read().strip()
        except OSError as exc:
            print(f"f5-report: could not read --f5mku-file: {exc}", file=sys.stderr)
            return 2

    try:
        sources = load_paths(
            args.inputs, passphrase=args.passphrase, include_extras=args.include_extras
        )
    except Exception as exc:  # noqa: BLE001 — surface a clean CLI error
        print(f"f5-report: failed to load inputs: {exc}", file=sys.stderr)
        return 2

    try:
        if args.json:
            out = json.dumps(
                collect_model(sources, title=args.title, master_key=master_key),
                indent=2, default=str,
            )
        else:
            out = build_report(sources, title=args.title,
                               embed_console=False if args.no_console else None,
                               master_key=master_key)
    except Exception as exc:  # noqa: BLE001 — surface a clean CLI error (e.g. wrong master key)
        print(f"f5-report: {exc}", file=sys.stderr)
        return 2

    if args.output == "-":
        sys.stdout.write(out)
    else:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(out)
        kind = "model" if args.json else "report"
        print(
            f"f5-report: wrote {kind} for {len(sources)} device(s) to {args.output} "
            f"({len(out):,} bytes)",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
