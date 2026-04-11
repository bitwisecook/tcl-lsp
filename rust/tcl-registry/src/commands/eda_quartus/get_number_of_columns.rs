//! `get_number_of_columns` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_number_of_columns",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the number of columns in a report panel.",
            &["get_number_of_columns -name panel_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
