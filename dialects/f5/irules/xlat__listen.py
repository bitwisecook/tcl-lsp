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
"""XLAT::listen -- Creates a related ephemeral listener."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__listen.html"


@register
class XlatListenCommand(CommandDef):
    name = "XLAT::listen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::listen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a related ephemeral listener.",
                synopsis=(
                    "XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
                ),
                snippet="Creates a related ephemeral listener and returns the TCL handle for the listener. bind address and port can be omitted. It is recommend that users don't set this, so the command can choose an IP:port based on the server address specified and also conforms to source translation config. If the server address is on the clientside, then bind IP::port will be a valid endpoint on the clientside and conforms to the source translation config on the clientside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set listen [XLAT::listen -inherit-main-rules 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [LINK::vlan_id],/Common/public1 -ip [serverside {IP::local_addr}]\n"
                    "        server [IP::client_addr] 7000\n"
                    "        allow [LB::server addr] 0\n"
                    "        inherit-vs [virtual]\n"
                    "    }]\n"
                    '    log local0. "LISTEN: $listen"\n'
                    "\n"
                    "    # hairpin\n"
                    "    set listen_hairpin [XLAT::listen -hairpin 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [clientside {LINK::vlan_id}]"
                ),
                return_value='Return the TCL handle to the created listener. String representaion of the handle: "<local addr>%<local route domain id>,<local port>,<remote addr>%<remote route domain id>,<remote port>,<server addr>%<server route domain id>,<server port>,<vlan id>,<protocol number>".',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
                    options=(
                        OptionSpec(
                            name="-hairpin",
                            detail="Enable hairpin mode for the listener.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-inherit-main-rules",
                            detail="Execute main rules attached to parent virtual.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-single-connection",
                            detail="Listener expires after one connection.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-translation-loose",
                            detail="Use hint data as suggestion; don't fail if unusable.",
                            takes_value=False,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
