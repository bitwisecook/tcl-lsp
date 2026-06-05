//! `DIAMETER::retransmission_default` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission_default",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets of sets the current connection's retransmission settings.",
            synopsis: &["DIAMETER::retransmission_default action"],
            snippet: "This command allows the setting or getting of the current\nconnection\\'s retransmission settings. All request messages on the\ncurrent connection will be initailized with the connection\\'s setings.\nThe messages\\'s settings may be changed with the\nDIAMETER::retransmission command.\n        \nGets the current connection\\'s retransmission action.\nPossible actions are:\n\n * \"disabled\" - request messages will not be queued for retransmission\n\n * \"busy\" - when retransmission is triggered for a request message an\n   answer message with a DIAMETER_TOO_BUSY result code will be\n   returned to the originator.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_default.html",
            examples: "when CLIENT_ACCEPTED {\n    DIAMETER::retransmission_default action busy\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
