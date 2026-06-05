//! `FTP::allow_active_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::allow_active_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set the state of allow active mode.",
            synopsis: &["FTP::allow_active_mode (enable | disable)?"],
            snippet: "Enable or disable active transfer mode. Returns the current status if no option is specified.",
            source: "https://clouddocs.f5.com/api/irules/FTP__allow_active_mode.html",
            examples: "when CLIENT_ACCEPTED {\n                FTP::allow_active_mode disable\n            }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
