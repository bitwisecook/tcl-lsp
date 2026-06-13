//! `characterize` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "characterize ?-constraints? instance_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "characterize",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Characterize a subdesign for context-dependent optimization.",
            &["characterize ?-constraints? instance_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
