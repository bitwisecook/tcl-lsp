//! `verify` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "verify ?-verbose?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run formal equivalence checking.",
            &["verify ?-verbose?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
