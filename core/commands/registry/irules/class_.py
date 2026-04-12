# Enriched from F5 iRules reference documentation.
"""class -- Advanced access of classes."""

# Introduced: BIG-IP v9+ (core data group command) (approximate, from F5 documentation)

from __future__ import annotations

from ....compiler.side_effects import (
    ConnectionSide,
    SideEffect,
    SideEffectTarget,
    StorageScope,
)
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/class.html"


_av = make_av(_SOURCE)


@register
class ClassCommand(CommandDef):
    name = "class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="class",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Advanced access of classes.",
                synopsis=(
                    "class 'match' (((CLASS_SEARCH_OPTION) ('-all'))#)? ('--')? ITEM CLASS_OPERATOR CLASS_OBJ",
                    "class match attempts to match the provided <item> to an element in <class> by applying the <operator> to the <item>.",
                    "class match [HTTP::uri] ends_with image_class",
                ),
                snippet=(
                    "The class command, implemented in v10.0.0, allows you to query data groups and data group properties.\n"
                    "\n"
                    "These commands work for both internal (defined in the bigip.conf) and external (custom file) data groups. Internal data groups were not able to make use of the name/value pairing with the := separator until version 10.1. As of 10.1 all classes support the name/value pairing.\n"
                    "\n"
                    "The class command deprecates the findclass and matchclass commands as it offers better functionality and performance than the older commands."
                ),
                source=_SOURCE,
                examples=(
                    "when LB_FAILED {\n"
                    '      HTTP::respond 200 content [b64decode [class element -value 0 img]] "Content-Type" "image/png"\n'
                    "   }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="class <subcommand> ?options? ?--? args...",
                    options=(
                        OptionSpec(name="-all", detail="Return all matches.", takes_value=False),
                        OptionSpec(
                            name="-value", detail="Return value instead of name.", takes_value=False
                        ),
                        OptionSpec(name="-name", detail="Return name.", takes_value=False),
                        OptionSpec(name="-index", detail="Return index.", takes_value=False),
                        OptionSpec(
                            name="-element", detail="Return full element.", takes_value=False
                        ),
                        OptionSpec(
                            name="-nocase", detail="Case-insensitive comparison.", takes_value=False
                        ),
                        OptionSpec(
                            name="-list",
                            detail="Return value always as a list.",
                            takes_value=False,
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "match",
                                "Match an item against a data group.",
                                "class match ?options? ?--? <item> <operator> <class>",
                            ),
                            _av(
                                "search",
                                "Search a data group for an item.",
                                "class search ?options? ?--? <class> <operator> <item>",
                            ),
                            _av(
                                "lookup",
                                "Return the value paired with a name.",
                                "class lookup <name> <class>",
                            ),
                            _av(
                                "element",
                                "Return an element by index.",
                                "class element ?-value|-name? ?--? <index> <class>",
                            ),
                            _av(
                                "type",
                                "Return the data type of a data group.",
                                "class type <class>",
                            ),
                            _av("exists", "Check if a data group exists.", "class exists <class>"),
                            _av("size", "Return the number of elements.", "class size <class>"),
                            _av(
                                "names",
                                "Return list of data group names.",
                                "class names ?-nocase? ?-list? ?--? <class> ?pattern?",
                            ),
                            _av(
                                "get",
                                "Return all elements as a list.",
                                "class get ?-nocase? ?-list? ?--? <class> ?pattern?",
                            ),
                            _av(
                                "startsearch",
                                "Begin iterating over a data group.",
                                "class startsearch <class>",
                            ),
                            _av(
                                "nextelement",
                                "Get next element during iteration.",
                                "class nextelement ?options? ?--? <search_id>",
                            ),
                            _av(
                                "anymore",
                                "Check if more elements remain.",
                                "class anymore <search_id>",
                            ),
                            _av(
                                "donesearch",
                                "End a data group iteration.",
                                "class donesearch <search_id>",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            subcommands={
                "match": SubCommand(
                    name="match",
                    arity=Arity(),
                    detail="Match an item against a data group.",
                    synopsis="class match ?options? ?--? <item> <operator> <class>",
                    pure=True,
                    options=(
                        OptionSpec(name="-all", detail="Return all matches."),
                        OptionSpec(name="-value", detail="Return value instead of name."),
                        OptionSpec(name="-name", detail="Return name."),
                        OptionSpec(name="-index", detail="Return index."),
                        OptionSpec(name="-element", detail="Return full element."),
                        OptionSpec(name="-nocase", detail="Case-insensitive comparison."),
                        OptionSpec(name="--"),
                    ),
                ),
                "search": SubCommand(
                    name="search",
                    arity=Arity(),
                    detail="Search a data group for an item.",
                    synopsis="class search ?options? ?--? <class> <operator> <item>",
                    pure=True,
                    options=(
                        OptionSpec(name="-all", detail="Return all matches."),
                        OptionSpec(name="-value", detail="Return value instead of name."),
                        OptionSpec(name="-name", detail="Return name."),
                        OptionSpec(name="-index", detail="Return index."),
                        OptionSpec(name="-element", detail="Return full element."),
                        OptionSpec(name="-nocase", detail="Case-insensitive comparison."),
                        OptionSpec(name="--"),
                    ),
                ),
                "lookup": SubCommand(
                    name="lookup",
                    arity=Arity(2, 2),
                    detail="Return the value paired with a name.",
                    synopsis="class lookup <name> <class>",
                    pure=True,
                ),
                "element": SubCommand(
                    name="element",
                    arity=Arity(2, 2),
                    detail="Return an element by index.",
                    synopsis="class element ?-value|-name? ?--? <index> <class>",
                    pure=True,
                    options=(
                        OptionSpec(name="-value", detail="Return value instead of name."),
                        OptionSpec(name="-name", detail="Return name."),
                        OptionSpec(name="--"),
                    ),
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(1, 1),
                    detail="Return the data type of a data group.",
                    synopsis="class type <class>",
                    pure=True,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Check if a data group exists.",
                    synopsis="class exists <class>",
                    pure=True,
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Return the number of elements.",
                    synopsis="class size <class>",
                    pure=True,
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(1, 2),
                    detail="Return list of data group names.",
                    synopsis="class names ?-nocase? ?-list? ?--? <class> ?pattern?",
                    pure=True,
                    options=(
                        OptionSpec(name="-nocase", detail="Case-insensitive comparison."),
                        OptionSpec(name="-list", detail="Return value always as a list."),
                        OptionSpec(name="--"),
                    ),
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(1, 2),
                    detail="Return all elements as a list.",
                    synopsis="class get ?-nocase? ?-list? ?--? <class> ?pattern?",
                    pure=True,
                    options=(
                        OptionSpec(name="-nocase", detail="Case-insensitive comparison."),
                        OptionSpec(name="-list", detail="Return value always as a list."),
                        OptionSpec(name="--"),
                    ),
                ),
                "startsearch": SubCommand(
                    name="startsearch",
                    arity=Arity(1, 1),
                    detail="Begin iterating over a data group.",
                    synopsis="class startsearch <class>",
                ),
                "nextelement": SubCommand(
                    name="nextelement",
                    arity=Arity(1, 1),
                    detail="Get next element during iteration.",
                    synopsis="class nextelement ?options? ?--? <search_id>",
                    options=(
                        OptionSpec(name="-value", detail="Return value instead of name."),
                        OptionSpec(name="-name", detail="Return name."),
                        OptionSpec(name="--"),
                    ),
                ),
                "anymore": SubCommand(
                    name="anymore",
                    arity=Arity(1, 1),
                    detail="Check if more elements remain.",
                    synopsis="class anymore <search_id>",
                    pure=True,
                ),
                "donesearch": SubCommand(
                    name="donesearch",
                    arity=Arity(1, 1),
                    detail="End a data group iteration.",
                    synopsis="class donesearch <search_id>",
                ),
            },
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    scope=StorageScope.DATA_GROUP,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
