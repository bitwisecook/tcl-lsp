//! `xff_uniq_ordered_ip_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xff_uniq_ordered_ip_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Return a deduplicated list of valid non-loopback IP addresses from the X-Forward",
            &["call xff_uniq_ordered_ip_list"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
