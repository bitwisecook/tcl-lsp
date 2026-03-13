# Enriched from F5 iRules reference documentation.
"""class -- Advanced access of classes."""

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
    OptionTerminatorSpec,
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
                                "class lookup ?--? <name> <class>",
                            ),
                            _av(
                                "element",
                                "Return an element by index.",
                                "class element ?-value|-name? <index> <class>",
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
                                "class names ?-nocase? ?pattern?",
                            ),
                            _av("get", "Return all elements as a list.", "class get <class>"),
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
            option_terminator_profiles=(
                OptionTerminatorSpec(scan_start=1, subcommand="match"),
                OptionTerminatorSpec(scan_start=1, subcommand="search"),
                OptionTerminatorSpec(scan_start=1, subcommand="nextelement"),
            ),
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
