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

"""Per-package registry for iApps and tmsh command definitions."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_registry  # noqa: F401
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

_IAPPS_ONLY = frozenset({"f5-iapps"})
# tmsh commands are available in both iApps and standalone tmsh scripts.
_TMSH_DIALECTS = frozenset({"f5-iapps", "f5-tmsh"})

_SOURCE_TMSH = "F5 TMSH Scripting Reference"

_REGISTRY, register = make_registry()


def _tmsh_spec(
    name: str,
    summary: str,
    synopsis: str = "",
    *,
    min_args: int = 0,
    max_args: int | None = None,
) -> CommandSpec:
    """Build a CommandSpec for a tmsh:: command."""
    if not synopsis:
        synopsis = f"{name} ?arg ...?"
    arity = Arity(min=min_args) if max_args is None else Arity(min=min_args, max=max_args)
    return CommandSpec(
        name=name,
        dialects=_TMSH_DIALECTS,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(synopsis,),
            source=_SOURCE_TMSH,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=synopsis),),
        validation=ValidationSpec(arity=arity),
    )
