//! `math::statistics::interval-mean-stdev` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::interval-mean-stdev data confidence",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::interval-mean-stdev",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Compute confidence interval for mean and stdev.",
            synopsis: &["math::statistics::interval-mean-stdev data confidence"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
