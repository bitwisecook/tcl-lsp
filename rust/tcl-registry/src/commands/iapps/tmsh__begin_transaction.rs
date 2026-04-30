//! `tmsh::begin_transaction` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::begin_transaction",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Begins an update transaction.",
            &["tmsh::begin_transaction"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
