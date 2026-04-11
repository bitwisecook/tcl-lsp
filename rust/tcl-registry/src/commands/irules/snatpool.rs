//! `snatpool` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snatpool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Assigns the specified SNAT pool or SNAT pool member to the current connection.",
            &["snatpool SNAT_POOL_OBJ (member IP_ADDR)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
