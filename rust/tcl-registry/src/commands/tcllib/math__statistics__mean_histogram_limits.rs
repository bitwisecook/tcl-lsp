//! `math::statistics::mean-histogram-limits` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::mean-histogram-limits mean stdev ?number?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::mean-histogram-limits",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute histogram limits from mean and stdev.",
            synopsis: &["math::statistics::mean-histogram-limits mean stdev ?number?"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
