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
"""members -- Lists all members of a given pool for v10.x.x."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/members.html"


@register
class MembersCommand(CommandDef):
    name = "members"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="members",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Lists all members of a given pool for v10.x.x.",
                synopsis=("members ('-list')? (POOL_OBJ)",),
                snippet=(
                    "This command behaves much like active_members, but counts or lists all\n"
                    "members (IP+port combinations) in a pool, not just active ones.\n"
                    "\n"
                    "Note\n"
                    "\n"
                    '   When assigning a snatpool to static variable and using "members -list"\n'
                    "   to reference it in RULE_INIT, failures will be observed at startup but\n"
                    "   won't show up in a reload afterwards. Expected behavior is to fail it\n"
                    '   in any case as "members -list" is not designed to reference a snatpool\n'
                    "   name."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set response "<?xml version=\\"1.0\\" encoding=\\"utf-8\\"?><rss version=\\"2.0\\"><channel>"\n'
                    '    append response "<title>BigIP Server Pool Status</title>"\n'
                    '    append response "<description>Server Pool Status</description>"\n'
                    '    append response "<language>en</language>"\n'
                    '    append response "<pubDate>[clock format [clock seconds]]</pubDate>"\n'
                    '    append response "<ttl>60</ttl>"\n'
                    '    if { [HTTP::uri] eq "/status" } {\n'
                    "                foreach { selectedpool } [class get pooltest] {"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="members ('-list')? (POOL_OBJ)",
                    options=(
                        OptionSpec(
                            name="-list",
                            detail="Return as list instead of count.",
                            takes_value=False,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
