//! `DNS::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::header",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets (v11.0+) or sets (v11.1+) simple bits or byte fields.",
            &["DNS::header <field> ?value?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
