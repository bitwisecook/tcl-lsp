//! `tmsh::add_help` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::add_help <help_data>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::add_help",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Displays context-sensitive help when the user types ``?``.",
            &["tmsh::add_help <help_data>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
