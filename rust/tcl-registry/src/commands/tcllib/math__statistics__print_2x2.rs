//! `math::statistics::print-2x2` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::print-2x2 a b c d",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::print-2x2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Format a 2x2 contingency table.",
            synopsis: &["math::statistics::print-2x2 a b c d"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
