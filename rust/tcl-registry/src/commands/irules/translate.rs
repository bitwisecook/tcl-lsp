//! `translate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "translate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables, disables, or queries (as specified) destination address or port translation.",
            synopsis: &["translate (address | port | service)", "translate (address | port | service) ((enable | disable)"],
            snippet: "Enables, disables, or queries (as specified) destination address or\nport translation",
            source: "https://clouddocs.f5.com/api/irules/translate.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [IP::addr [IP::remote_addr] equals 10.0.8.0/24] } {\n        translate address disable\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
