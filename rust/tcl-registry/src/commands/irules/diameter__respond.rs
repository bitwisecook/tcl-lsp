//! `DIAMETER::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Sends message to client or server (based on context).", &["DIAMETER::respond DIAMETER_VERSION RFLAG_BINARY PFLAG_BINARY EFLAG_BINARY TFLAG_BINARY"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
