//! `uniquify` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uniquify ?-force? ?-dont_skip_empty_designs?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uniquify",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Make each instance of a subdesign unique.",
            &["uniquify ?-force? ?-dont_skip_empty_designs?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
