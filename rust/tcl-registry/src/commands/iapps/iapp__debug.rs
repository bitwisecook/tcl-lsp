//! `iapp::debug` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "iapp::debug ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::debug",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::debug`.",
            &["iapp::debug ?arg ...?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
