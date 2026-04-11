//! `xff_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xff_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Return a sorted, deduplicated list of valid non-loopback IP addresses from the X",
            &["call xff_list"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
