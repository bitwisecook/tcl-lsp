//! `CATEGORY::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get category of URL.", &["CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_cust"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
