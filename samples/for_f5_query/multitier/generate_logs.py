"""Generate matching BIG-IP log files for the multi-device topology.

Reads each ``.conf`` in this directory to discover real object names
(VS, pool, node, wide-IP) and emits a coherent log timeline per
device under ``logs/<host>.log``.  The lines use F5's wire format
(``Mon DD HH:MM:SS host severity daemon[pid]: code:level: message``)
and the message codes are the ones operators actually see on a
BIG-IP — see the inline catalogues below.

Run from the repository root:

    python samples/for_f5_query/multitier/generate_logs.py

The generated lines cover the full mix that ``f5log_load`` is
expected to handle: boot, configuration loads, audit / tmsh edits,
HA sync transitions, monitor state changes, connection-limit
warnings, GTM wide-IP resolution, and a sprinkling of ``err`` /
``crit`` lines so severity-filtered queries have something to
catch.  Lines are deterministic (seeded random) so tests can
assert exact counts.
"""

from __future__ import annotations

import random
import re
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOGS_DIR = HERE / "logs"

# ---------------------------------------------------------------------------
# Message code catalogue
# ---------------------------------------------------------------------------
# A *small* curated set of codes that real BIG-IPs emit, with the
# severity nibble F5 uses.  Picked so each surface in the
# multi-device topology has a plausible mix.

CODES = {
    "boot": ("01071db9", 6, "info"),
    "config_load": ("01071a08", 6, "info"),
    "save": ("01071681", 6, "info"),
    "sync_state": ("010719e8", 5, "notice"),
    "ha_active": ("01340038", 6, "info"),
    "ha_standby": ("01340039", 6, "info"),
    "monitor_down": ("01340011", 4, "warning"),
    "monitor_up": ("01340012", 6, "info"),
    "pool_down": ("01070638", 4, "warning"),
    "pool_up": ("01070639", 6, "info"),
    "conn_limit": ("01230140", 5, "notice"),
    "ssl_handshake_fail": ("01260009", 4, "warning"),
    "tmm_overload": ("01340042", 3, "err"),
    "audit_tmsh": ("01070417", 6, "info"),
    "audit_save": ("01071be3", 6, "info"),
    "gtm_zone_reload": ("011a3003", 6, "info"),
    "gtm_wideip_resolve": ("011a3055", 6, "info"),
    "gtm_member_down": ("011a4001", 4, "warning"),
    "gtm_member_up": ("011a4002", 6, "info"),
    "icmd_fail": ("01010013", 3, "err"),
}

# ---------------------------------------------------------------------------
# Config introspection — pull real object names out of each .conf so
# the generated log lines reference the same VSes, pools, nodes that
# the corresponding query examples will navigate.
# ---------------------------------------------------------------------------

_HEADER_RE = re.compile(
    r"^(?P<kind>ltm virtual|ltm pool|ltm node|gtm pool a|gtm wideip a|gtm server)"
    r"\s+(?P<name>\S+)\s*\{",
    re.MULTILINE,
)
_HOSTNAME_RE = re.compile(r"hostname\s+(\S+)")
_MEMBER_RE = re.compile(r"\s+(/[A-Za-z0-9_./-]+:\d+)\s*\{")


@dataclass
class DeviceConfig:
    label: str  # short device tag e.g. "t1-a"
    hostname: str  # full hostname for the syslog field
    config_path: Path
    virtuals: list[str] = field(default_factory=list)
    pools: list[str] = field(default_factory=list)
    nodes: list[str] = field(default_factory=list)
    members: list[str] = field(default_factory=list)
    gtm_wideips: list[str] = field(default_factory=list)
    gtm_pools: list[str] = field(default_factory=list)


