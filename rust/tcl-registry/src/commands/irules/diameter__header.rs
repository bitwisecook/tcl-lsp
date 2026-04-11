//! `DIAMETER::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the DIAMETER header fields.",
            &["DIAMETER::header <field> ?value?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
