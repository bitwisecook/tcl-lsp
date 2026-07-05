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
"""AAA::acct_result -- This command is used to check whether the accounting information is sent successfully to IVS(internal virtual server) or not."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AAA__acct_result.html"


@register
class AaaAcctResultCommand(CommandDef):
    name = "AAA::acct_result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AAA::acct_result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to check whether the accounting information is sent successfully to IVS(internal virtual server) or not.",
                synopsis=("AAA::acct_result AAA_REQUEST_ID",),
                snippet="This command is used to check whether the accounting information is sent successfully to IVS(internal virtual server) or not.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_DATA {\n"
                    "    set aaa_result [AAA::acct_result $request_id]\n"
                    '    if { $aaa_result == "INPROGRESS"  } {\n'
                    "        after 200\n"
                    "        continue\n"
                    "    }\n"
                    "\n"
                    '    if { $aaa_result == "OK" } {\n'
                    "        # request was successfull\n"
                    "    } else {\n"
                    "        # handle errors\n"
                    "    }\n"
                    "}"
                ),
                return_value='There are 4 possible return values for this command (All STRING type): "OK" - the request was successful. "FAIL" - the request has been rejected. "INPROGRESS" - the request is still in progress (asyncronous). "ERROR" - there was an error during the request.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AAA::acct_result AAA_REQUEST_ID",
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
