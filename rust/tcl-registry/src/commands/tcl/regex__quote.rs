//! `regex::quote` — regex quoting helper alias (`::` spelling).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex::quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Escape regex metacharacters in a string.",
            &["regex::quote string"],
            "Tcl",
        )),
        // GAP-D2: regex-escaped literal output; double-encode → T106.
        // Mirrors `tcl/regex__quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        ..CommandSpec::DEFAULT
    }
}
