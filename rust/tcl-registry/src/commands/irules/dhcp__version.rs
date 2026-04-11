//! `DHCP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns version number (4 or 6) of DHCP packet.",
            &["DHCP::version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
