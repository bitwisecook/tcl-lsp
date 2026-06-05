//! `math::statistics::var` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::var data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::var",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the variance of a list of values.",
            synopsis: &["math::statistics::var data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "The variance.",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
