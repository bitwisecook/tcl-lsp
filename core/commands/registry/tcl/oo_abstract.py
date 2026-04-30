# Scaffolded from class.n -- refine and commit
"""oo::abstract -- metaclass for abstract classes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity, BodyKind
from ._base import register
from .oo_class import _oo_metaclass_arg_roles

_SOURCE = "Tcl man page abstract.n"


_av = make_av(_SOURCE)


@register
class OoAbstractCommand(CommandDef):
    name = "oo::abstract"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::abstract",
            is_oo_metaclass=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="metaclass for abstract classes",
                synopsis=("oo::abstract method ?arg ...?",),
                snippet="The oo::abstract command creates a class that cannot be directly instantiated. Only subclasses of an abstract class may be instantiated.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::abstract method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "This creates a new abstract class called name. Note: create is not exported on instances of oo::abstract; abstract classes cannot be directly instantiated.",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "This creates a new abstract class with a unique name. Note: new is not exported on instances of oo::abstract; abstract classes cannot be directly instantiated.",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "This creates a new abstract class with an explicitly chosen namespace. Note: createWithNamespace is not exported on instances of oo::abstract.",
                                "cls createWithNamespace name nsName ?arg ...?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            arg_role_resolver=_oo_metaclass_arg_roles,
            # See ``oo::class`` — the metaclass body runs in the class's
            # own definition context, not the caller's scope.
            body_kind=BodyKind.STRUCTURAL,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
