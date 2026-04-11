//! `TCP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::collect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collects the specified amount of data for delivery to the iRule.",
            &["TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
