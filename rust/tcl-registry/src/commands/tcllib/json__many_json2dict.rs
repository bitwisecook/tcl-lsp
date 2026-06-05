//! `json::many-json2dict` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::many-json2dict jsonText ?max?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::many-json2dict",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Convert a string containing multiple JSON values to a list of dicts.",
            synopsis: &["json::many-json2dict jsonText ?max?"],
            snippet: "",
            source: "tcllib json package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
