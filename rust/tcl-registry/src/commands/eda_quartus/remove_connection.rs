//! `remove_connection` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "remove_connection -from source -to destination",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_connection",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Remove a connection in an ECO change.",
            &["remove_connection -from source -to destination"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
