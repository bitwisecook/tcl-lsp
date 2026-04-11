//! `HTTP::has_responded` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::has_responded",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns true if this HTTP transaction has been prematurely completed by an iRule",
            &["HTTP::has_responded"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
