//! `iapp::is` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "iapp::is ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::is",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::is`.",
            &["iapp::is ?arg ...?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
