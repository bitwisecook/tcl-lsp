#!/usr/bin/env python3
"""Render disposable lab pool/VIP objects; never connects to BIG-IP."""

import argparse
import hashlib
import ipaddress
import json
import re
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--run", required=True)
p.add_argument("--vip", required=True)
p.add_argument("--vip-port", type=int, default=18086)
p.add_argument("--backend", required=True)
p.add_argument("--backend-port", type=int, default=18080)
p.add_argument(
    "--rule",
    action="append",
    required=True,
    help="exact /Common generated rule object; order retained",
)
p.add_argument("--out", type=Path, required=True)
a = p.parse_args()
if not re.fullmatch("[A-Za-z][A-Za-z0-9_]{0,23}", a.run):
    p.error("invalid run")
try:
    vip = ipaddress.IPv4Address(a.vip)
    backend = ipaddress.IPv4Address(a.backend)
except ValueError as error:
    p.error(str(error))
if vip.is_loopback or backend.is_loopback or vip == backend:
    p.error("use distinct non-loopback external backend and lab VIP")
if not all(1 <= n <= 65535 for n in [a.vip_port, a.backend_port]):
    p.error("invalid port")
prefix = "__tcl_lsp_probe_2286_" + a.run
if not all(
    re.fullmatch("/Common/" + re.escape(prefix) + "_[A-Za-z0-9_]+", r) for r in a.rule
):
    p.error("rule must be an exact generated object for this run")
pool = "/Common/" + prefix + "_pool"
virtual = "/Common/" + prefix + "_vs"
text = """ltm pool {} {{
    members {{ {}:{:d} {{ address {} }} }}
}}
ltm virtual {} {{
    destination {}:{:d}
    mask 255.255.255.255
    ip-protocol tcp
    source 0.0.0.0/0
    profiles {{ /Common/tcp {{ }} /Common/http {{ }} }}
    pool {}
    rules {{ {} }}
    source-address-translation {{ type automap }}
    cmp-enabled yes
}}
""".format(
    *(
        pool,
        backend,
        a.backend_port,
        backend,
        virtual,
        vip,
        a.vip_port,
        pool,
        " ".join(a.rule),
    )
)
a.out.write_bytes(text.encode("ascii"))
print(
    json.dumps(
        {
            "pool": pool,
            "virtual": virtual,
            "sha256": hashlib.sha256(a.out.read_bytes()).hexdigest(),
            "required_precondition": "prove BOTH exact objects absent; no production attachment or config save",
        },
        indent=2,
    )
)
