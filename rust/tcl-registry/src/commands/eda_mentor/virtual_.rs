//! `virtual` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "virtual",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create virtual signals or regions.",
            &["virtual ?-install | -env env? ?signal | function? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
