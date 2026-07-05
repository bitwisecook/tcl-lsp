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

"""Procedure-parameter usage trait enum.

`ProcArgTrait` describes how a proc parameter is used inside its body —
the same information drives shimmer analysis, taint propagation, the
unused-variable diagnostic, and the interprocedural summariser. The
enum is pulled into `shared/` so both the analyser (which derives the
traits) and the compiler optimiser/taint (which consume them through
interprocedural summaries) can name them without forcing one to
depend on the other.
"""

from __future__ import annotations

from enum import Enum, auto


class ProcArgTrait(Enum):
    """How a proc parameter is used inside the proc body.

    These traits drive optimisation, shimmer analysis, taint propagation,
    and diagnostics by telling downstream passes how a parameter value
    flows through the proc.
    """

    EVAL = auto()  # Argument is eval'd as a script (eval, uplevel, subst)
    BODY = auto()  # Argument is used as a loop/control body
    VAR_WRITE = auto()  # Argument names a CALLER-frame variable via upvar that the proc writes
    VAR_READ = auto()  # Argument names a CALLER-frame variable via upvar that the proc reads
    EXPR = auto()  # Argument is evaluated as an expression
    LOOP_LIST = auto()  # Argument is used as the list in a foreach/lmap
    # The parameter's VALUE is used as a variable name in the CALLEE's
    # local scope (e.g. ``proc f {p} { set \$p 1 }`` or
    # ``proc f {p} { info exists \$p }``).  Distinct from VAR_WRITE /
    # VAR_READ: those imply the param aliases a caller-frame variable
    # via ``upvar``; this trait is callee-local only.  The caller's
    # literal arg ``f x`` does NOT consume the caller's ``x`` --
    # it's just a string the callee uses to name its OWN local.
    # (PR #498 deep review finding 10.)
    DYNAMIC_NAME_LOCAL = auto()
