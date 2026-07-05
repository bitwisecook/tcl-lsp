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

"""log_file -- Control logging to a file."""

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

_SOURCE = "Expect log_file(1)"


@register
class LogFileCommand(CommandDef):
    name = "log_file"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="log_file",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control logging of session output to a file.",
                synopsis=(
                    "log_file ?-option ...? ?file?",
                    "log_file -info",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="log_file ?-option ...? ?file?",
                    options=(
                        OptionSpec(name="-a", detail="Append to existing log file."),
                        OptionSpec(name="-noappend", detail="Overwrite existing log file."),
                        OptionSpec(
                            name="-open",
                            takes_value=True,
                            value_hint="fileId",
                            detail="Log to an already-open Tcl file id.",
                        ),
                        OptionSpec(name="-leaveopen", detail="Leave the file open on close."),
                        OptionSpec(name="-info", detail="Return current log file settings."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
