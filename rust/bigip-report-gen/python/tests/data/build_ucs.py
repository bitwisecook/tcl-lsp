#!/usr/bin/env python3
"""Package the committed BIG-IP config trees into `.ucs` archives.

A UCS archive is simply a gzip-compressed tar of the device's `/config` tree.
This script wraps each `device-NN.bigip.conf` (+ its `bigip_base.conf`) into a
matching `lab-device-NN.ucs`, so the test/demo sources are reproducible from the
plain-text configs. Run it after editing any `device-NN.*.conf`::

    python build_ucs.py

The source configs are real, published F5 example configurations — see
PROVENANCE.md for exact origins.
"""

from __future__ import annotations

import gzip
import io
import pathlib
import tarfile

HERE = pathlib.Path(__file__).parent

# (output .ucs, [(tar member, source file), ...])
ARCHIVES = [
    (
        "lab-device-01.ucs",
        [
            ("config/bigip.conf", "device-01.bigip.conf"),
            ("config/bigip_base.conf", "device-01.bigip_base.conf"),
        ],
    ),
    (
        "lab-device-02.ucs",
        [
            ("config/bigip.conf", "device-02.bigip.conf"),
            ("config/bigip_base.conf", "device-02.bigip_base.conf"),
        ],
    ),
]


def build(out_name: str, members: list[tuple[str, str]]) -> None:
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w") as tar:
        for member, src in members:
            data = (HERE / src).read_bytes()
            info = tarfile.TarInfo(member)
            info.size = len(data)
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(data))
    (HERE / out_name).write_bytes(gzip.compress(buf.getvalue()))
    print(f"wrote {out_name} ({(HERE / out_name).stat().st_size:,} bytes)")


if __name__ == "__main__":
    for out_name, members in ARCHIVES:
        build(out_name, members)
