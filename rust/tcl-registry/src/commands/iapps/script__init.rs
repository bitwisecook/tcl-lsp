//! `script::init` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "script::init",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::init",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Called before ``script::run``, ``script::help``, and ``script::tabc``.",
            &["script::init"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
