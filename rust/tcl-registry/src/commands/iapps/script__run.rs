//! `script::run` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "script::run",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::run",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Invoked when the tmsh ``run cli script`` command is issued.",
            &["script::run"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
