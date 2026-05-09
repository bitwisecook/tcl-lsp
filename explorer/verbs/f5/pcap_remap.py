"""``f5 pcap-remap`` — apply a redaction map to a PCAP capture."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.pcap_remap import remap_pcap
from core.bigip.redact_map import RedactionMap

from ._registry import verb


@verb(
    "pcap-remap",
    aliases=("pcapmap",),
    help="Apply a `f5 redact` map to a PCAP capture (rewrites IP layer + F5 trailer).",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Rewrite every IPv4 / IPv6 source and destination address in a "
        "PCAP capture using the same TOML map produced by `f5 redact`. "
        "IPv4 header and TCP / UDP / ICMP / ICMPv6 checksums are "
        "recomputed.  In addition to the IP layer, any 4-byte (or "
        "16-byte) sequence in the F5 HSB trailer (everything past "
        "`IP total length` — what `tcpdump -i 0.0:nnnp` adds) that "
        "matches a known real IP from the map is rewritten in place. "
        "Replacements are length-preserving so trailer TLV structures "
        "stay valid.  Application payload bytes are NOT scanned. "
        "Use --reverse to apply the map in the opposite direction "
        "(redacted -> original)."
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
    p.set_defaults(handler=_run_pcap_remap)


def _run_pcap_remap(args: argparse.Namespace) -> int:
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
            result = remap_pcap(in_fh, out_fh, rm, reverse=args.reverse)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(
        f"pcap-remap: {result.packets_rewritten}/{result.packets_total} packet(s) rewritten, "
        f"{result.addresses_rewritten} address occurrence(s) changed",
        file=sys.stderr,
    )
    return 0
