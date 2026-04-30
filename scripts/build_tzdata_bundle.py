"""Build the bundled tzdata blob compiled into the WASM runtime.

The bundle is a fallback for sandboxed environments that don't
preopen ``/usr/share/zoneinfo`` into the WASI sandbox — when the
host probe in :file:`runtime/zig/io/tcl_tz.zig` misses, the resolver
walks the in-memory bundle and uses whatever zones we baked in here.

Bundle binary format
--------------------

::

    magic       4 bytes   "TZBL"
    version     u8        1
    pad         3 bytes   0
    n_entries   u32 LE
    repeat n_entries:
        name_len   u8                # bytes (no NUL)
        name       N bytes           # zone name (e.g. "America/New_York")
        blob_off   u32 LE            # offset of TZif data from start of bundle
        blob_len   u32 LE            # bytes of TZif data
    pad to 4-byte boundary
    payload     concatenated TZif blobs

Names are stored sorted ascending so the resolver can binary-search
the index without a hash table.

Trimming policy
---------------

Today the script bundles the on-disk TZif files verbatim — modern
``/usr/share/zoneinfo`` files are already small (3-4 KB each, ~80
zones × 4 KB = ~320 KB total).  The decade ± 5 years trimmer
sketched in
``docs/design/compiler/wasm-runtime-primitives.md`` is the planned
follow-up; bundling untrimmed first lets us land the resolver
plumbing on a known-working blob and shrink it later without
re-litigating the header layout.

Curated zone list
-----------------

We bake in the IANA primary zones for the common Olson regions —
"common" defined as "regions with > 1 % of the world's population
or that the project's existing test fixtures reference".  The full
3000-zone catalogue would balloon the wasm binary; the curated set
covers virtually every script we've seen in practice and keeps the
bundle below ~150 KB.

Aliases (``US/Eastern`` → ``America/New_York``) are kept as
separate entries pointing at the same payload so callers can use
either spelling.
"""

from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path

