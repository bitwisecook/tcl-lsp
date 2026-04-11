//! `UDP::debug_queue` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::debug_queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to enable/disable printing debug messages when UDP::max",
            &["UDP::debug_queue BOOL_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
