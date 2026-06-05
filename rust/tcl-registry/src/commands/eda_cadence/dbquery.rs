//! `dbQuery` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dbQuery ?-area box? ?-objType type?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dbQuery",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Query design database objects (legacy).",
            &["dbQuery ?-area box? ?-objType type?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
