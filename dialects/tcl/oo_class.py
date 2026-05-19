# Scaffolded from class.n -- refine and commit
"""oo::class -- class of all classes."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page class.n"


_BODY = frozenset({ArgRole.BODY})


def _oo_metaclass_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY roles for OO metaclass commands (create/new/createWithNamespace)."""
    if len(args) < 2:
        return {}
    if args[0] == "create" and len(args) >= 3:
        return {2: _BODY}
    if args[0] == "new" and len(args) >= 2:
        return {1: _BODY}
    if args[0] == "createWithNamespace" and len(args) >= 4:
        return {3: _BODY}
    return {}


_av = make_av(_SOURCE)


@register
class OoClassCommand(CommandDef):
    name = "oo::class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::class",
            is_language_keyword=True,
            is_oo_metaclass=True,
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
            arg_role_resolver=_oo_metaclass_arg_roles,
            # ``oo::class create FOO { ... }`` and friends carry a class
            # definition script that runs in the class's definition
            # context, not the caller's scope.  STRUCTURAL excludes the
            # body from the enclosing block's data flow; the OO analyser
            # (``_handle_oo_class_command``) recurses into it separately.
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
