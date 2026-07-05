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

"""send -- Send a string to a spawned process."""

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

_SOURCE = "Expect send(1)"

_SEND_OPTIONS = (
    OptionSpec(
        name="-i",
        takes_value=True,
        value_hint="spawn_id",
        detail="Send to the specified spawn id.",
    ),
    OptionSpec(name="-raw", detail="Send without any translation."),
    OptionSpec(name="-null", detail="Send null characters."),
    OptionSpec(name="-break", detail="Send a break condition."),
    OptionSpec(name="-s", detail="Send slowly (obey send_slow parameters)."),
    OptionSpec(name="-h", detail="Send as if a human were typing (obey send_human parameters)."),
    OptionSpec(name="--", detail="End of options."),
)


@register
class SendCommand(CommandDef):
    name = "send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the current spawned process.",
                synopsis=("send ?-flags? string",),
                snippet=(
                    "Sends *string* to the process identified by the current "
                    "``spawn_id``. Use ``-s`` for slow sending or ``-h`` for "
                    "human-like typing."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send ?-flags? string",
                    options=_SEND_OPTIONS,
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
