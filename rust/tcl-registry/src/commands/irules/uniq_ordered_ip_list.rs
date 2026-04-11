//! `uniq_ordered_ip_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uniq_ordered_ip_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return a deduplicated list of valid IP addresses from the given arguments, prese",
            &["call uniq_ordered_ip_list $ip_string"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
