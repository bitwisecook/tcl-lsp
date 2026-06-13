//! `set_technology` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_technology ?technology?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_technology",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the target technology.",
            &["set_technology ?technology?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
