//! `tmsh::log` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::log <message>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Logs the specified message.",
            &["tmsh::log <message>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
