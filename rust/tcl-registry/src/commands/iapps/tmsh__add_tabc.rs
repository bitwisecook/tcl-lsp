//! `tmsh::add_tabc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::add_tabc <tabc_data>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::add_tabc",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Adds tab completion datasets.",
            &["tmsh::add_tabc <tabc_data>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
