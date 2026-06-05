//! `TCP::pacing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::pacing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles TCP rate pacing.",
            synopsis: &["TCP::pacing (BOOL_VALUE)?"],
            snippet: "Rate pacing limits the data send rate to the physical limitations of the interface to reduce the chance of queue drops.",
            source: "https://clouddocs.f5.com/api/irules/TCP__pacing.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side rate pacing to enabled.\n    clientside {\n        log local0. \"Client: pacing [TCP::pacing], enabling\"\n        TCP::pacing enable\n    }\n    # Set server-side rate pacing to disabled.\n    serverside {\n        log local0. \"Server: pacing [TCP::pacing], disabling\"\n        TCP::pacing disable\n    }\n}",
            return_value: "TCP::pacing returns whether TCP rate pacing is enabled.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::pacing (BOOL_VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
