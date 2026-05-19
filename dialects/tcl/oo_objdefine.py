# Scaffolded from define.n -- refine and commit
"""oo::objdefine -- define and configure classes and objects."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .oo_define import _oo_define_arg_roles

_SOURCE = "Tcl man page define.n"


_av = make_av(_SOURCE)


@register
class OoObjdefineCommand(CommandDef):
    name = "oo::objdefine"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::objdefine",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="define and configure classes and objects",
                synopsis=(
                    "oo::objdefine object defScript",
                    "oo::objdefine object subcommand arg ?arg ...?",
                ),
                snippet="The oo::define command is used to control the configuration of classes, and the oo::objdefine command is used to control the configuration of objects (including classes as instance objects), with the configuration being applied to the entity named in the class or the object argument.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::objdefine object defScript",
                    arg_values={
                        0: (
                            _av(
                                "method",
                                "This creates or updates a method that is implemented as a procedure-like script.",
                                "method name ?option? argList bodyScript",
                            ),
                            _av(
                                "forward",
                                "This creates or updates a forwarded method called name.",
                                "forward name cmdName ?arg ...?",
                            ),
                            _av(
                                "export",
                                "This arranges for each of the named methods, name, to be exported.",
                                "export name ?name ...?",
                            ),
                            _av(
                                "unexport",
                                "This arranges for each of the named methods, name, to be not exported.",
                                "unexport name ?name ...?",
                            ),
                            _av(
                                "mixin",
                                "This slot sets or updates the list of additional classes that are to be mixed into the object.",
                                "mixin ?-slotOperation? ?className ...?",
                            ),
                            _av(
                                "variable",
                                "This slot arranges for each of the named variables to be automatically made available in the methods declared by the object.",
                                "variable ?-slotOperation? ?name ...?",
                            ),
                            _av(
                                "filter",
                                "This slot sets or updates the list of method names that are used to guard method calls on the object.",
                                "filter ?-slotOperation? ?methodName ...?",
                            ),
                            _av(
                                "deletemethod",
                                "This deletes each of the methods called name from the object.",
                                "deletemethod name ?name ...?",
                            ),
                            _av(
                                "renamemethod",
                                "This renames the method called fromName in the object to toName.",
                                "renamemethod fromName toName",
                            ),
                            _av(
                                "class",
                                "This allows the class of an object to be changed after creation.",
                                "class className",
                            ),
                            _av(
                                "private",
                                "This evaluates the script in a context where the definitions made on the current object will be private definitions.",
                                "private cmd arg...",
                            ),
                            _av(
                                "self",
                                "Returns the name of the object being configured (no arguments allowed in objdefine).",
                                "self",
                            ),
                            _av(
                                "property",
                                "This defines configurable properties on the object, with optional getter/setter scripts and access kind.",
                                "property name ?name ...? ?-get getBody? ?-set setBody? ?-kind access?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_role_resolver=_oo_define_arg_roles,
            # Per-instance counterpart of ``oo::define``: body forms
            # (``method``, ``forward``, ``private``, ``property
            # -get/-set``, and the bare ``oo::objdefine $obj { defScript
            # }`` form) all run in the object's own definition context,
            # not the caller's scope.  STRUCTURAL keeps the body out of
            # the enclosing block's data flow.
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
