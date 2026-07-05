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

"""lseq -- Generate a sequence of numbers as a list (Tcl 9.0)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lseq.n (Tcl 9.0)"


@register
class LseqCommand(CommandDef):
    name = "lseq"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lseq",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Generate a list of numeric values in a range.",
                synopsis=(
                    "lseq n",
                    "lseq start end ?step?",
                    "lseq start to end",
                    "lseq start 'count' count",
                    "lseq start 'by' step 'count' count",
                ),
                snippet=(
                    "Returns a list of numbers from start through end "
                    "(inclusive) with optional step.  One-arg form yields "
                    "0..n-1.  Float and double values are supported."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    # C Tcl 9.0 ``Tcl_LseqObjCmd`` rejects objc > 6 and
                    # falls to the syntax-error path when the key
                    # decoder can't make sense of fewer than 2 objc's
                    # worth.  Args after the command name: 1..5.
                    synopsis="lseq ?start? ?op? end ?by step? ?count n?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 5),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            side_effect_hints=(),
        )
