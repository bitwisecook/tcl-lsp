//! `CATEGORY::filetype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::filetype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get mime type and mime subtype of payload.",
            &["CATEGORY::filetype HTTP_PAYLOAD"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
