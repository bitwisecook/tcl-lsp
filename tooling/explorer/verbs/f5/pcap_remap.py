"""``f5 pcap-remap`` — apply a redaction map to a PCAP capture."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.f5_trailer import load_schema_overlay, schema_summary
from dialects.f5.bigip.pcap_remap import UnknownTrailerError, remap_pcap
from dialects.f5.bigip.redact_map import RedactionMap

from ._registry import verb


@verb(
    "pcap-remap",
    aliases=("pcapmap",),
    help="Apply a `f5 redact` map to a PCAP capture (rewrites IP layer + parsed F5 trailer).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.epilog = (
        "Examples:\n"
        "  f5 pcap-remap redact.map.toml capture.pcap remapped.pcap\n"
        "  f5 pcap-remap redact.map.toml redacted.pcap original.pcap --reverse\n"
        "  f5 pcap-remap redact.map.toml in.pcap out.pcap --on-unknown sweep\n"
        "  f5 pcap-remap --list-schemas\n"
    )
    p.description = (
        "Rewrite every IPv4 / IPv6 source and destination address in a "
        "PCAP capture using the same TOML map produced by `f5 redact`. "
        "IPv4 header and TCP / UDP / ICMP / ICMPv6 checksums are "
        "recomputed.  In addition to the IP layer, the F5 Ethernet "
        "trailer (legacy and DPT formats; what `tcpdump -i 0.0:nnnp` "
        "adds) is parsed via dialects.f5.bigip.f5_trailer and each peer-IP "
        "field is rewritten at its schema-known offset.  L4 payload "
        "bytes are NOT scanned.  Use --reverse to apply the map in the "
        "opposite direction."
    )
    p.add_argument(
        "map_file",
        help="The TOML map file produced by `f5 redact`.",
    )
    p.add_argument("input", help="Input PCAP file.")
    p.add_argument("output", help="Output PCAP file.")
    p.add_argument(
        "--reverse",
        action="store_true",
        help="Apply the map in reverse (recover originals from a redacted capture).",
    )
    p.add_argument(
        "--on-unknown",
        choices=("error", "preserve", "sweep"),
        default="error",
        help=(
            "What to do when an F5 trailer TLV's (type, version) has no "
            "registered IP-field schema.  error (default): raise and "
            "refuse to write the output.  preserve: leave that TLV "
            "unchanged; report the count.  sweep: byte-replace known "
            "IPs within just that TLV's data section."
        ),
    )
    p.add_argument(
        "--schema",
        action="append",
        default=[],
        metavar="FILE",
        help=(
            "Additional F5 trailer schema TOML to overlay on top of the "
            "built-ins (repeatable).  See dialects/f5/bigip/f5_trailer.py for "
            "the format."
        ),
    )
    p.add_argument(
        "--list-schemas",
        action="store_true",
        help="Print every registered trailer schema and exit.",
    )
    p.set_defaults(handler=_run_pcap_remap)


def _run_pcap_remap(args: argparse.Namespace) -> int:
    for overlay in args.schema:
        try:
            load_schema_overlay(Path(overlay).read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            print(f"error: cannot load schema {overlay}: {exc}", file=sys.stderr)
            return 2

    if args.list_schemas:
        summary = schema_summary()
        for fmt, entries in summary.items():
            print(f"{fmt}:")
            for line in entries:
                print(f"  {line}")
        return 0

    try:
        map_text = Path(args.map_file).read_text(encoding="utf-8")
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        rm = RedactionMap.from_toml(map_text)
    except (ValueError, RuntimeError) as exc:
        print(f"error: cannot load map: {exc}", file=sys.stderr)
        return 2

    try:
        with open(args.input, "rb") as in_fh, open(args.output, "wb") as out_fh:
            result = remap_pcap(
                in_fh,
                out_fh,
                rm,
                reverse=args.reverse,
                unknown_policy=args.on_unknown,
            )
    except UnknownTrailerError as exc:
        # Output file may be partially written; remove so the caller
        # doesn't accidentally consume a half-redacted capture.
        try:
            Path(args.output).unlink(missing_ok=True)
        except OSError:
            pass
        print(
            f"error: {exc}\n"
            f"  -> rerun with --on-unknown=preserve|sweep, or supply a "
            f"matching --schema file to add the missing layout.",
            file=sys.stderr,
        )
        return 2
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(
        f"pcap-remap: {result.packets_rewritten}/{result.packets_total} packet(s) rewritten, "
        f"{result.addresses_rewritten} address(es) changed; "
        f"trailer TLVs: {result.trailer_tlvs_rewritten} rewritten / "
        f"{result.trailer_tlvs_unknown} unknown / "
        f"{result.trailer_tlvs_total} total",
        file=sys.stderr,
    )
    return 0
