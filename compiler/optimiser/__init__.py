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

"""Static source optimiser for Tcl.

Current safe subset -- see individual pass modules for details:
    O100--O125 optimisation passes.
"""

from __future__ import annotations

from ._expr_simplify import demorgan_transform, invert_expression
from ._manager import (
    apply_optimisations,
    find_optimisations,
    optimise_source,
    optimise_source_multipass,
)
from ._types import Optimisation

__all__ = [
    "Optimisation",
    "apply_optimisations",
    "demorgan_transform",
    "find_optimisations",
    "invert_expression",
    "optimise_source",
    "optimise_source_multipass",
]
