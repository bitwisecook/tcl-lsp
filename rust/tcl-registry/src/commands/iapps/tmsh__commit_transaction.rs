//! `tmsh::commit_transaction` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::commit_transaction",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Runs all commands issued since the last ``tmsh::begin_transaction``.",
            &["tmsh::commit_transaction"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
