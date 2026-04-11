//! `string` — perform one of several string operations.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bytelength",
        arity: Arity::exact(1),
        detail: "Return number of bytes used to represent the string in memory.",
        synopsis: "string bytelength string",
        pure: true,
        return_type: Some(TclType::Int),
        dialects: Some(
            DialectSet::TCL84
                .union(DialectSet::TCL85)
                .union(DialectSet::TCL86),
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cat",
        arity: Arity::any(),
        detail: "Concatenate strings.",
        synopsis: "string cat ?string1? ?string2 ...?",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "compare",
        arity: Arity::at_least(2),
        detail: "Compare two strings lexicographically.",
        synopsis: "string compare ?-nocase? ?-length length? string1 string2",
        pure: true,
        return_type: Some(TclType::Int),
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-length",
                takes_value: true,
                value_hint: "int",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "equal",
        arity: Arity::at_least(2),
        detail: "Test string equality.",
        synopsis: "string equal ?-nocase? ?-length length? string1 string2",
        pure: true,
        return_type: Some(TclType::Boolean),
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-length",
                takes_value: true,
                value_hint: "int",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "first",
        arity: Arity::new(2, 3),
        detail: "Find first occurrence of needle in haystack.",
        synopsis: "string first needleString haystackString ?startIndex?",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            2,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(2),
        detail: "Return character at index.",
        synopsis: "string index string charIndex",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(3),
        detail: "Insert string at index.",
        synopsis: "string insert string index insertString",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "is",
        arity: Arity::at_least(2),
        detail: "Test if string is a member of a character class.",
        synopsis: "string is class ?-strict? ?-failindex varname? string",
        return_type: Some(TclType::Boolean),
        options: &[
            OptionSpec {
                name: "-strict",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-failindex",
                takes_value: true,
                value_hint: "varname",
                detail: "",
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "last",
        arity: Arity::new(2, 3),
        detail: "Find last occurrence of needle in haystack.",
        synopsis: "string last needleString haystackString ?lastIndex?",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            2,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(1),
        detail: "Return number of characters.",
        synopsis: "string length string",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::at_least(2),
        detail: "Map substrings via key-value pairs.",
        synopsis: "string map ?-nocase? mapping string",
        pure: true,
        return_type: Some(TclType::String),
        options: &[OptionSpec {
            name: "-nocase",
            takes_value: false,
            value_hint: "",
            detail: "",
        }],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "match",
        arity: Arity::at_least(2),
        detail: "Test glob-style pattern match.",
        synopsis: "string match ?-nocase? pattern string",
        pure: true,
        return_type: Some(TclType::Boolean),
        options: &[OptionSpec {
            name: "-nocase",
            takes_value: false,
            value_hint: "",
            detail: "",
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "range",
        arity: Arity::exact(3),
        detail: "Return substring by index range.",
        synopsis: "string range string first last",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeat",
        arity: Arity::exact(2),
        detail: "Repeat string N times.",
        synopsis: "string repeat string count",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(3, 4),
        detail: "Replace range with new string.",
        synopsis: "string replace string first last ?newString?",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reverse",
        arity: Arity::exact(1),
        detail: "Reverse character order.",
        synopsis: "string reverse string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tolower",
        arity: Arity::new(1, 3),
        detail: "Convert to lower case.",
        synopsis: "string tolower string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "totitle",
        arity: Arity::new(1, 3),
        detail: "Convert to title case.",
        synopsis: "string totitle string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "toupper",
        arity: Arity::new(1, 3),
        detail: "Convert to upper case.",
        synopsis: "string toupper string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trim",
        arity: Arity::new(1, 2),
        detail: "Trim leading and trailing characters.",
        synopsis: "string trim string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trimleft",
        arity: Arity::new(1, 2),
        detail: "Trim leading characters.",
        synopsis: "string trimleft string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trimright",
        arity: Arity::new(1, 2),
        detail: "Trim trailing characters.",
        synopsis: "string trimright string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "wordend",
        arity: Arity::exact(2),
        detail: "Index of character after end of word.",
        synopsis: "string wordend string charIndex",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "wordstart",
        arity: Arity::exact(2),
        detail: "Index of first character of word.",
        synopsis: "string wordstart string charIndex",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `string`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "string",
        traits: Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Perform one of several string operations.",
            &["string option arg ?arg ...?"],
            "Tcl string(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
