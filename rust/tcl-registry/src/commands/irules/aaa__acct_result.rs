//! `AAA::acct_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::acct_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to check whether the accounting information is sent success",
            &["AAA::acct_result AAA_REQUEST_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
