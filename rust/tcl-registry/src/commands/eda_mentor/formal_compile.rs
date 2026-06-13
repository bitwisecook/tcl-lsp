//! `formal_compile` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "formal_compile ?-d design? ?-work library?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "formal_compile",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Compile design for formal verification.",
            &["formal_compile ?-d design? ?-work library?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
