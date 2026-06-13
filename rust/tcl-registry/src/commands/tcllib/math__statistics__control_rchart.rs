//! `math::statistics::control-Rchart` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::control-Rchart data ?nsamples?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::control-Rchart",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Compute R-chart control limits.",
            synopsis: &["math::statistics::control-Rchart data ?nsamples?"],
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
