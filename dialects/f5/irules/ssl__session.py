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
"""SSL::session -- Drops a session from the SSL session cache."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__session.html"


_av = make_av(_SOURCE)


@register
class SslSessionCommand(CommandDef):
    name = "SSL::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drops a session from the SSL session cache.",
                synopsis=("SSL::session invalidate ( drop | nodrop )?",),
                snippet='Invalidates the current session. If no parameter is specified, or the "drop" parameter is specified, this commands drops the current SSL session ID from the session cache to prevent reuse of the session. If "nodrop" is specified, the current session will be invalidated but the session will not be dropped from the session cache.',
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [HTTP::uri] contains "/maint_mode" } {\n'
                    "        ## send content and die\n"
                    "        HTTP::respond 200 content $::error_html Connection Close\n"
                    "        event HTTP_REQUEST disable\n"
                    "        SSL::session invalidate\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::session invalidate Invalidates the current session. Specifically, this command drops the current SSL session ID from the session cache to prevent reuse of the session.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::session invalidate ( drop | nodrop )?",
                    arg_values={
                        0: (
                            _av(
                                "drop",
                                "SSL::session drop",
                                "SSL::session invalidate ( drop | nodrop )?",
                            ),
                            _av(
                                "nodrop",
                                "SSL::session nodrop",
                                "SSL::session invalidate ( drop | nodrop )?",
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
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
