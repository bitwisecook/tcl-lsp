//! `DHCPv6::option` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command retrieves, sets or deletes the option by id.",
            synopsis: &["DHCPv6::option (delete)? OPTION (VALUE)?"],
            snippet: "This command retrieves, sets or deletes the option by id\n\nDetails (syntax);\nDHCPv6::option <id>\nDHCPv6::option <id> <value>\nDHCPv6::option delete <id>",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__option.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Option [DHCPv6::option 18]\"\n    }",
            return_value: "when retrieving, this command returns the value of the option via option id",
        }),
        ..CommandSpec::DEFAULT
    }
}
