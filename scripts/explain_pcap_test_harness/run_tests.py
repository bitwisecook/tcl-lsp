#!/usr/bin/env python3
"""run_tests.py — explain-pcap end-to-end test orchestrator.

Drives the full lab loop:

1. Generates the lab CA + leaf certs (via :file:`gen_certs.sh`).
2. SCPs the certs and the SCF onto the BIG-IP, then loads the SCF via
   ``tmsh load sys config merge file``.  The SCF intentionally lives in
   its own ``explain_pcap_lab`` partition so it's easy to delete.
3. Starts a tcpdump on the BIG-IP via SSH on every relevant interface
   (``-i 0.0:nnnp`` so the F5 ethernet trailer is included; one capture
   per scenario keeps file sizes manageable).
4. Optionally starts :file:`test_server.py` locally so the
   automap-SNATted back-side traffic comes back to this host (handy
   when you don't have a separate backend to point the pool at; in that
   case point the pool members at the host running ``run_tests.py``).
5. For each scenario in :mod:`scenarios`:
   a. Sets up ``SSLKEYLOGFILE`` for the curl invocation if the scenario
      asks for it.
   b. Runs curl (or netcat for the TCP-payload scenario), capturing
      stdout/stderr.
   c. Stops the BIG-IP-side tcpdump and SCPs the resulting ``.pcap``
      back into this run's output directory.
6. Runs ``f5 explain-pcap`` over each captured pcap with the lab SCF
   and the keylog file, dumping both text and JSON reports.
7. Writes a per-scenario manifest so an LLM (or a human) can read what
   was captured and what the expected outcome was.

Usage:
  ./run_tests.py --bigip 10.0.0.5 --user admin --password ... \\
                 [--ssh-key ~/.ssh/id_rsa] \\
                 [--mgmt-iface mgmt] \\
                 [--data-iface 0.0] \\
                 [--out ./runs/$(date +%s)] \\
                 [--scenarios http_block_by_host,https_sni_route_api] \\
                 [--server-here]

Authentication: either ``--ssh-key`` or ``--password`` must be set.
``--password`` uses ``sshpass`` if it's available; otherwise the user
is prompted by the OS for each ssh/scp call (set up an ssh-agent or
ssh-key for non-interactive use).

Requires: openssl, ssh, scp, curl on the local machine; tmsh + tcpdump
on the BIG-IP.  ``f5 explain-pcap`` is invoked as
``python -m explorer.f5_cli explain-pcap`` from the repo root if
present, otherwise skipped with a warning.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import signal
import subprocess
import sys
import textwrap
import time
from dataclasses import asdict
from pathlib import Path

# Module-relative imports work both when invoked directly and when the
# harness directory is on sys.path.  We don't depend on tcl-lsp packages
# here so the script is portable to any Python 3.10+ host.
HARNESS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(HARNESS_DIR))
from scenarios import SCENARIOS, Scenario  # noqa: E402, I001


# ── argument parsing ──────────────────────────────────────────────────


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    p.add_argument("--bigip", required=True, help="BIG-IP management IP / hostname.")
    p.add_argument("--user", default="admin", help="BIG-IP SSH user (default admin).")
    auth = p.add_mutually_exclusive_group(required=False)
    auth.add_argument("--ssh-key", help="Path to SSH private key for the BIG-IP.")
    auth.add_argument("--password", help="SSH password for the BIG-IP (uses sshpass if available).")
    p.add_argument(
        "--data-iface",
        default="0.0",
        help="Interface name for tcpdump on the BIG-IP (default 0.0 = all). "
        "Pass with `:nnnp` already appended if you want to override.",
    )
    p.add_argument(
        "--mgmt-iface",
        default="mgmt",
        help="Interface name to ignore when capturing (default 'mgmt').",
    )
    p.add_argument(
        "--out",
        default=f"runs/{int(time.time())}",
        help="Output directory for pcaps + reports (default ./runs/<epoch>).",
    )
    p.add_argument(
        "--scenarios",
        default="",
        help="Comma-separated scenario names to run (default: all).",
    )
    p.add_argument(
        "--server-here",
        action="store_true",
        help="Start test_server.py locally on this host so automap-SNAT "
        "traffic comes back to the same machine that ran curl.",
    )
    p.add_argument(
        "--skip-deploy",
        action="store_true",
        help="Skip cert/SCF deployment — assume the BIG-IP is already configured.",
    )
    p.add_argument(
        "--skip-explain",
        action="store_true",
        help="Skip running `f5 explain-pcap` over the captured pcaps.",
    )
    p.add_argument(
        "--repo-root",
        default=str(HARNESS_DIR.parent.parent),
        help="tcl-lsp repo root (default: derived from script location).",
    )
    return p.parse_args(argv)


# ── ssh / scp helpers ────────────────────────────────────────────────


class SshRunner:
    """Thin wrapper around ssh/scp invocations for the BIG-IP.

    Doesn't depend on paramiko — uses subprocess + ssh so the harness
    has no Python deps beyond the stdlib.
    """

    def __init__(self, host: str, user: str, *, ssh_key: str = "", password: str = "") -> None:
        self.host = host
        self.user = user
        self.ssh_key = ssh_key
        self.password = password
        self._sshpass = shutil.which("sshpass") if password else ""
        if password and not self._sshpass:
            print(
                "warning: --password supplied but sshpass not on PATH; "
                "you'll be prompted by ssh/scp interactively.",
                file=sys.stderr,
            )

    def _ssh_cmd(self, *extra: str) -> list[str]:
        cmd: list[str] = []
        if self._sshpass and self.password:
            cmd += [self._sshpass, "-p", self.password]
        cmd += [
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "LogLevel=ERROR",
        ]
        if self.ssh_key:
            cmd += ["-i", self.ssh_key]
        cmd += [f"{self.user}@{self.host}", *extra]
        return cmd

    def _scp_cmd(self, src: str, dst: str, *, to_remote: bool = True) -> list[str]:
        cmd: list[str] = []
        if self._sshpass and self.password:
            cmd += [self._sshpass, "-p", self.password]
        cmd += [
            "scp",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "LogLevel=ERROR",
        ]
        if self.ssh_key:
            cmd += ["-i", self.ssh_key]
        if to_remote:
            cmd += [src, f"{self.user}@{self.host}:{dst}"]
        else:
            cmd += [f"{self.user}@{self.host}:{src}", dst]
        return cmd

    def run(
        self, command: str, *, check: bool = True, capture: bool = False
    ) -> subprocess.CompletedProcess:
        """Run *command* on the remote host."""
        cmd = self._ssh_cmd(command)
        return subprocess.run(
            cmd,
            check=check,
            capture_output=capture,
            text=True,
        )

    def upload(self, local: str | Path, remote: str) -> None:
        cmd = self._scp_cmd(str(local), remote, to_remote=True)
        subprocess.run(cmd, check=True)

    def download(self, remote: str, local: str | Path) -> None:
        cmd = self._scp_cmd(remote, str(local), to_remote=False)
        subprocess.run(cmd, check=True)

    def start_background(self, command: str) -> subprocess.Popen:
        """Spawn a long-running command on the remote host (e.g. tcpdump)."""
        cmd = self._ssh_cmd(command)
        return subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )


# ── BIG-IP deployment ─────────────────────────────────────────────────


_REMOTE_LAB_DIR = "/var/tmp/explain_pcap_lab"
_REMOTE_SSL_DIR = "/config/ssl/ssl.crt"
_REMOTE_SSL_KEY_DIR = "/config/ssl/ssl.key"


def deploy_lab(ssh: SshRunner, harness_dir: Path) -> None:
    """Generate certs, SCP everything onto the BIG-IP, and merge the SCF."""
    cert_dir = harness_dir / "certs"
    if not cert_dir.is_dir() or not (cert_dir / "lab_ca.crt").is_file():
        print("→ generating certs")
        subprocess.run(
            ["bash", str(harness_dir / "gen_certs.sh"), str(cert_dir)],
            check=True,
        )

    print("→ ensuring remote staging dir")
    ssh.run(f"mkdir -p {_REMOTE_LAB_DIR}")

    print("→ uploading certs and SCF")
    for crt in cert_dir.glob("*.crt"):
        ssh.upload(crt, f"{_REMOTE_LAB_DIR}/{crt.name}")
    for key in cert_dir.glob("*.key"):
        ssh.upload(key, f"{_REMOTE_LAB_DIR}/{key.name}")
    scf = harness_dir / "bigip_test_lab.scf"
    ssh.upload(scf, f"{_REMOTE_LAB_DIR}/{scf.name}")

    print("→ installing certs into /config/ssl")
    ssh.run(
        "tmsh install sys crypto cert /explain_pcap_lab/lab_ca "
        f"from-local-file {_REMOTE_LAB_DIR}/lab_ca.crt",
        check=False,
    )
    for label, crt, key in (
        ("lab_server_valid", "lab_server_valid.crt", "lab_server_valid.key"),
        ("lab_server_expired", "lab_server_expired.crt", "lab_server_expired.key"),
        ("lab_server_selfsigned", "lab_server_selfsigned.crt", "lab_server_selfsigned.key"),
    ):
        ssh.run(
            f"tmsh install sys crypto cert /explain_pcap_lab/{label} "
            f"from-local-file {_REMOTE_LAB_DIR}/{crt}",
            check=False,
        )
        ssh.run(
            f"tmsh install sys crypto key /explain_pcap_lab/{label} "
            f"from-local-file {_REMOTE_LAB_DIR}/{key}",
            check=False,
        )

    print("→ tmsh load sys config merge")
    ssh.run(f"tmsh load sys config merge file {_REMOTE_LAB_DIR}/bigip_test_lab.scf")
    ssh.run("tmsh save sys config")


def teardown_lab(ssh: SshRunner) -> None:
    """Delete the explain_pcap_lab partition (and everything in it)."""
    ssh.run("tmsh delete auth partition explain_pcap_lab", check=False)
    ssh.run(f"rm -rf {_REMOTE_LAB_DIR}", check=False)


# ── Local test server ────────────────────────────────────────────────


def start_local_server(harness_dir: Path, cert: Path, key: Path) -> subprocess.Popen:
    """Run test_server.py in the background bound to 0.0.0.0:8080 / :8443.

    We don't try to fight with port-already-in-use — caller can stop us
    cleanly via the returned Popen.
    """
    cmd = [
        sys.executable,
        str(harness_dir / "test_server.py"),
        "--bind",
        "0.0.0.0",
        "--http-port",
        "8080",
        "--https-port",
        "8443",
        "--cert",
        str(cert),
        "--key",
        str(key),
    ]
    print(f"→ starting local test_server: {' '.join(shlex.quote(c) for c in cmd)}")
    return subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


# ── tcpdump on the BIG-IP ────────────────────────────────────────────


def _remote_pcap_path(scenario_name: str) -> str:
    return f"{_REMOTE_LAB_DIR}/cap_{scenario_name}.pcap"


def start_remote_tcpdump(ssh: SshRunner, scenario: Scenario, iface: str) -> subprocess.Popen:
    """Start tcpdump on the BIG-IP scoped to this scenario's VIP."""
    pcap = _remote_pcap_path(scenario.name)
    bpf = f"host {scenario.vip}"
    if scenario.port:
        bpf += f" and port {scenario.port}"
    # Use ":nnnp" so the trailer is included; "noproc/noinplace" preserves
    # the proxied peer-side packet for `:np` parity.
    iface_spec = f"{iface}:nnnp" if ":" not in iface else iface
    # ``-U`` flushes per-packet; ``-w`` writes raw pcap.
    cmd = f"tcpdump -i {iface_spec} -s 0 -w {pcap} -U '{bpf}' >/dev/null 2>&1 & echo $!"
    proc = ssh.start_background(cmd)
    # tcpdump's pid is echoed on stdout by the wrapper above.
    return proc


