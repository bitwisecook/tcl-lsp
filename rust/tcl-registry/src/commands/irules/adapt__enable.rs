//! `ADAPT::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables, disables or returns the enable state.",
            synopsis: &["ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?"],
            snippet: "The ADAPT::enable command enables, disables or returns the enable\nstate of the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__enable.html",
            examples: "when HTTP_REQUEST {\n     ADAPT::enable true\n     ADAPT::enable response false\n}",
            return_value: "Returns the current of modified enable state.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