def _read_devices() -> list[DeviceConfig]:
    """Walk the multitier dir + invent one device per HA peer.

    Each HA pair config carries two hostnames; the loop expands into
    a device entry per host so the generator can write distinct log
    files for the A side and the B side (real F5 deployments keep
    log streams per device, not per pair).
    """
    devices: list[DeviceConfig] = []
    for conf in sorted(HERE.glob("*.conf")):
        text = conf.read_text()
        hostnames = _HOSTNAME_RE.findall(text)
        virtuals = []
        pools = []
        nodes = []
        gtm_wideips = []
        gtm_pools = []
        for m in _HEADER_RE.finditer(text):
            kind, name = m.group("kind"), m.group("name")
            if kind == "ltm virtual":
                virtuals.append(name)
            elif kind == "ltm pool":
                pools.append(name)
            elif kind == "ltm node":
                nodes.append(name)
            elif kind == "gtm pool a":
                gtm_pools.append(name)
            elif kind == "gtm wideip a":
                gtm_wideips.append(name)
        members = _MEMBER_RE.findall(text)
        if not hostnames:
            # Single-device standalones (gtm-dns, tier3): synthesise one host name.
            label = conf.stem
            hostname = f"{label.split('-')[0]}-a.example.test"
            devices.append(
                DeviceConfig(
                    label=label,
                    hostname=hostname,
                    config_path=conf,
                    virtuals=virtuals,
                    pools=pools,
                    nodes=nodes,
                    members=members,
                    gtm_wideips=gtm_wideips,
                    gtm_pools=gtm_pools,
                )
            )
            continue
        for hostname in hostnames:
            short = hostname.split(".")[0]
            devices.append(
                DeviceConfig(
                    label=short,
                    hostname=hostname,
                    config_path=conf,
                    virtuals=virtuals,
                    pools=pools,
                    nodes=nodes,
                    members=members,
                    gtm_wideips=gtm_wideips,
                    gtm_pools=gtm_pools,
                )
            )
    return devices


# ---------------------------------------------------------------------------
# Line emitter
# ---------------------------------------------------------------------------


def _emit(
    lines: list[str],
    *,
    when: str,
    host: str,
    daemon: str,
    pid: int,
    code_key: str,
    message: str,
) -> None:
    """Append one syslog-format line using a canonical F5 code."""
    code, level, severity = CODES[code_key]
    lines.append(f"{when} {host} {severity} {daemon}[{pid}]: {code}:{level}: {message}")


def _times(start_hour: int, count: int, rng: random.Random) -> list[str]:
    """Return *count* increasing timestamps spread across two days.

    Day 1 is ``Mar 14``, day 2 is ``Mar 15``; ``HH:MM:SS`` is
    monotonically increasing inside each day.  The timeline is
    deterministic given a seeded *rng* so test assertions stay
    stable across runs.
    """
    out: list[str] = []
    day1 = []
    day2 = []
    half = count // 2
    for _ in range(half):
        h = rng.randint(start_hour, 23)
        m = rng.randint(0, 59)
        s = rng.randint(0, 59)
        day1.append((h, m, s))
    for _ in range(count - half):
        h = rng.randint(0, 23)
        m = rng.randint(0, 59)
        s = rng.randint(0, 59)
        day2.append((h, m, s))
    day1.sort()
    day2.sort()
    for h, m, s in day1:
        out.append(f"Mar 14 {h:02d}:{m:02d}:{s:02d}")
    for h, m, s in day2:
        out.append(f"Mar 15 {h:02d}:{m:02d}:{s:02d}")
    return out


def _basename(full_path: str) -> str:
    return full_path.rsplit("/", 1)[-1]


# ---------------------------------------------------------------------------
# Per-device timelines
# ---------------------------------------------------------------------------


