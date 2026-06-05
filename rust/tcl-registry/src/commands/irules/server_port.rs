//! `server_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "server_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the TCP port/service number of the specified server.",
            synopsis: &["server_port"],
            snippet: "Returns the TCP port/service number of the specified server. This is a\nBIG-IP version 4.X variable, provided for backward compatibility. You\ncan use the equivalent 9.X command TCP::server_port instead.",
            source: "https://clouddocs.f5.com/api/irules/server_port.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
