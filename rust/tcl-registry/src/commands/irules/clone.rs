//! `clone` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clone",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the system to clone traffic to the specified pool, pool member or vlan re",
            &["clone pool POOL_OBJ (member IP_ADDR (PORT)?)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
