"""Cross-reference validator for BIG-IP configurations.

Checks that iRules reference objects (data-groups, pools, profiles, etc.)
that actually exist in the parsed BIG-IP configuration, and that virtual
servers reference valid iRules, pools, and profiles.

Diagnostic codes
================

- **BIGIP6001** (WARNING): iRule references data-group not found in config
- **BIGIP6002** (WARNING): iRule references pool not found in config
- **BIGIP6003** (WARNING): virtual server references iRule not found in config
- **BIGIP6004** (HINT): iRule uses command requiring profile not on virtual
- **BIGIP6005** (WARNING): virtual server references pool not found in config
- **BIGIP6006** (HINT): data-group defined but never referenced by any iRule
- **BIGIP6007** (WARNING): iRule references SNAT pool not found in config
- **BIGIP6008** (HINT): pool has no members
- **BIGIP6009** (WARNING): virtual server has duplicate iRule attachment
- **BIGIP6010** (HINT): iRule ``persist`` references profile not on virtual
- **BIGIP6011** (WARNING): invalid IP address in IP-type data-group record
"""

from __future__ import annotations

import ipaddress
import re

from ..analysis.semantic_model import Diagnostic, Range, Severity
from ..common.source_map import SourceMap
from ..parsing.tokens import SourcePosition
from .model import BigipConfig, BigipRule, ProfileType

# iRule source scanning helpers

# class match/search: class <subcmd> ?opts? <item> <operator> <dg_name>
# class lookup:       class lookup ?--? <name> <dg_name>
# class exists/size/type/get/startsearch: class <subcmd> <dg_name>

_CLASS_OPERATORS = r"equals|starts_with|ends_with|contains|matches_glob|matches_regex"

# class match/search — data-group name is the argument AFTER the operator
_CLASS_MATCH_RE = re.compile(
    r"\bclass\s+(?:match|search)\s+"
    r"(?:(?:-\w+\s+)*)"  # optional flags
    r"(?:--\s+)?"  # optional --
    r"\S+\s+"  # item (may be [cmd], $var, literal)
    r"(?:" + _CLASS_OPERATORS + r")\s+"
    r"(\S+)",  # capture data-group name
    re.DOTALL,
)

# class lookup — data-group name is the last argument
_CLASS_LOOKUP_RE = re.compile(
    r"\bclass\s+lookup\s+"
    r"(?:--\s+)?"
    r"\S+\s+"  # name/key argument
    r"(\S+)",  # capture data-group name
    re.DOTALL,
)

# class exists/size/type/get/startsearch — data-group name is the sole argument
_CLASS_SINGLE_RE = re.compile(
    r"\bclass\s+(?:exists|size|type|get|startsearch)\s+"
    r"(\S+)",  # capture data-group name
)

# Match: pool <pool_name>
_POOL_CMD_RE = re.compile(
    r"\bpool\s+"
    r"(?!member\b)"  # exclude "pool member" subcommand
    r"(/[\w/.-]+|\w[\w.-]*)",
)

# Match: snatpool <name>
_SNATPOOL_CMD_RE = re.compile(
    r"\bsnatpool\s+(/[\w/.-]+|\w[\w.-]*)",
)

# Match: persist <profile_name>
_PERSIST_CMD_RE = re.compile(
    r"\bpersist\s+"
    r"(?!none\b)"
    r"(/[\w/.-]+|\w[\w.-]*)",
)

# HTTP:: commands that require an HTTP profile
_HTTP_COMMANDS_RE = re.compile(
    r"\bHTTP::\w+",
)

# SSL:: commands that require a client-ssl or server-ssl profile
_SSL_COMMANDS_RE = re.compile(
    r"\b(?:SSL|ssl)::\w+",
)


def _range_from_match(source_map: SourceMap, match: re.Match, group: int = 0) -> Range:
    """Create a Range from a regex match."""
    start = match.start(group)
    end = match.end(group)
    return source_map.range_from_offsets(start, max(start, end - 1))


