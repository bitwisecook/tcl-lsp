//! `DIAMETER::avp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides detailed access to diameter attribute-value pairs.",
            &["DIAMETER::avp <subcommand> ?args?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
