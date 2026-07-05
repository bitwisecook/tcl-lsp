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

"""S4 inlining: catalogue and (forthcoming) IR-level splicer.

The package owns the inliner's policy and mechanism.  The policy
(``decision``) tags every ``IRProcedure`` with an
:class:`~compiler.ir.InlineDecision` after var-escape analysis
has computed ``pure_leaf``.  The mechanism (added in S4.2) walks the
IR module post-lowering and substitutes callee bodies into caller
``IRBlock`` s where the tag is ``ALWAYS`` or ``IF_SINGLE_CALL``.
"""

from .decision import (
    SMALL_BODY_THRESHOLD,
    apply_inline_catalogue,
    classify_proc,
    count_statements,
    count_static_calls,
)
from .inline_pass import inline_module

__all__ = [
    "SMALL_BODY_THRESHOLD",
    "apply_inline_catalogue",
    "classify_proc",
    "count_statements",
    "count_static_calls",
    "inline_module",
]