def _gen_ltm_lines(device: DeviceConfig, rng: random.Random) -> list[str]:
    """Generate ~80 lines for a tier-1 / tier-2 / tier-3 LTM device.

    Mix:
    - boot + config-load (always the first few lines)
    - HA sync state if there's a peer (label looks like ``x-a``)
    - tmsh AUDIT lines referencing real VS / pool names from the
      device's config
    - monitor down/up pairs for at least one pool member
    - one connection-limit warning per VS
    - a couple of ``err`` lines so severity filters have hits
    """
    lines: list[str] = []
    times = _times(start_hour=6, count=80, rng=rng)
    cursor = iter(times)
    pid_mcpd = rng.randint(1500, 9999)
    pid_tmm = rng.randint(10000, 19999)
    pid_bigd = rng.randint(2000, 4999)
    pid_audit = rng.randint(30000, 39999)
    host = device.hostname

    # Boot
    _emit(
        lines,
        when=next(cursor),
        host=host,
        daemon="mcpd",
        pid=pid_mcpd,
        code_key="boot",
        message=f"Boot complete on {host}",
    )
    _emit(
        lines,
        when=next(cursor),
        host=host,
        daemon="mcpd",
        pid=pid_mcpd,
        code_key="config_load",
        message="Configuration loaded from /config/bigip.conf successfully.",
    )

    # HA sync — A is Active, B is Standby
    if device.label.endswith("-a"):
        _emit(
            lines,
            when=next(cursor),
            host=host,
            daemon="mcpd",
            pid=pid_mcpd,
            code_key="ha_active",
            message=f"Device {device.label} entered Active state for /Common/{device.label.rsplit('-', 1)[0]}-failover.",
        )
    elif device.label.endswith("-b"):
        _emit(
            lines,
            when=next(cursor),
            host=host,
            daemon="mcpd",
            pid=pid_mcpd,
            code_key="ha_standby",
            message=f"Device {device.label} entered Standby state for /Common/{device.label.rsplit('-', 1)[0]}-failover.",
        )

    # AUDIT: tmsh edits referencing real VSes (one per virtual).
    for vs in device.virtuals:
        when = next(cursor, "Mar 15 12:00:00")
        _emit(
            lines,
            when=when,
            host=host,
            daemon="logger",
            pid=pid_audit,
            code_key="audit_tmsh",
            message=(
                f"AUDIT - jadmin - tmsh - tmsh -c modify ltm virtual {vs} "
                f'description "managed by netops"'
            ),
        )

    # Pool member monitor down/up cycles
    for member in device.members:
        pool = device.pools[0] if device.pools else "/Common/unknown_pool"
        _emit(
            lines,
            when=next(cursor, "Mar 15 12:00:00"),
            host=host,
            daemon="bigd",
            pid=pid_bigd,
            code_key="monitor_down",
            message=f"Pool {pool} member {member} monitor status down.",
        )
        _emit(
            lines,
            when=next(cursor, "Mar 15 12:00:00"),
            host=host,
            daemon="bigd",
            pid=pid_bigd,
            code_key="monitor_up",
            message=f"Pool {pool} member {member} monitor status up.",
        )

    # Connection limit warnings — one per VS at high water marks
    for vs in device.virtuals:
        _emit(
            lines,
            when=next(cursor, "Mar 15 12:00:00"),
            host=host,
            daemon="tmm",
            pid=pid_tmm,
            code_key="conn_limit",
            message=f"Connection limit approached on {vs}.",
        )

    # SSL handshake warning
    if any("ssl" in p.lower() or "tls" in p.lower() for p in device.pools):
        pass  # purely cosmetic — handled by other devices
    for vs in device.virtuals[:1]:
        _emit(
            lines,
            when=next(cursor, "Mar 15 13:00:00"),
            host=host,
            daemon="tmm",
            pid=pid_tmm,
            code_key="ssl_handshake_fail",
            message=f"SSL handshake failed for client 198.51.100.7 on {vs}.",
        )

    # A few err lines so severity filters have hits
    for member in device.members[:2]:
        pool = device.pools[0] if device.pools else "/Common/unknown_pool"
        _emit(
            lines,
            when=next(cursor, "Mar 15 14:00:00"),
            host=host,
            daemon="tmm",
            pid=pid_tmm,
            code_key="icmd_fail",
            message=f"monitor /Common/tcp failing for {member} (pool {pool}).",
        )

    # Saving config
    _emit(
        lines,
        when=next(cursor, "Mar 15 20:00:00"),
        host=host,
        daemon="logger",
        pid=pid_audit,
        code_key="audit_save",
        message="AUDIT - jadmin - tmsh - tmsh -c save sys config",
    )
    _emit(
        lines,
        when=next(cursor, "Mar 15 20:00:01"),
        host=host,
        daemon="mcpd",
        pid=pid_mcpd,
        code_key="save",
        message="Configuration saved to /config/bigip.conf.",
    )
    return lines


