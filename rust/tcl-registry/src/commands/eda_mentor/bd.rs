//! `bd` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "bd ?breakpoint_id | -all?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bd",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Delete breakpoints.",
            &["bd ?breakpoint_id | -all?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
