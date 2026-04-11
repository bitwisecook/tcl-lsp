//! `tmsh::get_type` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_type",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Returns the type identifier associated with the object.",
            &["tmsh::get_type <object>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
