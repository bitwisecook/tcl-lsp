//! `DIAMETER::realm` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::realm",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the value of the origin-realm or destination-realm AVP.",
            &["DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
