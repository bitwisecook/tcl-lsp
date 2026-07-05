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
"""RADIUS::avp -- This command returns or adds/changes/removes RADIUS attribute-value pairs."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__avp.html"


_av = make_av(_SOURCE)


@register
class RadiusAvpCommand(CommandDef):
    name = "RADIUS::avp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::avp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns or adds/changes/removes RADIUS attribute-value pairs.",
                synopsis=(
                    "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                    "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)",
                ),
                snippet="This command returns or adds/changes/removes RADIUS attribute-value pairs. Radius profile must be applied for access to this command.",
                source=_SOURCE,
                examples=('when RULE_INIT {\n        set static::secret "linus"\n    }'),
                return_value="RADIUS::avp attr [attr_type] Returns the value of the specified RADIUS attribute. optional attr_type = ( octet | ip4 | ip6 | integer | string)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                    arg_values={
                        0: (
                            _av(
                                "index",
                                "RADIUS::avp index",
                                "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                            ),
                            _av(
                                "insert",
                                "RADIUS::avp insert",
                                "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_CLOSED",
                        "CLIENT_DATA",
                        "SERVER_CLOSED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                )
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
