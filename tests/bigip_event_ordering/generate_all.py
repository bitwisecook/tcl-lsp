#!/usr/bin/env python3
"""Generate all event-ordering artifacts for BigIP deployment.

Writes iRules, tmsh configs, and HSL pool setup for every scenario
to the ``output/`` directory.  Run from the ``tcl-lsp/`` directory::

    python tests/bigip_event_ordering/generate_all.py

To customise network addresses::

    python tests/bigip_event_ordering/generate_all.py \\
        --vip-ip 10.1.1.10 --server-ip 10.1.1.100 \\
        --hsl-receiver-ip 10.1.1.200
"""

from __future__ import annotations

import argparse
import os
import sys

# Ensure tcl-lsp/ is on the path so we can import project packages.
_HERE = os.path.dirname(os.path.abspath(__file__))
_TCL_LSP = os.path.dirname(os.path.dirname(_HERE))
if _TCL_LSP not in sys.path:
    sys.path.insert(0, _TCL_LSP)

from tests.bigip_event_ordering.irule_gen import generate_irule  # noqa: E402
from tests.bigip_event_ordering.scenarios import (  # noqa: E402
    NetworkConfig,
    build_scenarios,
)
from tests.bigip_event_ordering.tmsh_gen import (  # noqa: E402
    generate_cleanup_hsl,
    generate_hsl_pool,
    generate_tmsh,
)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate BigIP event-ordering artifacts",
    )
    parser.add_argument(
        "--vip-ip", default="10.1.1.10", help="Virtual server IP (default: 10.1.1.10)"
    )
    parser.add_argument(
        "--server-ip", default="10.1.1.100", help="Pool member IP (default: 10.1.1.100)"
    )
    parser.add_argument(
        "--hsl-receiver-ip", default="10.1.1.200", help="HSL log receiver IP (default: 10.1.1.200)"
    )
    parser.add_argument(
        "--hsl-receiver-port", type=int, default=5514, help="HSL log receiver port (default: 5514)"
    )
    args = parser.parse_args()

    net = NetworkConfig(
        vip_ip=args.vip_ip,
        pool_member_ip=args.server_ip,
        hsl_receiver_ip=args.hsl_receiver_ip,
        hsl_receiver_port=args.hsl_receiver_port,
    )
    scenarios = build_scenarios(net)

    outdir = os.path.join(_HERE, "output")
    os.makedirs(outdir, exist_ok=True)

    # HSL pool setup (shared by all scenarios)
    hsl_path = os.path.join(outdir, "hsl_pool.tmsh")
    hsl_setup = generate_hsl_pool(net.hsl_receiver_ip, net.hsl_receiver_port)
    hsl_cleanup = generate_cleanup_hsl()
    with open(hsl_path, "w") as f:
        f.write(hsl_setup)
        f.write("\n")
        f.write(hsl_cleanup)
    print(f"  {hsl_path}")

    # Per-scenario artifacts
    for name, scenario in scenarios.items():
        irule_path = os.path.join(outdir, f"{name}.irul")
        tmsh_path = os.path.join(outdir, f"{name}.tmsh")

        irule_src = generate_irule(scenario)
        tmsh_src = generate_tmsh(scenario)

        with open(irule_path, "w") as f:
            f.write(irule_src)
        with open(tmsh_path, "w") as f:
            f.write(tmsh_src)

        print(f"  {irule_path}")
        print(f"  {tmsh_path}")

    print(f"\nGenerated {len(scenarios)} scenarios + HSL pool in {outdir}/")

    # Print a summary of scenarios and their client/server scripts
    print("\n--- Scenario Quick Reference ---")
    print(f"{'Scenario':<42} {'Client':<25} {'Server':<20} {'VIP Port'}")
    print(f"{'-' * 42} {'-' * 25} {'-' * 20} {'-' * 8}")
    client_map = {
        "tcp": "client_tcp.py",
        "http": "client_http.py",
        "https": "client_https.py",
        "https_post": "client_https_post.py",
        "dns": "(dig / nslookup)",
    }
    server_map = {
        "tcp": "server_tcp.py",
        "http": "server_http.py",
        "https": "server_http.py",
        "https_post": "server_https.py",
        "dns": "(named / dnsmasq)",
    }
    for name, scenario in scenarios.items():
        client = client_map.get(scenario.client_protocol, "?")
        server = server_map.get(scenario.client_protocol, "?")
        print(f"  {name:<40} {client:<25} {server:<20} {scenario.vip_port}")


if __name__ == "__main__":
    main()
