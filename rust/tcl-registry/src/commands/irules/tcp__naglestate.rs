//! `TCP::naglestate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::naglestate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns current state of Nagle algorithm.",
            synopsis: &["TCP::naglestate"],
            snippet: "If the Nagle mode is \"enabled\" or \"disabled\", it returns that mode. If \"auto\", it returns the current selection of the autotuning.",
            source: "https://clouddocs.f5.com/api/irules/TCP__naglestate.html",
            examples: "# Get the TCP Nagle state of the TCP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"TCP Nagle state: [TCP::naglestate]\"\n}",
            return_value: "The string \"disabled\" or \"enabled\"",
        }),
        ..CommandSpec::DEFAULT
    }
}
