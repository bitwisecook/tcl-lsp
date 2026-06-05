//! Regex quoting helper aliases.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for regexp.",
            &["regexp_quote string"],
            "Tcl",
        )),
        // GAP-D2: regex-escaped literal output; double-encode → T106.
        // Mirrors `tcl/regexp__quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        ..CommandSpec::DEFAULT
    }
}
