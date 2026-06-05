//! `iapp::substa` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "iapp::substa ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::substa",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::substa`.",
            &["iapp::substa ?arg ...?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