def _clean_name(name: str) -> str:
    """Strip braces, quotes, and brackets from a captured name."""
    name = name.strip("{}\"'[]")
    return name


# iRule-level checks (run per iRule body)


def _iter_class_dg_references(
    source: str,
) -> list[tuple[re.Match, str]]:
    """Return ``(match, data_group_name)`` for all class command data-group refs."""
    results: list[tuple[re.Match, str]] = []
    for regex in (_CLASS_MATCH_RE, _CLASS_LOOKUP_RE, _CLASS_SINGLE_RE):
        for m in regex.finditer(source):
            name = _clean_name(m.group(1))
            if name.startswith("$") or name.startswith("["):
                continue
            results.append((m, name))
    return results


def _check_irule_data_groups(rule: BigipRule, config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6001: iRule references data-group not found in config."""
    diagnostics: list[Diagnostic] = []
    if not rule.source:
        return diagnostics
    source_map = SourceMap(rule.source)

    for m, dg_name in _iter_class_dg_references(rule.source):
        if config.resolve_data_group(dg_name) is None:
            diagnostics.append(
                Diagnostic(
                    range=_range_from_match(source_map, m, 1),
                    message=(f"Data-group '{dg_name}' not found in BIG-IP configuration."),
                    severity=Severity.WARNING,
                    code="BIGIP6001",
                )
            )
    return diagnostics


def _check_irule_pools(rule: BigipRule, config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6002: iRule references pool not found in config."""
    diagnostics: list[Diagnostic] = []
    if not rule.source:
        return diagnostics
    source_map = SourceMap(rule.source)

    for m in _POOL_CMD_RE.finditer(rule.source):
        pool_name = _clean_name(m.group(1))
        if pool_name.startswith("$") or pool_name.startswith("["):
            continue
        if config.resolve_pool(pool_name) is None:
            diagnostics.append(
                Diagnostic(
                    range=_range_from_match(source_map, m, 1),
                    message=f"Pool '{pool_name}' not found in BIG-IP configuration.",
                    severity=Severity.WARNING,
                    code="BIGIP6002",
                )
            )
    return diagnostics


def _check_irule_snatpools(rule: BigipRule, config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6007: iRule references SNAT pool not found in config."""
    diagnostics: list[Diagnostic] = []
    if not rule.source:
        return diagnostics
    source_map = SourceMap(rule.source)

    for m in _SNATPOOL_CMD_RE.finditer(rule.source):
        sp_name = _clean_name(m.group(1))
        if sp_name.startswith("$") or sp_name.startswith("["):
            continue
        if config.resolve_snat_pool(sp_name) is None:
            diagnostics.append(
                Diagnostic(
                    range=_range_from_match(source_map, m, 1),
                    message=(f"SNAT pool '{sp_name}' not found in BIG-IP configuration."),
                    severity=Severity.WARNING,
                    code="BIGIP6007",
                )
            )
    return diagnostics


def _collect_referenced_data_groups(rule: BigipRule) -> set[str]:
    """Return all data-group names referenced in an iRule body."""
    if not rule.source:
        return set()
    return {name for _, name in _iter_class_dg_references(rule.source)}


# Virtual-server-level checks


def _check_virtual_rules(config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6003: Virtual server references iRule not found in config."""
    diagnostics: list[Diagnostic] = []
    for vs in config.virtual_servers.values():
        seen_rules: set[str] = set()
        for rule_ref in vs.rules:
            # BIGIP6009: duplicate iRule attachment
            if rule_ref in seen_rules:
                diagnostics.append(
                    Diagnostic(
                        range=vs.range or _null_range(),
                        message=(
                            f"Virtual server '{vs.name}' has duplicate "
                            f"iRule attachment '{rule_ref}'."
                        ),
                        severity=Severity.WARNING,
                        code="BIGIP6009",
                    )
                )
            seen_rules.add(rule_ref)

            if config.resolve_rule(rule_ref) is None:
                diagnostics.append(
                    Diagnostic(
                        range=vs.range or _null_range(),
                        message=(
                            f"Virtual server '{vs.name}' references "
                            f"iRule '{rule_ref}' which is not defined."
                        ),
                        severity=Severity.WARNING,
                        code="BIGIP6003",
                    )
                )
    return diagnostics


def _check_virtual_pools(config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6005: Virtual server references pool not found in config."""
    diagnostics: list[Diagnostic] = []
    for vs in config.virtual_servers.values():
        if vs.pool and config.resolve_pool(vs.pool) is None:
            diagnostics.append(
                Diagnostic(
                    range=vs.range or _null_range(),
                    message=(
                        f"Virtual server '{vs.name}' references "
                        f"pool '{vs.pool}' which is not defined."
                    ),
                    severity=Severity.WARNING,
                    code="BIGIP6005",
                )
            )
    return diagnostics


def _check_virtual_profile_requirements(
    config: BigipConfig,
) -> list[Diagnostic]:
    """BIGIP6004: iRule uses commands requiring profiles not on its virtual."""
    diagnostics: list[Diagnostic] = []
    for vs in config.virtual_servers.values():
        ptypes = config.profile_types_for_virtual(vs.full_path)
        has_http = ProfileType.HTTP in ptypes
        has_ssl = ProfileType.CLIENT_SSL in ptypes or ProfileType.SERVER_SSL in ptypes

        for rule_ref in vs.rules:
            resolved = config.resolve_rule(rule_ref)
            if resolved is None:
                continue
            rule = config.rules[resolved]
            if not rule.source:
                continue

            if not has_http and _HTTP_COMMANDS_RE.search(rule.source):
                diagnostics.append(
                    Diagnostic(
                        range=vs.range or _null_range(),
                        message=(
                            f"iRule '{rule.name}' on virtual '{vs.name}' uses "
                            f"HTTP:: commands but no HTTP profile is attached."
                        ),
                        severity=Severity.HINT,
                        code="BIGIP6004",
                    )
                )

            if not has_ssl and _SSL_COMMANDS_RE.search(rule.source):
                diagnostics.append(
                    Diagnostic(
                        range=vs.range or _null_range(),
                        message=(
                            f"iRule '{rule.name}' on virtual '{vs.name}' uses "
                            f"SSL:: commands but no SSL profile is attached."
                        ),
                        severity=Severity.HINT,
                        code="BIGIP6004",
                    )
                )
    return diagnostics


def _check_virtual_persistence(
    config: BigipConfig,
) -> list[Diagnostic]:
    """BIGIP6010: iRule ``persist`` references profile not on its virtual."""
    diagnostics: list[Diagnostic] = []
    for vs in config.virtual_servers.values():
        vs_persist_set = set(vs.persist)
        for rule_ref in vs.rules:
            resolved = config.resolve_rule(rule_ref)
            if resolved is None:
                continue
            rule = config.rules[resolved]
            if not rule.source:
                continue
            for m in _PERSIST_CMD_RE.finditer(rule.source):
                persist_name = _clean_name(m.group(1))
                if persist_name.startswith("$") or persist_name.startswith("["):
                    continue
                # Check if this persistence profile is attached to the VS
                resolved_persist = config.resolve_persistence(persist_name)
                if resolved_persist is None:
                    continue  # Unknown profile — BIGIP6002 will catch missing objects
                # Check if it's in the VS persist list
                in_vs = False
                for vp in vs_persist_set:
                    if config.resolve_persistence(vp) == resolved_persist:
                        in_vs = True
                        break
                if not in_vs:
                    diagnostics.append(
                        Diagnostic(
                            range=vs.range or _null_range(),
                            message=(
                                f"iRule '{rule.name}' on virtual '{vs.name}' "
                                f"uses persistence profile '{persist_name}' "
                                f"which is not attached to the virtual server."
                            ),
                            severity=Severity.HINT,
                            code="BIGIP6010",
                        )
                    )
    return diagnostics


# Config-wide checks


def _check_unused_data_groups(config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6006: Data-group defined but never referenced by any iRule."""
    diagnostics: list[Diagnostic] = []
    all_referenced: set[str] = set()
    for rule in config.rules.values():
        raw_names = _collect_referenced_data_groups(rule)
        for raw in raw_names:
            resolved = config.resolve_data_group(raw)
            if resolved:
                all_referenced.add(resolved)

    for dg_path, dg in config.data_groups.items():
        if dg_path not in all_referenced:
            diagnostics.append(
                Diagnostic(
                    range=dg.range or _null_range(),
                    message=(
                        f"Data-group '{dg.name}' is defined but not "
                        f"referenced by any iRule in this configuration."
                    ),
                    severity=Severity.HINT,
                    code="BIGIP6006",
                )
            )
    return diagnostics


def _check_empty_pools(config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6008: Pool has no members."""
    diagnostics: list[Diagnostic] = []
    for pool in config.pools.values():
        if not pool.members:
            diagnostics.append(
                Diagnostic(
                    range=pool.range or _null_range(),
                    message=f"Pool '{pool.name}' has no members defined.",
                    severity=Severity.HINT,
                    code="BIGIP6008",
                )
            )
    return diagnostics


# Helpers


def _null_range() -> Range:
    """Return a zero-width range at position 0 (fallback)."""
    pos = SourcePosition(line=0, character=0, offset=0)
    return Range(start=pos, end=pos)


# Public API


def validate_bigip_config(config: BigipConfig) -> list[Diagnostic]:
    """Run all BIG-IP cross-reference validations.

    Returns a list of :class:`Diagnostic` objects covering the entire
    configuration.
    """
    diagnostics: list[Diagnostic] = []

    # Per-iRule checks
    for rule in config.rules.values():
        diagnostics.extend(_check_irule_data_groups(rule, config))
        diagnostics.extend(_check_irule_pools(rule, config))
        diagnostics.extend(_check_irule_snatpools(rule, config))

    # Virtual server reference checks
    diagnostics.extend(_check_virtual_rules(config))
    diagnostics.extend(_check_virtual_pools(config))
    diagnostics.extend(_check_virtual_profile_requirements(config))
    diagnostics.extend(_check_virtual_persistence(config))

    # Config-wide checks
    diagnostics.extend(_check_unused_data_groups(config))
    diagnostics.extend(_check_empty_pools(config))

    # Data-group checks
    diagnostics.extend(_check_ip_data_group_records(config))

    return diagnostics


def _check_ip_data_group_records(config: BigipConfig) -> list[Diagnostic]:
    """BIGIP6011: Validate records in IP-type data-groups.

    Checks that each record key in an ``ip``-type data-group is a valid
    IPv4 or IPv6 address (with optional /prefix).
    """
    diagnostics: list[Diagnostic] = []
    for dg in config.data_groups.values():
        if dg.value_type != "ip":
            continue
        for record in dg.records:
            # Records may contain host-and-network forms: "10.0.0.0/8"
            addr_text = record.split("/")[0].strip() if "/" in record else record.strip()
            if not addr_text:
                continue
            try:
                ipaddress.ip_address(addr_text)
            except ValueError:
                # Try as network (e.g. "10.0.0.0/8")
                try:
                    ipaddress.ip_network(record.strip(), strict=False)
                except ValueError:
                    diagnostics.append(
                        Diagnostic(
                            range=dg.range or _null_range(),
                            message=(
                                f"Invalid IP address '{record}' in IP-type data-group '{dg.name}'"
                            ),
                            severity=Severity.WARNING,
                            code="BIGIP6011",
                        )
                    )
    return diagnostics
