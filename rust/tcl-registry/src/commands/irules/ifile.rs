//! `ifile` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ifile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns content and attributes from external files on the BIG-IP system.",
            &["ifile 'listall'"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
