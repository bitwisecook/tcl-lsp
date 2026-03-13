"""string -- Manipulate strings."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionTerminatorSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register


def _av(value: str, detail: str, synopsis: str = "", summary: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=summary or detail,
        synopsis=(synopsis,) if synopsis else (),
        source="Tcl man page string.n",
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av("cat", "Concatenate strings.", "string cat ?string1? ?string2 ...?"),
    _av(
        "compare",
        "Compare two strings lexicographically.",
        "string compare ?-nocase? ?-length length? string1 string2",
    ),
    _av(
        "equal", "Test string equality.", "string equal ?-nocase? ?-length length? string1 string2"
    ),
    _av(
        "first",
        "Find first occurrence of needle in haystack.",
        "string first needleString haystackString ?startIndex?",
    ),
    _av("index", "Return character at index.", "string index string charIndex"),
    _av("insert", "Insert string at index.", "string insert string index insertString"),
    _av(
        "is",
        "Test if string is a member of a character class.",
        "string is class ?-strict? ?-failindex varname? string",
    ),
    _av(
        "last",
        "Find last occurrence of needle in haystack.",
        "string last needleString haystackString ?lastIndex?",
    ),
    _av("length", "Return number of characters.", "string length string"),
    _av("map", "Map substrings via key-value pairs.", "string map ?-nocase? mapping string"),
    _av("match", "Test glob-style pattern match.", "string match ?-nocase? pattern string"),
    _av("range", "Return substring by index range.", "string range string first last"),
    _av("repeat", "Repeat string N times.", "string repeat string count"),
    _av(
        "replace", "Replace range with new string.", "string replace string first last ?newString?"
    ),
    _av("reverse", "Reverse character order.", "string reverse string"),
    _av("tolower", "Convert to lower case.", "string tolower string ?first? ?last?"),
    _av("totitle", "Convert to title case.", "string totitle string ?first? ?last?"),
    _av("toupper", "Convert to upper case.", "string toupper string ?first? ?last?"),
    _av("trim", "Trim leading and trailing characters.", "string trim string ?chars?"),
    _av("trimleft", "Trim leading characters.", "string trimleft string ?chars?"),
    _av("trimright", "Trim trailing characters.", "string trimright string ?chars?"),
    _av("wordend", "Index of character after end of word.", "string wordend string charIndex"),
    _av("wordstart", "Index of first character of word.", "string wordstart string charIndex"),
)


def _is_class(value: str, detail: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail,
            synopsis=(f"string is {value} ?-strict? ?-failindex varname? string",),
            source="Tcl man page string.n",
        ),
    )


_IS_CLASSES = (
    _is_class("alnum", "Any Unicode alphabet or digit character."),
    _is_class("alpha", "Any Unicode alphabet character."),
    _is_class("ascii", "Any character with a value less than U+0080 (7-bit ASCII)."),
    _is_class("boolean", "Any valid boolean value (true/false/yes/no/on/off/0/1)."),
    _is_class("control", "Any Unicode control character."),
    _is_class("dict", "Any proper dict structure, with optional surrounding whitespace."),
    _is_class("digit", "Any Unicode digit character."),
    _is_class("double", "Any valid floating-point number."),
    _is_class("entier", "Synonym for integer."),
    _is_class("false", "Any valid boolean false value."),
    _is_class("graph", "Any Unicode printing character, except space."),
    _is_class("integer", "Any valid integer of arbitrary size."),
    _is_class("list", "Any proper list structure, with optional surrounding whitespace."),
    _is_class("lower", "Any Unicode lower case alphabet character."),
    _is_class("print", "Any Unicode printing character, including space."),
    _is_class("punct", "Any Unicode punctuation character."),
    _is_class("space", "Any Unicode whitespace character."),
    _is_class("true", "Any valid boolean true value."),
    _is_class("upper", "Any upper case alphabet character."),
    _is_class("wideinteger", "Any valid wide integer."),
    _is_class("wordchar", "Any Unicode word character (alphanumeric + connector punctuation)."),
    _is_class("xdigit", "Any hexadecimal digit character (0-9, A-F, a-f)."),
)


@register
class StringCommand(CommandDef):
    name = "string"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="string",
            hover=HoverSnippet(
                summary="Perform one of several string operations.",
                synopsis=("string option arg ?arg ...?",),
                snippet="Use subcommands like `length`, `match`, `is`, `compare`, `map`, `range`, etc.",
                source="Tcl man page string.n",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="string option arg ?arg ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "cat": SubCommand(
                    name="cat",
                    arity=Arity(0),
                    detail="Concatenate strings.",
                    synopsis="string cat ?string1? ?string2 ...?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "compare": SubCommand(
                    name="compare",
                    arity=Arity(2),
                    detail="Compare two strings lexicographically.",
                    synopsis="string compare ?-nocase? ?-length length? string1 string2",
                    pure=True,
                    return_type=TclType.INT,
                    option_terminator=OptionTerminatorSpec(
                        scan_start=1, options_with_values=frozenset({"-length"})
                    ),
                ),
                "equal": SubCommand(
                    name="equal",
                    arity=Arity(2),
                    detail="Test string equality.",
                    synopsis="string equal ?-nocase? ?-length length? string1 string2",
                    pure=True,
                    return_type=TclType.BOOLEAN,
                    option_terminator=OptionTerminatorSpec(
                        scan_start=1, options_with_values=frozenset({"-length"})
                    ),
                ),
                "first": SubCommand(
                    name="first",
                    arity=Arity(2, 3),
                    detail="Find first occurrence of needle in haystack.",
                    synopsis="string first needleString haystackString ?startIndex?",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "index": SubCommand(
                    name="index",
                    arity=Arity(2, 2),
                    detail="Return character at index.",
                    synopsis="string index string charIndex",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(3, 3),
                    detail="Insert string at index.",
                    synopsis="string insert string index insertString",
                ),
                "is": SubCommand(
                    name="is",
                    arity=Arity(2),
                    detail="Test if string is a member of a character class.",
                    synopsis="string is class ?-strict? ?-failindex varname? string",
                    pure=True,
                    return_type=TclType.BOOLEAN,
                    arg_values={0: _IS_CLASSES},
                ),
                "last": SubCommand(
                    name="last",
                    arity=Arity(2, 3),
                    detail="Find last occurrence of needle in haystack.",
                    synopsis="string last needleString haystackString ?lastIndex?",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "length": SubCommand(
                    name="length",
                    arity=Arity(1, 1),
                    detail="Return number of characters.",
                    synopsis="string length string",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "map": SubCommand(
                    name="map",
                    arity=Arity(2),
                    detail="Map substrings via key-value pairs.",
                    synopsis="string map ?-nocase? mapping string",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "match": SubCommand(
                    name="match",
                    arity=Arity(2),
                    detail="Test glob-style pattern match.",
                    synopsis="string match ?-nocase? pattern string",
                    pure=True,
                    return_type=TclType.BOOLEAN,
                    option_terminator=OptionTerminatorSpec(scan_start=1),
                ),
                "range": SubCommand(
                    name="range",
                    arity=Arity(3, 3),
                    detail="Return substring by index range.",
                    synopsis="string range string first last",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "repeat": SubCommand(
                    name="repeat",
                    arity=Arity(2, 2),
                    detail="Repeat string N times.",
                    synopsis="string repeat string count",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(3, 4),
                    detail="Replace range with new string.",
                    synopsis="string replace string first last ?newString?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "reverse": SubCommand(
                    name="reverse",
                    arity=Arity(1, 1),
                    detail="Reverse character order.",
                    synopsis="string reverse string",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "tolower": SubCommand(
                    name="tolower",
                    arity=Arity(1, 3),
                    detail="Convert to lower case.",
                    synopsis="string tolower string ?first? ?last?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "totitle": SubCommand(
                    name="totitle",
                    arity=Arity(1, 3),
                    detail="Convert to title case.",
                    synopsis="string totitle string ?first? ?last?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "toupper": SubCommand(
                    name="toupper",
                    arity=Arity(1, 3),
                    detail="Convert to upper case.",
                    synopsis="string toupper string ?first? ?last?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "trim": SubCommand(
                    name="trim",
                    arity=Arity(1, 2),
                    detail="Trim leading and trailing characters.",
                    synopsis="string trim string ?chars?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "trimleft": SubCommand(
                    name="trimleft",
                    arity=Arity(1, 2),
                    detail="Trim leading characters.",
                    synopsis="string trimleft string ?chars?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "trimright": SubCommand(
                    name="trimright",
                    arity=Arity(1, 2),
                    detail="Trim trailing characters.",
                    synopsis="string trimright string ?chars?",
                    pure=True,
                    return_type=TclType.STRING,
                ),
                "wordend": SubCommand(
                    name="wordend",
                    arity=Arity(2, 2),
                    detail="Index of character after end of word.",
                    synopsis="string wordend string charIndex",
                    pure=True,
                    return_type=TclType.INT,
                ),
                "wordstart": SubCommand(
                    name="wordstart",
                    arity=Arity(2, 2),
                    detail="Index of first character of word.",
                    synopsis="string wordstart string charIndex",
                    pure=True,
                    return_type=TclType.INT,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            option_terminator_profiles=(
                OptionTerminatorSpec(scan_start=1, subcommand="match"),
                OptionTerminatorSpec(
                    scan_start=1,
                    options_with_values=frozenset({"-length"}),
                    subcommand="equal",
                ),
                OptionTerminatorSpec(
                    scan_start=1,
                    options_with_values=frozenset({"-length"}),
                    subcommand="compare",
                ),
            ),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
