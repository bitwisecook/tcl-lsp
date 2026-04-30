//! `GENERICMESSAGE::route` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Adds, deletes, or looks up message routes.",
            &["GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
