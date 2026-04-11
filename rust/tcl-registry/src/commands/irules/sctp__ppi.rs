//! `SCTP::ppi` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::ppi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the SCTP payload protocol indicator.",
            &["SCTP::ppi (PPI_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
