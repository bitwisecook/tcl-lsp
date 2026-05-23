"""tcl::build-info -- Compile-time build metadata for Tcl.

Tcl 9 exposes a ``tcl::build-info`` command that returns strings like
``patchlevel``, ``version``, ``commit``, ``branch``, and ``compiler``
sourced from the build's ``tclConfig.sh``.  Test frameworks
(notably ``format.test`` from the vendored Tcl 9.0.3 tcltest suite)
read these values during their setup phase, so the WASM runtime
ships a stub implementation in ``runtime/zig/cmds/inspect.zig``
that returns sensible hard-coded answers for the keys actually
consulted.

Two name forms are registered because tcltest invokes both the
namespaced ``tcl::build-info`` and the fully-qualified
``::tcl::build-info`` (depending on whether the call site is
inside a ``namespace eval ::tcl`` block).  The Zig builtin table
exposes both spellings and the parity gate would otherwise flag
either as an orphan_builtins entry.
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

_SOURCE = "Tcl tcl::build-info (internal)"

_HOVER = HoverSnippet(
    summary="Return compile-time build metadata for the Tcl runtime.",
    synopsis=("tcl::build-info ?key?",),
    snippet=(
        "Returns the compile-time build metadata for the running Tcl "
        "runtime.  With no arguments, returns the patchlevel.  With a "
        "key argument, returns the value associated with that key "
        "(e.g. ``version``, ``commit``, ``branch``, ``compiler``)."
    ),
    source=_SOURCE,
)

_FORMS = (
    FormSpec(
        kind=FormKind.DEFAULT,
        synopsis="tcl::build-info ?key?",
    ),
)

_VALIDATION = ValidationSpec(arity=Arity(0, 1))


def _make_spec(name: str) -> CommandSpec:
    return CommandSpec(
        name=name,
        dialects=DIALECTS_EXCEPT_IRULES,
        hover=_HOVER,
        forms=_FORMS,
        validation=_VALIDATION,
        return_type=TclType.STRING,
    )


@register
class TclBuildInfoCommand(CommandDef):
    name = "tcl::build-info"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)


@register
class TclBuildInfoQualifiedCommand(CommandDef):
    name = "::tcl::build-info"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _make_spec(cls.name)
