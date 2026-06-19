//! `lsort` — sort a list.
use crate::prelude::*;

/// GAP-D1: `lsort`'s options were silently dropped in the Rust port (0
/// of 12). Restore the full option set so completion and option-aware
/// arity work; mirrors the DEFAULT-form options in `tcl/lsort_.py`.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-ascii",
        takes_value: false,
        value_hint: "",
        detail: "Compare as ASCII strings (the default).",
        dialects: None,
    },
    OptionSpec {
        name: "-dictionary",
        takes_value: false,
        value_hint: "",
        detail: "Compare using dictionary-style ordering (case-insensitive, embedded numbers).",
        dialects: None,
    },
    OptionSpec {
        name: "-integer",
        takes_value: false,
        value_hint: "",
        detail: "Compare elements as integers.",
        dialects: None,
    },
    OptionSpec {
        name: "-real",
        takes_value: false,
        value_hint: "",
        detail: "Compare elements as floating-point values.",
        dialects: None,
    },
    OptionSpec {
        name: "-nocase",
        takes_value: false,
        value_hint: "",
        detail: "Case-insensitive comparison.",
        dialects: None,
    },
    OptionSpec {
        name: "-increasing",
        takes_value: false,
        value_hint: "",
        detail: "Sort in increasing order (the default).",
        dialects: None,
    },
    OptionSpec {
        name: "-decreasing",
        takes_value: false,
        value_hint: "",
        detail: "Sort in decreasing order.",
        dialects: None,
    },
    OptionSpec {
        name: "-indices",
        takes_value: false,
        value_hint: "",
        detail: "Return the indices of the sorted elements rather than the elements.",
        dialects: None,
    },
    OptionSpec {
        name: "-unique",
        takes_value: false,
        value_hint: "",
        detail: "Retain only the last of a run of equal elements.",
        dialects: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "cmdPrefix",
        detail: "Use a custom comparison command prefix.",
        dialects: None,
    },
    OptionSpec {
        name: "-index",
        takes_value: true,
        value_hint: "index",
        detail: "Sort on a sub-element selected by index path.",
        dialects: None,
    },
    OptionSpec {
        name: "-stride",
        takes_value: true,
        value_hint: "length",
        detail: "Treat the list as groups of this many elements and sort by group.",
        dialects: None,
    },
];

/// GAP-D1: restore the single DEFAULT invocation form (`n_forms` 0 → 1).
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsort ?options? list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsort",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        options: OPTIONS,
        forms: FORMS,
        hover: Some(HoverSnippet::brief(
            "Sort the elements of a list.",
            &["lsort ?option ...? list"],
            "Tcl lsort(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
