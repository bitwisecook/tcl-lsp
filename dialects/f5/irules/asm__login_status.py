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
"""ASM::login_status -- Request status of the login session tracked by one of the login pages defined in the policy."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__login_status.html"


@register
class AsmLoginStatusCommand(CommandDef):
    name = "ASM::login_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::login_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Request status of the login session tracked by one of the login pages defined in the policy.",
                synopsis=("ASM::login_status",),
                snippet=(
                    "Returns status of the login session tracked by one of the login pages defined in the policy. Following are the possible values:\n"
                    "\n"
                    "                not_logged_in: The request is not within a login session.\n"
                    "                logging_in: The request is to a login URL.\n"
                    "                logged_in: The request is within a login session, indicates a successful login in the ASM_RESPONSE_LOGIN event.\n"
                    "                failed: The login attempt is failed, triggered only in the ASM_RESPONSE_LOGIN event."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_RESPONSE_LOGIN {\n"
                    '                if {[ASM::login_status] eq "logged_in"} {\n'
                    '                    log local0. "User [ASM::username] logged in succesfully."\n'
                    "                }\n"
                    "                else {\n"
                    '                    log local0. "Login attempt to [ASM::username] failed."\n'
                    "                }\n"
                    "            }"
                ),
                return_value="Returns status of the login session.;",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::login_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
