//! `HSL::open` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::open",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Channel),
        hover: Some(HoverSnippet::brief(
            "Opens a handle for High Speed Logging communication.",
            &["HSL::open ('-publisher' | '-pub') PUBLISHER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
