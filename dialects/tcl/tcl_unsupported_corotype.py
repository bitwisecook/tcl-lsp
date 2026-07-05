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

"""``::tcl::unsupported::corotype`` -- inspect a coroutine's suspension state.

Tcl 9 exposes ``::tcl::unsupported::corotype CORONAME`` as part of its
``::tcl::unsupported::*`` namespace — a documented-but-internal API
the runtime ships for tooling and tests.  ``coroutine.test`` 11.1
round-trips the four observable states (``yield`` / ``yieldto`` /
``active`` / ``dead``) through this command, so the WASM runtime
implements it in ``runtime/zig/cmds/coroutine.zig::eval_corotype``
to keep that test honest.

The ``::tcl::unsupported`` namespace is the user-visible spelling
exposed by reference Tcl 9 and the WASM ``ns_resolve_qualified``
path; only the fully-qualified form is registered.  Without this
file the WASM command-parity gate flags the Zig-side BUILTIN
registration as an ``orphan_builtins`` entry.
"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
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

_SOURCE = "Tcl ::tcl::unsupported::corotype (internal)"

_HOVER = HoverSnippet(
    summary="Return the suspension state of a coroutine.",
    synopsis=("::tcl::unsupported::corotype coroName",),
    snippet=(
        "Returns the suspension state of the named coroutine.  Possible "
        "values: ``yield`` (suspended at a plain ``yield`` call), "
        "``yieldto`` (suspended at a ``yieldto`` call), ``active`` "
        "(currently running), or ``dead`` (terminated).  Internal "
        "introspection API exposed under the ``::tcl::unsupported`` "
        "namespace; primarily used by tooling and the tcltest harness."
    ),
    source=_SOURCE,
)

_FORMS = (
    FormSpec(
        kind=FormKind.DEFAULT,
        synopsis="::tcl::unsupported::corotype coroName",
    ),
)

_VALIDATION = ValidationSpec(arity=Arity(1, 1))


@register
class TclUnsupportedCorotypeCommand(CommandDef):
    name = "::tcl::unsupported::corotype"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=_HOVER,
            forms=_FORMS,
            validation=_VALIDATION,
            return_type=TclType.STRING,
        )
