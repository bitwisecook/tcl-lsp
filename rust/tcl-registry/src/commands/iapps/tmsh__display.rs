//! `tmsh::display` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::display <text>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::display",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Provides access to the tmsh pager.",
            &["tmsh::display <text>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
