//! `server_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "server_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP address of the server.",
            synopsis: &["server_addr"],
            snippet: "Returns the IP address of the server. This is a BIG-IP version 4.X\nvariable, provided for backward compatibility. You can use the\nequivalent 9.X command IP::server_addr instead.",
            source: "https://clouddocs.f5.com/api/irules/server_addr.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
