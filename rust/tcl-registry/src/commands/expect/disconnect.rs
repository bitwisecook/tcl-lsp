//! `disconnect` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "disconnect",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "disconnect",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Disconnect the process from the controlling terminal (daemonise).",
            &["disconnect"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
