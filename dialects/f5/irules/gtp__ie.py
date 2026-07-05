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
"""GTP::ie -- This set of commands allows for the parsing and interpretation of GTP IE elements."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
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

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__ie.html"


_av = make_av(_SOURCE)


@register
class GtpIeCommand(CommandDef):
    name = "GTP::ie"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::ie",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This set of commands allows for the parsing and interpretation of GTP IE elements.",
                synopsis=(
                    "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                    "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                    "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                    "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                ),
                snippet=(
                    "This set of commands allows for the parsing and interpretation of GTP\n"
                    "IE elements."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    "    if { [GTP::ie exists imsi:0] } {\n"
                    '        log local0. "GTP imsi [GTP::ie get value imsi:0]"\n'
                    "    }\n"
                    '    log local0. "Total number of top level IEs [GTP::ie count]"\n'
                    "    set ie_list [ GTP::ie get list]\n"
                    "    foreach ie $ie_list {\n"
                    '        log local0. "IE $ie"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                    options=(
                        OptionSpec(
                            name="-message",
                            detail="Operate on a specific GTP message object.",
                            takes_value=True,
                            value_hint="MESSAGE",
                        ),
                        OptionSpec(
                            name="-type",
                            detail="Filter by IE type value.",
                            takes_value=True,
                            value_hint="TYPE",
                        ),
                        OptionSpec(
                            name="-instance",
                            detail="Filter by IE instance.",
                            takes_value=True,
                            value_hint="INSTANCE",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "exists",
                                "GTP::ie exists",
                                "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                            ),
                            _av(
                                "count",
                                "GTP::ie count",
                                "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                            ),
                            _av(
                                "get",
                                "GTP::ie get",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "instance",
                                "GTP::ie instance",
                                "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                            ),
                            _av(
                                "length",
                                "GTP::ie length",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "encode-type",
                                "GTP::ie encode-type",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "value",
                                "GTP::ie value",
                                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                            ),
                            _av(
                                "list",
                                "GTP::ie list",
                                "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
