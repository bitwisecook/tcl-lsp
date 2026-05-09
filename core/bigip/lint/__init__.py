"""Lint registry for ``f5 validate`` and ``f5 irule lint``.

Each rule is a small module exposing:

- ``ID``: string id (e.g. ``"orphan-monitor"``)
- ``SEVERITY``: ``"error"``, ``"warning"``, or ``"info"``
- ``CATEGORY``: ``"config"`` or ``"irule"``
- ``check(cfg, sources, configs) -> Iterable[Finding]``

The :func:`run_lint` helper iterates every registered rule and yields
findings in ``(severity, full_path, rule_id, message)`` order.
"""

from __future__ import annotations

import re
from collections.abc import Iterable
from dataclasses import dataclass
from typing import Callable, Protocol

from ..link_extract import build_bigip_object_graph
from ..model import BigipConfig

CATEGORIES = ("config", "irule")
SEVERITIES = ("error", "warning", "info")


@dataclass(frozen=True, slots=True)
class Finding:
    rule_id: str
    severity: str
    category: str
    full_path: str
    message: str


class LintRule(Protocol):
    ID: str
    SEVERITY: str
    CATEGORY: str

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]: ...


_RULES: list[LintRule] = []


def register(rule: LintRule) -> LintRule:
    _RULES.append(rule)
    return rule


def all_rules(category: str | None = None) -> list[LintRule]:
    if category is None:
        return list(_RULES)
    return [r for r in _RULES if r.CATEGORY == category]


def run_lint(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    category: str | None = None,
    severity: str | None = None,
) -> list[Finding]:
    """Run every registered rule across *configs* and return all findings."""
    findings: list[Finding] = []
    for rule in all_rules(category=category):
        for cfg in configs.values():
            for finding in rule.check(cfg, sources=sources, configs=configs):
                if severity is not None and finding.severity != severity:
                    continue
                findings.append(finding)
    return findings


# ── Built-in rules ───────────────────────────────────────────────────


def _is_root_kind(kind: str | None) -> bool:
    from ..cleanup import _is_root_kind as impl

    return impl(kind)


@dataclass(frozen=True, slots=True)
class _OrphanMonitor:
    ID: str = "orphan-monitor"
    SEVERITY: str = "warning"
    CATEGORY: str = "config"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        # A monitor is orphan if no pool's monitor field references it
        # and no pool member's monitor field references it.
        in_use: set[str] = set()
        for pool in cfg.pools.values():
            if pool.monitor:
                resolved = cfg.resolve_name(pool.monitor.split(" ", 1)[0], cfg.monitors)
                if resolved:
                    in_use.add(resolved)
            for member in pool.members:
                if member.monitor:
                    resolved = cfg.resolve_name(member.monitor.split(" ", 1)[0], cfg.monitors)
                    if resolved:
                        in_use.add(resolved)
        for path, monitor in cfg.monitors.items():
            if path in in_use:
                continue
            if path.startswith("/Common/") and monitor.full_path == path:
                # /Common/<factory monitor> may be referenced from outside this file.
                # Mark as info, not warning.
                yield Finding(
                    rule_id=self.ID,
                    severity="info",
                    category=self.CATEGORY,
                    full_path=path,
                    message="monitor not referenced by any pool in this config",
                )
            else:
                yield Finding(
                    rule_id=self.ID,
                    severity=self.SEVERITY,
                    category=self.CATEGORY,
                    full_path=path,
                    message="monitor not referenced by any pool",
                )


@dataclass(frozen=True, slots=True)
class _EmptyPool:
    ID: str = "empty-pool"
    SEVERITY: str = "warning"
    CATEGORY: str = "config"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        for path, pool in cfg.pools.items():
            if not pool.members:
                yield Finding(
                    rule_id=self.ID,
                    severity=self.SEVERITY,
                    category=self.CATEGORY,
                    full_path=path,
                    message="pool has no members",
                )


