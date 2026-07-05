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
"""ASM::captcha -- Responds to the client with a CAPTCHA challenge."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__captcha.html"


@register
class AsmCaptchaCommand(CommandDef):
    name = "ASM::captcha"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::captcha",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Responds to the client with a CAPTCHA challenge.",
                synopsis=("ASM::captcha",),
                snippet=(
                    "Responds to the client with a CAPTCHA challenge. \n"
                    "            Note although ASM will send the CAPTCHA challenge screen back to the user, the enforcement is not always done automatically. \n"
                    "            To enforce the correct CAPTCHA response, the ASM::captcha_status command should be used."
                ),
                source=_SOURCE,
                examples=(
                    "le counts the number of violations, and if it exceeds 3,\n"
                    "            # it issues a CAPTCHA action.\n"
                    "            when ASM_REQUEST_DONE {\n"
                    '                if {[ASM::violation count] > 3 and [ASM::severity] eq "Error"} {\n'
                    "                    ASM::captcha\n"
                    "                }\n"
                    "            }"
                ),
                return_value='Returns a string signifying if the challenge was sent successfully: "ok" - CAPTCHA challenge was sent successfully "nok asm blocked request" - CAPTCHA challenge was not sent, because a blocking page action was performed "nok asm uncaptcha command was raised" - CAPTCHA challenge was not sent, because…',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::captcha",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
