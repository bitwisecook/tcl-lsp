//! `get_name_info` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_name_info",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get detailed information about a named signal.",
            &["get_name_info -info info_type name_id"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
