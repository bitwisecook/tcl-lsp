//! `math::statistics::histogram-alt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::histogram-alt limits values ?weights?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::histogram-alt",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute a histogram (alternate boundaries).",
            synopsis: &["math::statistics::histogram-alt limits values ?weights?"],
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