# Curated zone list — keep small + diverse.  Each entry maps a zone
# name (used by callers like ``-timezone :America/New_York``) to
# the path under the host tzdata directory.  Aliases share a path.
_CURATED_ZONES: dict[str, str] = {
    # UTC family
    "UTC": "Etc/UTC",
    "GMT": "Etc/GMT",
    "Etc/UTC": "Etc/UTC",
    "Etc/GMT": "Etc/GMT",
    # Etc fixed-offset zones for ``-timezone Etc/GMT+5`` style use.
    "Etc/GMT-12": "Etc/GMT-12",
    "Etc/GMT-11": "Etc/GMT-11",
    "Etc/GMT-10": "Etc/GMT-10",
    "Etc/GMT-9": "Etc/GMT-9",
    "Etc/GMT-8": "Etc/GMT-8",
    "Etc/GMT-7": "Etc/GMT-7",
    "Etc/GMT-6": "Etc/GMT-6",
    "Etc/GMT-5": "Etc/GMT-5",
    "Etc/GMT-4": "Etc/GMT-4",
    "Etc/GMT-3": "Etc/GMT-3",
    "Etc/GMT-2": "Etc/GMT-2",
    "Etc/GMT-1": "Etc/GMT-1",
    "Etc/GMT+1": "Etc/GMT+1",
    "Etc/GMT+2": "Etc/GMT+2",
    "Etc/GMT+3": "Etc/GMT+3",
    "Etc/GMT+4": "Etc/GMT+4",
    "Etc/GMT+5": "Etc/GMT+5",
    "Etc/GMT+6": "Etc/GMT+6",
    "Etc/GMT+7": "Etc/GMT+7",
    "Etc/GMT+8": "Etc/GMT+8",
    "Etc/GMT+9": "Etc/GMT+9",
    "Etc/GMT+10": "Etc/GMT+10",
    "Etc/GMT+11": "Etc/GMT+11",
    "Etc/GMT+12": "Etc/GMT+12",
    # Americas
    "America/New_York": "America/New_York",
    "America/Chicago": "America/Chicago",
    "America/Denver": "America/Denver",
    "America/Los_Angeles": "America/Los_Angeles",
    "America/Phoenix": "America/Phoenix",
    "America/Anchorage": "America/Anchorage",
    "America/Halifax": "America/Halifax",
    "America/St_Johns": "America/St_Johns",
    "America/Toronto": "America/Toronto",
    "America/Vancouver": "America/Vancouver",
    "America/Mexico_City": "America/Mexico_City",
    "America/Sao_Paulo": "America/Sao_Paulo",
    "America/Buenos_Aires": "America/Argentina/Buenos_Aires",
    "America/Argentina/Buenos_Aires": "America/Argentina/Buenos_Aires",
    "America/Santiago": "America/Santiago",
    "America/Bogota": "America/Bogota",
    "America/Lima": "America/Lima",
    "America/Caracas": "America/Caracas",
    # Europe
    "Europe/London": "Europe/London",
    "Europe/Paris": "Europe/Paris",
    "Europe/Berlin": "Europe/Berlin",
    "Europe/Amsterdam": "Europe/Amsterdam",
    "Europe/Brussels": "Europe/Brussels",
    "Europe/Madrid": "Europe/Madrid",
    "Europe/Rome": "Europe/Rome",
    "Europe/Vienna": "Europe/Vienna",
    "Europe/Zurich": "Europe/Zurich",
    "Europe/Dublin": "Europe/Dublin",
    "Europe/Lisbon": "Europe/Lisbon",
    "Europe/Stockholm": "Europe/Stockholm",
    "Europe/Oslo": "Europe/Oslo",
    "Europe/Copenhagen": "Europe/Copenhagen",
    "Europe/Helsinki": "Europe/Helsinki",
    "Europe/Warsaw": "Europe/Warsaw",
    "Europe/Prague": "Europe/Prague",
    "Europe/Budapest": "Europe/Budapest",
    "Europe/Athens": "Europe/Athens",
    "Europe/Istanbul": "Europe/Istanbul",
    "Europe/Moscow": "Europe/Moscow",
    "Europe/Kiev": "Europe/Kyiv",
    "Europe/Kyiv": "Europe/Kyiv",
    # Africa
    "Africa/Cairo": "Africa/Cairo",
    "Africa/Johannesburg": "Africa/Johannesburg",
    "Africa/Lagos": "Africa/Lagos",
    "Africa/Nairobi": "Africa/Nairobi",
    "Africa/Casablanca": "Africa/Casablanca",
    # Asia
    "Asia/Tokyo": "Asia/Tokyo",
    "Asia/Shanghai": "Asia/Shanghai",
    "Asia/Hong_Kong": "Asia/Hong_Kong",
    "Asia/Singapore": "Asia/Singapore",
    "Asia/Seoul": "Asia/Seoul",
    "Asia/Taipei": "Asia/Taipei",
    "Asia/Bangkok": "Asia/Bangkok",
    "Asia/Jakarta": "Asia/Jakarta",
    "Asia/Manila": "Asia/Manila",
    "Asia/Kolkata": "Asia/Kolkata",
    "Asia/Calcutta": "Asia/Kolkata",
    "Asia/Karachi": "Asia/Karachi",
    "Asia/Dhaka": "Asia/Dhaka",
    "Asia/Dubai": "Asia/Dubai",
    "Asia/Riyadh": "Asia/Riyadh",
    "Asia/Tehran": "Asia/Tehran",
    "Asia/Jerusalem": "Asia/Jerusalem",
    "Asia/Tel_Aviv": "Asia/Jerusalem",
    "Asia/Baghdad": "Asia/Baghdad",
    # Pacific / Oceania
    "Australia/Sydney": "Australia/Sydney",
    "Australia/Melbourne": "Australia/Melbourne",
    "Australia/Brisbane": "Australia/Brisbane",
    "Australia/Perth": "Australia/Perth",
    "Australia/Adelaide": "Australia/Adelaide",
    "Pacific/Auckland": "Pacific/Auckland",
    "Pacific/Honolulu": "Pacific/Honolulu",
    "Pacific/Fiji": "Pacific/Fiji",
    "Pacific/Guam": "Pacific/Guam",
    # POSIX-style aliases — Tcl scripts and tcllib hand these around.
    "US/Eastern": "America/New_York",
    "US/Central": "America/Chicago",
    "US/Mountain": "America/Denver",
    "US/Pacific": "America/Los_Angeles",
    "US/Hawaii": "Pacific/Honolulu",
    "US/Alaska": "America/Anchorage",
    "EST": "EST",
    "EST5EDT": "EST5EDT",
    "CST6CDT": "CST6CDT",
    "MST": "MST",
    "MST7MDT": "MST7MDT",
    "PST8PDT": "PST8PDT",
    "HST": "HST",
}

