"""string -- Manipulate strings."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
    WasmRuntimeImport,
)
from ..signatures import Arity
from ..type_hints import ArgTypeHint
from ._base import register
from .const_fold import (
    fold_string_cat,
    fold_string_compare,
    fold_string_equal,
    fold_string_first,
    fold_string_index,
    fold_string_is,
    fold_string_last,
    fold_string_length,
    fold_string_map,
    fold_string_match,
    fold_string_range,
    fold_string_repeat,
    fold_string_replace,
    fold_string_reverse,
    fold_string_tolower,
    fold_string_totitle,
    fold_string_toupper,
    fold_string_trim,
    fold_string_trimleft,
    fold_string_trimright,
)


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
                    dialects=frozenset({"tcl8.6", "tcl9.0"}),
                    arity=Arity(0),
                    detail="Concatenate strings.",
                    synopsis="string cat ?string1? ?string2 ...?",
                    pure=True,
                    const_fold=fold_string_cat,
                    return_type=TclType.STRING,
                ),
                "compare": SubCommand(
                    name="compare",
                    # C Tcl 9.0 ``StringCmpOpts``: objc must be 3..6
                    # (sub-name + ``-nocase?`` + ``-length N?`` + s1 +
                    # s2).  Args after sub-name: 2..5.
                    arity=Arity(2, 5),
                    detail="Compare two strings lexicographically.",
                    synopsis="string compare ?-nocase? ?-length length? string1 string2",
                    pure=True,
                    const_fold=fold_string_compare,
                    return_type=TclType.INT,
                    options=(
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-length", takes_value=True, value_hint="int"),
                    ),
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_compare",
                        export_name="string_compare",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "equal": SubCommand(
                    name="equal",
                    # C Tcl 9.0 ``StringEqualCmd`` shares ``StringCmpOpts``:
                    # objc 3..6, i.e. args after sub-name 2..5 (with
                    # optional ``-nocase`` and ``-length N``).
                    arity=Arity(2, 5),
                    detail="Test string equality.",
                    synopsis="string equal ?-nocase? ?-length length? string1 string2",
                    pure=True,
                    const_fold=fold_string_equal,
                    return_type=TclType.BOOLEAN,
                    options=(
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-length", takes_value=True, value_hint="int"),
                    ),
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_equal",
                        export_name="string_equal",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "first": SubCommand(
                    name="first",
                    arity=Arity(2, 3),
                    detail="Find first occurrence of needle in haystack.",
                    synopsis="string first needleString haystackString ?startIndex?",
                    pure=True,
                    const_fold=fold_string_first,
                    return_type=TclType.INT,
                    arg_types={2: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_first",
                        export_name="string_first",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "index": SubCommand(
                    name="index",
                    arity=Arity(2, 2),
                    detail="Return character at index.",
                    synopsis="string index string charIndex",
                    pure=True,
                    const_fold=fold_string_index,
                    return_type=TclType.STRING,
                    arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_index",
                        export_name="string_index",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "insert": SubCommand(
                    name="insert",
                    dialects=frozenset({"tcl9.0"}),
                    arity=Arity(3, 3),
                    detail="Insert string at index.",
                    synopsis="string insert string index insertString",
                    pure=True,
                    return_type=TclType.STRING,
                    arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                ),
                "is": SubCommand(
                    name="is",
                    # C Tcl 9.0 ``StringIsCmd``: objc 3..6 → args after
                    # sub-name 2..5 (class + ``-strict?`` +
                    # ``-failindex V?`` + string).
                    arity=Arity(2, 5),
                    detail="Test if string is a member of a character class.",
                    synopsis="string is class ?-strict? ?-failindex varname? string",
                    const_fold=fold_string_is,
                    return_type=TclType.BOOLEAN,
                    arg_values={0: _IS_CLASSES},
                    options=(
                        OptionSpec(name="-strict"),
                        OptionSpec(name="-failindex", takes_value=True, value_hint="varname"),
                    ),
                ),
                "last": SubCommand(
                    name="last",
                    arity=Arity(2, 3),
                    detail="Find last occurrence of needle in haystack.",
                    synopsis="string last needleString haystackString ?lastIndex?",
                    pure=True,
                    const_fold=fold_string_last,
                    return_type=TclType.INT,
                    arg_types={2: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_last",
                        export_name="string_last",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "length": SubCommand(
                    name="length",
                    arity=Arity(1, 1),
                    detail="Return number of characters.",
                    synopsis="string length string",
                    pure=True,
                    const_fold=fold_string_length,
                    return_type=TclType.INT,
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_length",
                        export_name="string_length",
                        params=("i32",),
                        results=("i32",),
                    ),
                ),
                "map": SubCommand(
                    name="map",
                    # C Tcl 9.0 ``StringMapCmd``: objc 3..4 → args after
                    # sub-name 2..3 (``-nocase?`` + mapping + string).
                    arity=Arity(2, 3),
                    detail="Map substrings via key-value pairs.",
                    synopsis="string map ?-nocase? mapping string",
                    pure=True,
                    const_fold=fold_string_map,
                    return_type=TclType.STRING,
                    options=(OptionSpec(name="-nocase"),),
                    arg_types={0: ArgTypeHint(expected=TclType.DICT, shimmers=True)},
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_map",
                        export_name="string_map",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "match": SubCommand(
                    name="match",
                    # C Tcl 9.0 ``StringMatchCmd``: objc 3..4 → args after
                    # sub-name 2..3 (``-nocase?`` + pattern + string).
                    arity=Arity(2, 3),
                    detail="Test glob-style pattern match.",
                    synopsis="string match ?-nocase? pattern string",
                    pure=True,
                    const_fold=fold_string_match,
                    return_type=TclType.BOOLEAN,
                    options=(OptionSpec(name="-nocase"),),
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_match",
                        export_name="string_match",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "range": SubCommand(
                    name="range",
                    arity=Arity(3, 3),
                    detail="Return substring by index range.",
                    synopsis="string range string first last",
                    pure=True,
                    const_fold=fold_string_range,
                    return_type=TclType.STRING,
                    arg_types={
                        1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                        2: ArgTypeHint(expected=TclType.INT, shimmers=True),
                    },
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_range",
                        export_name="string_range",
                        params=("i32", "i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "repeat": SubCommand(
                    name="repeat",
                    arity=Arity(2, 2),
                    detail="Repeat string N times.",
                    synopsis="string repeat string count",
                    pure=True,
                    const_fold=fold_string_repeat,
                    return_type=TclType.STRING,
                    arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_repeat",
                        export_name="string_repeat",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(3, 4),
                    detail="Replace range with new string.",
                    synopsis="string replace string first last ?newString?",
                    pure=True,
                    const_fold=fold_string_replace,
                    return_type=TclType.STRING,
                    arg_types={
                        1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                        2: ArgTypeHint(expected=TclType.INT, shimmers=True),
                    },
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_replace",
                        export_name="string_replace",
                        params=("i32", "i32", "i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "reverse": SubCommand(
                    name="reverse",
                    arity=Arity(1, 1),
                    detail="Reverse character order.",
                    synopsis="string reverse string",
                    pure=True,
                    const_fold=fold_string_reverse,
                    return_type=TclType.STRING,
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_reverse",
                        export_name="string_reverse",
                        params=("i32",),
                        results=("i32",),
                    ),
                ),
                "tolower": SubCommand(
                    name="tolower",
                    arity=Arity(1, 3),
                    detail="Convert to lower case.",
                    synopsis="string tolower string ?first? ?last?",
                    pure=True,
                    const_fold=fold_string_tolower,
                    return_type=TclType.STRING,
                    arg_types={
                        1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                        2: ArgTypeHint(expected=TclType.INT, shimmers=True),
                    },
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_tolower",
                        export_name="string_tolower",
                        params=("i32",),
                        results=("i32",),
                    ),
                ),
                "totitle": SubCommand(
                    name="totitle",
                    arity=Arity(1, 3),
                    detail="Convert to title case.",
                    synopsis="string totitle string ?first? ?last?",
                    pure=True,
                    const_fold=fold_string_totitle,
                    return_type=TclType.STRING,
                    arg_types={
                        1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                        2: ArgTypeHint(expected=TclType.INT, shimmers=True),
                    },
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_totitle",
                        export_name="string_totitle",
                        params=("i32",),
                        results=("i32",),
                    ),
                ),
                "toupper": SubCommand(
                    name="toupper",
                    arity=Arity(1, 3),
                    detail="Convert to upper case.",
                    synopsis="string toupper string ?first? ?last?",
                    pure=True,
                    const_fold=fold_string_toupper,
                    return_type=TclType.STRING,
                    arg_types={
                        1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                        2: ArgTypeHint(expected=TclType.INT, shimmers=True),
                    },
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_toupper",
                        export_name="string_toupper",
                        params=("i32",),
                        results=("i32",),
                    ),
                ),
                "trim": SubCommand(
                    name="trim",
                    arity=Arity(1, 2),
                    detail="Trim leading and trailing characters.",
                    synopsis="string trim string ?chars?",
                    pure=True,
                    const_fold=fold_string_trim,
                    return_type=TclType.STRING,
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_trim",
                        export_name="string_trim",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "trimleft": SubCommand(
                    name="trimleft",
                    arity=Arity(1, 2),
                    detail="Trim leading characters.",
                    synopsis="string trimleft string ?chars?",
                    pure=True,
                    const_fold=fold_string_trimleft,
                    return_type=TclType.STRING,
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_trimleft",
                        export_name="string_trimleft",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "trimright": SubCommand(
                    name="trimright",
                    arity=Arity(1, 2),
                    detail="Trim trailing characters.",
                    synopsis="string trimright string ?chars?",
                    pure=True,
                    const_fold=fold_string_trimright,
                    return_type=TclType.STRING,
                    wasm_runtime_import=WasmRuntimeImport(
                        import_key="tcl_string_trimright",
                        export_name="string_trimright",
                        params=("i32", "i32"),
                        results=("i32",),
                    ),
                ),
                "wordend": SubCommand(
                    name="wordend",
                    arity=Arity(2, 2),
                    detail="Index of character after end of word.",
                    synopsis="string wordend string charIndex",
                    pure=True,
                    return_type=TclType.INT,
                    arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                ),
                "wordstart": SubCommand(
                    name="wordstart",
                    arity=Arity(2, 2),
                    detail="Index of first character of word.",
                    synopsis="string wordstart string charIndex",
                    pure=True,
                    return_type=TclType.INT,
                    arg_types={1: ArgTypeHint(expected=TclType.INT, shimmers=True)},
                ),
                "bytelength": SubCommand(
                    name="bytelength",
                    dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6"}),
                    arity=Arity(1, 1),
                    detail="Return number of bytes used to represent the string in memory.",
                    synopsis="string bytelength string",
                    pure=True,
                    return_type=TclType.INT,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            cse_candidate=True,
            side_effect_hints=(),
        )
