//! `ILX::call` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::call",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Calls an ILX method.",
            &["ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
