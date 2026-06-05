//! `math::statistics::control-xbar` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::control-xbar data ?nsamples?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::control-xbar",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Compute X-bar control chart limits.",
            synopsis: &["math::statistics::control-xbar data ?nsamples?"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
