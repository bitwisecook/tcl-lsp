//! `regex::quote` — regex quoting helper alias (`::` spelling).
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regex::quote STRING",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex::quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Escape regex metacharacters in a string.",
    synopsis: &["regex::quote STRING", "regex::quote string"],
    snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.",
    source: "",
    examples: "set safe_pattern [regex::quote $user_input]\nif {[regexp $safe_pattern $haystack]} { ... }",
    return_value: "Returns a regex-escaped string.",
}),
        // GAP-D2: regex-escaped literal output; double-encode → T106.
        // Mirrors `tcl/regex__quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
