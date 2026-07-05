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
"""TCP::snd_ssthresh -- Returns the TCP slow start threshold (ssthresh)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__snd_ssthresh.html"


@register
class TcpSndSsthreshCommand(CommandDef):
    name = "TCP::snd_ssthresh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::snd_ssthresh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP slow start threshold (ssthresh).",
                synopsis=("TCP::snd_ssthresh",),
                snippet=(
                    "The slow start threshold (ssthresh) is the point at which the\n"
                    "congestion window (cwnd) grows less aggressively. When the cwnd is\n"
                    "less than ssthresh, it roughly doubles for every cwnd worth of\n"
                    "acknowledged data. When cwnd is greater than ssthresh, it increases\n"
                    "by approximately one MSS for each cwnd worth of acknowledged data.\n"
                    "\n"
                    "ssthresh starts at 1,073,725,440 bytes unless there is a cmetrics\n"
                    "cache entry. When TCP detects packet loss it usually sets ssthresh\n"
                    "to a value between 1/2 cwnd and cwnd, depending on  connection\n"
                    "conditions and the congestion control algorithm."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Get BIGIP's last slow-start threshold.\n"
                    '    log local0. "BIGIP\'s ssthresh: [TCP::snd_ssthresh]"\n'
                    "}"
                ),
                return_value="The connection slow start threshold in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::snd_ssthresh",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