_MAGIC = b"TZBL"
_VERSION = 1


def build_bundle(zoneinfo: Path) -> bytes:
    """Read the curated zones from *zoneinfo* and pack into a bundle blob."""
    if not zoneinfo.is_dir():
        msg = f"{zoneinfo} is not a directory; pass --zoneinfo /usr/share/zoneinfo"
        raise SystemExit(msg)

    # Deduplicate payloads — multiple aliases share one TZif blob.
    payloads: dict[str, bytes] = {}  # rel-path -> bytes
    entries: list[tuple[str, str]] = []  # (alias, rel-path)
    skipped: list[str] = []

    for alias, rel in _CURATED_ZONES.items():
        path = zoneinfo / rel
        if not path.is_file():
            skipped.append(f"{alias} → {rel}")
            continue
        if rel not in payloads:
            payloads[rel] = path.read_bytes()
        entries.append((alias, rel))

    if skipped:
        sys.stderr.write(
            "build_tzdata_bundle: skipped (not present on host):\n  "
            + "\n  ".join(skipped)
            + "\n"
        )

    if not entries:
        msg = f"no zones resolved under {zoneinfo}; tzdata package missing?"
        raise SystemExit(msg)

    # Sort entries by alias name (binary search in the resolver wants
    # an ordered index).
    entries.sort(key=lambda e: e[0])

    # First pass: layout payload addresses.
    payload_order: list[str] = []
    payload_offsets: dict[str, int] = {}
    payload_block = bytearray()
    for _alias, rel in entries:
        if rel in payload_offsets:
            continue
        payload_offsets[rel] = len(payload_block)
        payload_order.append(rel)
        payload_block.extend(payloads[rel])

    # Index entries: serialise to compute the index length, then we
    # know where the payload starts.
    index_records: list[bytes] = []
    for alias, rel in entries:
        name_bytes = alias.encode("utf-8")
        if len(name_bytes) > 255:
            msg = f"zone alias too long: {alias!r}"
            raise SystemExit(msg)
        # Placeholder offsets — fixed up below once the index size is known.
        index_records.append(
            bytes([len(name_bytes)])
            + name_bytes
            + struct.pack("<I", 0)  # blob_off — fixed below
            + struct.pack("<I", len(payloads[rel]))
        )
    index_size = sum(len(r) for r in index_records)

    # Header is 12 bytes: 4 magic + 1 version + 3 pad + 4 n_entries.
    header_size = 12
    payload_start = header_size + index_size
    pad = (-payload_start) & 3
    payload_start += pad

    # Re-emit index records with correct offsets.
    final_index = bytearray()
    for (alias, rel), rec in zip(entries, index_records, strict=True):
        name_bytes = alias.encode("utf-8")
        blob_off = payload_start + payload_offsets[rel]
        blob_len = len(payloads[rel])
        final_index.extend(
            bytes([len(name_bytes)])
            + name_bytes
            + struct.pack("<I", blob_off)
            + struct.pack("<I", blob_len)
        )
        _ = rec

    out = bytearray()
    out.extend(_MAGIC)
    out.append(_VERSION)
    out.extend(b"\x00\x00\x00")
    out.extend(struct.pack("<I", len(entries)))
    out.extend(final_index)
    out.extend(b"\x00" * pad)
    out.extend(payload_block)
    return bytes(out)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument(
        "--zoneinfo",
        type=Path,
        default=Path("/usr/share/zoneinfo"),
        help="path to a host zoneinfo directory (default: /usr/share/zoneinfo)",
    )
    p.add_argument(
        "--output",
        type=Path,
        required=True,
        help="path to write the packed bundle (e.g. runtime/zig/data/tzdata.bin)",
    )
    args = p.parse_args()
    blob = build_bundle(args.zoneinfo)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(blob)
    sys.stderr.write(
        f"build_tzdata_bundle: wrote {len(blob):,} bytes "
        f"({_count_entries(blob)} zones) to {args.output}\n"
    )


def _count_entries(blob: bytes) -> int:
    if blob[:4] != _MAGIC:
        return 0
    return struct.unpack_from("<I", blob, 8)[0]


if __name__ == "__main__":
    main()
