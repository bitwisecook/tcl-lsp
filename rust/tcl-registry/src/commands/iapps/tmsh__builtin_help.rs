//! `tmsh::builtin_help` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::builtin_help ?args?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::builtin_help",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Presents the same results as typing ``?`` while entering a tmsh command.",
            &["tmsh::builtin_help ?args?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