def _gen_gtm_lines(device: DeviceConfig, rng: random.Random) -> list[str]:
    """Generate GTM-specific lines: zone reload + wide-IP resolves +
    member up/down on the configured pools.

    Real GTMs emit one ``011a3055`` per A-record response when
    debug logging is on; we throttle to a couple of resolves per
    wide-IP so the file stays focused.
    """
    lines: list[str] = []
    times = _times(start_hour=6, count=60, rng=rng)
    cursor = iter(times)
    pid_mcpd = rng.randint(1500, 9999)
    pid_gtmd = rng.randint(2000, 4999)
    pid_named = rng.randint(5000, 7999)
    pid_audit = rng.randint(30000, 39999)
    host = device.hostname

    _emit(
        lines,
        when=next(cursor),
        host=host,
        daemon="mcpd",
        pid=pid_mcpd,
        code_key="boot",
        message=f"Boot complete on {host}",
    )
    _emit(
        lines,
        when=next(cursor),
        host=host,
        daemon="mcpd",
        pid=pid_mcpd,
        code_key="config_load",
        message="Configuration loaded from /config/bigip.conf successfully.",
    )
    for wideip in device.gtm_wideips:
        _emit(
            lines,
            when=next(cursor, "Mar 15 12:00:00"),
            host=host,
            daemon="named",
            pid=pid_named,
            code_key="gtm_zone_reload",
            message=f"zone {_basename(wideip)} reloaded",
        )
        for _ in range(3):
            _emit(
                lines,
                when=next(cursor, "Mar 15 12:00:00"),
                host=host,
                daemon="gtmd",
                pid=pid_gtmd,
                code_key="gtm_wideip_resolve",
                message=(
                    f"wide-IP {wideip} returning A 203.0.113.10 via /Common/border-fw:public_a"
                ),
            )
    for pool in device.gtm_pools:
        _emit(
            lines,
            when=next(cursor, "Mar 15 13:00:00"),
            host=host,
            daemon="gtmd",
            pid=pid_gtmd,
            code_key="gtm_member_down",
            message=f"Pool {pool} member /Common/border-fw:public_b transitioned to down.",
        )
        _emit(
            lines,
            when=next(cursor, "Mar 15 13:05:00"),
            host=host,
            daemon="gtmd",
            pid=pid_gtmd,
            code_key="gtm_member_up",
            message=f"Pool {pool} member /Common/border-fw:public_b transitioned to up.",
        )
    _emit(
        lines,
        when=next(cursor, "Mar 15 20:00:00"),
        host=host,
        daemon="logger",
        pid=pid_audit,
        code_key="audit_save",
        message="AUDIT - jadmin - tmsh - tmsh -c save sys config",
    )
    return lines


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    LOGS_DIR.mkdir(exist_ok=True)
    for device in _read_devices():
        rng = random.Random(hash(device.hostname) & 0xFFFF_FFFF)
        if device.gtm_wideips:
            lines = _gen_gtm_lines(device, rng)
        else:
            lines = _gen_ltm_lines(device, rng)
        out = LOGS_DIR / f"{device.label}.log"
        out.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"wrote {out.relative_to(HERE.parent.parent)} ({len(lines)} lines)")


if __name__ == "__main__":
    main()
