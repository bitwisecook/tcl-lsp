//! `ASM::fingerprint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::fingerprint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the fingerprint (device id) of the client device.",
            &["ASM::fingerprint"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
