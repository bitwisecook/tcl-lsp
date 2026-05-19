"""SSH/scp transport for ``f5 fetch``.

Wraps the system ``ssh`` and ``scp`` binaries via :mod:`subprocess`
to avoid pulling in a paramiko dependency.  Falls back with a clear
error if either binary is not on ``PATH``.

Password authentication uses ``sshpass`` when available; otherwise
the caller is expected to have key-based auth configured (and the
``password`` field in :class:`Credentials` is ignored).
"""

from __future__ import annotations

import os
import shutil
import subprocess  # noqa: S404 — wrapping ssh/scp is the whole point
import tempfile
import time
from pathlib import Path

from .auth import Credentials


class SSHError(RuntimeError):
    """Raised for any non-zero exit from ssh/scp."""


def _which_or_raise(binary: str) -> str:
    found = shutil.which(binary)
    if not found:
        raise SSHError(f"{binary!r} not found on PATH")
    return found


def _ssh_base_args(credentials: Credentials) -> list[str]:
    return [
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "BatchMode=no",
        "-p",
        str(credentials.ssh_port),
    ]


def _run_ssh(
    credentials: Credentials,
    remote_command: str,
    *,
    timeout: float,
) -> str:
    ssh = _which_or_raise("ssh")
    sshpass = shutil.which("sshpass")
    target = f"{credentials.user}@{credentials.host}"
    if sshpass:
        cmd = [sshpass, "-e", ssh, *_ssh_base_args(credentials), target, remote_command]
        env = {**os.environ, "SSHPASS": credentials.password}
    else:
        cmd = [ssh, *_ssh_base_args(credentials), target, remote_command]
        env = os.environ.copy()
    proc = subprocess.run(  # noqa: S603 — we control argv
        cmd,
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if proc.returncode != 0:
        raise SSHError(
            f"ssh {target!r}: exit {proc.returncode}: {proc.stderr.strip() or proc.stdout.strip()}"
        )
    return proc.stdout


def _run_scp(
    credentials: Credentials,
    remote_path: str,
    local_path: Path,
    *,
    timeout: float,
) -> None:
    scp = _which_or_raise("scp")
    sshpass = shutil.which("sshpass")
    target = f"{credentials.user}@{credentials.host}:{remote_path}"
    base = [
        "-O",  # legacy scp protocol; UCS BIG-IPs commonly need this
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-P",
        str(credentials.ssh_port),
    ]
    if sshpass:
        cmd = [sshpass, "-e", scp, *base, target, str(local_path)]
        env = {**os.environ, "SSHPASS": credentials.password}
    else:
        cmd = [scp, *base, target, str(local_path)]
        env = os.environ.copy()
    proc = subprocess.run(  # noqa: S603
        cmd,
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if proc.returncode != 0:
        raise SSHError(
            f"scp {target!r}: exit {proc.returncode}: {proc.stderr.strip() or proc.stdout.strip()}"
        )


def fetch(
    credentials: Credentials,
    *,
    fmt: str = "scf",
    timeout: float = 120.0,
    name: str | None = None,
) -> tuple[str, bytes | None]:
    """Pull config via ssh+scp.  Returns ``(scf_text, ucs_bytes_or_None)``."""
    from .ucs import ucs_to_scf

    if name is None:
        name = f"f5_fetch_{int(time.time())}"

    scf_text = ""
    ucs_bytes: bytes | None = None

    with tempfile.TemporaryDirectory(prefix="f5fetch-") as tmp:
        tmp_path = Path(tmp)

        if fmt in {"scf", "both"}:
            _run_ssh(
                credentials,
                f"tmsh save sys config file {name} no-passphrase",
                timeout=timeout,
            )
            local_scf = tmp_path / f"{name}.scf"
            _run_scp(
                credentials,
                f"/var/local/scf/{name}.scf",
                local_scf,
                timeout=timeout,
            )
            scf_text = local_scf.read_text(encoding="utf-8", errors="replace")

        if fmt in {"ucs", "both"}:
            _run_ssh(
                credentials,
                f"tmsh save sys ucs {name}",
                timeout=timeout,
            )
            local_ucs = tmp_path / f"{name}.ucs"
            _run_scp(
                credentials,
                f"/var/local/ucs/{name}.ucs",
                local_ucs,
                timeout=timeout,
            )
            ucs_bytes = local_ucs.read_bytes()
            if not scf_text:
                scf_text = ucs_to_scf(ucs_bytes)

    return scf_text, ucs_bytes
