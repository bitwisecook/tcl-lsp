//! `onbreak` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "onbreak {command}",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "onbreak",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Define action when a breakpoint is hit.",
            &["onbreak {command}"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
