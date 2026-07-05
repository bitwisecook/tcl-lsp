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

"""Per-analysis builtin-trust context, derived from the command-binding lattice.

:mod:`compiler.command_binding` is the source of truth for what each command name
resolves to.  Constant-folding a *builtin* (``string length`` → ``5``,
``incr c`` → a constant) is only sound while that name still denotes its core
command — but the fold happens deep in SCCP / the registry folder, where
threading a per-call-site binding through every helper is impractical.

So this module holds the lattice's *derived* verdict in a **scoped** global, set
around the analysis or optimisation of one compilation unit (mirroring how
:func:`configure_signatures` scopes the active dialect) and always restored.

Representation
--------------
The scoped state is exactly the lattice's own whole-module summary —
:class:`~compiler.command_binding.ModuleCommandMutations` — rather than a bespoke
``(frozenset, bool)`` pair.  One vocabulary spans the analysis result and the
scoped environment: ``scan_module_command_mutations`` produces it, and the trust
query is just its :meth:`~compiler.command_binding.ModuleCommandMutations.trusts`
method.  The verdict is intentionally flow-insensitive for builtins (a builtin
renamed *anywhere* in the unit is distrusted *everywhere*); flow-sensitive
precision is reserved for *procedure* calls, which the optimiser gates against the
lattice directly at each call site.
"""

from __future__ import annotations

from contextlib import contextmanager
from typing import TYPE_CHECKING

from .command_binding import ModuleCommandMutations, scan_module_command_mutations

if TYPE_CHECKING:
    from collections.abc import Iterator

    from .ir import IRModule

# "Everything is its own builtin" — the module-load default, so callers that
# never establish a trust context behave exactly as before.
_TRUST_ALL = ModuleCommandMutations(names=frozenset(), dynamic=False)
_active: ModuleCommandMutations = _TRUST_ALL


def builtin_is_trusted(name: str) -> bool:
    """True when *name* still denotes its original builtin in the active unit.

    Consulted by the registry folder and the SCCP ``incr`` evaluator before they
    apply builtin semantics.
    """
    return _active.trusts(name)


def module_command_trust(ir_module: IRModule) -> ModuleCommandMutations:
    """The unit's command-mutation summary — the scoped trust value to apply."""
    return scan_module_command_mutations(ir_module)


@contextmanager
def command_trust(mutations: ModuleCommandMutations) -> Iterator[None]:
    """Scope *mutations* as the active builtin-trust verdict for one unit's work."""
    global _active
    prev = _active
    _active = mutations
    try:
        yield
    finally:
        _active = prev
