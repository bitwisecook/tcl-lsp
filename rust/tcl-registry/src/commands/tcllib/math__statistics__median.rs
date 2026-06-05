//! `math::statistics::median` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::median data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::median",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the median of a list of values.",
            synopsis: &["math::statistics::median data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "The median value.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
