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

"""spawn -- Start a new process for interaction."""

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

_SOURCE = "Expect spawn(1)"


@register
class SpawnCommand(CommandDef):
    name = "spawn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="spawn",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Start a new process and prepare it for interaction.",
                synopsis=("spawn ?-option ...? program ?args ...?",),
                snippet=(
                    "Starts a new process and connects its stdin/stdout to "
                    "the Expect channel. Returns the spawn id in the variable "
                    "`spawn_id`."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="spawn ?-option ...? program ?args ...?",
                    options=(
                        OptionSpec(name="-noecho", detail="Suppress echoing of the command."),
                        OptionSpec(name="-console", detail="Redirect console output to spawn."),
                        OptionSpec(
                            name="-ignore",
                            takes_value=True,
                            value_hint="signal",
                            detail="Ignore the named signal in the spawned process.",
                        ),
                        OptionSpec(name="-leaveopen", detail="Leave the file descriptor open."),
                        OptionSpec(name="-pty", detail="Open a pty for the process."),
                        OptionSpec(name="-nottycopy", detail="Do not copy tty modes."),
                        OptionSpec(name="-nottyinit", detail="Do not initialise the tty."),
                        OptionSpec(
                            name="-open",
                            takes_value=True,
                            value_hint="fileId",
                            detail="Use an already-open file id.",
                        ),
                        OptionSpec(name="-trap", detail="Enable signal trapping."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
