//! `math::statistics::t-test-mean` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::t-test-mean data est_mean est_stdev alpha",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::t-test-mean",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Perform a t-test on the mean.",
            synopsis: &["math::statistics::t-test-mean data est_mean est_stdev alpha"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