def stop_remote_tcpdump(ssh: SshRunner, scenario: Scenario, popen: subprocess.Popen) -> None:
    """Kill any tcpdump on the BIG-IP capturing this scenario's pcap."""
    pcap = _remote_pcap_path(scenario.name)
    ssh.run(
        f"pkill -f 'tcpdump.*{shlex.quote(pcap)}' || true",
        check=False,
    )
    # Drain the spawn pipe so we don't leak the local Popen.
    try:
        popen.wait(timeout=5)
    except subprocess.TimeoutExpired:
        popen.kill()


# ── Scenario execution ───────────────────────────────────────────────


def _curl_for(
    scenario: Scenario, *, vip: str, keylog: Path | None
) -> tuple[list[str], dict[str, str]]:
    """Build the curl command + env for *scenario*."""
    scheme = "https" if scenario.https else "http"
    url = f"{scheme}://{scenario.host or vip}:{scenario.port}{scenario.path}"
    args: list[str] = ["curl", "-sS", "-o", "/dev/null", "-D-", "--max-time", "15"]
    # --resolve forces SNI=host but routes the connection to the VIP.
    if scenario.host:
        args += ["--resolve", f"{scenario.host}:{scenario.port}:{vip}"]
    args += list(scenario.curl_args)
    args += [url]
    env = os.environ.copy()
    if keylog and scenario.needs_keylog:
        env["SSLKEYLOGFILE"] = str(keylog)
    return args, env


