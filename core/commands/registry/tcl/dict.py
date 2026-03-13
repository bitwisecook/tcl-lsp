# Scaffolded from dict.n -- refine and commit
"""dict -- Manipulate dictionaries."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page dict.n"


_av = make_av(_SOURCE)


@register
class DictCommand(CommandDef):
    name = "dict"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="dict",
            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Manipulate dictionaries",
                synopsis=("dict option arg ?arg ...?",),
                snippet="Performs one of several operations on dictionary values or variables containing dictionary values (see the DICTIONARY VALUES section below for a description), depending on option.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="dict option arg ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "This appends the given string (or strings) to the value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary value back to that variable.",
                                "dict append dictionaryVariable key ?string ...?",
                            ),
                            _av(
                                "create",
                                "Return a new dictionary that contains each of the key/value mappings listed as arguments (keys and values alternating, with each key being followed by its associated value.)",
                                "dict create ?key value ...?",
                            ),
                            _av(
                                "exists",
                                "This returns a boolean value indicating whether the given key (or path of keys through a set of nested dictionaries) exists in the given dictionary value.",
                                "dict exists dictionaryValue key ?key ...?",
                            ),
                            _av(
                                "filter",
                                "This takes a dictionary value and returns a new dictionary that contains just those key/value pairs that match the specified filter type (which may be abbreviated.) Supported filter types are: dict filter dictionaryValu…",
                                "dict filter dictionaryValue filterType arg ?arg ...?",
                            ),
                            _av(
                                "for",
                                "This command takes three arguments, the first a two-element list of variable names (for the key and value respectively of each mapping in the dictionary), the second the dictionary value to iterate across, and the third…",
                                "dict for {keyVariable valueVariable} dictionaryValue body",
                            ),
                            _av(
                                "get",
                                "Given a dictionary value (first argument) and a key (second argument), this will retrieve the value for that key.",
                                "dict get dictionaryValue ?key ...?",
                            ),
                            _av(
                                "incr",
                                "This adds the given increment value (an integer that defaults to 1 if not specified) to the value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary valu…",
                                "dict incr dictionaryVariable key ?increment?",
                            ),
                            _av(
                                "info",
                                "This returns information (intended for display to people) about the given dictionary though the format of this data is dependent on the implementation of the dictionary.",
                                "dict info dictionaryValue",
                            ),
                            _av(
                                "keys",
                                "Return a list of all keys in the given dictionary value.",
                                "dict keys dictionaryValue ?globPattern?",
                            ),
                            _av(
                                "lappend",
                                "This appends the given items to the list value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary value back to that variable.",
                                "dict lappend dictionaryVariable key ?value ...?",
                            ),
                            _av(
                                "map",
                                "This command applies a transformation to each element of a dictionary, returning a new dictionary.",
                                "dict map {keyVariable valueVariable} dictionaryValue body",
                            ),
                            _av(
                                "merge",
                                "Return a dictionary that contains the contents of each of the dictionaryValue arguments.",
                                "dict merge ?dictionaryValue ...?",
                            ),
                            _av(
                                "remove",
                                "Return a new dictionary that is a copy of an old one passed in as first argument except without mappings for each of the keys listed.",
                                "dict remove dictionaryValue ?key ...?",
                            ),
                            _av(
                                "replace",
                                "Return a new dictionary that is a copy of an old one passed in as first argument except with some values different or some extra key/value pairs added.",
                                "dict replace dictionaryValue ?key value ...?",
                            ),
                            _av(
                                "set",
                                "This operation takes the name of a variable containing a dictionary value and places an updated dictionary value in that variable containing a mapping from the given key to the given value.",
                                "dict set dictionaryVariable key ?key ...? value",
                            ),
                            _av(
                                "size",
                                "Return the number of key/value mappings in the given dictionary value.",
                                "dict size dictionaryValue",
                            ),
                            _av(
                                "unset",
                                "This operation (the companion to dict set) takes the name of a variable containing a dictionary value and places an updated dictionary value in that variable that does not contain a mapping for the given key.",
                                "dict unset dictionaryVariable key ?key ...?",
                            ),
                            _av(
                                "update",
                                "Execute the Tcl script in body with the value for each key (as found by reading the dictionary value in dictionaryVariable) mapped to the variable varName.",
                                "dict update dictionaryVariable key varName ?key varName ...? body",
                            ),
                            _av(
                                "values",
                                "Return a list of all values in the given dictionary value.",
                                "dict values dictionaryValue ?globPattern?",
                            ),
                            _av(
                                "with",
                                "Execute the Tcl script in body with the value for each key in dictionaryVariable mapped (in a manner similarly to dict update) to a variable with the same name.",
                                "dict with dictionaryVariable ?key ...? body",
                            ),
                            _av("getdef", "dict getdef", "dict getdef"),
                            _av(
                                "getwithdefault",
                                "dict getwithdefault dictionaryValue ?key ...?",
                                "dict getdef dictionaryValue ?key ...? key default",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "append": SubCommand(
                    name="append",
                    arity=Arity(2),
                    detail="This appends the given string (or strings) to the value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary value back to that variable.",
                    synopsis="dict append dictionaryVariable key ?string ...?",
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(0),
                    detail="Return a new dictionary that contains each of the key/value mappings listed as arguments (keys and values alternating, with each key being followed by its associated value.)",
                    synopsis="dict create ?key value ...?",
                    pure=True,
                    return_type=TclType.DICT,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(2),
                    detail="This returns a boolean value indicating whether the given key (or path of keys through a set of nested dictionaries) exists in the given dictionary value.",
                    synopsis="dict exists dictionaryValue key ?key ...?",
                    pure=True,
                    return_type=TclType.BOOLEAN,
                ),
                "filter": SubCommand(
                    name="filter",
                    arity=Arity(3),
                    detail="This takes a dictionary value and returns a new dictionary that contains just those key/value pairs that match the specified filter type (which may be abbreviated.) Supported filter types are: dict filter dictionaryValu…",
                    synopsis="dict filter dictionaryValue filterType arg ?arg ...?",
                    return_type=TclType.DICT,
                ),
                "for": SubCommand(
                    name="for",
                    arity=Arity(3, 3),
                    detail="This command takes three arguments, the first a two-element list of variable names (for the key and value respectively of each mapping in the dictionary), the second the dictionary value to iterate across, and the third…",
                    synopsis="dict for {keyVariable valueVariable} dictionaryValue body",
                    arg_roles={2: ArgRole.BODY},
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(1),
                    detail="Given a dictionary value (first argument) and a key (second argument), this will retrieve the value for that key.",
                    synopsis="dict get dictionaryValue ?key ...?",
                    pure=True,
                    return_type=TclType.STRING,
                    arg_types={0: ArgTypeHint(expected=TclType.DICT)},
                ),
                "incr": SubCommand(
                    name="incr",
                    arity=Arity(2, 3),
                    detail="This adds the given increment value (an integer that defaults to 1 if not specified) to the value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary valu…",
                    synopsis="dict incr dictionaryVariable key ?increment?",
                    return_type=TclType.INT,
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(1, 1),
                    detail="This returns information (intended for display to people) about the given dictionary though the format of this data is dependent on the implementation of the dictionary.",
                    synopsis="dict info dictionaryValue",
                    return_type=TclType.STRING,
                ),
                "keys": SubCommand(
                    name="keys",
                    arity=Arity(1, 2),
                    detail="Return a list of all keys in the given dictionary value.",
                    synopsis="dict keys dictionaryValue ?globPattern?",
                    pure=True,
                    return_type=TclType.LIST,
                    arg_types={0: ArgTypeHint(expected=TclType.DICT)},
                ),
                "lappend": SubCommand(
                    name="lappend",
                    arity=Arity(2),
                    detail="This appends the given items to the list value that the given key maps to in the dictionary value contained in the given variable, writing the resulting dictionary value back to that variable.",
                    synopsis="dict lappend dictionaryVariable key ?value ...?",
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "map": SubCommand(
                    name="map",
                    arity=Arity(3, 3),
                    detail="This command applies a transformation to each element of a dictionary, returning a new dictionary.",
                    synopsis="dict map {keyVariable valueVariable} dictionaryValue body",
                    arg_roles={2: ArgRole.BODY},
                ),
                "merge": SubCommand(
                    name="merge",
                    arity=Arity(0),
                    detail="Return a dictionary that contains the contents of each of the dictionaryValue arguments.",
                    synopsis="dict merge ?dictionaryValue ...?",
                    pure=True,
                    return_type=TclType.DICT,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1),
                    detail="Return a new dictionary that is a copy of an old one passed in as first argument except without mappings for each of the keys listed.",
                    synopsis="dict remove dictionaryValue ?key ...?",
                    return_type=TclType.DICT,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(1),
                    detail="Return a new dictionary that is a copy of an old one passed in as first argument except with some values different or some extra key/value pairs added.",
                    synopsis="dict replace dictionaryValue ?key value ...?",
                    return_type=TclType.DICT,
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(3),
                    detail="This operation takes the name of a variable containing a dictionary value and places an updated dictionary value in that variable containing a mapping from the given key to the given value.",
                    synopsis="dict set dictionaryVariable key ?key ...? value",
                    return_type=TclType.DICT,
                    arg_roles={0: ArgRole.VAR_NAME},
                    arg_types={0: ArgTypeHint(expected=TclType.DICT)},
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Return the number of key/value mappings in the given dictionary value.",
                    synopsis="dict size dictionaryValue",
                    pure=True,
                    return_type=TclType.INT,
                    arg_types={0: ArgTypeHint(expected=TclType.DICT)},
                ),
                "unset": SubCommand(
                    name="unset",
                    arity=Arity(2),
                    detail="This operation (the companion to dict set) takes the name of a variable containing a dictionary value and places an updated dictionary value in that variable that does not contain a mapping for the given key.",
                    synopsis="dict unset dictionaryVariable key ?key ...?",
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "update": SubCommand(
                    name="update",
                    arity=Arity(3),
                    detail="Execute the Tcl script in body with the value for each key (as found by reading the dictionary value in dictionaryVariable) mapped to the variable varName.",
                    synopsis="dict update dictionaryVariable key varName ?key varName ...? body",
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "values": SubCommand(
                    name="values",
                    arity=Arity(1, 2),
                    detail="Return a list of all values in the given dictionary value.",
                    synopsis="dict values dictionaryValue ?globPattern?",
                    pure=True,
                    return_type=TclType.LIST,
                    arg_types={0: ArgTypeHint(expected=TclType.DICT)},
                ),
                "with": SubCommand(
                    name="with",
                    arity=Arity(2),
                    detail="Execute the Tcl script in body with the value for each key in dictionaryVariable mapped (in a manner similarly to dict update) to a variable with the same name.",
                    synopsis="dict with dictionaryVariable ?key ...? body",
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
