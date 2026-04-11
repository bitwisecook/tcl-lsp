//! `tmsh::cancel_transaction` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::cancel_transaction",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Cancels all commands issued since the last ``tmsh::begin_transaction``.",
            &["tmsh::cancel_transaction"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