@dataclass(frozen=True, slots=True)
class _VirtualWithoutPool:
    ID: str = "virtual-without-pool"
    SEVERITY: str = "info"
    CATEGORY: str = "config"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        for path, vs in cfg.virtual_servers.items():
            if vs.pool:
                continue
            # No default pool AND no iRules — likely a misconfigured forwarding VIP.
            if vs.rules:
                continue
            yield Finding(
                rule_id=self.ID,
                severity=self.SEVERITY,
                category=self.CATEGORY,
                full_path=path,
                message="virtual has no default pool and no iRules attached",
            )


@dataclass(frozen=True, slots=True)
class _PoolWithoutMonitor:
    ID: str = "pool-without-monitor"
    SEVERITY: str = "warning"
    CATEGORY: str = "config"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        for path, pool in cfg.pools.items():
            if pool.monitor:
                continue
            if not pool.members:
                continue  # already covered by empty-pool
            if any(m.monitor for m in pool.members):
                continue
            yield Finding(
                rule_id=self.ID,
                severity=self.SEVERITY,
                category=self.CATEGORY,
                full_path=path,
                message="pool has no health monitor (pool-level or per-member)",
            )


_DEPRECATED_IRULE_COMMANDS = {
    "X509::extensions": "X509::extensions is deprecated; use X509::extensions_count + X509::extensions_get",
    "stream::expression": "use STREAM::expression",
    "log_local0": "use 'log local0.<facility> ...' (no underscore form)",
}

_WHEN_RE = re.compile(r"\bwhen\s+([A-Z][A-Z0-9_]*)\b")


@dataclass(frozen=True, slots=True)
class _IruleDeprecatedCommand:
    ID: str = "irule-deprecated-command"
    SEVERITY: str = "warning"
    CATEGORY: str = "irule"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        for path, rule in cfg.rules.items():
            for cmd, msg in _DEPRECATED_IRULE_COMMANDS.items():
                if cmd in rule.source:
                    yield Finding(
                        rule_id=self.ID,
                        severity=self.SEVERITY,
                        category=self.CATEGORY,
                        full_path=path,
                        message=msg,
                    )


@dataclass(frozen=True, slots=True)
class _IruleEmptyWhenBlock:
    ID: str = "irule-empty-when"
    SEVERITY: str = "info"
    CATEGORY: str = "irule"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        # Cheap regex: detect `when EVENT { }` (only whitespace / comments).
        empty_re = re.compile(r"when\s+[A-Z_][A-Z0-9_]*\s*\{\s*(?:#[^\n]*\n\s*)*\}")
        for path, rule in cfg.rules.items():
            if empty_re.search(rule.source):
                yield Finding(
                    rule_id=self.ID,
                    severity=self.SEVERITY,
                    category=self.CATEGORY,
                    full_path=path,
                    message="iRule contains a `when` block with no statements",
                )


@dataclass(frozen=True, slots=True)
class _IruleUnknownEvent:
    ID: str = "irule-unknown-event"
    SEVERITY: str = "warning"
    CATEGORY: str = "irule"

    def check(
        self,
        cfg: BigipConfig,
        *,
        sources: dict[str, str],
        configs: dict[str, BigipConfig],
    ) -> Iterable[Finding]:
        try:
            from core.commands.registry.info import lookup_event_info
        except ImportError:
            return
        for path, rule in cfg.rules.items():
            for event in set(_WHEN_RE.findall(rule.source)):
                info = lookup_event_info(event, dialect="f5-irules")
                if not info.known:
                    yield Finding(
                        rule_id=self.ID,
                        severity=self.SEVERITY,
                        category=self.CATEGORY,
                        full_path=path,
                        message=f"iRule references unknown event {event!r}",
                    )


# Register the built-in rules in a deterministic order.
register(_OrphanMonitor())
register(_EmptyPool())
register(_VirtualWithoutPool())
register(_PoolWithoutMonitor())
register(_IruleDeprecatedCommand())
register(_IruleEmptyWhenBlock())
register(_IruleUnknownEvent())
