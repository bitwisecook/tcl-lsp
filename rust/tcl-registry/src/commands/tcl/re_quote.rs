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
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["re_quote STRING", "re_quote string"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters backslash-escaped so it can be\nused as a literal pattern in ``regexp`` or\n``regsub``.  Alias for ``regex::quote``.",
            source: "",
            examples: "",
            return_value: "Returns a regex-escaped string.",
        }),
        // GAP-D2: output is a regex-escaped literal; re-quoting an
        // already-escaped value double-encodes it (T106).
        // Mirrors `tcl/re_quote.py`.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
