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
"""AUTH::response_data -- Returns pairwise auth query results."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__response_data.html"


@register
class AuthResponseDataCommand(CommandDef):
    name = "AUTH::response_data"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::response_data",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns pairwise auth query results.",
                synopsis=("AUTH::response_data (AUTH_ID)?",),
                snippet=(
                    "AUTH::response_data returns the a set of name/value query results from\n"
                    "the most recent query. This command would normally be called from the\n"
                    "AUTH_RESULT event. The format of the data returned is suitable for\n"
                    "setting as the value of a TCL array.\n"
                    "AUTH::subscribe must first be called to register interest in query\n"
                    "results prior to calling AUTH::authenticate. As a convenience when\n"
                    "using the builtin system auth rules, these rules will call\n"
                    "AUTH::subscribe if the variable tmm_auth_subscription is set."
                ),
                source=_SOURCE,
                examples=('when CLIENT_ACCEPTED {\n        set tmm_auth_subscription "*"\n    }'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::response_data (AUTH_ID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
