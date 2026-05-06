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

from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
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
