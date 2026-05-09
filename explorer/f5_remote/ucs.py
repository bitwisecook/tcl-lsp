"""UCS (BIG-IP backup archive) handling.

A UCS file is a gzip-compressed tar containing a snapshot of
``/config``.  This module provides the inverse of ``tmsh load sys ucs``:
extract one or more UCS archives and reassemble a single SCF (Single
Configuration File) text by concatenating the relevant ``bigip*.conf``
members in a deterministic order.
"""

from __future__ import annotations

import io
import tarfile
from pathlib import Path

# Order matters: base must come first so partition declarations exist
# before objects that reference them.
_SCF_MEMBER_ORDER = (
    "config/bigip_base.conf",
    "config/bigip.conf",
    "config/bigip_gtm.conf",
    "config/bigip_user.conf",
    "config/bigip_script.conf",
)


def is_ucs_bytes(data: bytes) -> bool:
    """Return True if *data* looks like a UCS archive (gzip magic)."""
    return len(data) >= 2 and data[0] == 0x1F and data[1] == 0x8B


def ucs_to_scf(ucs_bytes: bytes, *, include_extras: bool = False) -> str:
    """Extract *ucs_bytes* and return a concatenated SCF text.

    When *include_extras* is true, any additional ``config/*.conf``
    files not in the canonical order are appended at the end (still
    in deterministic alphabetical order).  Members that are not
    present in the archive are silently skipped.
    """
    chunks: list[str] = []
    seen: set[str] = set()
    extras: list[tuple[str, str]] = []

    with tarfile.open(fileobj=io.BytesIO(ucs_bytes), mode="r:*") as tf:
        members_by_name = {m.name.lstrip("./"): m for m in tf.getmembers() if m.isfile()}

        for canonical in _SCF_MEMBER_ORDER:
            member = members_by_name.get(canonical)
            if member is None:
                continue
            text = _read_member(tf, member)
            chunks.append(f"#\n# {canonical}\n#\n{text.rstrip()}\n")
            seen.add(canonical)

        if include_extras:
            for name, member in sorted(members_by_name.items()):
                if name in seen:
                    continue
                if not name.startswith("config/"):
                    continue
                if not name.endswith(".conf"):
                    continue
                text = _read_member(tf, member)
                extras.append((name, text))

    for name, text in extras:
        chunks.append(f"#\n# {name}\n#\n{text.rstrip()}\n")

    return "\n".join(chunks)


def _read_member(tf: tarfile.TarFile, member: tarfile.TarInfo) -> str:
    fh = tf.extractfile(member)
    if fh is None:
        return ""
    raw = fh.read()
    return raw.decode("utf-8", errors="replace")


def extract_ucs_file(path: Path | str) -> str:
    """Convenience: read a UCS file from disk and return its SCF text."""
    data = Path(path).read_bytes()
    if not is_ucs_bytes(data):
        raise ValueError(f"{path}: not a UCS archive (gzip magic missing)")
    return ucs_to_scf(data)


def make_test_ucs(members: dict[str, str]) -> bytes:
    """Build a minimal in-memory UCS archive from *members* dict.

    Used by tests; not part of the production fetch path.  Keys are
    paths inside the archive (e.g. ``"config/bigip.conf"``); values
    are file contents.
    """
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tf:
        for name, content in members.items():
            data = content.encode("utf-8")
            info = tarfile.TarInfo(name=name)
            info.size = len(data)
            tf.addfile(info, io.BytesIO(data))
    return buf.getvalue()
