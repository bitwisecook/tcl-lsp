//! `CACHE::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get/modify the content of an header related to an object stored in the RAM Cache",
            &["CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
