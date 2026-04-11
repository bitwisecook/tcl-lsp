#!/usr/bin/env python3
"""Unified deployment harness for BigIP event-ordering tests.

Orchestrates the full cycle: certificate generation, artifact
generation, BigIP deployment, server startup, client execution,
HSL log collection, and result analysis with graph output.

Usage::

    python harness.py \\
        --bigip-host 10.1.1.1 --bigip-user admin --bigip-pass admin \\
        --vip-ip 10.1.1.10 --server-ip 10.1.1.100 \\
        --hsl-receiver-ip 10.1.1.200 \\
        [--snat-ip 10.1.1.50] \\
        [--scenarios plain_tcp,tcp_http] \\
        [--graph ascii]

Steps:
  1. Generate self-signed certificates
  2. Build parameterized scenarios
  3. Generate iRules + tmsh artifacts
  4. Deploy to BigIP via SSH (scp + tmsh)
  5. Start HSL log receiver
  6. Start pool member servers
  7. Run client scripts
  8. Collect and parse HSL logs
  9. Generate event flow graph
  10. Cleanup (optional, with --cleanup)
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time

_HERE = os.path.dirname(os.path.abspath(__file__))
_TCL_LSP = os.path.dirname(os.path.dirname(_HERE))
if _TCL_LSP not in sys.path:
    sys.path.insert(0, _TCL_LSP)

from tests.bigip_event_ordering.cert_gen import generate_certs  # noqa: E402
from tests.bigip_event_ordering.graph_gen import generate_graph  # noqa: E402
from tests.bigip_event_ordering.irule_gen import generate_irule  # noqa: E402
from tests.bigip_event_ordering.parse_logs import (  # noqa: E402
    compare_with_expected,
    group_by_session,
    parse_lines,
    print_table,
)
from tests.bigip_event_ordering.scenarios import (  # noqa: E402
    NetworkConfig,
    Scenario,
    build_scenarios,
)
from tests.bigip_event_ordering.tmsh_gen import (  # noqa: E402
    generate_cleanup,
    generate_cleanup_hsl,
    generate_hsl_pool,
    generate_setup,
)

# SSH helpers


def _ssh(host: str, user: str, password: str, cmd: str) -> str:
    """Run a command on the BigIP via SSH.  Returns stdout."""
    result = subprocess.run(
        [
            "sshpass",
            "-p",
            password,
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            f"{user}@{host}",
            cmd,
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        print(f"[ssh] FAILED: {cmd}", file=sys.stderr)
        print(f"  stderr: {result.stderr.strip()}", file=sys.stderr)
    return result.stdout


def _scp_to(host: str, user: str, password: str, local: str, remote: str) -> None:
    """Copy a local file to the BigIP."""
    subprocess.run(
        [
            "sshpass",
            "-p",
            password,
            "scp",
            "-o",
            "StrictHostKeyChecking=no",
            local,
            f"{user}@{host}:{remote}",
        ],
        check=True,
        capture_output=True,
        timeout=30,
    )


# Server process management

_SERVER_SCRIPTS = {
    "tcp": ("server_tcp.py", 9080),
    "http": ("server_http.py", 9081),
    "https": ("server_https.py", 9443),
}


def _start_servers(
    scenarios: dict[str, Scenario],
    bind: str = "0.0.0.0",
    cert_dir: str = "certs",
) -> list[subprocess.Popen]:
    """Start pool member servers needed for the selected scenarios."""
    needed: dict[str, int] = {}
    for sc in scenarios.values():
        proto = sc.client_protocol
        if proto == "https_post":
            proto = "https"
        if proto in _SERVER_SCRIPTS:
            needed[proto] = sc.pool_member_port

    procs: list[subprocess.Popen] = []
    for proto, port in needed.items():
        script, _default_port = _SERVER_SCRIPTS[proto]
        script_path = os.path.join(_HERE, script)
        cmd = [sys.executable, script_path, "--port", str(port), "--bind", bind]
        if proto == "https":
            cmd.extend(
                [
                    "--cert",
                    os.path.join(cert_dir, "serverssl.crt"),
                    "--key",
                    os.path.join(cert_dir, "serverssl.key"),
                ]
            )
        print(f"[harness] Starting {script} on port {port}")
        proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        procs.append(proc)

    return procs


# Client execution

_CLIENT_SCRIPTS = {
    "tcp": "client_tcp.py",
    "http": "client_http.py",
    "https": "client_https.py",
    "https_post": "client_https_post.py",
}


def _run_clients(
    scenarios: dict[str, Scenario],
    count: int = 3,
    pause: float = 1.0,
) -> None:
    """Run client scripts for each selected scenario."""
    for name, sc in scenarios.items():
        script = _CLIENT_SCRIPTS.get(sc.client_protocol)
        if script is None:
            print(f"[harness] Skipping client for {name} (protocol={sc.client_protocol})")
            continue

        script_path = os.path.join(_HERE, script)
        cmd = [
            sys.executable,
            script_path,
            "--host",
            sc.vip,
            "--port",
            str(sc.vip_port),
            "--count",
            str(count),
        ]
        print(f"[harness] Running {script} -> {sc.vip}:{sc.vip_port}")
        subprocess.run(cmd, timeout=30)
        time.sleep(pause)


# Main harness


def run_harness(
    *,
    bigip_host: str,
    bigip_user: str,
    bigip_pass: str,
    vip_ip: str,
    server_ip: str,
    hsl_receiver_ip: str,
    hsl_receiver_port: int = 5514,
    snat_ip: str = "",
    scenario_names: list[str] | None = None,
    output_dir: str = "output",
    cert_dir: str = "certs",
    graph_format: str = "ascii",
    request_count: int = 3,
    cleanup: bool = False,
) -> None:
    """Execute the full harness workflow."""
    # 1. Generate certs
    print("[harness] Generating certificates...")
    certs = generate_certs(cert_dir)

    # 2. Build parameterized scenarios
    net = NetworkConfig(
        vip_ip=vip_ip,
        pool_member_ip=server_ip,
        snat_ip=snat_ip,
        hsl_receiver_ip=hsl_receiver_ip,
        hsl_receiver_port=hsl_receiver_port,
    )
    all_scenarios = build_scenarios(net)
    if scenario_names:
        selected = {k: v for k, v in all_scenarios.items() if k in scenario_names}
    else:
        selected = all_scenarios

    # 3. Generate artifacts
    os.makedirs(output_dir, exist_ok=True)
    print(f"[harness] Generating artifacts for {len(selected)} scenarios...")

    for name, sc in selected.items():
        irule_path = os.path.join(output_dir, f"{name}.irul")
        tmsh_path = os.path.join(output_dir, f"{name}.tmsh")
        with open(irule_path, "w") as f:
            f.write(generate_irule(sc))
        with open(tmsh_path, "w") as f:
            f.write(generate_setup(sc))

    # HSL pool tmsh
    hsl_tmsh_path = os.path.join(output_dir, "hsl_pool_setup.tmsh")
    with open(hsl_tmsh_path, "w") as f:
        f.write(generate_hsl_pool(hsl_receiver_ip, hsl_receiver_port))

    # 4. Deploy to BigIP
    print("[harness] Deploying to BigIP...")

    # Upload certs
    for local_file in [certs.ca_cert, certs.clientssl_cert, certs.clientssl_key]:
        _scp_to(
            bigip_host,
            bigip_user,
            bigip_pass,
            local_file,
            f"/var/tmp/{os.path.basename(local_file)}",
        )

    # Install certs on BigIP
    _ssh(
        bigip_host,
        bigip_user,
        bigip_pass,
        "tmsh install sys crypto cert evt_order_ca.crt from-local-file /var/tmp/ca.crt",
    )
    _ssh(
        bigip_host,
        bigip_user,
        bigip_pass,
        "tmsh install sys crypto cert evt_order_clientssl.crt "
        "from-local-file /var/tmp/clientssl.crt",
    )
    _ssh(
        bigip_host,
        bigip_user,
        bigip_pass,
        "tmsh install sys crypto key evt_order_clientssl.key "
        "from-local-file /var/tmp/clientssl.key",
    )

    # Create HSL pool
    _scp_to(bigip_host, bigip_user, bigip_pass, hsl_tmsh_path, "/var/tmp/hsl_pool_setup.tmsh")
    _ssh(bigip_host, bigip_user, bigip_pass, 'tmsh -c "source /var/tmp/hsl_pool_setup.tmsh"')

    # Deploy each scenario
    for name in selected:
        tmsh_path = os.path.join(output_dir, f"{name}.tmsh")
        _scp_to(bigip_host, bigip_user, bigip_pass, tmsh_path, f"/var/tmp/{name}.tmsh")
        _ssh(bigip_host, bigip_user, bigip_pass, f'tmsh -c "source /var/tmp/{name}.tmsh"')

    # 5. Start HSL receiver
    log_file = os.path.join(output_dir, "hsl_events.log")
    print(f"[harness] Starting HSL receiver on port {hsl_receiver_port}...")
    receiver_proc = subprocess.Popen(
        [
            sys.executable,
            os.path.join(_HERE, "hsl_receiver.py"),
            "--port",
            str(hsl_receiver_port),
            "--output",
            log_file,
            "--timeout",
            "15",
        ],
    )

    # 6. Start pool member servers
    time.sleep(1)
    print("[harness] Starting pool member servers...")
    server_procs = _start_servers(selected, cert_dir=cert_dir)

    # 7. Run clients
    time.sleep(2)
    print("[harness] Running clients...")
    _run_clients(selected, count=request_count)

    # 8. Wait for HSL receiver to collect and timeout
    print("[harness] Waiting for log collection...")
    receiver_proc.wait(timeout=60)

    # 9. Parse and report
    print("\n[harness] === Results ===\n")
    if os.path.exists(log_file):
        with open(log_file) as f:
            lines = f.readlines()
        records = parse_lines(lines)
        if records:
            groups = group_by_session(records)
            print_table(groups)
            compare_with_expected(groups)
            if graph_format:
                for sid, session_records in sorted(groups.items()):
                    scenario = session_records[0].scenario
                    print(f"\n=== Graph: {sid} ({scenario}) ===")
                    print(generate_graph(graph_format, session_records))
        else:
            print("[harness] No EVTORD records found in logs")
    else:
        print(f"[harness] Log file not found: {log_file}")

    # 10. Cleanup
    for proc in server_procs:
        proc.terminate()

    if cleanup:
        print("\n[harness] Cleaning up BigIP objects...")
        for name in selected:
            _ssh(
                bigip_host,
                bigip_user,
                bigip_pass,
                f'tmsh -c "{generate_cleanup(selected[name]).strip()}"',
            )
        _ssh(bigip_host, bigip_user, bigip_pass, f'tmsh -c "{generate_cleanup_hsl().strip()}"')


def main() -> None:
    parser = argparse.ArgumentParser(
        description="BigIP event-ordering test harness",
    )
    parser.add_argument("--bigip-host", required=True, help="BigIP management IP")
    parser.add_argument("--bigip-user", default="admin", help="BigIP username (default: admin)")
    parser.add_argument("--bigip-pass", required=True, help="BigIP password")
    parser.add_argument("--vip-ip", required=True, help="Virtual server IP")
    parser.add_argument("--server-ip", required=True, help="Pool member / backend server IP")
    parser.add_argument(
        "--hsl-receiver-ip", required=True, help="IP where the HSL receiver will listen"
    )
    parser.add_argument(
        "--hsl-receiver-port", type=int, default=5514, help="HSL receiver UDP port (default: 5514)"
    )
    parser.add_argument("--snat-ip", default="", help="SNAT IP (empty=automap)")
    parser.add_argument("--scenarios", help="Comma-separated scenario names (default: all)")
    parser.add_argument(
        "--output-dir", default="output", help="Artifact output directory (default: output)"
    )
    parser.add_argument(
        "--cert-dir", default="certs", help="Certificate directory (default: certs)"
    )
    parser.add_argument(
        "--graph",
        choices=["ascii", "mermaid", "dot"],
        default="ascii",
        help="Graph format (default: ascii)",
    )
    parser.add_argument("--count", type=int, default=3, help="Requests per scenario (default: 3)")
    parser.add_argument("--cleanup", action="store_true", help="Remove BigIP objects after test")
    args = parser.parse_args()

    run_harness(
        bigip_host=args.bigip_host,
        bigip_user=args.bigip_user,
        bigip_pass=args.bigip_pass,
        vip_ip=args.vip_ip,
        server_ip=args.server_ip,
        hsl_receiver_ip=args.hsl_receiver_ip,
        hsl_receiver_port=args.hsl_receiver_port,
        snat_ip=args.snat_ip,
        scenario_names=args.scenarios.split(",") if args.scenarios else None,
        output_dir=args.output_dir,
        cert_dir=args.cert_dir,
        graph_format=args.graph,
        request_count=args.count,
        cleanup=args.cleanup,
    )


if __name__ == "__main__":
    main()
