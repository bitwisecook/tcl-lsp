//! `lsearch` — search a list for a pattern.
use crate::prelude::*;

/// Option table for `lsearch`.  Most flags exist since Tcl 8.4;
/// `-stride` was added in 8.6 (TIP 351) and is dialect-gated so
/// W004 fires on `lsearch -stride` in pre-8.6 dialects.
static OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        takes_value: false,
        value_hint: "",
        detail: "Return list of all matching indices.",
        dialects: None,
    },
    OptionSpec {
        name: "-ascii",
        takes_value: false,
        value_hint: "",
        detail: "ASCII string comparison.",
        dialects: None,
    },
    OptionSpec {
        name: "-bisect",
        takes_value: false,
        value_hint: "",
        detail: "Binary search a sorted list (implies -sorted).",
        dialects: None,
    },
    OptionSpec {
        name: "-decreasing",
        takes_value: false,
        value_hint: "",
        detail: "List is sorted in decreasing order (with -sorted).",
        dialects: None,
    },
    OptionSpec {
        name: "-dictionary",
        takes_value: false,
        value_hint: "",
        detail: "Dictionary-order comparison.",
        dialects: None,
    },
    OptionSpec {
        name: "-exact",
        takes_value: false,
        value_hint: "",
        detail: "Exact equality match.",
        dialects: None,
    },
    OptionSpec {
        name: "-glob",
        takes_value: false,
        value_hint: "",
        detail: "Glob-pattern match (default).",
        dialects: None,
    },
    OptionSpec {
        name: "-increasing",
        takes_value: false,
        value_hint: "",
        detail: "List is sorted in increasing order (with -sorted).",
        dialects: None,
    },
    OptionSpec {
        name: "-index",
        takes_value: true,
        value_hint: "indexList",
        detail: "Compare against nested sub-element at indexList.",
        dialects: None,
    },
    OptionSpec {
        name: "-inline",
        takes_value: false,
        value_hint: "",
        detail: "Return matching values instead of indices.",
        dialects: None,
    },
    OptionSpec {
        name: "-integer",
        takes_value: false,
        value_hint: "",
        detail: "Integer comparison.",
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
        name: "-not",
        takes_value: false,
        value_hint: "",
        detail: "Invert the sense of the match.",
        dialects: None,
    },
    OptionSpec {
        name: "-real",
        takes_value: false,
        value_hint: "",
        detail: "Floating-point comparison.",
        dialects: None,
    },
    OptionSpec {
        name: "-regexp",
        takes_value: false,
        value_hint: "",
        detail: "Regular-expression match.",
        dialects: None,
    },
    OptionSpec {
        name: "-sorted",
        takes_value: false,
        value_hint: "",
        detail: "List is sorted; use binary search.",
        dialects: None,
    },
    OptionSpec {
        name: "-start",
        takes_value: true,
        value_hint: "index",
        detail: "Start the search at the given index.",
        dialects: None,
    },
    // `lsearch -stride` is Tcl 8.6+ (TIP 351).
    OptionSpec {
        name: "-stride",
        takes_value: true,
        value_hint: "strideLength",
        detail: "Treat the list as a sequence of fixed-size records.",
        dialects: Some(DialectSet::TCL86_PLUS),
    },
    OptionSpec {
        name: "-subindices",
        takes_value: false,
        value_hint: "",
        detail: "Combine result with -index value for nested addressing.",
        dialects: None,
    },
    // NOTE: `lsearch` does NOT declare `--` in its option table.
    // The convention matches the Python registry (`lsearch_.py`)
    // and keeps W304 (missing-option-terminator) silent for
    // `lsearch -exact $x pattern` — the existing
    // `analyse_no_w304_for_lsearch` regression test depends on this.
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsearch",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Int),
        options: OPTIONS,
        hover: Some(HoverSnippet::brief(
            "Search a list for a pattern.",
            &["lsearch ?option ...? list pattern"],
            "Tcl lsearch(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
