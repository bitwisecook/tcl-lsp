//! `math::statistics::crosscorr` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::crosscorr data1 data2",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::crosscorr",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the cross-correlation.",
            synopsis: &["math::statistics::crosscorr data1 data2"],
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
