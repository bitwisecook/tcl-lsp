//! `tmsh::get_status` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_status",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Returns a list of config item statuses as Tcl objects.",
            &["tmsh::get_status <component> ?name? ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
