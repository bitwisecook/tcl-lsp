//! `md4` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "md4",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the RSA MD4 Message Digest Algorithm message digest of the specified str",
            &["md4"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
