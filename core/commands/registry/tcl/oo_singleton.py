# Scaffolded from singleton.n -- refine and commit
"""oo::singleton -- metaclass for singleton classes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity, BodyKind
from ._base import register
from .oo_class import _oo_metaclass_arg_roles

_SOURCE = "Tcl man page singleton.n"


_av = make_av(_SOURCE)


@register
class OoSingletonCommand(CommandDef):
    name = "oo::singleton"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::singleton",
            is_oo_metaclass=True,
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
                                "Note: create is not exported on instances of oo::singleton. Use new instead.",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "Returns the existing singleton instance if one exists; creates a new one only if no instance exists. Constructor arguments are only used during initial construction.",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "Note: createWithNamespace is not exported on instances of oo::singleton.",
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
