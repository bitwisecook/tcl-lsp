# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
    # Read-only members: rule metadata is constant per class and the
    # concrete rules are frozen dataclasses, so declaring these as
    # writable attributes would make the (invariant) protocol match fail.
    @property
    def ID(self) -> str: ...
    @property
    def SEVERITY(self) -> str: ...
    @property
    def CATEGORY(self) -> str: ...

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


def _merge_configs(configs: dict[str, BigipConfig]) -> BigipConfig:
    """Return a single :class:`BigipConfig` that unions every input.

    Rules that reason about references (orphan-monitor, pool-without-
    monitor, etc.) need to see cross-file links — a monitor declared in
    one split file may be referenced by a pool in another.  Without this
    merge, ``f5 validate split/*.conf`` would emit false-positive
    orphan findings.  Later configs win on key collisions, matching the
    order the user passed them on the CLI.
    """
    merged = BigipConfig()
    for cfg in configs.values():
        merged.data_groups.update(cfg.data_groups)
        merged.pools.update(cfg.pools)
        merged.virtual_servers.update(cfg.virtual_servers)
        merged.nodes.update(cfg.nodes)
        merged.profiles.update(cfg.profiles)
        merged.monitors.update(cfg.monitors)
        merged.snat_pools.update(cfg.snat_pools)
        merged.persistence.update(cfg.persistence)
        merged.rules.update(cfg.rules)
        merged.generic_objects.update(cfg.generic_objects)
    return merged


def run_lint(
    *,
    sources: dict[str, str],
    configs: dict[str, BigipConfig],
    category: str | None = None,
    severity: str | None = None,
) -> list[Finding]:
    """Run every registered rule against the union of *configs* and return findings.

    Inputs are merged so rules see cross-file references; passing one
    file or many is otherwise equivalent.
    """
    merged = _merge_configs(configs) if len(configs) > 1 else next(iter(configs.values()))
    findings: list[Finding] = []
    for rule in all_rules(category=category):
        for finding in rule.check(merged, sources=sources, configs=configs):
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
            from compiler.registry.info import lookup_event_info
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


# ── iRule cross-reference checks ─────────────────────────────────────
#
# These rules walk each iRule body for object references (pool /…/foo,
# class match … <dg>, persist <profile>, …) and flag any reference that
# fails to resolve against the surrounding bigip.conf.  When the merged
# config has *no* objects of a given kind at all, the corresponding
# checks are skipped — typically that means the user passed a single
# .irule file with no surrounding context, and we don't want to spam
# false positives.


def _has_config_context(cfg: BigipConfig) -> bool:
    """Return True if *cfg* carries surrounding-context objects.

    A "surrounding-context" config is one that contains objects other
    than the iRules themselves — pools, data groups, monitors, etc.
    Standalone ``.irule`` input goes through a synthetic single-rule
    config with all other collections empty; the bigip parser also
    emits the rule itself into ``generic_objects`` keyed by
    ``ltm::rule::*``, so when checking for "real" context we exclude
    those entries from the count.
    """
    if (
        cfg.pools
        or cfg.data_groups
        or cfg.persistence
        or cfg.snat_pools
        or cfg.monitors
        or cfg.profiles
        or cfg.nodes
        or cfg.virtual_servers
        or cfg.policies
    ):
        return True
    for key in cfg.generic_objects:
        if not key.startswith("ltm::rule::"):
            return True
    return False


def _classify_reference_kind(ref_kinds: tuple[str, ...]) -> str | None:
    """Map an :class:`IrulesObjectReference`'s kinds to a coarse bucket."""
    if any(k == "ltm_pool" for k in ref_kinds):
        return "pool"
    if any(k.startswith("ltm_data_group") or k == "sys_file_data_group" for k in ref_kinds):
        return "data-group"
    if any(k.startswith("ltm_persistence") for k in ref_kinds):
        return "persistence"
    if any(k == "ltm_snatpool" for k in ref_kinds):
        return "snat-pool"
    if any(k.startswith("ltm_monitor") for k in ref_kinds):
        return "monitor"
    if any(k.startswith("ltm_profile") for k in ref_kinds):
        return "profile"
    if any(k == "ltm_node" for k in ref_kinds):
        return "node"
    if any(k == "ltm_rule" for k in ref_kinds):
        return "rule"
    return None


def _resolve_reference(cfg: BigipConfig, kind: str, name: str) -> str | None:
    if kind == "pool":
        return cfg.resolve_pool(name)
    if kind == "data-group":
        return cfg.resolve_data_group(name)
    if kind == "persistence":
        return cfg.resolve_persistence(name)
    if kind == "snat-pool":
        return cfg.resolve_snat_pool(name)
    if kind == "monitor":
        return cfg.resolve_name(name, cfg.monitors)
    if kind == "profile":
        return cfg.resolve_profile(name)
    if kind == "node":
        return cfg.resolve_name(name, cfg.nodes)
    if kind == "rule":
        return cfg.resolve_rule(name)
    return None


@dataclass(frozen=True, slots=True)
class _IruleMissingObject:
    ID: str = "irule-missing-object"
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
            from ..irules_refs import extract_irules_object_references
        except ImportError:
            return

        # Degraded analysis: standalone-.irule input has a synthetic
        # single-rule config and no surrounding objects.  Without that
        # context we cannot meaningfully resolve references, so skip
        # the entire rule rather than emit false positives.
        if not _has_config_context(cfg):
            return

        for path, rule in cfg.rules.items():
            seen: set[tuple[str, str]] = set()
            for ref in extract_irules_object_references(rule.source):
                kind = _classify_reference_kind(ref.kinds)
                if kind is None:
                    continue
                # ``persist <kind> {insert|lookup} <path>`` emits a
                # heuristic candidate ref at argument_index 2 — F5
                # treats the value as a session-key string, not a
                # profile reference.  Some configs use a TMSH path
                # there as a self-imposed convention, so the resolver
                # surfaces it for query / cross-config audits, but
                # the lint must not warn when the value happens not
                # to name a real persistence profile (most session
                # keys won't).  The canonical reference form
                # ``persist <profile>`` lives at argument_index 0.
                if (
                    kind == "persistence"
                    and ref.command.lower() == "persist"
                    and ref.argument_index >= 2
                ):
                    continue
                resolved = _resolve_reference(cfg, kind, ref.name)
                if resolved is not None:
                    continue
                key = (kind, ref.name)
                if key in seen:
                    continue
                seen.add(key)
                yield Finding(
                    rule_id=f"irule-missing-{kind}",
                    severity=self.SEVERITY,
                    category=self.CATEGORY,
                    full_path=path,
                    message=(
                        f"iRule references {kind} {ref.name!r} "
                        f"(via `{ref.command}`) but no such object exists"
                    ),
                )


# Register the built-in rules in a deterministic order.
register(_OrphanMonitor())
register(_EmptyPool())
register(_VirtualWithoutPool())
register(_PoolWithoutMonitor())
register(_IruleDeprecatedCommand())
register(_IruleEmptyWhenBlock())
register(_IruleUnknownEvent())
register(_IruleMissingObject())
