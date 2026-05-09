"""SCF → AS3 declaration emitter (best-effort, partial coverage).

AS3 (F5 Application Services 3) is a declarative API for BIG-IP
configuration.  This module converts the most common LTM objects from
a parsed :class:`BigipConfig` into an AS3 ``Application``.  Anything
that doesn't have a clear AS3 mapping is recorded in
:attr:`AS3ConversionReport.unmapped` so the user knows what was lost.

References: F5 AS3 schema reference (well-known F5 documentation).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from ..model import BigipConfig, ProfileType


@dataclass
class AS3ConversionReport:
    declaration: dict
    mapped: int = 0
    unmapped: list[str] = field(default_factory=list)


def _short_name(full_path: str) -> str:
    return full_path.rsplit("/", 1)[-1] if full_path else ""


def _split_destination(dest: str) -> tuple[str, int | None]:
    """Parse ``/Common/10.0.0.5:80`` -> ``("10.0.0.5", 80)``."""
    if not dest:
        return "", None
    base = dest.rsplit("/", 1)[-1]
    if ":" in base:
        host, _, port = base.rpartition(":")
        try:
            return host, int(port)
        except ValueError:
            return base, None
    return base, None


def _convert_pool(cfg: BigipConfig, name: str) -> dict:
    pool = cfg.pools[name]
    members: list[dict] = []
    for member in pool.members:
        addr = member.address or _split_destination(member.name)[0]
        port = member.port or _split_destination(member.name)[1] or 0
        members.append(
            {
                "servicePort": int(port),
                "serverAddresses": [addr] if addr else [],
            }
        )
    out: dict[str, object] = {
        "class": "Pool",
        "members": members,
    }
    if pool.load_balancing_mode:
        out["loadBalancingMode"] = pool.load_balancing_mode
    if pool.monitor:
        first = pool.monitor.split(" ", 1)[0]
        monitor_name = _short_name(first)
        if monitor_name:
            out["monitors"] = [{"use": monitor_name}]
    return out


_PROFILE_TYPE_TO_AS3 = {
    ProfileType.HTTP: "HTTP_Profile",
    ProfileType.TCP: "TCP_Profile",
    ProfileType.UDP: "UDP_Profile",
    ProfileType.CLIENT_SSL: "TLS_Server",
    ProfileType.SERVER_SSL: "TLS_Client",
    ProfileType.ONE_CONNECT: "Multiplex_Profile",
}


def _classify_virtual_class(cfg: BigipConfig, vs_name: str) -> str:
    types = cfg.profile_types_for_virtual(vs_name)
    if ProfileType.HTTP in types:
        return "Service_HTTP"
    if ProfileType.CLIENT_SSL in types:
        return "Service_HTTPS"
    if ProfileType.UDP in types:
        return "Service_UDP"
    if ProfileType.TCP in types:
        return "Service_TCP"
    return "Service_L4"


def _convert_virtual(cfg: BigipConfig, name: str) -> dict:
    vs = cfg.virtual_servers[name]
    addr, port = _split_destination(vs.destination)
    out: dict[str, object] = {
        "class": _classify_virtual_class(cfg, name),
        "virtualAddresses": [addr] if addr else [],
    }
    if port is not None:
        out["virtualPort"] = port
    if vs.pool:
        pool_name = _short_name(cfg.resolve_pool(vs.pool) or vs.pool)
        out["pool"] = pool_name
    if vs.rules:
        out["iRules"] = [_short_name(cfg.resolve_rule(r) or r) for r in vs.rules]
    if vs.persist:
        out["persistenceMethods"] = [
            _short_name(cfg.resolve_persistence(p) or p) for p in vs.persist
        ]
    return out


def _convert_irule(cfg: BigipConfig, name: str) -> dict:
    rule = cfg.rules[name]
    return {"class": "iRule", "iRule": rule.source}


def _convert_monitor(cfg: BigipConfig, name: str) -> dict:
    monitor = cfg.monitors[name]
    return {
        "class": "Monitor",
        "monitorType": monitor.monitor_type or "tcp",
    }


def scf_to_as3(
    cfg: BigipConfig,
    *,
    tenant: str = "Common",
    application: str = "app",
) -> AS3ConversionReport:
    """Convert *cfg* to a minimal AS3 declaration.

    Output shape::

        {
          "class": "AS3",
          "declaration": {
            "class": "ADC",
            "schemaVersion": "3.45.0",
            "<tenant>": {
              "class": "Tenant",
              "<application>": {
                "class": "Application",
                "template": "generic",
                "<object_name>": { ... },
                ...
              }
            }
          }
        }
    """
    app: dict[str, object] = {
        "class": "Application",
        "template": "generic",
    }
    mapped = 0
    unmapped: list[str] = []

    for name in cfg.pools:
        app[_short_name(name)] = _convert_pool(cfg, name)
        mapped += 1
    for name in cfg.virtual_servers:
        app[_short_name(name)] = _convert_virtual(cfg, name)
        mapped += 1
    for name in cfg.rules:
        app[_short_name(name)] = _convert_irule(cfg, name)
        mapped += 1
    for name in cfg.monitors:
        app[_short_name(name)] = _convert_monitor(cfg, name)
        mapped += 1

    # Profiles, data-groups, persistence, snat-pools and generics get
    # listed as unmapped — supporting them needs deeper schema work.
    for name in cfg.profiles:
        unmapped.append(f"profile:{name}")
    for name in cfg.data_groups:
        unmapped.append(f"data-group:{name}")
    for name in cfg.persistence:
        unmapped.append(f"persistence:{name}")
    for name in cfg.snat_pools:
        unmapped.append(f"snatpool:{name}")
    for name in cfg.nodes:
        # Nodes are implicit in pool member serverAddresses; flagging
        # standalone nodes as unmapped is correct.
        unmapped.append(f"node:{name}")
    for name, obj in cfg.generic_objects.items():
        if (obj.module, obj.object_type) in {
            ("ltm", "pool"),
            ("ltm", "virtual"),
            ("ltm", "rule"),
            ("ltm", "monitor"),
            ("ltm", "node"),
            ("ltm", "profile"),
            ("ltm", "data-group"),
            ("ltm", "persistence"),
            ("ltm", "snatpool"),
        }:
            continue
        unmapped.append(f"generic:{name}")

    declaration = {
        "class": "AS3",
        "declaration": {
            "class": "ADC",
            "schemaVersion": "3.45.0",
            tenant: {
                "class": "Tenant",
                application: app,
            },
        },
    }
    return AS3ConversionReport(
        declaration=declaration,
        mapped=mapped,
        unmapped=unmapped,
    )
