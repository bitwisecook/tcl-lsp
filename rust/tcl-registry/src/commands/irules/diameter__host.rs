//! `DIAMETER::host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::host",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the value of the origin-host or destination-host AVP.",
            &["DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