def run_payload_reset_scenario(scenario: Scenario, out_dir: Path) -> dict:
    """Special-case the TCP-payload scenario via /bin/bash + nc."""
    nc_path = shutil.which("nc") or shutil.which("ncat")
    if not nc_path:
        return {"error": "nc/ncat not found on PATH"}
    log = out_dir / f"{scenario.name}.log"
    cmd = f"printf 'RESETME\\n' | {shlex.quote(nc_path)} -w 5 {scenario.vip} {scenario.port}"
    proc = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)
    log.write_text(
        f"$ {cmd}\nexit: {proc.returncode}\n--- stdout ---\n{proc.stdout}\n"
        f"--- stderr ---\n{proc.stderr}\n"
    )
    return {"exit": proc.returncode, "log": str(log)}


def run_scenario(
    scenario: Scenario,
    *,
    ssh: SshRunner,
    out_dir: Path,
    iface: str,
    keylog_path: Path,
) -> dict:
    """Run one scenario end-to-end and return a manifest dict."""
    print(f"\n=== {scenario.name} ===")
    print(textwrap.indent(scenario.description, "  "))

    pcap_local = out_dir / f"{scenario.name}.pcap"
    log_path = out_dir / f"{scenario.name}.log"

    # 1. Start tcpdump on the BIG-IP.
    tcpdump = start_remote_tcpdump(ssh, scenario, iface)
    time.sleep(1)  # let tcpdump open the file before we drive traffic

    # 2. Drive traffic.
    if scenario.name == "payload_reset":
        run_info = run_payload_reset_scenario(scenario, out_dir)
    else:
        cmd, env = _curl_for(scenario, vip=scenario.vip, keylog=keylog_path)
        proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
        log_path.write_text(
            "$ " + " ".join(shlex.quote(a) for a in cmd) + f"\n"
            f"exit: {proc.returncode}\n--- headers + stdout ---\n{proc.stdout}\n"
            f"--- stderr ---\n{proc.stderr}\n"
        )
        run_info = {"exit": proc.returncode, "log": str(log_path)}

    # 3. Stop tcpdump and pull pcap back.
    stop_remote_tcpdump(ssh, scenario, tcpdump)
    time.sleep(1)
    try:
        ssh.download(_remote_pcap_path(scenario.name), pcap_local)
        run_info["pcap"] = str(pcap_local)
    except subprocess.CalledProcessError as exc:
        run_info["pcap_error"] = str(exc)

    return {
        "scenario": asdict(scenario),
        "run": run_info,
    }


