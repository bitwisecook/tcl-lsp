//! Regex quoting helper aliases.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regex_quote STRING",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for regex.",
            &["regex_quote string"],
            "Tcl",
        )),
        // GAP-D2: regex-escaped literal output; double-encode → T106.
        // Mirrors `tcl/regex_quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
