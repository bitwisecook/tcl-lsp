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
"""IMAP::activation_mode -- Get or set the activation mode for IMAP STARTTLS."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IMAP__activation_mode.html"


_av = make_av(_SOURCE)


@register
class ImapActivationModeCommand(CommandDef):
    name = "IMAP::activation_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IMAP::activation_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the activation mode for IMAP STARTTLS.",
                synopsis=("IMAP::activation_mode (none | allow | require)?",),
                snippet="Sets the IMAP activation mode to none (IMAP STARTTLS detection will not activate), allow (IMAP will optionally activate TLS if client or server support STARTTLS), or require (IMAP will require that both client and server support STARTTLS). Returns the current activation mode if no option is specified.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    IMAP::activation_mode require\n"
                    "                }\n"
                    "\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    set mode [IMAP::activation_mode]\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IMAP::activation_mode (none | allow | require)?",
                    arg_values={
                        0: (
                            _av(
                                "none",
                                "IMAP::activation_mode none",
                                "IMAP::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "allow",
                                "IMAP::activation_mode allow",
                                "IMAP::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "require",
                                "IMAP::activation_mode require",
                                "IMAP::activation_mode (none | allow | require)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
