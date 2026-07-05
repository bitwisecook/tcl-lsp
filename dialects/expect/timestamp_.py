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

"""timestamp -- Return or format a timestamp."""

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
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect timestamp(1)"


@register
class TimestampCommand(CommandDef):
    name = "timestamp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timestamp",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Return the current time or format a timestamp.",
                synopsis=(
                    "timestamp ?-seconds N? ?-format fmt? ?-gmt?",
                    "timestamp",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timestamp ?-seconds N? ?-format fmt? ?-gmt?",
                    options=(
                        OptionSpec(
                            name="-seconds",
                            takes_value=True,
                            value_hint="N",
                            detail="Specify epoch seconds instead of current time.",
                        ),
                        OptionSpec(
                            name="-format",
                            takes_value=True,
                            value_hint="fmt",
                            detail="strftime-style format string.",
                        ),
                        OptionSpec(name="-gmt", detail="Use GMT instead of local time."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
