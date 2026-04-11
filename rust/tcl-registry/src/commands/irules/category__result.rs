//! `CATEGORY::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Returns the category or safesearch results retrieved during normal traffic flow.", &["CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
