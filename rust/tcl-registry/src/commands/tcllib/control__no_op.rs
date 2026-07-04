//! `control::no-op` command.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "control::no-op ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "control::no-op",
        dialects: None,
        arity: Arity::any(),
        traits: Traits::PURE,
        hover: Some(HoverSnippet {
            summary: "Take any number of arguments and do nothing.",
            synopsis: &["control::no-op ?arg arg ...?"],
            snippet: "Accepts any number of arguments, ignores them, and returns an empty string.",
            source: "tcllib control package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("control"),
        required_package: Some("control"),
        ..CommandSpec::DEFAULT
    }
}
