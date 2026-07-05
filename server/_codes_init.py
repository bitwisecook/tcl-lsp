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

"""Import all modules that register diagnostic and optimisation codes.

Importing this module guarantees all codes are registered and queryable
via :mod:`shared.codes`.  Each module registers its codes via
``@diag(...)`` / ``@opt(...)`` decorators at import time.
"""

# analysis
# compiler
import analyser  # noqa: F401
import analyser._analyser._utils  # noqa: F401
import analyser.checks._domain  # noqa: F401
import analyser.checks._security  # noqa: F401
import analyser.checks._style  # noqa: F401
import analyser.checks._syntax  # noqa: F401

# extensions
import analyser.checks.tk  # noqa: F401
import analyser.compiler_checks  # noqa: F401
import analyser.irules_checks  # noqa: F401
import compiler.gvn  # noqa: F401
import compiler.irules_flow  # noqa: F401
import compiler.optimiser._branch_folding  # noqa: F401
import compiler.optimiser._code_sinking  # noqa: F401
import compiler.optimiser._elimination  # noqa: F401
import compiler.optimiser._pattern_recognition  # noqa: F401
import compiler.optimiser._propagation  # noqa: F401
import compiler.optimiser._structure_elimination  # noqa: F401
import compiler.optimiser._tail_call  # noqa: F401
import compiler.optimiser._unused_procs  # noqa: F401
import compiler.shimmer  # noqa: F401
import compiler.taint._sinks  # noqa: F401
import compiler.taint._uri_split  # noqa: F401
import dialects.f5.bigip.iapp_diagnostics  # noqa: F401
import dialects.f5.bigip.validator  # noqa: F401
import dialects.f5.xc.translator  # noqa: F401
