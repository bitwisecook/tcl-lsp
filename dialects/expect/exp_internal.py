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

"""exp_internal -- Control Expect internal diagnostics."""

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

_SOURCE = "Expect exp_internal(1)"


@register
class ExpInternalCommand(CommandDef):
    name = "exp_internal"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_internal",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control Expect internal diagnostic output.",
                synopsis=("exp_internal ?-f file? 0|1",),
                snippet=(
                    "With ``1``, Expect prints diagnostic information about "
                    "pattern matching and other internal activity. Useful for "
                    "debugging scripts."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exp_internal ?-f file? 0|1",
                    options=(
                        OptionSpec(
                            name="-f",
                            takes_value=True,
                            value_hint="file",
                            detail="Log diagnostics to the specified file.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
