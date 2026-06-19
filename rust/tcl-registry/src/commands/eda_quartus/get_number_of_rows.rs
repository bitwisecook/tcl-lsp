//! `get_number_of_rows` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_number_of_rows -name panel_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_number_of_rows",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the number of rows in a report panel.",
            &["get_number_of_rows -name panel_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
