//! `tmsh::log_dest` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::log_dest ?destination?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log_dest",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies where the system sends events.",
            &["tmsh::log_dest ?destination?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
