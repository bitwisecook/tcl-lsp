#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Render Common and temporary-partition SNAT-automap procedure test VIPs."""

import argparse
import hashlib
import ipaddress
import json
import re
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--run", required=True)
parser.add_argument("--vip", required=True)
parser.add_argument("--backend", required=True)
parser.add_argument("--backend-port", type=int, default=18084)
parser.add_argument("--common-vip-port", type=int, default=18087)
parser.add_argument("--partition-vip-port", type=int, default=18088)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()
if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,15}", args.run):
    parser.error("invalid run")
vip = ipaddress.IPv4Address(args.vip)
backend = ipaddress.IPv4Address(args.backend)
if vip.is_loopback or backend.is_loopback or vip == backend:
    parser.error("use distinct non-loopback external backend and lab VIP")
ports = [args.backend_port, args.common_vip_port, args.partition_vip_port]
if not all(1 <= port <= 65535 for port in ports) or len(set(ports)) != len(ports):
    parser.error("ports must be distinct and valid")

prefix = f"__tcl_lsp_probe_2286_{args.run}"
partition = f"R2286_{args.run}"
common_pool = f"/Common/{prefix}_pool"
common_virtual = f"/Common/{prefix}_vs"
common_node = f"/Common/{prefix}_node"
partition_pool = f"/{partition}/{prefix}_pool"
partition_virtual = f"/{partition}/{prefix}_vs"
text = f'''ltm pool {common_pool} {{
    members {{ {common_node}:{args.backend_port} {{ address {backend} }} }}
}}
ltm virtual {common_virtual} {{
    destination {vip}:{args.common_vip_port}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {common_pool}
    rules {{ /Common/{prefix}_caller }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
ltm pool {partition_pool} {{
    members {{ {common_node}:{args.backend_port} {{ address {backend} }} }}
}}
ltm virtual {partition_virtual} {{
    destination {vip}:{args.partition_vip_port}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {partition_pool}
    rules {{ /{partition}/{prefix}_caller }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
'''.encode("ascii")
args.out.write_bytes(text)
print(
    json.dumps(
        {
            "common_pool": common_pool,
            "common_virtual": common_virtual,
            "common_node": common_node,
            "partition_pool": partition_pool,
            "partition_virtual": partition_virtual,
            "sha256": hashlib.sha256(text).hexdigest(),
            "snat": "automap on both virtual servers",
        },
        indent=2,
    )
)
