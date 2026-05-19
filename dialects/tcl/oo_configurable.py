# Scaffolded from configurable.n -- refine and commit
"""oo::configurable -- class that supports configurable properties."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .oo_class import _oo_metaclass_arg_roles

_SOURCE = "Tcl man page configurable.n"


_av = make_av(_SOURCE)


@register
class OoConfigurableCommand(CommandDef):
    name = "oo::configurable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::configurable",
            is_oo_metaclass=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="class that supports configurable properties",
                synopsis=("oo::configurable method ?arg ...?",),
                snippet="The oo::configurable command creates a class that automatically supports the property definition command and a configure method for getting and setting property values on instances.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::configurable method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "This creates a new configurable class called name, passing the arguments, arg ..., to the constructor.",
                                "cls create name ?arg ...?",
                            ),
                            _av(
                                "new",
                                "This creates a new configurable class with a new unique name, passing the arguments, arg ..., to the constructor.",
                                "cls new ?arg ...?",
                            ),
                            _av(
                                "createWithNamespace",
                                "This creates a new configurable class called name with an explicitly chosen namespace nsName.",
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
