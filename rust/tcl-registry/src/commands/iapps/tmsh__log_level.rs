//! `tmsh::log_level` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::log_level ?level?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log_level",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies the default severity level.",
            &["tmsh::log_level ?level?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
