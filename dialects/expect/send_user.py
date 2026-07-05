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

"""send_user -- Send a string to the user (stdout)."""

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

_SOURCE = "Expect send_user(1)"


@register
class SendUserCommand(CommandDef):
    name = "send_user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send_user",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the user (standard output).",
                synopsis=("send_user ?-flags? string",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send_user ?-flags? string",
                    options=(
                        OptionSpec(name="-raw", detail="Send without translation."),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
