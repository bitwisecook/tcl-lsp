//! `call` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "call",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Calls an iRule procedure.",
            &["call ?-debug? <proc_name> ?arg ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