# ── explain-pcap invocation ──────────────────────────────────────────


def run_explain(
    repo_root: Path,
    out_dir: Path,
    scenarios: list[Scenario],
    keylog_path: Path,
    scf_path: Path,
) -> None:
    """Run `f5 explain-pcap` over every captured pcap."""
    f5_cli = repo_root / "explorer" / "f5_cli.py"
    if not f5_cli.is_file():
        print("(skipping explain-pcap — explorer/f5_cli.py not found)")
        return
    explain_dir = out_dir / "explain"
    explain_dir.mkdir(parents=True, exist_ok=True)
    for scenario in scenarios:
        pcap = out_dir / f"{scenario.name}.pcap"
        if not pcap.is_file():
            continue
        for fmt in ("text", "json"):
            args = [
                sys.executable,
                "-m",
                "explorer.f5_cli",
                "explain-pcap",
                "--tshark",
            ]
            if scenario.needs_keylog and keylog_path.is_file():
                args += ["--keylog", str(keylog_path)]
            if fmt == "json":
                args.append("--json")
            args += [str(pcap), str(scf_path)]
            out = explain_dir / f"{scenario.name}.{fmt}"
            with open(out, "w") as fh:
                subprocess.run(args, stdout=fh, cwd=repo_root, check=False)
        print(f"  → explain output: {explain_dir / scenario.name}.text")


