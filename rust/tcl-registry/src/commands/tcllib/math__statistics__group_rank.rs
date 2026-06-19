//! `math::statistics::group-rank` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::group-rank args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::group-rank",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Rank grouped data.",
            synopsis: &["math::statistics::group-rank args"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
