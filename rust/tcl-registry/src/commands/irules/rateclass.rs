//! `rateclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rateclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Selects the specified rate class to use when transmitting packets.",
            synopsis: &["rateclass RATE_CLASS"],
            snippet: "Causes the system to select the specified rate class to use when\ntransmitting packets.",
            source: "https://clouddocs.f5.com/api/irules/rateclass.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::addr [IP::client_addr] equals xxx.xxx.xxx.xxx] } {\n    log local0. \"[IP::client_addr] being handled by rateclass class1\"\n    rateclass class1\n  }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
