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

# Enriched from F5 iRules reference documentation.
"""XLAT::src_config -- Retrieve the source-translation configuration."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__src_config.html"


@register
class XlatSrcConfigCommand(CommandDef):
    name = "XLAT::src_config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::src_config",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieve the source-translation configuration.",
                synopsis=("XLAT::src_config",),
                snippet=(
                    "Return the source translation configuration as a list. With the values in the following order: type,source translation object/pool.\n"
                    "\n"
                    "type - The source translation type as a string. Possible values are: NONE, AUTOMAP, SNAT, LSN, SECURITY-DYNAMIC-PAT, SECURITY-DYNAMIC-NAT, SECURITY-STATIC-NAT, SECURITY-STATIC-PAT\n"
                    "pool - the source translation object/pool name. NA when not applicable(NONE and AUTOMAP types)."
                ),
                source=_SOURCE,
                examples=('when SA_PICKED {\n    log local0. "[XLAT::src_config]"\n}'),
                return_value="Return the source translation configuration as a list. On error an exception is thrown with a message indicating the cause of failure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::src_config",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("RULE_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
