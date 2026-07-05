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

"""Verb registry for the ``f5`` CLI.

Mirrors :mod:`tooling.tcl.verbs._registry` but keeps the f5 verb list in
its own module-global so ``f5`` and ``tcl`` / ``irule`` brief-help
output do not bleed into one another.  The two registries are
intentionally decoupled — refactoring one must not require changing
the other.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from typing import Callable


@dataclass
class _VerbSpec:
    name: str
    configure: Callable
    aliases: tuple[str, ...]
    help: str
    formatter_class: type = argparse.HelpFormatter


_VERB_REGISTRY: list[_VerbSpec] = []


def verb(
    name: str,
    *,
    aliases: tuple[str, ...] | list[str] = (),
    help: str = "",
    formatter_class: type = argparse.HelpFormatter,
) -> Callable:
    """Decorator: register a verb-configuration function in the f5 CLI."""

    def decorator(fn: Callable) -> Callable:
        _VERB_REGISTRY.append(
            _VerbSpec(
                name=name,
                configure=fn,
                aliases=tuple(aliases),
                help=help,
                formatter_class=formatter_class,
            )
        )
        return fn

    return decorator


def get_verb_catalogue() -> list[tuple[str, str, str]]:
    """Return ``(name, primary_alias, help)`` tuples for the brief help screen."""
    return [
        (spec.name, spec.aliases[0] if spec.aliases else "", spec.help) for spec in _VERB_REGISTRY
    ]


def apply_verb_registrations(
    sub: argparse._SubParsersAction,  # noqa: SLF001
    *,
    prog_name: str,
    default_dialect: str,
) -> None:
    """Add every registered f5 verb to *sub*."""
    for spec in _VERB_REGISTRY:
        p = sub.add_parser(
            spec.name,
            aliases=list(spec.aliases),
            help=spec.help,
            formatter_class=spec.formatter_class,
        )
        spec.configure(p, prog_name=prog_name, default_dialect=default_dialect)
