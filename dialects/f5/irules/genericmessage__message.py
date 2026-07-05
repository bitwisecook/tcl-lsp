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
"""GENERICMESSAGE::message -- Returns or sets values for messages in the generic message profile."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__message.html"


_av = make_av(_SOURCE)


@register
class GenericmessageMessageCommand(CommandDef):
    name = "GENERICMESSAGE::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets values for messages in the generic message profile.",
                synopsis=(
                    "GENERICMESSAGE::message (len | length)",
                    "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                    "GENERICMESSAGE::message is_request (BOOLEAN)?",
                    "GENERICMESSAGE::message data (DATA)?",
                ),
                snippet=(
                    "The GENERICMESSAGE::message command returns or sets values from\n"
                    "the current message being processed by the generic message profile."
                ),
                source=_SOURCE,
                examples=(
                    "when GENERICMESSAGE_INGRESS {\n"
                    "    GENERICMESSAGE::message src us\n"
                    "    GENERICMESSAGE::message dst them\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::message (len | length)",
                    arg_values={
                        0: (
                            _av(
                                "len",
                                "GENERICMESSAGE::message len",
                                "GENERICMESSAGE::message (len | length)",
                            ),
                            _av(
                                "length",
                                "GENERICMESSAGE::message length",
                                "GENERICMESSAGE::message (len | length)",
                            ),
                            _av(
                                "src",
                                "GENERICMESSAGE::message src",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "source",
                                "GENERICMESSAGE::message source",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "dst",
                                "GENERICMESSAGE::message dst",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "dest",
                                "GENERICMESSAGE::message dest",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "destination",
                                "GENERICMESSAGE::message destination",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"GENERICMSG", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
