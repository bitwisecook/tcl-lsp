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
"""timing -- Enables or disables iRule timing statistics."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/timing.html"


@register
class TimingCommand(CommandDef):
    name = "timing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables or disables iRule timing statistics.",
                synopsis=("timing TIMING",),
                snippet=(
                    "The timing command can be used to enable iRule timing statistics. This\n"
                    "will then collect timing information as specified each time the rule is\n"
                    'evaluated. Statistics may be viewed with "b rule show all" or in the\n'
                    "Statistics tab of the iRules Editor.\n"
                    "\n"
                    "Note: In 11.5.0, timing was enabled by default for all iRules in\n"
                    "BZ375905. The performance impact is negligible. As a result, you no\n"
                    "longer need to use this command to view timing statistics."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    ...\n  }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timing TIMING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            irules_top_level_only=True,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
