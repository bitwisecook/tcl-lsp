//! `SIPALG::nonregister_subscriber_listener` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::nonregister_subscriber_listener",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the value of flag enabling creating an ephemeral listener for nonre",
            &["SIPALG::nonregister_subscriber_listener"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
