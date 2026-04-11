//! `GTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends the GTP message back to the remote node of this connection.",
            &["GTP::respond MESSAGE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
