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

By default the script bundles the on-disk TZif files verbatim —
modern ``/usr/share/zoneinfo`` files are already small (3-4 KB
each, ~80 zones × 4 KB = ~320 KB total).  Pass ``--trim-from`` /
``--trim-to`` (Unix epoch seconds) to enable the decade ± 5 years
trimmer sketched in
``docs/design/compiler/wasm-runtime-primitives.md``.  The trimmer
drops transitions whose timestamp falls outside the window while
preserving at least the last pre-window transition so
:func:`offset_at` lookups for in-window dates resolve to the
correct rule.  Leap-second tables are dropped unconditionally
(POSIX time pretends they don't exist, and Tcl follows).

The trimmer only touches the v1 body — the v2/v3 64-bit body and
TZ-string footer are stripped entirely.  ``offset_at`` only needs
the historical transition list, and the v1 record set covers
1901-12-13 to 2038-01-19 in 32-bit-signed seconds (a comfortable
superset of any decade window we'd realistically pick).

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
    "America/Detroit": "America/Detroit",
    "America/Indianapolis": "America/Indiana/Indianapolis",
    "America/Indiana/Indianapolis": "America/Indiana/Indianapolis",
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

_TZIF_MAGIC = b"TZif"


def _trim_tzif(blob: bytes, trim_from: int | None, trim_to: int | None) -> bytes:
    """Drop transitions outside ``[trim_from, trim_to]`` from a TZif blob.

    Returns a re-packed v1-only TZif blob.  Leap-second tables are
    stripped unconditionally.  When the input is malformed (bad magic,
    truncated header) the original bytes are returned unchanged so
    we never produce an unparseable output — the resolver's existing
    "unknown TZif → fall back to UTC" path stays sound.

    The trimmer keeps at least one pre-window transition (the last
    one whose timestamp is ``< trim_from``) so callers querying a
    timestamp at the very start of the window still see the correct
    historical rule rather than the type-0 default.  Symmetric
    treatment on the upper end isn't necessary — the post-window
    transitions only matter if the caller is reading a date *after*
    ``trim_to``, which is what the bound is meant to forbid.
    """
    if trim_from is None and trim_to is None:
        return blob
    if not blob.startswith(_TZIF_MAGIC) or len(blob) < 44:
        return blob

    # v1 header: magic(4) + version(1) + reserved(15) + 6×u32be counts.
    counts = struct.unpack_from(">6I", blob, 20)
    ttisutcnt, ttisstdcnt, leapcnt, timecnt, typecnt, charcnt = counts
    pos = 44
    times_raw = blob[pos : pos + 4 * timecnt]
    pos += 4 * timecnt
    type_idx_raw = blob[pos : pos + timecnt]
    pos += timecnt
    ttinfo_raw = blob[pos : pos + 6 * typecnt]
    pos += 6 * typecnt
    abbr_raw = blob[pos : pos + charcnt]
    pos += charcnt
    # Leap / std / utc tables follow; we drop the leap section
    # entirely and preserve std/utc bits because they classify the
    # ttinfo entries (which we keep).
    pos += 8 * leapcnt
    std_raw = blob[pos : pos + ttisstdcnt]
    pos += ttisstdcnt
    utc_raw = blob[pos : pos + ttisutcnt]
    pos += ttisutcnt
    if pos > len(blob):
        return blob  # truncated — bail.

    times = list(struct.unpack(f">{timecnt}i", times_raw)) if timecnt else []
    type_indices = list(type_idx_raw) if timecnt else []

    keep_times: list[int] = []
    keep_indices: list[int] = []
    last_pre_idx: int | None = None
    last_pre_t: int | None = None
    for t, ti in zip(times, type_indices, strict=True):
        if trim_from is not None and t < trim_from:
            last_pre_idx = ti
            last_pre_t = t
            continue
        if trim_to is not None and t > trim_to:
            continue
        keep_times.append(t)
        keep_indices.append(ti)

    # Anchor the start with the last dropped pre-window transition so
    # ``offset_at(t == trim_from)`` resolves to the correct rule.
    if last_pre_idx is not None and (not keep_times or keep_times[0] != last_pre_t):
        keep_times.insert(0, last_pre_t)  # type: ignore[arg-type]
        keep_indices.insert(0, last_pre_idx)

    # Re-emit a v1-only TZif blob.  Header counts: leapcnt = 0,
    # timecnt = trimmed; typecnt / charcnt / ttisutcnt / ttisstdcnt
    # carry through unchanged so the ttinfo entries / abbr table
    # don't need re-numbering.
    new_timecnt = len(keep_times)
    out = bytearray()
    out.extend(_TZIF_MAGIC)
    out.append(0)  # version 0 (v1).
    out.extend(b"\x00" * 15)
    out.extend(struct.pack(">6I", ttisutcnt, ttisstdcnt, 0, new_timecnt, typecnt, charcnt))
    if new_timecnt:
        out.extend(struct.pack(f">{new_timecnt}i", *keep_times))
        out.extend(bytes(keep_indices))
    out.extend(ttinfo_raw)
    out.extend(abbr_raw)
    # leap section omitted (leapcnt = 0).
    out.extend(std_raw)
    out.extend(utc_raw)
    return bytes(out)


def build_bundle(
    zoneinfo: Path,
    *,
    trim_from: int | None = None,
    trim_to: int | None = None,
) -> bytes:
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
            payloads[rel] = _trim_tzif(path.read_bytes(), trim_from, trim_to)
        entries.append((alias, rel))

    if skipped:
        sys.stderr.write(
            "build_tzdata_bundle: skipped (not present on host):\n  " + "\n  ".join(skipped) + "\n"
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
    p.add_argument(
        "--trim-from",
        type=int,
        default=None,
        metavar="EPOCH",
        help=(
            "drop TZif transitions strictly before this Unix epoch "
            "second (default: keep all).  The last pre-window "
            "transition is preserved so offset_at() at the lower "
            "bound still resolves to the correct historical rule."
        ),
    )
    p.add_argument(
        "--trim-to",
        type=int,
        default=None,
        metavar="EPOCH",
        help="drop TZif transitions strictly after this Unix epoch second",
    )
    args = p.parse_args()
    blob = build_bundle(
        args.zoneinfo,
        trim_from=args.trim_from,
        trim_to=args.trim_to,
    )
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
