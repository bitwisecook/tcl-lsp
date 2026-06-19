//! `script::help` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "script::help",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::help",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Scripts can provide the ``script::help`` procedure for help text.",
            &["script::help"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
