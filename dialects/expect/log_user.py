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

"""log_user -- Control logging of send/expect output to stdout."""

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

_SOURCE = "Expect log_user(1)"


@register
class LogUserCommand(CommandDef):
    name = "log_user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="log_user",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control whether send/expect output is logged to stdout.",
                synopsis=(
                    "log_user -info",
                    "log_user 0|1",
                ),
                snippet="With ``1`` (default), output is sent to stdout. With ``0``, output is suppressed.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="log_user ?-info | 0 | 1?",
                    options=(OptionSpec(name="-info", detail="Return current setting."),),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )
