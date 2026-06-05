//! `math::statistics::basic-stats` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::basic-stats data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::basic-stats",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute basic statistics (mean, min, max, count, stdev).",
            synopsis: &["math::statistics::basic-stats data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "A list: mean min max count stdev variance.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