# ── main ─────────────────────────────────────────────────────────────


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)

    if not args.ssh_key and not args.password:
        print(
            "run_tests.py: need --ssh-key or --password for BIG-IP SSH access",
            file=sys.stderr,
        )
        return 2

    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    keylog_path = out_dir / "sslkeys.log"
    keylog_path.touch()  # ensure file exists for SSLKEYLOGFILE append-mode

    selected: list[Scenario]
    if args.scenarios:
        names = {n.strip() for n in args.scenarios.split(",") if n.strip()}
        selected = [s for s in SCENARIOS if s.name in names]
        if not selected:
            print(f"no scenarios match: {args.scenarios}", file=sys.stderr)
            return 2
    else:
        selected = list(SCENARIOS)

    ssh = SshRunner(args.bigip, args.user, ssh_key=args.ssh_key or "", password=args.password or "")

    if not args.skip_deploy:
        deploy_lab(ssh, HARNESS_DIR)

    server_proc: subprocess.Popen | None = None
    if args.server_here:
        cert_dir = HARNESS_DIR / "certs"
        server_proc = start_local_server(
            HARNESS_DIR,
            cert_dir / "lab_server_valid.crt",
            cert_dir / "lab_server_valid.key",
        )

    manifest: list[dict] = []
    try:
        for scenario in selected:
            entry = run_scenario(
                scenario,
                ssh=ssh,
                out_dir=out_dir,
                iface=args.data_iface,
                keylog_path=keylog_path,
            )
            manifest.append(entry)
    finally:
        if server_proc is not None:
            print("→ stopping local test_server")
            server_proc.send_signal(signal.SIGTERM)
            try:
                server_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server_proc.kill()

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2))

    if not args.skip_explain:
        run_explain(
            Path(args.repo_root).resolve(),
            out_dir,
            selected,
            keylog_path,
            HARNESS_DIR / "bigip_test_lab.scf",
        )

    print(f"\n✓ done.  artefacts in: {out_dir}")
    print(f"  manifest:   {out_dir / 'manifest.json'}")
    print(f"  ssl keylog: {keylog_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
