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

"""Compact, language-agnostic presence rows for the registries.

This module is the single source of truth for the committed *presence
safety-net* of the command / event / profile / object registries.  It
produces small, line-oriented rows (one per command, event, profile, and
object) that serialise to the CSV golden fixtures under
``tests/baselines/registry/``.

The rows are consumed by:

- ``scripts/codegen/registry_baselines.py``, which writes / checks the
  CSV presence fixtures under ``tests/baselines/registry/``;
- the front-end-behaviour contract tests under
  ``tests/registry_contract/``.

CSVs are the only committed shape: a registry change produces a tiny,
reviewable diff (one row per command), not a multi-megabyte JSON blob.
The heavy coverage is behavioural — the registry generates real
scripts/iRules that the front-ends analyse — and structural invariants
read the in-memory registry directly.  See
``docs/design/contracts/registry-contract-tests.md``.
"""

from __future__ import annotations

from typing import Any

from compiler.registry import REGISTRY
from compiler.registry.info import lookup_event_info
from compiler.registry.signatures import Arity

# Dialects whose command registry we snapshot.  Ordered for stable output.
TCL_DIALECTS: tuple[str, ...] = ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1")
F5_DIALECTS: tuple[str, ...] = ("f5-irules", "f5-iapps")
ALL_DIALECTS: tuple[str, ...] = (*TCL_DIALECTS, *F5_DIALECTS)


def resolve_spec(name: str, dialect: str) -> Any:
    """Deterministically pick the ``CommandSpec`` for *name* in *dialect*.

    A handful of command names are overloaded across dialects (e.g.
    ``event`` is a subcommand ensemble in core Tcl but a bare command in
    f5-iRules).  ``CommandRegistry.get`` resolves such names in
    registration order, which depends on which dialect a process queried
    first — so the rows would drift between processes.  Here we resolve
    order-independently: among the specs that support *dialect*, prefer
    the most dialect-specific (a spec scoped to a small dialect set beats
    the catch-all ``dialects=None``), then the smallest scope, with the
    command name as a final stable tie-break.  The rows are derived from
    the spec this returns, so the fixtures are stable regardless of load
    history.
    """
    specs = [s for s in REGISTRY.specs_by_name.get(name, ()) if s.supports_dialect(dialect)]
    if not specs:
        return None

    def sort_key(spec: Any) -> tuple[int, int, str]:
        scoped = spec.dialects is not None
        size = len(spec.dialects) if spec.dialects is not None else 1_000_000
        return (0 if scoped else 1, size, spec.name)

    return sorted(specs, key=sort_key)[0]


def _subcommand_names(spec: Any, dialect: str) -> list[str]:
    """Sorted subcommand names of *spec* available in *dialect* (spec-local)."""
    return sorted(spec.subcommands_for_dialect(dialect))


# Compact presence rows
#
# The committed golden is a set of small CSVs proving every command,
# event, profile, and object is present in the registry with basic data
# (arities for commands).  The heavy behavioural coverage lives in the
# generated front-end tests; these rows are only the presence safety-net.


def _arity_cell(arity: Arity | None) -> tuple[str, str]:
    if arity is None:
        return "", ""
    return str(arity.min), ("" if arity.is_unlimited else str(arity.max))


def command_rows(dialect: str) -> list[dict[str, str]]:
    """One compact presence row per command in *dialect* (name + arity + counts)."""
    rows: list[dict[str, str]] = []
    for name in REGISTRY.command_names(dialect):
        spec = resolve_spec(name, dialect)
        if spec is None:  # pragma: no cover
            continue
        amin, amax = _arity_cell(spec.validation.arity if spec.validation is not None else None)
        rows.append(
            {
                "dialect": dialect,
                "command": name,
                "arity_min": amin,
                "arity_max": amax,
                "subcommands": str(len(_subcommand_names(spec, dialect))),
                "switches": str(len(spec.switch_names(dialect))),
            }
        )
    return rows


def all_command_rows(dialects: tuple[str, ...] = ALL_DIALECTS) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for dialect in dialects:
        rows.extend(command_rows(dialect))
    return rows


def event_rows(dialect: str = "f5-irules") -> list[dict[str, str]]:
    from compiler.registry.namespace_registry import NAMESPACE_REGISTRY

    rows: list[dict[str, str]] = []
    for name in sorted(NAMESPACE_REGISTRY.all_event_names()):
        info = lookup_event_info(name, dialect=dialect)
        rows.append(
            {
                "event": name,
                "known": str(info.known).lower(),
                "deprecated": str(info.deprecated).lower(),
                "side": info.side,
                "multiplicity": info.multiplicity,
                "valid_commands": str(info.valid_command_count),
            }
        )
    return rows


def profile_rows() -> list[dict[str, str]]:
    from compiler.registry import namespace_data as nd

    rows: list[dict[str, str]] = []
    for name, profile in sorted(nd.PROFILE_SPECS.items()):
        rows.append(
            {
                "profile": name,
                "layer": str(profile.layer),
                "side": str(profile.side),
            }
        )
    return rows


def object_rows() -> list[dict[str, str]]:
    from dialects.f5.bigip.registry.data import OBJECT_KIND_SPECS

    rows: list[dict[str, str]] = []
    for kind, spec in sorted(OBJECT_KIND_SPECS.items()):
        rows.append(
            {
                "kind": kind,
                "module": spec.module or "",
                "object_types": "|".join(spec.object_types),
            }
        )
    return rows
