//! `math::statistics::test-Rchart` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Rchart control data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Rchart",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Test data against R-chart control limits.",
            synopsis: &["math::statistics::test-Rchart control data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
