//! `PEM::subscriber` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::subscriber",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to create, delete or retreive information of a PEM subsc",
            &["PEM::subscriber config policy ( (get SUBS_ID (PEM_SUBS_TYPE2))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
