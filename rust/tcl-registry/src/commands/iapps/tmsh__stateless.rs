//! `tmsh::stateless` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::stateless",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Modifies the behaviour of ``tmsh::create`` and ``tmsh::delete``.",
            &["tmsh::stateless ?enabled?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
