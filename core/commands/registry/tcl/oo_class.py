# Scaffolded from class.n -- refine and commit
"""oo::class -- class of all classes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page class.n"


_av = make_av(_SOURCE)


@register
class OoClassCommand(CommandDef):
    name = "oo::class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::class",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="class of all classes",
                synopsis=("oo::class method ?arg ...?",),
                snippet="Classes are objects that can manufacture other objects according to a pattern stored in the factory object (the class).",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::class method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "This creates a new instance of the class cls called name (which is resolved within the calling context's namespace if not fully qualified), passing the arguments, arg ..., to the constructor, and (if that returns a succ…",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "This creates a new instance of the class cls with a new unique name, passing the arguments, arg ..., to the constructor, and (if that returns a successful result) returning the fully qualified name of the created object…",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "This creates a new instance of the class cls called name (which is resolved within the calling context's namespace if not fully qualified), passing the arguments, arg ..., to the constructor, and (if that returns a succ…",
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
