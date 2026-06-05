//! Regex quoting helper aliases.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regexp::quote STRING",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        // GAP-D1(d): the command is `regexp::quote` (`::`, not `_`) —
        // the Rust port mis-spelled it `regexp_quote`. Mirrors
        // `tcl/regexp__quote.py`.
        name: "regexp::quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Escape regex metacharacters in a string.",
    synopsis: &["regexp::quote STRING", "regexp::quote string"],
    snippet: "Returns *STRING* with all regular-expression\nmetacharacters backslash-escaped so it can be\nused as a literal pattern in ``regexp`` or\n``regsub``.  Alias for ``regex::quote``.",
    source: "",
    examples: "",
    return_value: "Returns a regex-escaped string.",
}),
        // GAP-D2: regex-escaped literal output; double-encode → T106.
        // Mirrors `tcl/regexp__quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
