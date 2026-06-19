//! `json::validate` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::validate jsonText",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::validate",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet {
            summary: "Validate whether a string is valid JSON.",
            synopsis: &["json::validate jsonText"],
            snippet: "",
            source: "tcllib json package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("json"),
        required_package: Some("json"),
        ..CommandSpec::DEFAULT
    }
}
