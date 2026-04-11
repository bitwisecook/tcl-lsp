//! `GENERICMESSAGE::peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::peer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the peer's route name.",
            &["GENERICMESSAGE::peer name (NAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
