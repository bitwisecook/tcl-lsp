//! `ANTIFRAUD::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables the anti-fraud plugin.",
            synopsis: &["ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?"],
            snippet: "Enables the anti-fraud plugin.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable.html",
            examples: "when HTTP_REQUEST {\n                # apply default anti-fraud profile on the transaction with Antifraud-Foo HTTP header\n                if { [HTTP::header exists \"Antifraud-Foo\" ] } {\n                    ANTIFRAUD::enable\n                }\n                # apply /Common/antifraud_bar profile on the transaction with Antifraud-Bar HTTP header\n                if { [HTTP::header exists \"Antifraud-Bar\" ] } {\n                    ANTIFRAUD::enable /Common/antifraud_bar\n                }\n            }",
            return_value: "ANTIFRAUD::enable Applies the default anti-fraud profile attached to the virtual server.",
        }),
        ..CommandSpec::DEFAULT
    }
}
