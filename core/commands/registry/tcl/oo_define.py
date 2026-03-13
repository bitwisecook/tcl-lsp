# Scaffolded from define.n -- refine and commit
"""oo::define -- define and configure classes and objects."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page define.n"


_av = make_av(_SOURCE)


@register
class OoDefineCommand(CommandDef):
    name = "oo::define"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::define",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="define and configure classes and objects",
                synopsis=(
                    "oo::define class defScript",
                    "oo::define class subcommand arg ?arg ...?",
                ),
                snippet="The oo::define command is used to control the configuration of classes, and the oo::objdefine command is used to control the configuration of objects (including classes as instance objects), with the configuration being applied to the entity named in the class or the object argument.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::define class defScript",
                    arg_values={
                        0: (
                            _av(
                                "classmethod",
                                "This creates a class method, or (if argList and bodyScript are omitted) promotes an existing method on the class object to be a class method.",
                                "classmethod name ?argList bodyScrip?",
                            ),
                            _av(
                                "constructor",
                                "This creates or updates the constructor for a class.",
                                "constructor argList bodyScript",
                            ),
                            _av(
                                "destructor",
                                "This creates or updates the destructor for a class.",
                                "destructor bodyScript",
                            ),
                            _av(
                                "export",
                                "This arranges for each of the named methods, name, to be exported (i.e.",
                                "export name ?name ...?",
                            ),
                            _av(
                                "forward",
                                "This creates or updates a forwarded method called name.",
                                "forward name cmdName ?arg ...?",
                            ),
                            _av(
                                "initialise",
                                "initialize script This evaluates script in a context which supports local variables and where the current namespace is the instance namespace of the class object itself.",
                                "initialise script",
                            ),
                            _av(
                                "method",
                                "This creates or updates a method that is implemented as a procedure-like script.",
                                "method name ?option? argList bodyScript",
                            ),
                            _av(
                                "private",
                                "private script This evaluates the script (or the list of command and arguments given by cmd and args) in a context where the definitions made on the current class will be private definitions.",
                                "private cmd arg...",
                            ),
                            _av(
                                "self",
                                "self script self This command is equivalent to calling oo::objdefine on the class being defined (see CONFIGURING OBJECTS below for a description of the supported values of subcommand).",
                                "self subcommand arg ...",
                            ),
                            _av(
                                "superclass",
                                "This slot (see SLOTTED DEFINITIONS below) allows the alteration of the superclasses of the class being defined.",
                                "superclass ?-slotOperation? ?className ...?",
                            ),
                            _av(
                                "unexport",
                                "This arranges for each of the named methods, name, to be not exported (i.e.",
                                "unexport name ?name ...?",
                            ),
                            _av(
                                "variable",
                                "This slot (see SLOTTED DEFINITIONS below) arranges for each of the named variables to be automatically made available in the methods, constructor and destructor declared by the class being defined.",
                                "variable ?-slotOperation? ?name ...?",
                            ),
                            _av(
                                "definitionnamespace",
                                "This allows control over what namespace will be used by the oo::define and oo::objdefine commands to look up the definition commands they use.",
                                "definitionnamespace ?kind? namespaceName",
                            ),
                            _av(
                                "deletemethod",
                                "This deletes each of the methods called name from a class.",
                                "deletemethod name ?name ...?",
                            ),
                            _av(
                                "filter",
                                "This slot (see SLOTTED DEFINITIONS below) sets or updates the list of method names that are used to guard whether method call to instances of the class may be called and what the method's results are.",
                                "filter ?-slotOperation? ?methodName ...?",
                            ),
                            _av(
                                "mixin",
                                "This slot (see SLOTTED DEFINITIONS below) sets or updates the list of additional classes that are to be mixed into all the instances of the class being defined.",
                                "mixin ?-slotOperation? ?className ...?",
                            ),
                            _av(
                                "renamemethod",
                                "This renames the method called fromName in a class to toName.",
                                "renamemethod fromName toName",
                            ),
                            _av(
                                "class",
                                "This allows the class of an object to be changed after creation.",
                                "class className",
                            ),
                            _av(
                                "Get",
                                "Returns a list that is the current contents of the slot, but does not modify the slot.",
                                "slot Get",
                            ),
                            _av(
                                "Resolve",
                                "Returns slotElement with a resolution operation applied to it, but does not modify the slot.",
                                "slot Resolve slotElement",
                            ),
                            _av(
                                "Set",
                                "Sets the contents of the slot to the list elementList and returns the empty string.",
                                "slot Set elementList",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
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
