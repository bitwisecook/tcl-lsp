//! `regsub` helper aliases and regex quoting commands.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "re_quote STRING",
}];

/// Command spec for Tcl `re_quote` (regex quoting helper).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "re_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for use as a regex literal.",
            &["re_quote string"],
            "Tcl",
        )),
        // GAP-D2: output is a regex-escaped literal; re-quoting an
        // already-escaped value double-encodes it (T106).
        // Mirrors `tcl/re_quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
