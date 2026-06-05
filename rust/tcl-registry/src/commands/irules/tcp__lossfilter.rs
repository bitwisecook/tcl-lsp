//! `TCP::lossfilter` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilter",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the TCP Loss Ignore Parameters.",
            synopsis: &["TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST"],
            snippet: "Sets the maximum size burst loss (in packets) and maximum number of packets per million lost before triggering congestion response.\n  * Burst range is valid from 0 to 32. Higher values decrease the\n    chance of performing congestion control.\n  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n    million before congestion control kicks in.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilter.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side loss filter.\n    # Ignore up to 150 losses per million packets and burst losses of up to 10 packets.\n    clientside {\n        TCP::lossfilter 150 10\n    }\n    # No loss filter on server-side.\n    serverside {\n        TCP::lossfilter 0 0\n    }\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST" },
        ],
        ..CommandSpec::DEFAULT
    }
}
