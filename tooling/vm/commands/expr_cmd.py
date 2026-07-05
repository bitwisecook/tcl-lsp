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

"""The ``expr`` command — expression evaluation."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_expr(interp: TclInterp, args: list[str]) -> TclResult:
    """expr arg ?arg ...?

    Concatenates all arguments and evaluates the result as a Tcl expression.
    """
    if not args:
        raise TclError('wrong # args: should be "expr arg ?arg ...?"')
    expr_str = " ".join(args)
    result = interp.eval_expr(expr_str)
    return TclResult(value=result)


def register() -> None:
    """Register the expr command."""
    from compiler.registry import REGISTRY

    REGISTRY.register_handler("expr", _cmd_expr)
