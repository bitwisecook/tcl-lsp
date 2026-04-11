//! `ACCESS::oauth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::oauth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "OAuth related ACCESS iRule",
            &["ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
