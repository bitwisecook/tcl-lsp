//! `when` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Define conditional actions during simulation.",
            &["when {condition} {action} ?-label label?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
