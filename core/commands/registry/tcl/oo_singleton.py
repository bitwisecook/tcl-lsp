# Scaffolded from singleton.n -- refine and commit
"""oo::singleton -- metaclass for singleton classes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page singleton.n"


_av = make_av(_SOURCE)


@register
class OoSingletonCommand(CommandDef):
    name = "oo::singleton"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::singleton",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="metaclass for singleton classes",
                synopsis=("oo::singleton method ?arg ...?",),
                snippet="The oo::singleton command creates a class that will only ever have one instance. Attempts to create more instances will return the existing instance.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::singleton method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "This creates a new singleton class called name, passing the arguments, arg ..., to the constructor.",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "This creates a new singleton class with a new unique name, passing the arguments, arg ..., to the constructor.",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "This creates a new singleton class called name with an explicitly chosen namespace nsName.",
                                "cls createWithNamespace name nsName ?arg ...?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
