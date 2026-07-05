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
"""ADAPT::allow -- Sets or returns the value of a boolean property."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__allow.html"


_av = make_av(_SOURCE)


@register
class AdaptAllowCommand(CommandDef):
    name = "ADAPT::allow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::allow",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the value of a boolean property.",
                synopsis=("ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?",),
                snippet=(
                    "The ADAPT::allow command sets or returns the value of one\n"
                    "of a set of boolean 'allow' properties for the current or\n"
                    "specified side of the virtual server connection for which\n"
                    "the iRule is being executed. They are not part of the profile\n"
                    "and therefore cannot be accessed via tmsh or the GUI.\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::allow [<context>] <property>\n"
                    "\n"
                    "    * Gets the property value for the current side\n"
                    "\n"
                    "ADAPT::allow [<context>] <property> request\n"
                    "\n"
                    "    * Gets the property value for the request-adapt side\n"
                    "\n"
                    "ADAPT::allow [<context>] <property> response\n"
                    "\n"
                    "    * Gets the property value for the response-adapt side"
                ),
                source=_SOURCE,
                examples=("when HTTP_RESPONSE {\n    ADAPT::allow http_v1.0 yes\n}"),
                return_value="Returns the current of modified value of the property.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?",
                    arg_values={
                        0: (
                            _av(
                                "http_v1.0",
                                "ADAPT::allow http_v1.0",
                                "ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"HTTP", "REQUESTADAPT", "RESPONSEADAPT"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
