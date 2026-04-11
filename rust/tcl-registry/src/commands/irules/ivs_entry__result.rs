//! `IVS_ENTRY::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IVS_ENTRY::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends a result code to the IVS client.",
            &["IVS_ENTRY::result (noop | modified | response)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
